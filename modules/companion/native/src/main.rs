//! The system side of the Companion module: keeps the mascot, brings it on the screen now and then,
//! and runs the Flair game from its large file of words, which is only downloaded on request.

#[cfg(windows)]
mod canvas;
#[cfg(windows)]
mod draw;
mod flair;
#[cfg_attr(not(windows), allow(dead_code))]
mod places;
mod progress;
#[cfg(windows)]
mod screen;

use std::{
    cell::OnceCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{SystemTime, UNIX_EPOCH},
};

use arthex_module::{Context, Events, Module};
use flair::{Guess, Library, Round};
use progress::{Frequency, Gain, Game, PET_GOAL, Progress, Species, word_reward};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What the calls and the thread of the screen have in common.
pub struct Shared {
    progress: Mutex<Progress>,
    folder: PathBuf,
    library: Arc<Library>,
    round: Mutex<Option<Round>>,
    events: Events,
    #[cfg(windows)]
    french: bool,
    #[cfg(windows)]
    screen: screen::Remote,
}

impl Shared {
    fn start(context: &Context) -> Self {
        Self {
            progress: Mutex::new(Progress::load(context.data_dir())),
            folder: context.data_dir().to_path_buf(),
            library: Arc::new(Library::new(context.data_dir(), context.locale())),
            round: Mutex::new(None),
            events: context.events().clone(),
            #[cfg(windows)]
            french: context.locale().starts_with("fr"),
            #[cfg(windows)]
            screen: screen::Remote::default(),
        }
    }

    fn progress(&self) -> MutexGuard<'_, Progress> {
        self.progress.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn round(&self) -> MutexGuard<'_, Option<Round>> {
        self.round.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn saved(&self, progress: &Progress) -> Result<(), String> {
        progress.save(&self.folder)
    }
}

/// What the interface shows of the mascot.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    #[serde(flatten)]
    progress: Progress,
    xp_needed: u32,
    pets_today: u32,
    pet_goal: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    view: View,
    gain: Option<Gain>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Today {
    day: i64,
}

#[derive(Deserialize)]
struct Adoption {
    species: Species,
    name: String,
    day: i64,
}

#[derive(Deserialize)]
struct Finished {
    #[serde(flatten)]
    game: Game,
    day: i64,
}

#[derive(Deserialize)]
struct Item {
    item: Option<String>,
    day: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScreenSettings {
    on_screen: bool,
    frequency: Frequency,
    day: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Motion {
    reduced_motion: bool,
}

#[derive(Deserialize)]
struct Word {
    word: String,
    day: i64,
}

fn parsed<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}

fn answer(progress: &mut Progress, day: i64, gain: Option<Gain>) -> Result<Value, String> {
    serde_json::to_value(Answer {
        view: View {
            pets_today: progress.pets_today(day),
            pet_goal: PET_GOAL,
            xp_needed: progress.xp_needed(),
            progress: progress.clone(),
        },
        gain,
    })
    .map_err(|error| error.to_string())
}

fn state(shared: &Shared, params: Value) -> Result<Value, String> {
    let Today { day } = parsed(params)?;
    answer(&mut shared.progress(), day, None)
}

fn adopt(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Adoption = parsed(params)?;
    let mut progress = shared.progress();
    progress.adopt(params.species, &params.name)?;
    shared.saved(&progress)?;
    #[cfg(windows)]
    shared.screen.changed();
    answer(&mut progress, params.day, None)
}

fn tap(shared: &Shared, params: Value) -> Result<Value, String> {
    let Today { day } = parsed(params)?;
    let mut progress = shared.progress();
    let gain = progress.tap();
    shared.saved(&progress)?;
    answer(&mut progress, day, Some(gain))
}

fn pet(shared: &Shared, params: Value) -> Result<Value, String> {
    let Today { day } = parsed(params)?;
    let mut progress = shared.progress();
    let gain = progress.pet(day);
    shared.saved(&progress)?;
    answer(&mut progress, day, gain)
}

fn finish(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Finished = parsed(params)?;
    let mut progress = shared.progress();
    let gain = progress.finish(params.game);
    shared.saved(&progress)?;
    answer(&mut progress, params.day, Some(gain))
}

fn buy(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Item = parsed(params)?;
    let mut progress = shared.progress();
    progress.buy(params.item.as_deref().ok_or("no item")?)?;
    shared.saved(&progress)?;
    answer(&mut progress, params.day, None)
}

fn wear(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Item = parsed(params)?;
    let mut progress = shared.progress();
    progress.put_on(params.item.as_deref())?;
    shared.saved(&progress)?;
    answer(&mut progress, params.day, None)
}

fn set_screen(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: ScreenSettings = parsed(params)?;
    let mut progress = shared.progress();
    progress.on_screen = params.on_screen;
    progress.frequency = params.frequency;
    shared.saved(&progress)?;
    #[cfg(windows)]
    shared.screen.changed();
    answer(&mut progress, params.day, None)
}

/// The program has no other way to know this setting of Arthex, and needs it with the page closed.
fn set_motion(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Motion = parsed(params)?;
    let mut progress = shared.progress();
    if progress.reduced_motion != params.reduced_motion {
        progress.reduced_motion = params.reduced_motion;
        shared.saved(&progress)?;
    }
    Ok(Value::Null)
}

#[cfg(windows)]
fn hide_and_seek(shared: &Shared, params: Value) -> Result<Value, String> {
    let Today { day } = parsed(params)?;
    let found = shared.screen.hide_and_seek()?;
    let mut progress = shared.progress();
    let gain = found.then(|| progress.finish(Game::HideAndSeek));
    shared.saved(&progress)?;
    answer(&mut progress, day, gain)
}

#[cfg(not(windows))]
fn hide_and_seek(_: &Shared, _: Value) -> Result<Value, String> {
    Err("only on Windows".to_string())
}

fn flair_status(shared: &Shared) -> Result<Value, String> {
    serde_json::to_value(shared.library.status()).map_err(|error| error.to_string())
}

fn flair_download(shared: &Arc<Shared>) -> Result<Value, String> {
    let events = shared.events.clone();
    shared.library.download(move || {
        // The interface reads the status again: losing one of these only delays what it shows.
        let _ = events.emit("flair", Value::Null);
    });
    flair_status(shared)
}

/// Asks the file to be closed first, as Windows does not delete a file that is open.
fn flair_delete(shared: &Shared) -> Result<Value, String> {
    *shared.round() = None;
    shared.library.delete();
    flair_status(shared)
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| {
            time.as_secs().wrapping_mul(0x9E37_79B9) ^ u64::from(time.subsec_nanos())
        })
}

