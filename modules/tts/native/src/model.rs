//! The model of the voices (Supertonic 3, OpenRAIL-M licence): downloaded once from the releases of
//! sherpa-onnx, checked against its hash, then unpacked with the deep voice added.

use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::Duration,
};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::voices::Styles;

const URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2";
const ARCHIVE_FOLDER: &str = "sherpa-onnx-supertonic-3-tts-int8-2026-05-11";
const BYTES: u64 = 128_774_318;
const SHA256: &str = "82fa96f91c4ef8abaae3a14a3f4153facf88bed821d1f7331cec2700f432c427";
/// What the module keeps of the archive.
pub const FILES: [&str; 8] = [
    "duration_predictor.int8.onnx",
    "text_encoder.int8.onnx",
    "vector_estimator.int8.onnx",
    "vocoder.int8.onnx",
    "tts.json",
    "unicode_indexer.bin",
    "voice.bin",
    "LICENSE",
];
/// The styles with the deep voice, written last: while it is missing, the model is not there.
pub const VOICES: &str = "voices.bin";
const TIMEOUT: Duration = Duration::from_secs(20);
const BUFFER: usize = 1 << 16;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Missing,
    Downloading,
    Ready,
}

#[derive(Serialize)]
pub struct Status {
    pub state: State,
    pub percent: u8,
    pub failed: bool,
    pub bytes: u64,
}

struct Source {
    url: String,
    bytes: u64,
    sha256: String,
}

pub struct Model {
    folder: PathBuf,
    source: Source,
    cancelled: AtomicBool,
    loading: AtomicBool,
    percent: AtomicU8,
    failed: AtomicBool,
}

fn hex_of(hash: Sha256) -> String {
    hash.finalize()
        .iter()
        .fold(String::new(), |text, byte| format!("{text}{byte:02x}"))
}

/// Only the start of a download has a time limit: the rest takes as long as the connection needs.
fn download_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(TIMEOUT))
        .timeout_recv_response(Some(TIMEOUT))
        .user_agent(concat!("Arthex TTS/", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Takes from the archive only the files the module keeps, by their exact place, so that no entry
/// can write elsewhere.
fn unpack(archive: &Path, folder: &Path) -> io::Result<()> {
    let mut entries = tar::Archive::new(bzip2::read::BzDecoder::new(BufReader::new(File::open(
        archive,
    )?)));
    for entry in entries.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.into_owned();
        let mut parts = path.components();
        let (Some(Component::Normal(top)), Some(Component::Normal(name)), None) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if top != ARCHIVE_FOLDER {
            continue;
        }
        if let Some(name) = name.to_str().filter(|name| FILES.contains(name)) {
            let mut output = File::create(folder.join(name))?;
            io::copy(&mut entry, &mut output)?;
        }
    }
    if let Some(missing) = FILES.iter().find(|name| !folder.join(name).is_file()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{missing} is missing"),
        ));
    }
    let styles = Styles::parse(&fs::read(folder.join("voice.bin"))?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let part = folder.join(format!("{VOICES}.part"));
    fs::write(&part, styles.with_deep_voice().to_bytes())?;
    fs::rename(&part, folder.join(VOICES))
}

impl Model {
    pub fn new(folder: &Path) -> Self {
        Self::with_source(
            folder,
            Source {
                url: URL.to_string(),
                bytes: BYTES,
                sha256: SHA256.to_string(),
            },
        )
    }

