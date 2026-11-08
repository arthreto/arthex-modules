mod secret;
mod shell;

use std::{
    cell::OnceCell,
    collections::HashMap,
    fs,
    path::PathBuf,
    ptr,
    rc::Rc,
    sync::{Mutex, PoisonError},
    thread,
    time::Duration,
};

use arthex_module::{Context, Module};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use ureq::{
    Agent,
    tls::{TlsConfig, TlsProvider},
};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
};

use crate::request::{API, api_url, image_type, image_url, is_openable};

const KEY_FILE: &str = "key.bin";
const TIMEOUT: Duration = Duration::from_secs(15);
/// Images are fetched a few at a time: a page of posters arrives together, without flooding TMDB.
const IMAGES_AT_ONCE: usize = 6;
/// What TMDB answers for a key it does not know.
const UNAUTHORIZED: u16 = 401;

struct Shared {
    agent: Agent,
    key_file: PathBuf,
    key: Mutex<Option<String>>,
    images: Mutex<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct KeyParams {
    key: String,
}

#[derive(Deserialize)]
struct GetParams {
    path: String,
    #[serde(default)]
    params: Map<String, Value>,
}

#[derive(Deserialize)]
struct ImagesParams {
    paths: Vec<String>,
    size: String,
}

#[derive(Deserialize)]
struct OpenParams {
    address: String,
}

fn parse<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

impl Shared {
    fn start(context: &Context) -> Self {
        let key_file = context.data_dir().join(KEY_FILE);
        let key = fs::read(&key_file)
            .ok()
            .and_then(|bytes| secret::unprotect(&bytes))
            .and_then(|bytes| String::from_utf8(bytes).ok());
        let agent = Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            // Windows' own TLS: no other certificates to trust, and a smaller program.
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::NativeTls)
                    .build(),
            )
            .build()
            .into();
        Self {
            agent,
            key_file,
            key: Mutex::new(key),
            images: Mutex::new(HashMap::new()),
        }
    }

    fn key(&self) -> Option<String> {
        self.key
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The answer of TMDB to an address of `request`, with the errors the interface tells apart.
    fn fetch(&self, url: &str, key: &str) -> Result<Value, String> {
        let mut response = self
            .agent
            .get(url)
            .header("Authorization", &format!("Bearer {key}"))
            .header("Accept", "application/json")
            .call()
            .map_err(|_| "offline".to_string())?;
        match response.status().as_u16() {
            UNAUTHORIZED => Err("refused".to_string()),
            200..=299 => response
                .body_mut()
                .read_json()
                .map_err(|error| error.to_string()),
            status => Err(format!("TMDB answered {status}")),
        }
    }

    fn image(&self, path: &str, size: &str) -> Option<String> {
        let url = image_url(path, size).ok()?;
        let mut response = self.agent.get(&url).call().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = response.body_mut().read_to_vec().ok()?;
        Some(format!(
            "data:{};base64,{}",
            image_type(path),
            STANDARD.encode(bytes)
        ))
    }
}

#[allow(clippy::unnecessary_wraps)] // Every handler has the same signature.
fn has_key(shared: &Shared, _: Value) -> Result<Value, String> {
    Ok(json!(shared.key().is_some()))
}

/// Keeps a key once TMDB has accepted it.
fn save_key(shared: &Shared, params: Value) -> Result<Value, String> {
    let key = parse::<KeyParams>(params)?.key.trim().to_string();
    if key.is_empty()
        || key
            .chars()
            .any(|char| char.is_control() || char.is_whitespace())
    {
        return Err("refused".to_string());
    }
    shared.fetch(&format!("{API}/configuration"), &key)?;
    let encrypted = secret::protect(key.as_bytes()).ok_or("could not encrypt the key")?;
    fs::write(&shared.key_file, encrypted).map_err(|error| error.to_string())?;
    *shared.key.lock().unwrap_or_else(PoisonError::into_inner) = Some(key);
    Ok(Value::Null)
}

fn remove_key(shared: &Shared, _: Value) -> Result<Value, String> {
    *shared.key.lock().unwrap_or_else(PoisonError::into_inner) = None;
    match fs::remove_file(&shared.key_file) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(Value::Null),
    }
}

fn get(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: GetParams = parse(params)?;
    let url = api_url(&params.path, &params.params)?;
    let key = shared.key().ok_or("no-key")?;
    shared.fetch(&url, &key)
}

/// Each image as a `data:` address the window may show, keyed by its path; a missing one is left out.
fn images(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: ImagesParams = parse(params)?;
    let mut found = Map::new();
    let mut missing = Vec::new();
    {
        let cached = shared.images.lock().unwrap_or_else(PoisonError::into_inner);
        for path in params.paths {
            let cache_key = format!("{}{path}", params.size);
            match cached.get(&cache_key) {
                Some(image) => {
                    found.insert(path, json!(image));
                }
                None if !missing.contains(&path) => missing.push(path),
                None => {}
            }
        }
    }
    for group in missing.chunks(IMAGES_AT_ONCE) {
        let fetched: Vec<(String, Option<String>)> = thread::scope(|scope| {
            let workers: Vec<_> = group
                .iter()
                .map(|path| scope.spawn(|| (path.clone(), shared.image(path, &params.size))))
                .collect();
            workers
                .into_iter()
                .filter_map(|worker| worker.join().ok())
                .collect()
        });
        let mut cached = shared.images.lock().unwrap_or_else(PoisonError::into_inner);
        for (path, image) in fetched {
            if let Some(image) = image {
                cached.insert(format!("{}{path}", params.size), image.clone());
                found.insert(path, json!(image));
            }
        }
    }
    Ok(Value::Object(found))
}

fn open(_: &Shared, params: Value) -> Result<Value, String> {
    let address = parse::<OpenParams>(params)?.address;
    if !is_openable(&address) {
        return Err("this address cannot be opened".to_string());
    }
    shell::open(&address)?;
    Ok(Value::Null)
}

type Handler = fn(&Shared, Value) -> Result<Value, String>;

pub fn run() -> std::io::Result<()> {
    // SAFETY: once, before anything else; opening an address through the shell expects a
    // single-threaded apartment.
    unsafe {
        CoInitializeEx(
            ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).cast_unsigned(),
        );
    }
    let shared: Rc<OnceCell<Shared>> = Rc::default();
    let starting = Rc::clone(&shared);
    let mut module = Module::new().on_start(move |context| {
        let _ = starting.set(Shared::start(context));
    });
    let handlers: [(&str, Handler); 6] = [
        ("hasKey", has_key),
        ("saveKey", save_key),
        ("removeKey", remove_key),
        ("get", get),
        ("images", images),
        ("open", open),
    ];
    for (name, handler) in handlers {
        let shared = Rc::clone(&shared);
        module = module.on(name, move |_, params| {
            handler(shared.get().ok_or("the program has not started")?, params)
        });
    }
    module.run()
}
