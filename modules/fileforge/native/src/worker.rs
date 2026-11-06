//! Does the work, one file at a time, on its own thread.

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use arthex_module::Events;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

use crate::{
    documents,
    format::{self, Kind},
    media::{self, Job, Probe},
    queue::{Item, Mode, Queue, Settings, Status, free_output},
};

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_: &mut Command) {}

enum Failure {
    Cancelled,
    NeedsOffice,
    Failed,
}

/// What the interface and the worker share.
pub struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    current: Mutex<Option<Child>>,
    cancelled: AtomicBool,
    open_requested: AtomicBool,
    events: Events,
    ffmpeg: PathBuf,
    /// By file, as data addresses: the interface cannot read a file itself.
    thumbnails: Mutex<HashMap<u64, String>>,
    wanted_thumbnails: Mutex<Sender<(u64, PathBuf)>>,
}

impl Shared {
    pub fn new(settings: Settings, events: Events, ffmpeg: Option<&Path>) -> Arc<Self> {
        let ffmpeg = ffmpeg.map_or_else(
            || PathBuf::from("ffmpeg"),
            |folder| folder.join("ffmpeg.exe"),
        );
        let (wanted, wanted_receiver) = mpsc::channel();
        let shared = Arc::new(Self {
            queue: Mutex::new(Queue::new(settings)),
            wake: Condvar::new(),
            current: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            open_requested: AtomicBool::new(false),
            events,
            ffmpeg,
            thumbnails: Mutex::default(),
            wanted_thumbnails: Mutex::new(wanted),
        });
        let thumbnails = Arc::clone(&shared);
        thread::spawn(move || thumbnails.make_thumbnails(&wanted_receiver));
        let worker = Arc::clone(&shared);
        thread::spawn(move || worker.run());
        shared
    }

    pub fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn want_thumbnail(&self, id: u64, path: &Path) {
        let wanted = self
            .wanted_thumbnails
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // The thread only ends with the program.
        let _ = wanted.send((id, path.to_path_buf()));
    }

