//! The word game: how close in meaning a word is to a secret one, read from a large file of word
//! vectors that is only downloaded when the user turns the game on.
//!
//! The file holds `FLR1`, the number of words and of dimensions (`u32`, little-endian), then an
//! offset per word into the words, the words sorted as bytes, and a vector of `i8` per word. A word
//! is found by bisection on the disk, so nothing of it stays in memory.

use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::Duration,
};

use serde::Serialize;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 4] = b"FLR1";
const HEADER: u64 = 12;
const BASE_URL: &str = "https://github.com/arthreto/arthex-modules/releases/download/flair-data-1/";
const FRENCH_SECRETS: &str = include_str!("../secrets-fr.txt");
const ENGLISH_SECRETS: &str = include_str!("../secrets-en.txt");
const BUFFER: usize = 1 << 20;
const MAX_WORD: usize = 64;
const TIMEOUT: Duration = Duration::from_secs(30);
const RECENT_KEPT: usize = 50;

/// A published file of words, checked by its hash before any use.
#[derive(Clone, Copy)]
pub struct Published {
    pub file: &'static str,
    pub bytes: u64,
    pub words: u32,
    pub sha256: &'static str,
}

const FRENCH: Published = Published {
    file: "fr.bin",
    bytes: 408_413_808,
    words: 1_300_644,
    sha256: "316e93ba8200ba054f891cf707c945b493b42d0df1e09b0d9d0cfc6e2e760cd1",
};
const ENGLISH: Published = Published {
    file: "en.bin",
    bytes: 378_526_859,
    words: 1_209_381,
    sha256: "8f52f1a9bb731a67b812e13a36da5760122a0d0f5915f41b650ef302dc4e403c",
};

/// Reads words and their vectors, from the disk.
pub struct Vectors {
    file: File,
    count: u32,
    dimensions: u32,
    words_start: u64,
    data_start: u64,
}

impl Vectors {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut header = [0; 12];
        file.read_exact(&mut header)?;
        let number = |at: usize| {
            u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
        };
        if &header[..4] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a file of words",
            ));
        }
        let (count, dimensions) = (number(4), number(8));
        let words_start = HEADER + (u64::from(count) + 1) * 4;
        file.seek(SeekFrom::Start(HEADER + u64::from(count) * 4))?;
        let mut end = [0; 4];
        file.read_exact(&mut end)?;
        Ok(Self {
            file,
            count,
            dimensions,
            words_start,
            data_start: words_start + u64::from(u32::from_le_bytes(end)),
        })
    }

    fn word(&mut self, index: u32) -> io::Result<Vec<u8>> {
        self.file
            .seek(SeekFrom::Start(HEADER + u64::from(index) * 4))?;
        let mut offsets = [0; 8];
        self.file.read_exact(&mut offsets)?;
        let from = u32::from_le_bytes([offsets[0], offsets[1], offsets[2], offsets[3]]);
        let to = u32::from_le_bytes([offsets[4], offsets[5], offsets[6], offsets[7]]);
        self.file
            .seek(SeekFrom::Start(self.words_start + u64::from(from)))?;
        let mut word = vec![0; (to - from) as usize];
        self.file.read_exact(&mut word)?;
        Ok(word)
    }

    pub fn find(&mut self, word: &str) -> io::Result<Option<Vec<f32>>> {
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let middle = low + (high - low) / 2;
            match self.word(middle)?.as_slice().cmp(word.as_bytes()) {
                std::cmp::Ordering::Equal => return self.vector(middle).map(Some),
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        Ok(None)
    }

    fn vector(&mut self, index: u32) -> io::Result<Vec<f32>> {
        let at = self.data_start + u64::from(index) * u64::from(self.dimensions);
        self.file.seek(SeekFrom::Start(at))?;
        let mut bytes = vec![0; self.dimensions as usize];
        self.file.read_exact(&mut bytes)?;
        Ok(bytes
            .into_iter()
            .map(|byte| f32::from(byte.cast_signed()))
            .collect())
    }
}

/// How close two vectors are, from `-100` to `100`, shown as degrees.
pub fn temperature(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let size = |vector: &[f32]| vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    let divisor = size(a) * size(b);
    if divisor == 0.0 {
        return 0.0;
    }
    (dot / divisor * 1000.0).round() / 10.0
}

