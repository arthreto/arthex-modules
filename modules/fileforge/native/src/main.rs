//! The system side of the module: its list of files, `FFmpeg` and the office suites it can call. The
//! interface only shows what this program says, so a right-click works with the page closed.

mod documents;
mod format;
mod media;
mod queue;
mod worker;

use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::Arc,
};

use arthex_module::{Context, Module};
use format::Kind;
use queue::{Mode, Settings, Status};
use serde::Deserialize;
use serde_json::{Value, json};
use worker::Shared;

const SETTINGS_FILE: &str = "settings.json";

struct App {
    shared: Arc<Shared>,
    settings_file: PathBuf,
}

impl App {
    fn start(context: &Context) -> Self {
        let settings_file = context.data_dir().join(SETTINGS_FILE);
        let settings: Settings = fs::read(&settings_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            shared: Shared::new(settings.normalized(), context.events().clone()),
            settings_file,
        }
    }

    /// The whole state: every call answers with it, so the page never has to guess.
    fn answer(&self) -> Result<Value, String> {
        self.shared.publish();
        serde_json::to_value(&*self.shared.queue()).map_err(|error| error.to_string())
    }

    fn add(&self, path: &Path) -> Option<u64> {
        let metadata = fs::metadata(path).ok().filter(fs::Metadata::is_file)?;
        let id = self.shared.queue().add(path, metadata.len())?;
        let is_image = path
            .extension()
            .and_then(|extension| format::kind_of(&extension.to_string_lossy()))
            == Some(Kind::Image);
        if is_image {
            self.shared.want_thumbnail(id, path);
        }
        Some(id)
    }

    fn save_settings(&self) {
        let settings = self.shared.queue().settings.clone();
        if let Ok(bytes) = serde_json::to_vec_pretty(&settings) {
            // Only a convenience: the defaults come back next time.
            let _ = fs::write(&self.settings_file, bytes);
        }
    }
}

fn parse<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

#[derive(Deserialize)]
struct IdParams {
    id: u64,
}

#[derive(Deserialize)]
struct ModeParams {
    mode: Mode,
}

#[derive(Deserialize)]
struct TargetParams {
    id: u64,
    target: String,
}

#[derive(Deserialize)]
struct KindTargetParams {
    kind: Kind,
    target: String,
}

fn state(app: &App, _: Value) -> Result<Value, String> {
    app.answer()
}

#[cfg(windows)]
fn pick_files(app: &App, _: Value) -> Result<Value, String> {
    if let Some(paths) = rfd::FileDialog::new().pick_files() {
        for path in paths {
            app.add(&path);
        }
    }
    app.answer()
}

#[cfg(windows)]
fn pick_folder(app: &App, _: Value) -> Result<Value, String> {
    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
        app.shared.queue().settings.folder = Some(folder.display().to_string());
        app.save_settings();
    }
    app.answer()
}

#[cfg(not(windows))]
fn pick_files(_: &App, _: Value) -> Result<Value, String> {
    Err("FileForge needs Windows".to_string())
}

#[cfg(not(windows))]
fn pick_folder(_: &App, _: Value) -> Result<Value, String> {
    Err("FileForge needs Windows".to_string())
}

fn set_mode(app: &App, params: Value) -> Result<Value, String> {
    let params: ModeParams = parse(params)?;
    let mut queue = app.shared.queue();
    if !queue.is_running() {
        queue.mode = params.mode;
    }
    drop(queue);
    app.answer()
}

fn set_target(app: &App, params: Value) -> Result<Value, String> {
    let params: TargetParams = parse(params)?;
    app.shared.queue().set_target(params.id, &params.target);
    app.answer()
}

fn set_kind_target(app: &App, params: Value) -> Result<Value, String> {
    let params: KindTargetParams = parse(params)?;
    app.shared
        .queue()
        .set_kind_target(params.kind, &params.target);
    app.answer()
}

