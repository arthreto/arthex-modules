//! The system side of the Calendar module: keeps its events and deadlines, and shows their reminders
//! on the screen while the interface is closed. Nothing leaves the computer.

#[cfg(windows)]
mod canvas;
#[cfg(windows)]
mod hud;
#[cfg_attr(not(windows), allow(dead_code))]
mod moment;
#[cfg_attr(not(windows), allow(dead_code))]
mod paint;
#[cfg_attr(not(windows), allow(dead_code))]
mod store;
#[cfg_attr(not(windows), allow(dead_code))]
mod text;

use std::{
    cell::OnceCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use arthex_module::{Context, Module};
use serde::Deserialize;
use serde_json::Value;
use store::{Item, Store};

/// What the calls and the thread of the HUD have in common.
pub struct Shared {
    store: Mutex<Store>,
    folder: PathBuf,
    #[cfg(windows)]
    wake: hud::Wake,
}

impl Shared {
    fn start(context: &Context) -> Self {
        Self {
            store: Mutex::new(Store::load(context.data_dir())),
            folder: context.data_dir().to_path_buf(),
            #[cfg(windows)]
            wake: hud::Wake::default(),
        }
    }

    fn store(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Keeps what changed. The reminders of an item that just changed must be looked at again.
    fn changed(&self, store: &Store) -> Result<(), String> {
        store.save(&self.folder)?;
        #[cfg(windows)]
        self.wake.now();
        Ok(())
    }
}

#[derive(Deserialize)]
struct RemoveParams {
    id: String,
}

fn items(shared: &Shared) -> Result<Value, String> {
    serde_json::to_value(&shared.store().items).map_err(|error| error.to_string())
}

fn save(shared: &Shared, params: Value) -> Result<Value, String> {
    let item: Item = serde_json::from_value(params).map_err(|error| error.to_string())?;
    let mut store = shared.store();
    store.put(item)?;
    shared.changed(&store)?;
    drop(store);
    items(shared)
}

fn remove(shared: &Shared, params: Value) -> Result<Value, String> {
    let params: RemoveParams = serde_json::from_value(params).map_err(|error| error.to_string())?;
    let mut store = shared.store();
    store.remove(&params.id);
    shared.changed(&store)?;
    drop(store);
    items(shared)
}

fn main() -> std::io::Result<()> {
    let shared: Rc<OnceCell<Arc<Shared>>> = Rc::default();
    let ready = |shared: &OnceCell<Arc<Shared>>| {
        shared
            .get()
            .cloned()
            .ok_or_else(|| "the program has not started".to_string())
    };
    let starting = Rc::clone(&shared);
    let (listed, saved, removed) = (Rc::clone(&shared), Rc::clone(&shared), Rc::clone(&shared));
    Module::new()
        .on_start(move |context| {
            let started = Arc::new(Shared::start(context));
            #[cfg(windows)]
            hud::start(
                Arc::clone(&started),
                text::Texts::for_locale(context.locale()),
            );
            let _ = starting.set(started);
        })
        .on("items", move |_, _| items(&*ready(&listed)?))
        .on("save", move |_, params| save(&*ready(&saved)?, params))
        .on("remove", move |_, params| {
            remove(&*ready(&removed)?, params)
        })
        .run()
}