pub fn normalize(word: &str) -> String {
    word.trim().to_lowercase()
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "result")]
pub enum Guess {
    Unknown,
    Close { word: String, temperature: f32 },
    Found { word: String, tries: u32 },
}

/// A round: the secret word stays here, out of reach of the interface until it is over.
pub struct Round {
    vectors: Vectors,
    secret: String,
    secret_vector: Vec<f32>,
    tries: u32,
}

impl Round {
    pub fn start(mut vectors: Vectors, secret: &str) -> io::Result<Option<Self>> {
        let Some(secret_vector) = vectors.find(secret)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            vectors,
            secret: secret.to_string(),
            secret_vector,
            tries: 0,
        }))
    }

    pub fn guess(&mut self, word: &str) -> io::Result<Guess> {
        let word = normalize(word);
        if word.is_empty() || word.len() > MAX_WORD {
            return Ok(Guess::Unknown);
        }
        let Some(vector) = self.vectors.find(&word)? else {
            return Ok(Guess::Unknown);
        };
        self.tries += 1;
        if word == self.secret {
            return Ok(Guess::Found {
                word,
                tries: self.tries,
            });
        }
        Ok(Guess::Close {
            temperature: temperature(&self.secret_vector, &vector),
            word,
        })
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }
}

/// Picks a secret word, avoiding the last ones played.
pub fn pick_secret<'a>(words: &[&'a str], seed: u64, recent: &[String]) -> Option<&'a str> {
    if words.is_empty() {
        return None;
    }
    let count = words.len() as u64;
    (0..count)
        .map(|step| words[usize::try_from(seed.wrapping_add(step) % count).unwrap_or(0)])
        .find(|word| !recent.iter().any(|played| played == word))
        .or_else(|| words.first().copied())
}

fn secrets(french: bool) -> Vec<&'static str> {
    (if french {
        FRENCH_SECRETS
    } else {
        ENGLISH_SECRETS
    })
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Off,
    Loading,
    Ready,
}

