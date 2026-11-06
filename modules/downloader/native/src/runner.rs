//! Runs yt-dlp on threads of its own, as it can take long, and tells the interface by events.

use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, Instant},
};

use arthex_module::Events;
use base64::Engine;
use serde::Serialize;
use serde_json::json;

use super::{
    job::{self, Group},
    ytdlp::{
        self, Bar, Failure, Info, LAST_PERCENT, Line, Request, Tools, analyse_args, download_args,
        failure_of, parse_info, parse_line,
    },
};

const WORK: &str = "work";
const SHORTEST_UPDATE: Duration = Duration::from_millis(200);
const MAX_THUMBNAIL_BYTES: u64 = 2 * 1024 * 1024;
/// What is kept of what yt-dlp says when it gives up.
const MAX_ERRORS: usize = 16 * 1024;

/// What the tool shows: it may have been closed and opened again in the meantime.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    pub percent: f64,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub processing: bool,
    pub file: Option<String>,
    pub failure: Option<Failure>,
    pub cancelled: bool,
}

#[derive(Default)]
struct Current {
    group: Option<Arc<Group>>,
    cancelled: bool,
}

pub struct Runner {
    events: Events,
    program: PathBuf,
    quickjs: PathBuf,
    ffmpeg: Option<PathBuf>,
    folder: PathBuf,
    status: Mutex<Status>,
    download: Mutex<Current>,
    analysis: Mutex<(u64, Option<Arc<Group>>)>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn command_for(program: &Path, args: &[String]) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn last_part(text: &mut String) {
    if text.len() > MAX_ERRORS {
        let mut cut = text.len() - MAX_ERRORS;
        while !text.is_char_boundary(cut) {
            cut += 1;
        }
        text.drain(..cut);
    }
}

/// Where it writes why it gave up, and also, once it has `--print` to do, what it is processing.
fn read_errors(stderr: impl Read, on_processing: impl Fn()) -> String {
    let mut errors = String::new();
    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
        if parse_line(&line) == Line::Processing {
            on_processing();
        } else {
            errors.push_str(&line);
            errors.push('\n');
            last_part(&mut errors);
        }
    }
    errors
}

/// A remote image cannot be loaded by the interface: it receives it as a data address.
fn thumbnail_data(address: &str) -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent.get(address).call().ok()?;
    let kind = response
        .headers()
        .get("content-type")?
        .to_str()
        .ok()?
        .to_string();
    if !kind.starts_with("image/") || kind.contains("svg") {
        return None;
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_THUMBNAIL_BYTES)
        .read_to_vec()
        .ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{kind};base64,{encoded}"))
}

impl Runner {
    pub fn new(
        events: Events,
        program_folder: &Path,
        ffmpeg: Option<PathBuf>,
        folder: PathBuf,
    ) -> Arc<Self> {
        let _ = fs::remove_dir_all(folder.join(WORK));
        Arc::new(Self {
            events,
            program: program_folder.join("yt-dlp").join("yt-dlp.exe"),
            quickjs: program_folder.join("qjs.exe"),
            ffmpeg,
            folder,
            status: Mutex::new(Status::default()),
            download: Mutex::new(Current::default()),
            analysis: Mutex::new((0, None)),
        })
    }