    fn with_source(folder: &Path, source: Source) -> Self {
        Self {
            folder: folder.to_path_buf(),
            source,
            cancelled: AtomicBool::new(false),
            loading: AtomicBool::new(false),
            percent: AtomicU8::new(0),
            failed: AtomicBool::new(false),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    fn archive(&self) -> PathBuf {
        self.folder.join("model.tar.bz2")
    }

    fn state(&self) -> State {
        if self.loading.load(Ordering::Relaxed) {
            State::Downloading
        } else if self.folder.join(VOICES).is_file() {
            State::Ready
        } else {
            State::Missing
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state() == State::Ready
    }

    /// Says once that a download failed, for the message it deserves.
    pub fn status(&self) -> Status {
        Status {
            state: self.state(),
            percent: self.percent.load(Ordering::Relaxed),
            failed: self.failed.swap(false, Ordering::Relaxed),
            bytes: self.source.bytes,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn download(self: &Arc<Self>) {
        if self.is_ready() || self.loading.swap(true, Ordering::Relaxed) {
            return;
        }
        self.cancelled.store(false, Ordering::Relaxed);
        self.failed.store(false, Ordering::Relaxed);
        self.percent.store(0, Ordering::Relaxed);
        let model = Arc::clone(self);
        thread::spawn(move || {
            let result = model.fetch().and_then(|()| {
                if model.cancelled.load(Ordering::Relaxed) {
                    return Err(io::Error::from(io::ErrorKind::Interrupted));
                }
                unpack(&model.archive(), &model.folder)
            });
            if model.cancelled.load(Ordering::Relaxed) {
                let _ = fs::remove_dir_all(&model.folder);
            } else if result.is_err() {
                for name in FILES.iter().chain([&VOICES]) {
                    let _ = fs::remove_file(model.folder.join(name));
                }
                model.failed.store(true, Ordering::Relaxed);
            }
            let _ = fs::remove_file(model.archive());
            model.loading.store(false, Ordering::Relaxed);
        });
    }

    /// Goes on from what is already there, as a cut download of this size must not start over.
    fn fetch(&self) -> io::Result<()> {
        fs::create_dir_all(&self.folder)?;
        let part = self.folder.join("model.tar.bz2.part");
        let mut hash = Sha256::new();
        let mut have = 0;
        if let Ok(mut existing) = File::open(&part) {
            have = io::copy(&mut existing, &mut hash)?;
        }
        let response = download_agent()
            .get(&self.source.url)
            .header("Range", &format!("bytes={have}-"))
            .header("Accept-Encoding", "identity")
            .call()
            .map_err(io::Error::other)?;
        let status = response.status().as_u16();
        // At 416, all of it is already here: only the check that ends a download is left.
        if status != 416 {
            if !matches!(status, 200 | 206) {
                return Err(io::Error::other(format!("status {status}")));
            }
            if status == 200 {
                // The server sends it all again: what is kept would be counted twice.
                have = 0;
                hash = Sha256::new();
            }
            let mut output = File::options()
                .create(true)
                .append(have > 0)
                .write(have == 0)
                .truncate(have == 0)
                .open(&part)?;
            let mut reader = response.into_body().into_reader();
            let mut buffer = vec![0; BUFFER];
            let mut done = have;
            loop {
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(io::Error::from(io::ErrorKind::Interrupted));
                }
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                output.write_all(&buffer[..read])?;
                hash.update(&buffer[..read]);
                done += read as u64;
                let percent = u8::try_from(done * 100 / self.source.bytes.max(1))
                    .unwrap_or(99)
                    .min(99);
                self.percent.store(percent, Ordering::Relaxed);
            }
        }
        if hex_of(hash) != self.source.sha256 {
            fs::remove_file(&part)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "altered download",
            ));
        }
        fs::rename(&part, self.archive())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::BufRead, net::TcpListener};

    /// A model archive whose files hold their own names, with a real `voice.bin` of ten voices.
    fn archive(extra: Option<&str>) -> Vec<u8> {
        let mut voices: Vec<u8> = [10_i64, 1, 2, 10, 1, 2]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        for _ in 0..40 {
            voices.extend(0.5_f32.to_le_bytes());
        }
        let mut builder = tar::Builder::new(Vec::new());
        let mut add = |path: &str, content: &[u8]| {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, content).unwrap();
        };
        for name in FILES {
            let content = if name == "voice.bin" {
                voices.clone()
            } else {
                name.as_bytes().to_vec()
            };
            add(&format!("{ARCHIVE_FOLDER}/{name}"), &content);
        }
        add(&format!("{ARCHIVE_FOLDER}/README.md"), b"readme");
        if let Some(path) = extra {
            add(path, b"elsewhere");
        }
        let tar = builder.into_inner().unwrap();
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast());
        encoder.write_all(&tar).unwrap();
        encoder.finish().unwrap()
    }

    fn source_of(content: &[u8], url: String) -> Source {
        let mut hash = Sha256::new();
        hash.update(content);
        Source {
            url,
            bytes: content.len() as u64,
            sha256: hex_of(hash),
        }
    }

    /// Serves `content` once, whatever is asked.
    fn serve(content: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/model.tar.bz2", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = io::BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                content.len()
            )
            .unwrap();
            stream.write_all(&content).unwrap();
        });
        address
    }

    fn wait(model: &Model) {
        while model.loading.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn downloads_checks_and_unpacks_with_the_deep_voice() {
        let folder = tempfile::tempdir().unwrap();
        let content = archive(None);
        let source = source_of(&content, serve(content.clone()));
        let model = Arc::new(Model::with_source(folder.path(), source));
        assert_eq!(model.status().state, State::Missing);
        model.download();
        wait(&model);
        assert!(model.is_ready());
        assert_eq!(
            fs::read(folder.path().join("tts.json")).unwrap(),
            b"tts.json"
        );
        assert!(!folder.path().join("README.md").exists());
        assert!(!model.archive().exists());
        let voices = fs::read(folder.path().join(VOICES)).unwrap();
        assert_eq!(i64::from_le_bytes(voices[..8].try_into().unwrap()), 11);
        // The server answered once: asking again must not reach it.
        model.download();
        wait(&model);
        assert!(model.is_ready());
        assert!(!model.status().failed);
    }

    #[test]
    fn refuses_an_altered_download() {
        let folder = tempfile::tempdir().unwrap();
        let content = archive(None);
        let mut source = source_of(&content, serve(content.clone()));
        source.sha256 = "0".repeat(64);
        let model = Arc::new(Model::with_source(folder.path(), source));
        model.download();
        wait(&model);
        let status = model.status();
        assert_eq!(status.state, State::Missing);
        assert!(status.failed);
        assert!(!model.status().failed);
    }

    #[test]
    fn writes_nothing_outside_its_folder() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("archive.tar.bz2");
        fs::write(&path, archive(Some("elsewhere.txt"))).unwrap();
        let target = folder.path().join("model");
        fs::create_dir(&target).unwrap();
        unpack(&path, &target).unwrap();
        assert!(!folder.path().join("elsewhere.txt").exists());
        assert!(!target.join("elsewhere.txt").exists());
    }
}
