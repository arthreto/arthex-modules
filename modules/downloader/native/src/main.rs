//! The system side of the Downloader module: runs yt-dlp on the links the user gives it.

mod job;
mod runner;
mod ytdlp;

use std::{
    cell::OnceCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex, PoisonError},
};

use arthex_module::{Context, Module};
use runner::Runner;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ytdlp::Request;

const SETTINGS: &str = "settings.json";

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    folder: Option<PathBuf>,
}

struct App {
    runner: Arc<Runner>,
    #[cfg_attr(not(windows), allow(dead_code))]
    data: PathBuf,
    folder: Mutex<PathBuf>,
}

/// The folder Windows calls Downloads, unless the user chose another one that still exists.
fn start_folder(data: &Path) -> PathBuf {
    let saved: Settings = fs::read(data.join(SETTINGS))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    saved
        .folder
        .filter(|folder| folder.is_dir())
        .or_else(downloads_folder)
        .unwrap_or_else(|| data.to_path_buf())
}

#[cfg(windows)]
fn downloads_folder() -> Option<PathBuf> {
    use windows_sys::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath};
    let mut path = std::ptr::null_mut();
    let known_folder = FOLDERID_Downloads;
    // SAFETY: writes a string that Windows allocates; it is read at once and never freed, as this
    // happens once in the life of the program.
    let answered = unsafe {
        SHGetKnownFolderPath(
            &raw const known_folder,
            0,
            std::ptr::null_mut(),
            &raw mut path,
        )
    };
    if answered != 0 || path.is_null() {
        return None;
    }
    // SAFETY: Windows ends this string with a zero.
    let mut length = 0;
    while unsafe { *path.add(length) } != 0 {
        length += 1;
    }
    // SAFETY: `length` characters were just read from it.
    let wide = unsafe { std::slice::from_raw_parts(path, length) };
    Some(PathBuf::from(String::from_utf16_lossy(wide)))
}

#[cfg(not(windows))]
fn downloads_folder() -> Option<PathBuf> {
    None
}

impl App {
    fn start(context: &Context) -> Self {
        let data = context.data_dir().to_path_buf();
        let program_folder = std::env::current_exe()
            .ok()
            .and_then(|program| Some(program.parent()?.to_path_buf()))
            .unwrap_or_default();
        Self {
            runner: Runner::new(
                context.events().clone(),
                &program_folder,
                context.shared("ffmpeg").map(Path::to_path_buf),
                data.clone(),
            ),
            folder: Mutex::new(start_folder(&data)),
            data,
        }
    }

    fn folder(&self) -> PathBuf {
        self.folder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[derive(Deserialize)]
struct Link {
    url: String,
}

fn parsed<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn state(app: &App, _: Value) -> Result<Value, String> {
    serde_json::to_value(json!({ "status": app.runner.status(), "folder": app.folder() }))
        .map_err(|error| error.to_string())
}

fn analyse(app: &App, params: Value) -> Result<Value, String> {
    let params: Link = parsed(params)?;
    Ok(json!(app.runner.analyse(params.url)?))
}

fn start(app: &App, params: Value) -> Result<Value, String> {
    let request: Request = parsed(params)?;
    app.runner.start(request, app.folder())?;
    Ok(Value::Null)
}

#[allow(clippy::unnecessary_wraps)]
fn cancel(app: &App, _: Value) -> Result<Value, String> {
    app.runner.cancel();
    Ok(Value::Null)
}

#[cfg(windows)]
fn pick_folder(app: &App, _: Value) -> Result<Value, String> {
    let Some(folder) = rfd::FileDialog::new()
        .set_directory(app.folder())
        .pick_folder()
    else {
        return Ok(Value::Null);
    };
    let bytes = serde_json::to_vec(&Settings {
        folder: Some(folder.clone()),
    })
    .map_err(|error| error.to_string())?;
    fs::write(app.data.join(SETTINGS), bytes).map_err(|error| error.to_string())?;
    app.folder
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone_from(&folder);
    Ok(json!(folder))
}

/// Only what was just downloaded here, which it knows by itself: nothing else on the disk.
#[cfg(windows)]
fn finished_file(app: &App) -> Result<PathBuf, String> {
    app.runner
        .status()
        .file
        .map(PathBuf::from)
        .filter(|file| file.is_file())
        .ok_or_else(|| "nothing was downloaded".to_string())
}

#[cfg(windows)]
fn open_file(app: &App, _: Value) -> Result<Value, String> {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let file = finished_file(app)?;
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() };
    let (verb, target) = (wide("open"), wide(&file.to_string_lossy()));
    // SAFETY: the strings outlive the call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Below this value, it is an error code.
    if result as usize > 32 {
        Ok(Value::Null)
    } else {
        Err("could not open it".to_string())
    }
}

#[cfg(windows)]
fn show_file(app: &App, _: Value) -> Result<Value, String> {
    use std::os::windows::process::CommandExt;
    let file = finished_file(app)?;
    // Explorer reads this one as a whole, with no quotes of its own to escape from.
    std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", file.display()))
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

#[cfg(not(windows))]
fn only_on_windows(_: &App, _: Value) -> Result<Value, String> {
    Err("only on Windows".to_string())
}

type Handler = fn(&App, Value) -> Result<Value, String>;

fn main() -> std::io::Result<()> {
    #[cfg(not(windows))]
    let (pick_folder, open_file, show_file): (Handler, Handler, Handler) =
        (only_on_windows, only_on_windows, only_on_windows);
    let handlers: [(&str, Handler); 7] = [
        ("state", state),
        ("analyse", analyse),
        ("start", start),
        ("cancel", cancel),
        ("pickFolder", pick_folder),
        ("openFile", open_file),
        ("showFile", show_file),
    ];
    let app: Rc<OnceCell<App>> = Rc::default();
    let starting = Rc::clone(&app);
    let mut module = Module::new().on_start(move |context| {
        let _ = starting.set(App::start(context));
    });
    for (name, handler) in handlers {
        let app = Rc::clone(&app);
        module = module.on(name, move |_, params| {
            handler(app.get().ok_or("the program has not started")?, params)
        });
    }
    module.run()
}
