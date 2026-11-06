//! The system side of the Binder module: keeps the documents, copies what they show and makes PDFs.

mod library;
#[cfg_attr(not(windows), allow(dead_code))]
mod media;
#[cfg(windows)]
mod pdf;

#[cfg(windows)]
use std::path::{Path, PathBuf};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use arthex_module::{Context, Module};
use base64::Engine;
use library::Library;
use media::Media;
#[cfg(windows)]
use media::{Kind, is_web_address};
use serde::Deserialize;
use serde_json::{Value, json};

struct App {
    library: RefCell<Library>,
    media: Media,
    #[cfg(windows)]
    folder: PathBuf,
}

impl App {
    fn start(context: &Context) -> Self {
        let folder = context.data_dir();
        let library = Library::load(folder);
        let media = Media::new(folder);
        let documents = library.documents();
        let texts: Vec<&str> = documents
            .iter()
            .map(|document| document.text.as_str())
            .collect();
        media.prune(&texts);
        Self {
            library: RefCell::new(library),
            media,
            #[cfg(windows)]
            folder: folder.to_path_buf(),
        }
    }
}

#[derive(Deserialize)]
struct SavedDocument {
    id: String,
    title: String,
    category: Option<String>,
    text: String,
}

#[derive(Deserialize)]
struct MovedDocument {
    id: String,
    category: Option<String>,
}

#[derive(Deserialize)]
struct Id {
    id: String,
}

#[derive(Deserialize)]
struct SavedCategory {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct FoldedCategory {
    id: String,
    folded: bool,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct PickedKind {
    kind: Kind,
}

#[derive(Deserialize)]
struct PastedImage {
    data: String,
    extension: String,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct MediaFile {
    file: String,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct Link {
    url: String,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct Export {
    title: String,
    html: String,
}

fn parsed<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| u64::try_from(time.as_millis()).unwrap_or(0))
}

#[allow(clippy::unnecessary_wraps)] // Every handler has this signature.
fn state(app: &App, _: Value) -> Result<Value, String> {
    let library = app.library.borrow();
    Ok(json!({
        "categories": library.categories(),
        "documents": library.documents(),
        "sizes": app.media.sizes(),
    }))
}

fn save_document(app: &App, params: Value) -> Result<Value, String> {
    let params: SavedDocument = parsed(params)?;
    let updated = app.library.borrow_mut().save_document(
        &params.id,
        &params.title,
        params.category.as_deref(),
        &params.text,
        now(),
    )?;
    Ok(json!(updated))
}

fn delete_document(app: &App, params: Value) -> Result<Value, String> {
    let params: Id = parsed(params)?;
    app.library.borrow_mut().remove_document(&params.id)?;
    Ok(Value::Null)
}

fn move_document(app: &App, params: Value) -> Result<Value, String> {
    let params: MovedDocument = parsed(params)?;
    app.library
        .borrow_mut()
        .move_document(&params.id, params.category.as_deref())?;
    Ok(Value::Null)
}

fn save_category(app: &App, params: Value) -> Result<Value, String> {
    let params: SavedCategory = parsed(params)?;
    app.library
        .borrow_mut()
        .save_category(&params.id, &params.name)?;
    Ok(Value::Null)
}

fn delete_category(app: &App, params: Value) -> Result<Value, String> {
    let params: Id = parsed(params)?;
    app.library.borrow_mut().remove_category(&params.id)?;
    Ok(Value::Null)
}

fn fold_category(app: &App, params: Value) -> Result<Value, String> {
    let params: FoldedCategory = parsed(params)?;
    app.library
        .borrow_mut()
        .fold_category(&params.id, params.folded)?;
    Ok(Value::Null)
}

fn paste_image(app: &App, params: Value) -> Result<Value, String> {
    let params: PastedImage = parsed(params)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(params.data)
        .map_err(|error| error.to_string())?;
    let imported = app.media.save_image(&bytes, &params.extension)?;
    serde_json::to_value(imported).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn pick_media(app: &App, params: Value) -> Result<Value, String> {
    let params: PickedKind = parsed(params)?;
    let Some(source) = rfd::FileDialog::new()
        .add_filter("", params.kind.extensions())
        .pick_file()
    else {
        return Ok(Value::Null);
    };
    let imported = app.media.import(&source, params.kind)?;
    serde_json::to_value(imported).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn open_with_system(target: &str) -> Result<(), String> {
    use windows::{
        Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL},
        core::{HSTRING, PCWSTR, w},
    };
    // SAFETY: the strings outlive the call; what is opened was checked by the callers.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(target),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Below this value, it is an error code.
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err("could not open it".to_string())
    }
}

#[cfg(windows)]
fn open_file(app: &App, params: Value) -> Result<Value, String> {
    let params: MediaFile = parsed(params)?;
    let path = app.media.path_of(&params.file).ok_or("unknown file")?;
    open_with_system(&path.to_string_lossy())?;
    Ok(Value::Null)
}

#[cfg(windows)]
fn open_link(_: &App, params: Value) -> Result<Value, String> {
    let params: Link = parsed(params)?;
    if !is_web_address(&params.url) {
        return Err("not an address of the web".to_string());
    }
    open_with_system(&params.url)?;
    Ok(Value::Null)
}

/// What a file name cannot hold on Windows.
#[cfg_attr(not(windows), allow(dead_code))]
fn file_name_of(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                ' '
            } else {
                character
            }
        })
        .collect();
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() {
        "document.pdf".to_string()
    } else {
        format!("{name}.pdf")
    }
}