/// What the interface shows of the game.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    pub percent: u8,
    pub failed: bool,
    pub bytes: u64,
    pub words: u32,
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
        .user_agent(concat!("Arthex Companion/", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Where the words are, and the download.
pub struct Library {
    folder: PathBuf,
    source: Published,
    base_url: String,
    french: bool,
    cancelled: AtomicBool,
    loading: AtomicBool,
    percent: AtomicU8,
    failed: AtomicBool,
    recent: Mutex<Vec<String>>,
}

impl Library {
    pub fn new(folder: &Path, locale: &str) -> Self {
        let french = locale.starts_with("fr");
        Self::with_source(
            folder,
            if french { FRENCH } else { ENGLISH },
            BASE_URL,
            french,
        )
    }

    fn with_source(folder: &Path, source: Published, base_url: &str, french: bool) -> Self {
        Self {
            folder: folder.join("flair"),
            source,
            base_url: base_url.to_string(),
            french,
            cancelled: AtomicBool::new(false),
            loading: AtomicBool::new(false),
            percent: AtomicU8::new(0),
            failed: AtomicBool::new(false),
            recent: Mutex::new(Vec::new()),
        }
    }

    fn path(&self) -> PathBuf {
        self.folder.join(self.source.file)
    }

    fn state(&self) -> State {
        if self.loading.load(Ordering::Relaxed) {
            State::Loading
        } else if self.path().exists() {
            State::Ready
        } else {
            State::Off
        }
    }

    /// Says once that a download failed, for the message it deserves.
    pub fn status(&self) -> Status {
        Status {
            state: self.state(),
            percent: self.percent.load(Ordering::Relaxed),
            failed: self.failed.swap(false, Ordering::Relaxed),
            bytes: self.source.bytes,
            words: self.source.words,
        }
    }

    /// Frees the disk. Without the words, the game is simply off again. A running download cleans
    /// up after itself, as its file cannot be deleted while it is open.
    pub fn delete(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if !self.loading.load(Ordering::Relaxed) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    pub fn download(self: &Arc<Self>, changed: impl Fn() + Send + 'static) {
        if self.loading.swap(true, Ordering::Relaxed) {
            return;
        }
        self.cancelled.store(false, Ordering::Relaxed);
        self.failed.store(false, Ordering::Relaxed);
        self.percent.store(0, Ordering::Relaxed);
        changed();
        let library = Arc::clone(self);
        thread::spawn(move || {
            let result = library.fetch(&changed);
            if library.cancelled.load(Ordering::Relaxed) {
                let _ = fs::remove_dir_all(&library.folder);
            } else if result.is_err() {
                library.failed.store(true, Ordering::Relaxed);
            }
            library.loading.store(false, Ordering::Relaxed);
            changed();
        });
    }

    /// Goes on from what is already there, as a cut download of this size must not start over.
    fn fetch(&self, changed: &impl Fn()) -> io::Result<()> {
        fs::create_dir_all(&self.folder)?;
        let part = self.folder.join(format!("{}.part", self.source.file));
        let mut hash = Sha256::new();
        let mut have = 0;
        if let Ok(mut existing) = File::open(&part) {
            have = io::copy(&mut existing, &mut hash)?;
        }
        let url = format!("{}{}", self.base_url, self.source.file);
        let response = download_agent()
            .get(&url)
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
                if self.percent.swap(percent, Ordering::Relaxed) != percent {
                    changed();
                }
            }
        }
        if hex_of(hash) != self.source.sha256 {
            fs::remove_file(&part)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "altered download",
            ));
        }
        fs::rename(&part, self.path())
    }

    pub fn new_round(&self, seed: u64) -> io::Result<Option<Round>> {
        let words = secrets(self.french);
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        for step in 0..words.len() as u64 {
            let Some(secret) = pick_secret(&words, seed.wrapping_add(step), &recent) else {
                break;
            };
            recent.push(secret.to_string());
            if recent.len() > RECENT_KEPT {
                recent.remove(0);
            }
            if let Some(round) = Round::start(Vectors::open(&self.path())?, secret)? {
                return Ok(Some(round));
            }
        }
        Ok(None)
    }
}

