mod canvas;
mod selection;
mod shell;
mod wifi;

use std::{env, ffi::OsStr, fs, path::Path, ptr};

use arthex_module::Module;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windows_sys::Win32::{
    System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx},
    UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
};

use crate::{address, wifi_profile::Security};

#[derive(Deserialize)]
struct SaveParams {
    name: String,
    base64: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct OpenParams {
    address: String,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum FileKind {
    Contact,
    Event,
}

#[derive(Deserialize)]
struct OpenFileParams {
    kind: FileKind,
    content: String,
}

#[derive(Deserialize)]
struct Network {
    ssid: String,
    password: String,
    security: Security,
    hidden: bool,
}

fn parse<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn answer(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

/// Traces a zone on the screen and gives it as a PNG, or nothing when the user cancels. Arthex
/// hides the overlay around the call.
fn select(_: Value) -> Result<Value, String> {
    let Some(image) = selection::run() else {
        return Ok(Value::Null);
    };
    answer(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(image.png()?)
    ))
}

/// Asks where to save a code, then writes it; the name of the file, or nothing when cancelled.
fn save(params: Value) -> Result<Value, String> {
    let params: SaveParams = parse(params)?;
    let name = Path::new(&params.name)
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or("no file name")?;
    let (label, extension) = match Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => ("PNG", "png"),
        Some("svg") => ("SVG", "svg"),
        _ => return Err("only PNG and SVG are saved".to_string()),
    };
    let bytes = match (params.base64, params.text) {
        (Some(base64), _) => STANDARD.decode(base64).map_err(|error| error.to_string())?,
        (None, Some(text)) => text.into_bytes(),
        (None, None) => return Err("nothing to save".to_string()),
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(name)
        .add_filter(label, &[extension])
        .save_file()
    else {
        return Ok(Value::Null);
    };
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    answer(path.file_name().and_then(OsStr::to_str).unwrap_or(name))
}

fn open(params: Value) -> Result<Value, String> {
    let params: OpenParams = parse(params)?;
    shell::open(address::checked(&params.address)?)?;
    Ok(Value::Null)
}

/// A contact or an event goes to the program that opens such files, as from Explorer. The file is
/// written in the temporary folder, under a fixed name, so that only the last one stays.
fn open_file(params: Value) -> Result<Value, String> {
    let params: OpenFileParams = parse(params)?;
    let folder = env::temp_dir().join("Arthex QR Code");
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let path = folder.join(match params.kind {
        FileKind::Contact => "contact.vcf",
        FileKind::Event => "event.ics",
    });
    fs::write(&path, params.content).map_err(|error| error.to_string())?;
    shell::open_file(&path)?;
    Ok(Value::Null)
}

fn connect(params: Value) -> Result<Value, String> {
    let network: Network = parse(params)?;
    wifi::connect(
        &network.ssid,
        &network.password,
        network.security,
        network.hidden,
    )?;
    Ok(Value::Null)
}

type Handler = fn(Value) -> Result<Value, String>;

pub fn run() -> std::io::Result<()> {
    // A program is scaled by Windows unless it says otherwise: on a screen at 125 %, the cursor, the
    // windows and the copy of the screen would not line up, and the wrong zone would be read.
    // SAFETY: changes how Windows treats this process, once, before it opens a window. Opening an
    // address through the shell expects a single-threaded apartment.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(
            ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).cast_unsigned(),
        );
    }
    let handlers: [(&str, Handler); 5] = [
        ("select", select),
        ("save", save),
        ("open", open),
        ("openFile", open_file),
        ("connect", connect),
    ];
    let mut module = Module::new();
    for (name, handler) in handlers {
        module = module.on(name, move |_, params| handler(params));
    }
    module.run()
}