fn flair_start(shared: &Shared) -> Result<Value, String> {
    let round = shared
        .library
        .new_round(seed())
        .map_err(|error| error.to_string())?
        .ok_or("no word to play")?;
    *shared.round() = Some(round);
    Ok(Value::Null)
}

fn flair_guess(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: Word = parsed(params)?;
    let mut round = shared.round();
    let guess = round
        .as_mut()
        .ok_or("no round")?
        .guess(&params.word)
        .map_err(|error| error.to_string())?;
    let found = matches!(guess, Guess::Found { .. });
    let reward = if let Guess::Found { tries, .. } = guess {
        Some(word_reward(tries.saturating_sub(1)))
    } else {
        None
    };
    if found {
        *round = None;
    }
    drop(round);
    let result = serde_json::to_value(&guess).map_err(|error| error.to_string())?;
    let mut progress = shared.progress();
    let gain = reward.map(|(xp, kibble)| progress.grant(xp, kibble));
    if gain.is_some() {
        shared.saved(&progress)?;
    }
    let answered = answer(&mut progress, params.day, gain)?;
    Ok(json!({ "guess": result, "answer": answered }))
}

/// Giving up pays nothing, and is the only way to read the word.
fn flair_give_up(shared: &Shared) -> Result<Value, String> {
    let round = shared.round().take().ok_or("no round")?;
    Ok(json!({ "word": round.secret() }))
}

fn take_open(shared: &Shared) -> Value {
    #[cfg(windows)]
    let asked = shared.screen.take_open();
    #[cfg(not(windows))]
    let asked = {
        let _ = shared;
        false
    };
    json!(asked)
}

type Handler = fn(&Arc<Shared>, Value) -> Result<Value, String>;

fn main() -> std::io::Result<()> {
    let shared: Rc<OnceCell<Arc<Shared>>> = Rc::default();
    let starting = Rc::clone(&shared);
    let mut module = Module::new().on_start(move |context| {
        let started = Arc::new(Shared::start(context));
        #[cfg(windows)]
        screen::start(Arc::clone(&started));
        let _ = starting.set(started);
    });
    let handlers: [(&str, Handler); 16] = [
        ("state", |shared, params| state(shared, params)),
        ("adopt", |shared, params| adopt(shared, params)),
        ("tap", |shared, params| tap(shared, params)),
        ("pet", |shared, params| pet(shared, params)),
        ("finish", |shared, params| finish(shared, params)),
        ("buy", |shared, params| buy(shared, params)),
        ("wear", |shared, params| wear(shared, params)),
        ("setScreen", |shared, params| set_screen(shared, params)),
        ("setMotion", |shared, params| set_motion(shared, params)),
        ("hideAndSeek", |shared, params| {
            hide_and_seek(shared, params)
        }),
        ("flairStatus", |shared, _| flair_status(shared)),
        ("flairDownload", |shared, _| flair_download(shared)),
        ("flairDelete", |shared, _| flair_delete(shared)),
        ("flairStart", |shared, _| flair_start(shared)),
        ("flairGuess", |shared, params| flair_guess(shared, params)),
        ("flairGiveUp", |shared, _| flair_give_up(shared)),
    ];
    for (name, handler) in handlers {
        let shared = Rc::clone(&shared);
        module = module.on(name, move |_, params| {
            handler(shared.get().ok_or("the program has not started")?, params)
        });
    }
    let asked = Rc::clone(&shared);
    module
        .on("takeOpen", move |_, _| {
            Ok(asked.get().map_or(json!(false), |shared| take_open(shared)))
        })
        .run()
}
