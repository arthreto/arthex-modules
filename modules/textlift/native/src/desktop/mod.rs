mod bubble;
mod canvas;
mod clipboard;
mod ocr;
mod selection;

use std::{
    cell::OnceCell,
    fs,
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

use arthex_module::{Context, Events, Module};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use windows_sys::Win32::{
    System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
    UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
};

use crate::bitmap::Bitmap;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Serialize, Deserialize)]
struct Settings {
    propose: bool,
}

/// What the calls and the bubble's thread have in common.
pub struct Shared {
    image: Mutex<Option<Bitmap>>,
    propose: AtomicBool,
    open_requested: AtomicBool,
    events: Events,
    settings_file: PathBuf,
}

impl Shared {
    fn start(context: &Context) -> Self {
        let settings_file = context.data_dir().join(SETTINGS_FILE);
        let propose = fs::read(&settings_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
            .is_none_or(|settings| settings.propose);
        Self {
            image: Mutex::new(None),
            propose: AtomicBool::new(propose),
            open_requested: AtomicBool::new(false),
            events: context.events().clone(),
            settings_file,
        }
    }

    fn set_image(&self, image: Bitmap) {
        *self.image.lock().unwrap_or_else(PoisonError::into_inner) = Some(image);
    }

    /// Opens the tool. At a cold start nobody hears the event: the interface asks `takeOpen`.
    pub fn show_image(&self, image: Bitmap) {
        self.set_image(image);
        self.open_requested.store(true, Ordering::Relaxed);
        // Arthex is closing if nobody listens any more.
        let _ = self.events.set_overlay_visible(true);
        let _ = self.events.emit("open", ());
    }

    pub fn proposes(&self) -> bool {
        self.propose.load(Ordering::Relaxed)
    }

    fn set_proposes(&self, propose: bool) {
        self.propose.store(propose, Ordering::Relaxed);
        if let Ok(bytes) = serde_json::to_vec(&Settings { propose }) {
            // Only a convenience: the default comes back next time.
            let _ = fs::write(&self.settings_file, bytes);
        }
    }

    fn image(&self) -> Option<Bitmap> {
        self.image
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[derive(Deserialize)]
struct ReadParams {
    language: Option<String>,
}

#[derive(Deserialize)]
struct ProposeParams {
    propose: bool,
}

fn parse<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn answer(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

/// Traces a zone on the screen; whether one was chosen. Arthex hides the overlay around the call.
#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn select(shared: &Shared, _: Value) -> Result<Value, String> {
    let Some(image) = selection::run() else {
        return Ok(json!(false));
    };
    shared.set_image(image);
    Ok(json!(true))
}

fn open_image(shared: &Shared, _: Value) -> Result<Value, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Images",
            &[
                "png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "heic",
            ],
        )
        .pick_file()
    else {
        return Ok(json!(false));
    };
    shared.set_image(ocr::load(&path.display().to_string())?);
    Ok(json!(true))
}

fn image(shared: &Shared, _: Value) -> Result<Value, String> {
    let Some(image) = shared.image() else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "src": ocr::data_url(&image)?,
        "width": image.width,
        "height": image.height,
    }))
}

fn read(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: ReadParams = parse(params)?;
    let image = shared.image().ok_or("no image")?;
    answer(ocr::read(&image, params.language.as_deref())?)
}

fn languages(_: &Shared, _: Value) -> Result<Value, String> {
    answer(ocr::languages()?)
}

fn settings(shared: &Shared, _: Value) -> Result<Value, String> {
    answer(Settings {
        propose: shared.proposes(),
    })
}

fn set_settings(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: ProposeParams = parse(params)?;
    shared.set_proposes(params.propose);
    settings(shared, Value::Null)
}

/// Whether a right-click or the bubble asked for the tool before the interface could hear about it.
#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn take_open(shared: &Shared, _: Value) -> Result<Value, String> {
    Ok(json!(shared.open_requested.swap(false, Ordering::Relaxed)))
}

/// A click in the right-click menu, on a path Arthex has checked.
fn context_menu(shared: &Shared, path: &Path) -> Result<Value, String> {
    shared.show_image(ocr::load(&path.display().to_string())?);
    Ok(Value::Null)
}

fn bubble_label(locale: &str) -> String {
    if locale.starts_with("fr") {
        "Récupérer le texte".to_string()
    } else {
        "Get text".to_string()
    }
}

type Handler = fn(&Shared, Value) -> Result<Value, String>;

pub fn run() -> std::io::Result<()> {
    // A program is scaled by Windows unless it says otherwise: on a screen at 125 %, the cursor, the
    // windows and the copy of the screen would not line up, and the wrong zone would be read.
    // SAFETY: changes how Windows treats this process, once, before it opens a window. The
    // recognition waits on its results here: a multithreaded apartment is what allows that.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(ptr::null(), COINIT_MULTITHREADED.cast_unsigned());
    }
    let shared: Rc<OnceCell<Arc<Shared>>> = Rc::default();
    let starting = Rc::clone(&shared);
    let mut module = Module::new().on_start(move |context| {
        let started = Arc::new(Shared::start(context));
        bubble::start(Arc::clone(&started), bubble_label(context.locale()));
        let _ = starting.set(started);
    });
    let handlers: [(&str, Handler); 8] = [
        ("takeOpen", take_open),
        ("select", select),
        ("openImage", open_image),
        ("image", image),
        ("read", read),
        ("languages", languages),
        ("settings", settings),
        ("setSettings", set_settings),
    ];
    for (name, handler) in handlers {
        let shared = Rc::clone(&shared);
        module = module.on(name, move |_, params| {
            handler(shared.get().ok_or("the program has not started")?, params)
        });
    }
    let clicked = Rc::clone(&shared);
    module
        .on_context_menu(move |_, _, path| {
            context_menu(clicked.get().ok_or("the program has not started")?, path)
        })
        .run()
}
