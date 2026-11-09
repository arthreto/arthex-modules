//! Makes the files one after the other on a thread of their own, so that a long video or a file
//! of several gigabytes never blocks the interface. Each one tells how it went with an event.

use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
    },
};

use arthex_module::Events;
use image::imageops::FilterType;
use serde_json::json;

use crate::{
    document::{self, Words},
    filler::{self, Samples},
    media, pdf, picture,
    spec::Spec,
    text::{self, Language},
};

pub struct Job {
    pub id: u64,
    pub path: PathBuf,
    pub file: File,
    pub format: String,
    pub spec: Spec,
    pub language: Language,
}

pub struct Tools {
    pub ffmpeg: PathBuf,
    pub pictures: PathBuf,
}

pub struct Jobs {
    sender: Mutex<Sender<Job>>,
    waiting: Arc<AtomicUsize>,
}

impl Jobs {
    pub fn start(tools: Tools, events: Events) -> Self {
        let (sender, jobs) = mpsc::channel::<Job>();
        let waiting = Arc::new(AtomicUsize::new(0));
        let left = Arc::clone(&waiting);
        std::thread::spawn(move || {
            for job in jobs {
                let (id, path) = (job.id, job.path.clone());
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                match make(&tools, job, &events) {
                    Ok(()) => {
                        let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
                        events
                            .emit(
                                "done",
                                json!({ "id": id, "file": file_name, "folder": folder }),
                            )
                            .ok();
                    }
                    Err(reason) => {
                        // Nothing half made is left behind.
                        fs::remove_file(&path).ok();
                        events
                            .emit(
                                "failed",
                                json!({ "id": id, "file": file_name, "reason": reason }),
                            )
                            .ok();
                    }
                }
                left.fetch_sub(1, Ordering::Relaxed);
            }
        });
        Self {
            sender: Mutex::new(sender),
            waiting,
        }
    }

    pub fn add(&self, job: Job) -> Result<(), String> {
        self.waiting.fetch_add(1, Ordering::Relaxed);
        self.sender
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(job)
            .map_err(|error| error.to_string())
    }

    /// How many files are still being made.
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::Relaxed)
    }
}

/// The picture shown in documents: the landscape, small enough to keep them light.
fn document_picture(tools: &Tools) -> Result<(Vec<u8>, (u32, u32)), String> {
    let landscape =
        picture::load(&tools.pictures, "landscape")?.resize_to_fill(900, 600, FilterType::Lanczos3);
    Ok((picture::encode_jpeg(&landscape, 82)?, (900, 600)))
}

fn write(mut file: File, bytes: &[u8]) -> Result<(), String> {
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|error| error.to_string())
}

fn make(tools: &Tools, job: Job, events: &Events) -> Result<(), String> {
    let Job {
        id,
        path,
        mut file,
        format,
        spec,
        language,
    } = job;
    let progress = |percent: u8| {
        events
            .emit("progress", json!({ "id": id, "percent": percent }))
            .ok();
    };
    let words = Words::new(language);
    match spec {
        Spec::Text { rows } => write(
            file,
            text::content(&format, rows, language)
                .unwrap_or_default()
                .as_bytes(),
        ),
        Spec::Image { picture, size } => {
            let image = picture::load(&tools.pictures, &picture)?;
            write(file, &picture::convert(&image, size, &format)?)
        }
        Spec::Document { pages } => {
            let (jpeg, jpeg_size) = document_picture(tools)?;
            let bytes = match format.as_str() {
                "pdf" => pdf::write(pages, &words, &jpeg, jpeg_size),
                "docx" => document::docx(pages, &words, &jpeg, jpeg_size)?,
                "xlsx" => document::xlsx(document::rows_for_pages(pages), &words)?,
                "pptx" => document::pptx(pages, &words, &jpeg, jpeg_size)?,
                _ => document::odt(pages, &words)?,
            };
            write(file, &bytes)
        }
        Spec::Media { seconds, height } => {
            drop(file);
            let args = media::args(&format, seconds, height, &path).ok_or("option")?;
            run_ffmpeg(&tools.ffmpeg, &args, seconds, progress)
        }
        Spec::Archive { files } => {
            let (jpeg_image, pdf_picture) = (
                picture::load(&tools.pictures, "food")?,
                document_picture(tools)?,
            );
            let samples = Samples {
                text: text::content("txt", 0, language).unwrap_or_default(),
                csv: text::content("csv", 20, language).unwrap_or_default(),
                markdown: text::content("md", 0, language).unwrap_or_default(),
                jpeg: picture::encode_jpeg(
                    &jpeg_image.resize_to_fill(960, 640, FilterType::Lanczos3),
                    82,
                )?,
                pdf: pdf::write(1, &words, &pdf_picture.0, pdf_picture.1),
            };
            let folders = match language {
                Language::French => ("documents", "images"),
                Language::English => ("documents", "pictures"),
            };
            write(file, &filler::archive(files, &samples, folders)?)
        }
        Spec::Sized { bytes, random } => {
            let folder = path.parent().unwrap_or(&path);
            if filler::free_space(folder).is_some_and(|free| free < bytes) {
                return Err("space".to_string());
            }
            filler::sized(&mut file, bytes, random, progress).map_err(|error| error.to_string())
        }
    }
}

fn run_ffmpeg(
    ffmpeg: &Path,
    args: &[std::ffi::OsString],
    seconds: u32,
    progress: impl Fn(u8),
) -> Result<(), String> {
    let mut command = Command::new(ffmpeg);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    let mut child = command.spawn().map_err(|_| "ffmpeg".to_string())?;
    let errors = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut text = String::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                text = line;
            }
            text
        })
    });
    if let Some(stdout) = child.stdout.take() {
        let mut shown = None;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(percent) =
                media::progress(&line, seconds).filter(|percent| shown != Some(*percent))
            {
                shown = Some(percent);
                progress(percent);
            }
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    let last_error = errors
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        Err(last_error)
    }
}