/// Writes a file of words, as the tool in `flair-data/` does, for the tests.
#[cfg(test)]
pub fn write_vectors(path: &Path, words: &[(&str, &[i8])]) {
    let mut sorted: Vec<_> = words.to_vec();
    sorted.sort_by_key(|(word, _)| word.as_bytes().to_vec());
    let dimensions = sorted[0].1.len();
    let mut bytes = MAGIC.to_vec();
    bytes.extend(u32::try_from(sorted.len()).unwrap().to_le_bytes());
    bytes.extend(u32::try_from(dimensions).unwrap().to_le_bytes());
    let mut offset = 0u32;
    for (word, _) in &sorted {
        bytes.extend(offset.to_le_bytes());
        offset += u32::try_from(word.len()).unwrap();
    }
    bytes.extend(offset.to_le_bytes());
    for (word, _) in &sorted {
        bytes.extend(word.as_bytes());
    }
    for (_, vector) in &sorted {
        bytes.extend(vector.iter().map(|value| value.cast_unsigned()));
    }
    fs::write(path, bytes).unwrap();
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    fn sample(folder: &Path) -> Vectors {
        let path = folder.join("words.bin");
        write_vectors(
            &path,
            &[
                ("table", &[0, 100, 0]),
                ("cacao", &[90, 40, 0]),
                ("chocolat", &[100, 0, 0]),
                ("écran", &[-100, 0, 0]),
                ("sucre", &[60, 80, 0]),
            ],
        );
        Vectors::open(&path).unwrap()
    }

    #[test]
    fn finds_any_word_of_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let mut vectors = sample(folder.path());
        assert_eq!(
            vectors.find("chocolat").unwrap(),
            Some(vec![100.0, 0.0, 0.0])
        );
        assert_eq!(vectors.find("cacao").unwrap(), Some(vec![90.0, 40.0, 0.0]));
        assert_eq!(vectors.find("table").unwrap(), Some(vec![0.0, 100.0, 0.0]));
        assert_eq!(vectors.find("écran").unwrap(), Some(vec![-100.0, 0.0, 0.0]));
    }

    #[test]
    fn says_when_a_word_is_not_there() {
        let folder = tempfile::tempdir().unwrap();
        let mut vectors = sample(folder.path());
        assert_eq!(vectors.find("zzz").unwrap(), None);
        assert_eq!(vectors.find("a").unwrap(), None);
        assert_eq!(vectors.find("").unwrap(), None);
    }

    #[test]
    fn refuses_another_kind_of_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("other.bin");
        fs::write(&path, b"not a file of words").unwrap();
        assert!(Vectors::open(&path).is_err());
    }

    #[test]
    fn measures_closeness() {
        assert_eq!(temperature(&[1.0, 0.0], &[1.0, 0.0]), 100.0);
        assert_eq!(temperature(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(temperature(&[1.0, 0.0], &[-1.0, 0.0]), -100.0);
        assert_eq!(temperature(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn a_round_goes_from_cold_to_found() {
        let folder = tempfile::tempdir().unwrap();
        let mut round = Round::start(sample(folder.path()), "chocolat")
            .unwrap()
            .unwrap();
        assert_eq!(round.guess("nope").unwrap(), Guess::Unknown);
        let Guess::Close {
            temperature: cold, ..
        } = round.guess("table").unwrap()
        else {
            panic!("a word of the file");
        };
        let Guess::Close {
            temperature: warm, ..
        } = round.guess(" Cacao ").unwrap()
        else {
            panic!("a word of the file");
        };
        assert!(warm > cold);
        assert_eq!(
            round.guess("CHOCOLAT").unwrap(),
            Guess::Found {
                word: "chocolat".to_string(),
                tries: 3
            }
        );
    }

    #[test]
    fn an_unknown_word_costs_no_try() {
        let folder = tempfile::tempdir().unwrap();
        let mut round = Round::start(sample(folder.path()), "chocolat")
            .unwrap()
            .unwrap();
        round.guess("nope").unwrap();
        round.guess(&"x".repeat(MAX_WORD + 1)).unwrap();
        assert_eq!(round.tries, 0);
    }

    #[test]
    fn a_secret_must_be_in_the_file() {
        let folder = tempfile::tempdir().unwrap();
        assert!(
            Round::start(sample(folder.path()), "absent")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn avoids_the_last_words_played() {
        let words = ["a", "b", "c"];
        assert_eq!(pick_secret(&words, 0, &[]), Some("a"));
        assert_eq!(pick_secret(&words, 0, &["a".to_string()]), Some("b"));
        let all = ["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(pick_secret(&words, 1, &all), Some("a"));
        assert_eq!(pick_secret(&[], 1, &[]), None);
    }

    #[test]
    fn the_secret_words_are_clean() {
        for french in [true, false] {
            let words = secrets(french);
            assert!(words.len() >= 300, "{french}: {}", words.len());
            for word in &words {
                assert_eq!(*word, normalize(word), "{word}");
                assert!(!word.contains(' '), "{word}");
            }
            let mut sorted = words.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), words.len(), "{french}: duplicates");
        }
    }

    fn library(folder: &Path, source: Published, url: &str) -> Arc<Library> {
        Arc::new(Library::with_source(folder, source, url, true))
    }

    fn published_as(content: &[u8]) -> Published {
        let mut hash = Sha256::new();
        hash.update(content);
        Published {
            file: "fr.bin",
            bytes: content.len() as u64,
            words: 1,
            sha256: Box::leak(hex_of(hash).into_boxed_str()),
        }
    }

    /// Answers each request from where its `Range` asks, and keeps the ranges it was asked for.
    fn serve(content: &'static [u8], cut_after: Option<usize>) -> (String, Arc<Mutex<Vec<u64>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/", listener.local_addr().unwrap());
        let asked = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&asked);
        thread::spawn(move || {
            let mut cut_after = cut_after;
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut request = [0; 2048];
                let length = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..length]).to_lowercase();
                let from: usize = request
                    .split("bytes=")
                    .nth(1)
                    .and_then(|rest| rest.split('-').next())
                    .and_then(|number| number.parse().ok())
                    .unwrap_or(0);
                seen.lock().unwrap().push(from as u64);
                if from >= content.len() {
                    let _ = stream.write_all(
                        b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let rest = &content[from..];
                let head = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    rest.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = match cut_after.take() {
                    Some(cut) => stream.write_all(&rest[..cut]),
                    None => stream.write_all(rest),
                };
            }
        });
        (address, asked)
    }

    fn wait(library: &Library) {
        while library.loading.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn downloads_and_checks_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let content: &'static [u8] = vec![7; 300_000].leak();
        let (url, _) = serve(content, None);
        let library = library(folder.path(), published_as(content), &url);
        assert_eq!(library.status().state, State::Off);
        library.download(|| ());
        wait(&library);
        let status = library.status();
        assert_eq!((status.state, status.failed), (State::Ready, false));
        assert_eq!(
            fs::read(folder.path().join("flair/fr.bin")).unwrap(),
            content
        );
    }

    #[test]
    fn a_cut_download_goes_on_from_where_it_stopped() {
        let folder = tempfile::tempdir().unwrap();
        let content: &'static [u8] = vec![9; 300_000].leak();
        let (url, asked) = serve(content, Some(120_000));
        let library = library(folder.path(), published_as(content), &url);
        library.download(|| ());
        wait(&library);
        assert!(library.status().failed);
        assert_eq!(library.status().state, State::Off);
        library.download(|| ());
        wait(&library);
        assert_eq!(library.status().state, State::Ready);
        assert_eq!(*asked.lock().unwrap(), [0, 120_000]);
        assert_eq!(
            fs::read(folder.path().join("flair/fr.bin")).unwrap(),
            content
        );
    }

    #[test]
    fn a_file_that_was_all_there_only_needs_its_check() {
        let folder = tempfile::tempdir().unwrap();
        let content: &[u8] = &[3; 1000];
        let (url, _) = serve(content, None);
        fs::create_dir_all(folder.path().join("flair")).unwrap();
        fs::write(folder.path().join("flair/fr.bin.part"), content).unwrap();
        let library = library(folder.path(), published_as(content), &url);
        library.download(|| ());
        wait(&library);
        assert_eq!(library.status().state, State::Ready);
    }

    #[test]
    fn refuses_an_altered_file() {
        let folder = tempfile::tempdir().unwrap();
        let (url, _) = serve(&[1; 1000], None);
        let library = library(folder.path(), published_as(&[2; 1000]), &url);
        library.download(|| ());
        wait(&library);
        let status = library.status();
        assert_eq!((status.state, status.failed), (State::Off, true));
        assert!(!folder.path().join("flair/fr.bin.part").exists());
    }

    #[test]
    fn a_failure_is_said_once() {
        let folder = tempfile::tempdir().unwrap();
        let library = library(folder.path(), published_as(b"x"), "http://127.0.0.1:9/");
        library.download(|| ());
        wait(&library);
        assert!(library.status().failed);
        assert!(!library.status().failed);
    }

    #[test]
    fn cancelling_leaves_nothing_on_the_disk() {
        let folder = tempfile::tempdir().unwrap();
        let content: &'static [u8] = vec![5; 300_000].leak();
        let (url, _) = serve(content, None);
        let library = library(folder.path(), published_as(content), &url);
        library.download(|| ());
        library.delete();
        wait(&library);
        assert_eq!(library.status().state, State::Off);
        assert!(!folder.path().join("flair").exists());
    }

    #[test]
    fn deleting_frees_the_disk() {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("flair")).unwrap();
        fs::write(folder.path().join("flair/fr.bin"), b"x").unwrap();
        let library = library(folder.path(), published_as(b"x"), "http://127.0.0.1:9/");
        assert_eq!(library.status().state, State::Ready);
        library.delete();
        assert_eq!(library.status().state, State::Off);
    }

    #[test]
    fn each_language_has_its_file() {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("flair")).unwrap();
        fs::write(folder.path().join("flair/fr.bin"), b"x").unwrap();
        assert_eq!(
            Library::new(folder.path(), "fr").status().state,
            State::Ready
        );
        assert_eq!(Library::new(folder.path(), "en").status().state, State::Off);
    }
}