    pub fn thumbnails(&self) -> HashMap<u64, String> {
        self.thumbnails
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// One at a time, away from the conversions: a whole folder can arrive at once.
    fn make_thumbnails(&self, wanted: &Receiver<(u64, PathBuf)>) {
        for (id, path) in wanted {
            let Some(url) = self.thumbnail_of(id, &path) else {
                continue;
            };
            self.thumbnails
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(id, url.clone());
            // Arthex is closing if nobody listens any more.
            let _ = self
                .events
                .emit("thumbnail", json!({ "id": id, "url": url }));
        }
    }

    fn thumbnail_of(&self, id: u64, input: &Path) -> Option<String> {
        let output =
            std::env::temp_dir().join(format!("fileforge-{}-{id}.jpg", std::process::id()));
        let mut command = Command::new(&self.ffmpeg);
        command
            .args(media::thumbnail_arguments(
                &input.display().to_string(),
                &output.display().to_string(),
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_window(&mut command);
        let succeeded = command.status().is_ok_and(|status| status.success());
        let bytes = fs::read(&output).ok();
        let _ = fs::remove_file(&output);
        let bytes = bytes.filter(|_| succeeded)?;
        Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
    }

    pub fn publish(&self) {
        let queue = self.queue();
        // Arthex is closing if nobody listens any more.
        let _ = self.events.emit("state", &*queue);
    }

    /// Shows the page. At a cold start nobody hears the event: the interface asks `take_open`.
    pub fn ask_to_open(&self) {
        self.open_requested.store(true, Ordering::Relaxed);
        // Arthex is closing if nobody listens any more.
        let _ = self.events.set_overlay_visible(true);
        let _ = self.events.emit("open", ());
    }

    pub fn take_open(&self) -> bool {
        self.open_requested.swap(false, Ordering::Relaxed)
    }

    pub fn wake(&self) {
        self.wake.notify_all();
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.queue().cancel();
        self.stop_current();
        self.publish();
    }

    /// Ends the program in progress, so that it never outlives this one.
    pub fn stop_current(&self) {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(child) = current.as_mut() {
            let _ = child.kill();
        }
    }

    fn run(&self) {
        loop {
            let item = {
                let mut queue = self.queue();
                loop {
                    if let Some(item) = queue.take_next() {
                        break item;
                    }
                    queue = self
                        .wake
                        .wait(queue)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            };
            self.cancelled.store(false, Ordering::Relaxed);
            self.publish();
            let result = self.work(&item);
            let mut queue = self.queue();
            if let Some(done) = queue.item_mut(item.id) {
                match result {
                    // `cancel` has already put it back.
                    Err(Failure::Cancelled) => {}
                    Err(Failure::NeedsOffice) => fail(done, "needsOffice"),
                    Err(Failure::Failed) => fail(done, "failed"),
                    Ok(Outcome::Written { path, size }) => {
                        done.status = Status::Done;
                        done.progress = 100;
                        done.output = Some(path.display().to_string());
                        done.output_size = Some(size);
                    }
                    Ok(Outcome::NoGain) => {
                        done.status = Status::Done;
                        done.progress = 100;
                        done.no_gain = true;
                    }
                }
            }
            drop(queue);
            self.publish();
        }
    }

    fn set_progress(&self, id: u64, progress: u8) {
        let mut queue = self.queue();
        if let Some(item) = queue.item_mut(id)
            && item.status == Status::Working
            && item.progress != progress
        {
            item.progress = progress;
            drop(queue);
            self.publish();
        }
    }

    fn work(&self, item: &Item) -> Result<Outcome, Failure> {
        let input = PathBuf::from(&item.path);
        let settings = self.queue().settings.clone();
        let extension = input
            .extension()
            .map(|extension| extension.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem = input
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        let folder = settings
            .folder
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| input.parent().map(Path::to_path_buf))
            .ok_or(Failure::Failed)?;
        let target = match item.mode {
            Mode::Convert => item.target.clone(),
            Mode::Compress => format::compressed_target(item.kind, &extension),
        };
        let out_extension = format::extension_of(&target).to_string();
        // Under a neutral name: a `%` in a file's name means something to FFmpeg.
        let temporary = folder.join(format!("fileforge-{}-work.{out_extension}", item.id));
        let produced = match item.kind {
            Kind::Document if item.mode == Mode::Compress => {
                self.compress_document(&input, &temporary, &extension, &settings)
            }
            Kind::Document => self.convert_document(&input, &temporary, &extension, &target),
            _ => self.run_media(item, &input, &temporary, &target, &settings),
        };
        if let Err(failure) = produced {
            let _ = fs::remove_file(&temporary);
            return Err(failure);
        }
        let size = fs::metadata(&temporary).map_err(|_| Failure::Failed)?.len();
        if item.mode == Mode::Compress && size >= item.size {
            let _ = fs::remove_file(&temporary);
            return Ok(Outcome::NoGain);
        }
        let stem = if item.mode == Mode::Compress {
            format!("{stem} (compressed)")
        } else {
            stem
        };
        let output = free_output(&folder, &stem, &out_extension, &|path| path.exists());
        fs::rename(&temporary, &output).map_err(|_| Failure::Failed)?;
        Ok(Outcome::Written { path: output, size })
    }

    /// Runs a program to its end, hearing each line it writes. It can be killed meanwhile.
    fn run_program(
        &self,
        mut command: Command,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<(), Failure> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(Failure::Cancelled);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_window(&mut command);
        let mut child = command.spawn().map_err(|_| Failure::Failed)?;
        let stdout = child.stdout.take();
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(child);
        if let Some(stdout) = stdout {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                on_line(&line);
            }
        }
        let child = self
            .current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let status = child.and_then(|mut child| child.wait().ok());
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(Failure::Cancelled);
        }
        if status.is_some_and(|status| status.success()) {
            Ok(())
        } else {
            Err(Failure::Failed)
        }
    }

    /// What `FFmpeg` says when it only opens a file. It then ends with an error, which is expected.
    fn probe(&self, input: &Path) -> Probe {
        let mut command = Command::new(&self.ffmpeg);
        command
            .args(["-hide_banner", "-nostdin", "-i"])
            .arg(input)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hide_window(&mut command);
        let Ok(mut child) = command.spawn() else {
            return Probe::default();
        };
        let mut log = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut log);
        }
        let _ = child.wait();
        media::parse_probe(&log)
    }

