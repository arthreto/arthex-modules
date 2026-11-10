//! The system side of the TTS module: downloads the model of the voices once, turns texts into
//! voices on this computer, and keeps them in the Music folder.

mod model;
mod save;
mod sound;
mod speech;
mod voices;
mod wav;

use std::{
    cell::OnceCell,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
};

use arthex_module::{Context, Module};
use base64::Engine as _;
use model::Model;
use serde::Deserialize;
use serde_json::{Value, json};
use speech::{Engine, Request, Spoken};

#[derive(Default)]
struct Speaking {
    running: AtomicBool,
    percent: Arc<AtomicU8>,
    cancelled: Arc<AtomicBool>,
    failed: AtomicBool,
    engine: Mutex<Option<Engine>>,
    spoken: Mutex<Option<Spoken>>,
}

struct Shared {
    model: Arc<Model>,
    speaking: Arc<Speaking>,
    ffmpeg: PathBuf,
    scratch: PathBuf,
    french: bool,
}

impl Shared {
    fn start(context: &Context) -> Self {
        let ffmpeg = context.shared("ffmpeg").map_or_else(
            || PathBuf::from("ffmpeg"),
            |folder| folder.join("ffmpeg.exe"),
        );
        Self {
            model: Arc::new(Model::new(&context.data_dir().join("voices"))),
            speaking: Arc::new(Speaking::default()),
            ffmpeg,
            scratch: context.data_dir().to_path_buf(),
            french: context.locale().starts_with("fr"),
        }
    }
}

type Handler = fn(&Shared, Value) -> Result<Value, String>;

/// Asked again and again by the interface while something runs: a call keeps the program alive.
fn status(shared: &Shared) -> Value {
    let speaking = &shared.speaking;
    let seconds = speaking
        .spoken
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|spoken| {
            f64::from(u32::try_from(spoken.samples.len()).unwrap_or(u32::MAX))
                / f64::from(spoken.rate)
        });
    json!({
        "model": shared.model.status(),
        "speaking": speaking.running.load(Ordering::Relaxed),
        "percent": speaking.percent.load(Ordering::Relaxed),
        "failed": speaking.failed.swap(false, Ordering::Relaxed),
        "seconds": seconds,
    })
}

fn speak(shared: &Shared, params: Value) -> Result<Value, String> {
    let request = serde_json::from_value::<Request>(params)
        .map_err(|error| error.to_string())?
        .checked()?;
    if !shared.model.is_ready() {
        return Err("model".to_string());
    }
    let speaking = Arc::clone(&shared.speaking);
    if speaking.running.swap(true, Ordering::Relaxed) {
        return Err("busy".to_string());
    }
    speaking.percent.store(0, Ordering::Relaxed);
    speaking.cancelled.store(false, Ordering::Relaxed);
    speaking.failed.store(false, Ordering::Relaxed);
    *speaking
        .spoken
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    let folder = shared.model.folder().to_path_buf();
    thread::spawn(move || {
        let mut engine = speaking
            .engine
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if engine.is_none() {
            *engine = Engine::load(&folder);
        }
        let spoken = engine
            .as_ref()
            .and_then(|engine| engine.speak(&request, &speaking.percent, &speaking.cancelled));
        let cancelled = speaking.cancelled.load(Ordering::Relaxed);
        if spoken.is_none() && !cancelled {
            speaking.failed.store(true, Ordering::Relaxed);
        }
        *speaking
            .spoken
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = spoken;
        speaking.running.store(false, Ordering::Relaxed);
    });
    Ok(Value::Null)
}

/// The last voice as a WAV file in base64, for the player of the interface.
fn audio(shared: &Shared, _: Value) -> Result<Value, String> {
    let spoken = shared
        .speaking
        .spoken
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let spoken = spoken.as_ref().ok_or("nothing")?;
    let wave = wav::encode(&spoken.samples, spoken.rate);
    Ok(json!(
        base64::engine::general_purpose::STANDARD.encode(wave)
    ))
}

#[derive(Deserialize)]
struct SaveParams {
    format: String,
}

fn keep(shared: &Shared, params: Value) -> Result<Value, String> {
    let SaveParams { format } =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    let spoken = shared
        .speaking
        .spoken
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let spoken = spoken.as_ref().ok_or("nothing")?;
    let folder = save::music_folder().ok_or("folder")?;
    let path = save::save(
        &spoken.samples,
        spoken.rate,
        &format,
        &folder,
        if shared.french { "voix" } else { "voice" },
        &shared.ffmpeg,
        &shared.scratch,
    )?;
    Ok(json!(path.file_name().map(|name| name.to_string_lossy())))
}

fn main() -> std::io::Result<()> {
    let shared: Rc<OnceCell<Shared>> = Rc::new(OnceCell::new());
    let with = {
        let shared = Rc::clone(&shared);
        move |handler: Handler| {
            let shared = Rc::clone(&shared);
            move |context: &Context, params: Value| {
                handler(shared.get_or_init(|| Shared::start(context)), params)
            }
        }
    };
    Module::new()
        .on("status", with(|shared, _| Ok(status(shared))))
        .on(
            "download",
            with(|shared, _| {
                shared.model.download();
                Ok(Value::Null)
            }),
        )
        .on("speak", with(speak))
        .on(
            "cancel",
            with(|shared, _| {
                shared.model.cancel();
                shared.speaking.cancelled.store(true, Ordering::Relaxed);
                Ok(Value::Null)
            }),
        )
        .on("audio", with(audio))
        .on("save", with(keep))
        .run()
}