fn set_settings(app: &App, params: Value) -> Result<Value, String> {
    let settings: Settings = parse(params)?;
    app.shared.queue().settings = settings.normalized();
    app.save_settings();
    app.answer()
}

fn start(app: &App, _: Value) -> Result<Value, String> {
    app.shared.queue().start();
    app.shared.wake();
    app.answer()
}

fn cancel(app: &App, _: Value) -> Result<Value, String> {
    app.shared.cancel();
    app.answer()
}

fn remove(app: &App, params: Value) -> Result<Value, String> {
    let params: IdParams = parse(params)?;
    app.shared.queue().remove(params.id);
    app.answer()
}

fn clear(app: &App, _: Value) -> Result<Value, String> {
    app.shared.queue().clear();
    app.answer()
}

/// Shows a result in Explorer. Only a file this program wrote, never a path from outside.
fn reveal(app: &App, params: Value) -> Result<Value, String> {
    let params: IdParams = parse(params)?;
    let output = app
        .shared
        .queue()
        .items
        .iter()
        .find(|item| item.id == params.id && item.status == Status::Done)
        .and_then(|item| item.output.clone().or_else(|| Some(item.path.clone())));
    if let Some(output) = output {
        open_folder_of(&output);
    }
    Ok(json!(null))
}

#[cfg(windows)]
fn open_folder_of(path: &str) {
    use std::os::windows::process::CommandExt;
    // Explorer needs its own quoting: `/select,"C:\a b.png"` as one piece.
    let _ = Command::new("explorer")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn();
}

#[cfg(not(windows))]
fn open_folder_of(path: &str) {
    let folder = Path::new(path).parent().unwrap_or(Path::new("."));
    let _ = Command::new("xdg-open").arg(folder).spawn();
}

/// `compress` or `convert-<type>:<format>`, on a file Arthex has checked.
fn context_menu(app: &App, action: &str, path: &Path) -> Result<Value, String> {
    let id = app.add(path).ok_or("not a file FileForge knows")?;
    let target = action.split_once(':').map(|(_, target)| target);
    let mode = if action == "compress" {
        Mode::Compress
    } else {
        Mode::Convert
    };
    app.shared.queue().queue_now(id, mode, target);
    app.shared.wake();
    app.shared.ask_to_open();
    app.answer()
}

type Handler = fn(&App, Value) -> Result<Value, String>;

fn main() -> std::io::Result<()> {
    let app: Rc<RefCell<Option<Rc<App>>>> = Rc::default();
    let ready = |app: &Rc<RefCell<Option<Rc<App>>>>| {
        app.borrow()
            .clone()
            .ok_or_else(|| "the program has not started".to_string())
    };
    let starting = Rc::clone(&app);
    let mut module = Module::new().on_start(move |context| {
        *starting.borrow_mut() = Some(Rc::new(App::start(context)));
    });
    let handlers: [(&str, Handler); 14] = [
        ("thumbnails", |app, _| Ok(json!(app.shared.thumbnails()))),
        // Whether a right-click happened before the interface could hear about it, once.
        ("takeOpen", |app, _| Ok(json!(app.shared.take_open()))),
        ("state", state),
        ("pickFiles", pick_files),
        ("pickFolder", pick_folder),
        ("setMode", set_mode),
        ("setTarget", set_target),
        ("setKindTarget", set_kind_target),
        ("setSettings", set_settings),
        ("start", start),
        ("cancel", cancel),
        ("remove", remove),
        ("clear", clear),
        ("reveal", reveal),
    ];
    for (name, handler) in handlers {
        let app = Rc::clone(&app);
        module = module.on(name, move |_, params| handler(&*ready(&app)?, params));
    }
    let clicked = Rc::clone(&app);
    let closing = Rc::clone(&app);
    let served = module
        .on_context_menu(move |_, action, path| context_menu(&*ready(&clicked)?, action, path))
        .run();
    if let Some(app) = closing.borrow().as_ref() {
        app.shared.stop_current();
    }
    served
}