#[cfg(windows)]
fn export_pdf(app: &App, params: Value) -> Result<Value, String> {
    let params: Export = parsed(params)?;
    let Some(output) = rfd::FileDialog::new()
        .set_file_name(file_name_of(&params.title))
        .add_filter("PDF", &["pdf"])
        .save_file()
    else {
        return Ok(Value::Null);
    };
    print_page(&app.folder, &params.html, &output)?;
    Ok(json!(
        output
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    ))
}

/// The page sits next to `media/`, so that its images are found.
#[cfg(windows)]
fn print_page(folder: &Path, html: &str, output: &Path) -> Result<(), String> {
    let page = folder.join("print.html");
    std::fs::write(&page, html).map_err(|error| error.to_string())?;
    let result = pdf::print(&page, output, &folder.join("webview"));
    let _ = std::fs::remove_file(&page);
    result
}

#[cfg(not(windows))]
fn only_on_windows(_: &App, _: Value) -> Result<Value, String> {
    Err("only on Windows".to_string())
}

type Handler = fn(&App, Value) -> Result<Value, String>;

fn main() -> std::io::Result<()> {
    #[cfg(not(windows))]
    let (pick_media, open_file, open_link, export_pdf): (Handler, Handler, Handler, Handler) = (
        only_on_windows,
        only_on_windows,
        only_on_windows,
        only_on_windows,
    );
    let handlers: [(&str, Handler); 12] = [
        ("state", state),
        ("saveDocument", save_document),
        ("deleteDocument", delete_document),
        ("moveDocument", move_document),
        ("saveCategory", save_category),
        ("deleteCategory", delete_category),
        ("foldCategory", fold_category),
        ("pasteImage", paste_image),
        ("pickMedia", pick_media),
        ("openFile", open_file),
        ("openLink", open_link),
        ("exportPdf", export_pdf),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_becomes_a_file_name_windows_accepts() {
        assert_eq!(file_name_of("Chapitre 2"), "Chapitre 2.pdf");
        assert_eq!(file_name_of("TD: 1/2 ?"), "TD  1 2.pdf");
        assert_eq!(file_name_of("  ..."), "document.pdf");
        assert_eq!(file_name_of(""), "document.pdf");
    }
}
