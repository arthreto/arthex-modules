//! The system side of Template Example: creates a sample file of the format asked for, in the
//! folder of a right-click or one the user picks. Nothing is downloaded: the pictures come with the
//! module, and sound and video are made by the shared `FFmpeg`.

mod document;
mod filler;
mod jobs;
mod media;
mod name;
mod pdf;
mod picture;
mod spec;
mod text;

use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use arthex_module::{Context, Module};
use jobs::{Job, Jobs, Tools};
use serde::Deserialize;
use serde_json::{Value, json};
use spec::Request;
use text::Language;

struct Shared {
    jobs: Jobs,
    next_id: AtomicU64,
    /// The folder of a right-click that came before the interface could hear it.
    asked_folder: Mutex<Option<PathBuf>>,
}

impl Shared {
    fn start(context: &Context) -> Self {
        let ffmpeg = context.shared("ffmpeg").map_or_else(
            || PathBuf::from("ffmpeg"),
            |folder| folder.join("ffmpeg.exe"),
        );
        let tools = Tools {
            ffmpeg,
            pictures: picture::folder(),
        };
        Self {
            jobs: Jobs::start(tools, context.events().clone()),
            next_id: AtomicU64::new(1),
            asked_folder: Mutex::new(None),
        }
    }
}

type Handler = fn(&Shared, &Context, Value) -> Result<Value, String>;

#[derive(Deserialize)]
struct PreviewParams {
    format: String,
    rows: Option<usize>,
}

fn preview(_: &Shared, context: &Context, params: Value) -> Result<Value, String> {
    let PreviewParams { format, rows } =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    // The preview shows the first lines: a few rows are enough to make them.
    let rows = rows.unwrap_or(100).min(20);
    let written = text::content(&format, rows, Language::of(context.locale()));
    Ok(written.map_or(Value::Null, |written| json!(text::preview(&written))))
}

#[derive(Deserialize)]
struct NameParams {
    folder: PathBuf,
    name: String,
    format: String,
}

/// The name the file would get, or nothing while the name or the format is refused.
fn free_name(_: &Shared, _: &Context, params: Value) -> Result<Value, String> {
    let NameParams {
        folder,
        name,
        format,
    } = serde_json::from_value(params).map_err(|error| error.to_string())?;
    let (Ok(name), Ok(format)) = (name::check_name(&name), name::check_format(&format)) else {
        return Ok(Value::Null);
    };
    Ok(json!(name::free_name(&folder, name, &format)))
}

fn create(shared: &Shared, context: &Context, params: Value) -> Result<Value, String> {
    let request: Request = serde_json::from_value(params).map_err(|error| error.to_string())?;
    let name = name::check_name(&request.name).map_err(|refused| refused.code().to_string())?;
    let format =
        name::check_format(&request.format).map_err(|refused| refused.code().to_string())?;
    if !request.folder.is_absolute() || !request.folder.is_dir() {
        return Err("folder".to_string());
    }
    let spec = spec::spec(&format, &request)?;
    let (path, file) =
        name::reserve(&request.folder, name, &format).map_err(|_| "folder".to_string())?;
    let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    shared.jobs.add(Job {
        id,
        path,
        file,
        format,
        spec,
        language: Language::of(context.locale()),
    })?;
    Ok(json!({ "id": id, "file": file_name }))
}

#[cfg(windows)]
fn pick_folder(params: &Value) -> Value {
    let mut dialog = rfd::FileDialog::new();
    if let Some(start) = params.get("folder").and_then(Value::as_str) {
        dialog = dialog.set_directory(start);
    }
    dialog
        .pick_folder()
        .map_or(Value::Null, |folder| json!(folder))
}

#[cfg(not(windows))]
fn pick_folder(_: &Value) -> Value {
    Value::Null
}

/// The Desktop of the user, even when it was moved (to `OneDrive`, for instance).
#[cfg(windows)]
fn desktop() -> Option<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, ptr, slice};
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Desktop, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    };

    let mut path = ptr::null_mut();
    // SAFETY: Windows allocates the path, which is read up to its NUL and then freed.
    unsafe {
        let found = SHGetKnownFolderPath(
            &FOLDERID_Desktop,
            KF_FLAG_DEFAULT as u32,
            ptr::null_mut(),
            &raw mut path,
        ) == 0;
        let desktop = found.then(|| {
            let mut length = 0;
            while *path.add(length) != 0 {
                length += 1;
            }
            PathBuf::from(OsString::from_wide(slice::from_raw_parts(path, length)))
        });
        CoTaskMemFree(path.cast());
        desktop
    }
}

#[cfg(not(windows))]
fn desktop() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Desktop"))
}

/// "Create a sample file" on a folder or in its empty space: the overlay opens on the tool.
fn context_menu(shared: &Shared, context: &Context, folder: &Path) -> Result<Value, String> {
    if !folder.is_dir() {
        return Err("folder".to_string());
    }
    *shared
        .asked_folder
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(folder.to_path_buf());
    let events = context.events();
    events
        .set_overlay_visible(true)
        .map_err(|error| error.to_string())?;
    events
        .emit("open", json!({ "folder": folder }))
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

fn main() -> std::io::Result<()> {
    let shared: Rc<OnceCell<Shared>> = Rc::new(OnceCell::new());
    let with = {
        let shared = Rc::clone(&shared);
        move |handler: Handler| {
            let shared = Rc::clone(&shared);
            move |context: &Context, params: Value| {
                handler(
                    shared.get_or_init(|| Shared::start(context)),
                    context,
                    params,
                )
            }
        }
    };
    Module::new()
        .on("preview", with(preview))
        .on("create", with(create))
        .on("freeName", with(free_name))
        .on(
            "takeOpen",
            with(|shared, _, _| {
                let folder = shared
                    .asked_folder
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
                Ok(json!(folder))
            }),
        )
        .on("defaultFolder", with(|_, _, _| Ok(json!(desktop()))))
        .on("pickFolder", with(|_, _, params| Ok(pick_folder(&params))))
        // The interface asks this while files are being made: a call keeps the program running.
        .on(
            "waiting",
            with(|shared, _, _| Ok(json!(shared.jobs.waiting()))),
        )
        .on_context_menu({
            let shared = Rc::clone(&shared);
            move |context, _, folder| {
                context_menu(
                    shared.get_or_init(|| Shared::start(context)),
                    context,
                    folder,
                )
            }
        })
        .run()
}
