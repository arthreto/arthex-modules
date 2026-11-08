//! The system side of Hash: the digests of a text or a file, and password hashes.
//!
//! Nothing is written to disk and nothing leaves the computer. A file is only read.

mod digests;
mod files;
mod passwords;

use std::{
    cell::OnceCell,
    path::Path,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use arthex_module::{Context, Events, Module};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{digests::Digests, files::Files, passwords::Kind};

struct Shared {
    files: Files,
    open_requested: AtomicBool,
    events: Events,
}

#[derive(Deserialize)]
struct DigestParams {
    text: String,
    key: Option<String>,
}

#[derive(Deserialize)]
struct HashParams {
    kind: Kind,
    password: String,
}

#[derive(Deserialize)]
struct VerifyParams {
    password: String,
    hash: String,
}

fn parse<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn answer(value: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

fn digest(_: &Shared, params: Value) -> Result<Value, String> {
    let params: DigestParams = parse(params)?;
    let mut digests = Digests::new(params.key.as_deref().map(str::as_bytes));
    digests.feed(params.text.as_bytes());
    answer(
        digests
            .finish()
            .into_iter()
            .map(|(id, hex)| json!({ "id": id, "hex": hex }))
            .collect::<Vec<_>>(),
    )
}

#[cfg(windows)]
fn pick_file(shared: &Shared, _: Value) -> Result<Value, String> {
    let Some(path) = rfd::FileDialog::new().pick_file() else {
        return Ok(Value::Null);
    };
    answer(shared.files.start(&path)?)
}

#[cfg(not(windows))]
fn pick_file(_: &Shared, _: Value) -> Result<Value, String> {
    Err("only on Windows".to_string())
}

#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn file_state(shared: &Shared, _: Value) -> Result<Value, String> {
    answer(shared.files.current())
}

#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn cancel_file(shared: &Shared, _: Value) -> Result<Value, String> {
    shared.files.cancel();
    Ok(Value::Null)
}

fn password_hash(_: &Shared, params: Value) -> Result<Value, String> {
    let params: HashParams = parse(params)?;
    answer(passwords::hash(params.kind, &params.password)?)
}

fn password_verify(_: &Shared, params: Value) -> Result<Value, String> {
    let params: VerifyParams = parse(params)?;
    answer(passwords::verify(&params.password, &params.hash)?)
}

/// Whether a right-click asked for the tool before the interface could hear about it.
#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn take_open(shared: &Shared, _: Value) -> Result<Value, String> {
    Ok(json!(shared.open_requested.swap(false, Ordering::Relaxed)))
}

/// A click in the right-click menu, on a file Arthex has checked: its digests, in the tool.
fn context_menu(shared: &Shared, path: &Path) -> Result<Value, String> {
    shared.files.start(path)?;
    shared.open_requested.store(true, Ordering::Relaxed);
    // Arthex is closing if nobody listens any more.
    let _ = shared.events.set_overlay_visible(true);
    let _ = shared.events.emit("open", ());
    Ok(Value::Null)
}

type Handler = fn(&Shared, Value) -> Result<Value, String>;

fn main() -> std::io::Result<()> {
    let shared: Rc<OnceCell<Arc<Shared>>> = Rc::default();
    let starting = Rc::clone(&shared);
    let mut module = Module::new().on_start(move |context: &Context| {
        let _ = starting.set(Arc::new(Shared {
            files: Files::new(context.events().clone()),
            open_requested: AtomicBool::new(false),
            events: context.events().clone(),
        }));
    });
    let handlers: [(&str, Handler); 7] = [
        ("digest", digest),
        ("pickFile", pick_file),
        ("fileState", file_state),
        ("cancelFile", cancel_file),
        ("passwordHash", password_hash),
        ("passwordVerify", password_verify),
        ("takeOpen", take_open),
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