    fn tools(&self) -> Tools<'_> {
        Tools {
            quickjs: &self.quickjs,
            ffmpeg: self.ffmpeg.as_deref(),
        }
    }

    pub fn status(&self) -> Status {
        locked(&self.status).clone()
    }

    fn set_status(&self, change: impl FnOnce(&mut Status)) {
        let mut status = locked(&self.status);
        change(&mut status);
        // Lost, this only delays what the bar shows.
        let _ = self.events.emit("progress", &*status);
    }

    /// Answers by the event `analysed`, with its number: only the latest link counts.
    pub fn analyse(self: &Arc<Self>, url: String) -> Result<u64, String> {
        if !ytdlp::is_link(&url) {
            return Err("not a link".to_string());
        }
        let token = {
            let mut analysis = locked(&self.analysis);
            if let Some(group) = analysis.1.take() {
                group.kill();
            }
            analysis.0 += 1;
            analysis.0
        };
        let runner = Arc::clone(self);
        thread::spawn(move || {
            let answer = runner.read_link(&url, token);
            if locked(&runner.analysis).0 != token {
                return;
            }
            let _ = runner.events.emit(
                "analysed",
                match answer {
                    Ok((info, thumbnail)) => {
                        json!({ "token": token, "info": info, "thumbnail": thumbnail })
                    }
                    Err(failure) => json!({ "token": token, "failure": failure }),
                },
            );
        });
        Ok(token)
    }

    fn read_link(&self, url: &str, token: u64) -> Result<(Info, Option<String>), Failure> {
        let args = analyse_args(url, &self.tools());
        let mut child = command_for(&self.program, &args)
            .spawn()
            .map_err(|_| Failure::Other)?;
        let group = Group::holding(&child).map(Arc::new);
        {
            let mut analysis = locked(&self.analysis);
            if analysis.0 != token {
                job::stop(group.as_deref(), &mut child);
                return Err(Failure::Other);
            }
            analysis.1.clone_from(&group);
        }
        let errors = child
            .stderr
            .take()
            .map(|stderr| thread::spawn(|| read_errors(stderr, || ())));
        let mut json = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_string(&mut json);
        }
        let done = child.wait().is_ok_and(|status| status.success());
        let errors = errors
            .and_then(|errors| errors.join().ok())
            .unwrap_or_default();
        if !done {
            return Err(failure_of(&errors));
        }
        let info = parse_info(&json).map_err(|reason| failure_of(&reason))?;
        let thumbnail = info.thumbnail.as_deref().and_then(thumbnail_data);
        Ok((info, thumbnail))
    }

    pub fn start(self: &Arc<Self>, request: Request, target: PathBuf) -> Result<(), String> {
        request.check()?;
        {
            let mut status = locked(&self.status);
            if status.running {
                return Err("already downloading".to_string());
            }
            *status = Status {
                running: true,
                ..Status::default()
            };
        }
        locked(&self.download).cancelled = false;
        let runner = Arc::clone(self);
        thread::spawn(move || runner.run(&request, &target));
        Ok(())
    }

    fn run(&self, request: &Request, target: &Path) {
        let work = self.folder.join(WORK);
        let result = fs::create_dir_all(&work)
            .map_err(|_| Failure::Other)
            .and_then(|()| self.download_to(request, target, &work));
        let _ = fs::remove_dir_all(&work);
        let cancelled = locked(&self.download).cancelled;
        locked(&self.download).group = None;
        self.set_status(|status| {
            status.running = false;
            status.processing = false;
            status.speed = None;
            status.eta = None;
            status.cancelled = cancelled;
            match result {
                Ok(file) => {
                    status.percent = 100.0;
                    status.file = Some(file);
                }
                Err(failure) if !cancelled => status.failure = Some(failure),
                Err(_) => {}
            }
        });
    }

    fn download_to(
        &self,
        request: &Request,
        target: &Path,
        work: &Path,
    ) -> Result<String, Failure> {
        let args = download_args(request, &self.tools(), target, work);
        let mut child = command_for(&self.program, &args)
            .spawn()
            .map_err(|_| Failure::Other)?;
        let group = Group::holding(&child).map(Arc::new);
        {
            let mut current = locked(&self.download);
            current.group.clone_from(&group);
            if current.cancelled {
                job::stop(group.as_deref(), &mut child);
            }
        }
        let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
        let (file, errors) = thread::scope(|scope| {
            let errors =
                stderr.map(|stderr| scope.spawn(|| read_errors(stderr, || self.mark_processing())));
            let file = stdout.and_then(|stdout| self.follow(stdout, request.audio));
            let errors = errors
                .and_then(|errors| errors.join().ok())
                .unwrap_or_default();
            (file, errors)
        });
        let done = child.wait().is_ok_and(|status| status.success());
        if !done {
            return Err(failure_of(&errors));
        }
        file.ok_or(Failure::Other)
    }

    /// Reads the lines as they come, and tells the interface at most a few times a second.
    fn follow(&self, stdout: impl Read, audio: bool) -> Option<String> {
        let mut bar = Bar::new(audio);
        let mut file = None;
        let mut told = Instant::now().checked_sub(SHORTEST_UPDATE);
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            match parse_line(&line) {
                Line::Progress {
                    file: part,
                    fraction,
                    speed,
                    eta,
                } => {
                    let percent = bar.advance(&part, fraction);
                    if told.is_none_or(|time| time.elapsed() >= SHORTEST_UPDATE) {
                        told = Some(Instant::now());
                        self.set_status(|status| {
                            status.percent = status.percent.max(percent);
                            status.speed = speed;
                            status.eta = eta;
                        });
                    }
                }
                Line::Processing => self.mark_processing(),
                Line::File(path) => file = Some(path),
                Line::Other => {}
            }
        }
        file
    }

    fn mark_processing(&self) {
        self.set_status(|status| {
            status.percent = LAST_PERCENT;
            status.processing = true;
            status.speed = None;
            status.eta = None;
        });
    }

    /// Stops the download. What it wrote so far sits in its own folder, emptied at the end.
    pub fn cancel(&self) {
        let mut current = locked(&self.download);
        current.cancelled = true;
        if let Some(group) = &current.group {
            group.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_end_of_what_it_said_is_kept() {
        let mut text = "é".repeat(MAX_ERRORS);
        last_part(&mut text);
        assert!(text.len() <= MAX_ERRORS);
        assert!(text.chars().all(|character| character == 'é'));
        let mut short = "ERROR".to_string();
        last_part(&mut short);
        assert_eq!(short, "ERROR");
    }
}