    fn run_media(
        &self,
        item: &Item,
        input: &Path,
        output: &Path,
        target: &str,
        settings: &Settings,
    ) -> Result<(), Failure> {
        let still_image = item.kind == Kind::Image;
        let probe = if still_image {
            Probe::default()
        } else {
            self.probe(input)
        };
        let quality = match item.mode {
            Mode::Convert => settings.quality,
            Mode::Compress => settings.level,
        };
        let (input_text, output_text) = (input.display().to_string(), output.display().to_string());
        let job = Job {
            input: &input_text,
            output: &output_text,
            target,
            quality,
            height: settings.height,
            strip_metadata: settings.strip_metadata,
            probe: &probe,
        };
        let mut command = Command::new(&self.ffmpeg);
        command.args(media::arguments(&job, still_image));
        let duration = probe.duration_us.unwrap_or(0);
        self.run_program(command, &mut |line| {
            if let Some(percent) = media::progress_percent(line, duration) {
                self.set_progress(item.id, percent);
            }
        })
    }

    fn convert_document(
        &self,
        input: &Path,
        output: &Path,
        extension: &str,
        target: &str,
    ) -> Result<(), Failure> {
        let office = documents::find_office(extension).ok_or(Failure::NeedsOffice)?;
        // LibreOffice names its result after the input: it gets a folder of its own.
        let work = output.with_extension("dir");
        fs::create_dir_all(&work).map_err(|_| Failure::Failed)?;
        let placed = work.join(output.file_name().unwrap_or_default());
        let result = self.run_program(
            documents::command(&office, input, &placed, target),
            &mut |_| {},
        );
        let found = match office {
            documents::Office::Word => Some(placed),
            documents::Office::LibreOffice(_) => input
                .file_stem()
                .map(|stem| work.join(stem).with_extension(format::extension_of(target))),
        };
        let moved = result.and_then(|()| {
            found
                .filter(|path| path.is_file())
                .ok_or(Failure::Failed)
                .and_then(|path| fs::rename(path, output).map_err(|_| Failure::Failed))
        });
        let _ = fs::remove_dir_all(&work);
        moved
    }

    fn compress_document(
        &self,
        input: &Path,
        output: &Path,
        extension: &str,
        settings: &Settings,
    ) -> Result<(), Failure> {
        if extension.eq_ignore_ascii_case("pdf") {
            return documents::compress_pdf(input, output).map_err(|_| Failure::Failed);
        }
        if !documents::is_office_archive(extension) {
            return Err(Failure::Failed);
        }
        let photo_in = output.with_extension("in.jpg");
        let photo_out = output.with_extension("out.jpg");
        let mut shrink = |bytes: &[u8]| {
            fs::write(&photo_in, bytes).ok()?;
            let probe = Probe::default();
            let (input_text, output_text) = (
                photo_in.display().to_string(),
                photo_out.display().to_string(),
            );
            let job = Job {
                input: &input_text,
                output: &output_text,
                target: "jpeg",
                quality: settings.level,
                height: None,
                strip_metadata: false,
                probe: &probe,
            };
            let mut command = Command::new(&self.ffmpeg);
            command.args(media::arguments(&job, true));
            self.run_program(command, &mut |_| {}).ok()?;
            documents::read(&photo_out)
        };
        let written = documents::compress_office(input, output, &mut shrink);
        let _ = fs::remove_file(&photo_in);
        let _ = fs::remove_file(&photo_out);
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(Failure::Cancelled);
        }
        written.map_err(|_| Failure::Failed)
    }
}

enum Outcome {
    Written { path: PathBuf, size: u64 },
    NoGain,
}

fn fail(item: &mut Item, error: &str) {
    item.status = Status::Failed;
    item.progress = 0;
    item.error = Some(error.to_string());
}
