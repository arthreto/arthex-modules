//! The program: keeps the connections, mounts them as drives, and tells the interface how each
//! one is doing. Connecting and disconnecting run on threads, as they wait on the network.

mod mount;
mod remote;
mod secret;
mod shell;
mod store;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
};

use arthex_module::{Context, Events, Module};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use self::{
    mount::{Mount, MountError},
    remote::Failure,
    store::{Field, Store},
};
use crate::model::Connection;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum Status {
    Off,
    Busy,
    On {
        place: String,
    },
    /// The SSH server is new: its key waits for the user's approval.
    Trust {
        algorithm: String,
        fingerprint: String,
    },
    Error {
        reason: &'static str,
        detail: String,
    },
}

impl Status {
    fn failed(error: MountError) -> Self {
        let (reason, detail) = match error {
            MountError::Remote(failure) => match failure {
                Failure::Unreachable(detail) => ("unreachable", detail),
                Failure::Refused => ("refused", String::new()),
                Failure::ChangedHost { fingerprint } => ("changedHost", fingerprint),
                Failure::Certificate(detail) => ("certificate", detail),
                Failure::BadKey => ("badKey", String::new()),
                Failure::NoFolder => ("noFolder", String::new()),
                Failure::UnknownHost { .. } => ("other", String::new()),
                Failure::Other(detail) => ("other", detail),
            },
            MountError::LetterTaken => ("letterTaken", String::new()),
            MountError::System(detail) => ("system", detail),
        };
        Self::Error { reason, detail }
    }
}

struct App {
    drives: PathBuf,
    store: Mutex<Store>,
    mounts: Mutex<HashMap<String, Mount>>,
    statuses: Mutex<HashMap<String, Status>>,
    /// The key each SSH server showed, until the user trusts it.
    shown: Mutex<HashMap<String, String>>,
    events: Events,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Deserialize)]
struct IdParams {
    id: String,
}

#[derive(Deserialize)]
struct SaveParams {
    connections: Vec<Connection>,
}

#[derive(Deserialize)]
struct SecretParams {
    id: String,
    field: Field,
    value: String,
}

fn params<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// The drives sit with the app's local data, beside its modules, never in a roaming profile.
fn drives_folder(data_dir: &Path) -> PathBuf {
    let app = data_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name);
    match (std::env::var_os("LOCALAPPDATA"), app) {
        (Some(local), Some(app)) => Path::new(&local).join(app).join("remote-drives"),
        _ => data_dir.join("drives"),
    }
}

impl App {
    fn set_status(&self, id: &str, status: Status) {
        let _ = self
            .events
            .emit("status", json!({ "id": id, "status": status }));
        locked(&self.statuses).insert(id.to_string(), status);
    }

    fn state(&self) -> Value {
        let store = locked(&self.store);
        let secrets: HashMap<&str, Value> = store
            .connections()
            .iter()
            .map(|connection| {
                let secrets = store.secrets(&connection.id);
                (
                    connection.id.as_str(),
                    json!({
                        "password": !secrets.password.is_empty(),
                        "passphrase": !secrets.passphrase.is_empty(),
                        "key": !secrets.key.is_empty(),
                    }),
                )
            })
            .collect();
        json!({
            "connections": store.connections(),
            "secrets": secrets,
            "statuses": *locked(&self.statuses),
        })
    }

    fn connect(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let (connection, secrets, trusted) = {
            let store = locked(&self.store);
            let connection = store.connection(id).ok_or("no such connection")?.clone();
            let secrets = store.secrets(id);
            let trusted = store.trusted(&connection).map(String::from);
            (connection, secrets, trusted)
        };
        if !connection.is_complete() {
            return Err("incomplete".into());
        }
        {
            let statuses = locked(&self.statuses);
            if matches!(statuses.get(id), Some(Status::Busy | Status::On { .. })) {
                return Ok(());
            }
        }
        self.set_status(id, Status::Busy);
        let app = Arc::clone(self);
        thread::spawn(move || {
            let status = match Mount::start(&connection, &secrets, trusted.as_deref(), &app.drives)
            {
                Ok(mount) => {
                    let place = mount.place().to_string_lossy().into_owned();
                    locked(&app.mounts).insert(connection.id.clone(), mount);
                    Status::On { place }
                }
                Err(MountError::Remote(Failure::UnknownHost {
                    algorithm,
                    fingerprint,
                })) => {
                    locked(&app.shown).insert(connection.id.clone(), fingerprint.clone());
                    Status::Trust {
                        algorithm,
                        fingerprint,
                    }
                }
                Err(error) => Status::failed(error),
            };
            app.set_status(&connection.id, status);
        });
        Ok(())
    }

    fn disconnect(self: &Arc<Self>, id: &str) {
        let Some(mount) = locked(&self.mounts).remove(id) else {
            self.set_status(id, Status::Off);
            return;
        };
        self.set_status(id, Status::Busy);
        let (app, id) = (Arc::clone(self), id.to_string());
        thread::spawn(move || {
            mount.stop();
            app.set_status(&id, Status::Off);
        });
    }

    fn trust(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let fingerprint = locked(&self.shown).remove(id).ok_or("nothing to trust")?;
        {
            let mut store = locked(&self.store);
            let connection = store.connection(id).ok_or("no such connection")?.clone();
            store
                .trust(&connection, &fingerprint)
                .map_err(|error| error.to_string())?;
        }
        locked(&self.statuses).remove(id);
        self.connect(id)
    }

    fn save(self: &Arc<Self>, connections: Vec<Connection>) -> Result<(), String> {
        let removed: Vec<Connection> = {
            let mut store = locked(&self.store);
            let before = store.connections().to_vec();
            store
                .replace(connections)
                .map_err(|error| error.to_string())?;
            before
                .into_iter()
                .filter(|connection| store.connection(&connection.id).is_none())
                .collect()
        };
        for connection in removed {
            let mount = locked(&self.mounts).remove(&connection.id);
            locked(&self.statuses).remove(&connection.id);
            let drives = self.drives.clone();
            thread::spawn(move || {
                if let Some(mount) = mount {
                    mount.stop();
                }
                mount::forget(&connection.id, &connection.letter, &drives);
            });
        }
        Ok(())
    }

    fn open(&self, id: &str) -> Result<(), String> {
        let place = locked(&self.mounts)
            .get(id)
            .map(Mount::place)
            .ok_or("not connected")?;
        shell::open(&place)
    }

    fn pick_key(&self, id: &str) -> Result<Value, String> {
        let Some((name, key)) = shell::pick_key()? else {
            return Ok(Value::Null);
        };
        locked(&self.store)
            .set_key(id, key)
            .map_err(|error| error.to_string())?;
        Ok(json!(name))
    }

    /// Connects those the user asked for, and those left with edits to send.
    fn resume(self: &Arc<Self>) {
        let ids: Vec<String> = locked(&self.store)
            .connections()
            .iter()
            .filter(|connection| {
                connection.auto_connect || mount::is_left(&connection.id, &self.drives)
            })
            .map(|connection| connection.id.clone())
            .collect();
        for id in ids {
            let _ = self.connect(&id);
        }
    }

    /// Before the program ends: every drive is sent, then removed.
    fn stop_all(&self) {
        let mounts: Vec<Mount> = locked(&self.mounts)
            .drain()
            .map(|(_, mount)| mount)
            .collect();
        let stopping: Vec<_> = mounts
            .into_iter()
            .map(|mount| thread::spawn(move || mount.stop()))
            .collect();
        for thread in stopping {
            let _ = thread.join();
        }
    }
}

pub fn run() -> std::io::Result<()> {
    let app: Arc<Mutex<Option<Arc<App>>>> = Arc::default();
    let get = {
        let app = Arc::clone(&app);
        move || {
            locked(&app)
                .clone()
                .ok_or_else(|| "not started".to_string())
        }
    };
    let started = Arc::clone(&app);
    let handlers = Handlers(Arc::new(get));
    Module::new()
        .on_start(move |context: &Context| {
            let data_dir = context.data_dir();
            let new = Arc::new(App {
                drives: drives_folder(data_dir),
                store: Mutex::new(Store::load(data_dir)),
                mounts: Mutex::default(),
                statuses: Mutex::default(),
                shown: Mutex::default(),
                events: context.events().clone(),
            });
            new.resume();
            *locked(&started) = Some(new);
        })
        .on("state", handlers.with(|app, _| Ok(app.state())))
        .on(
            "save",
            handlers.with(|app, value| {
                app.save(params::<SaveParams>(value)?.connections)?;
                Ok(Value::Null)
            }),
        )
        .on(
            "secret",
            handlers.with(|app, value| {
                let SecretParams { id, field, value } = params(value)?;
                locked(&app.store)
                    .set_field(&id, field, value)
                    .map_err(|error| error.to_string())?;
                Ok(Value::Null)
            }),
        )
        .on(
            "pickKey",
            handlers.with(|app, value| app.pick_key(&params::<IdParams>(value)?.id)),
        )
        .on(
            "connect",
            handlers.with(|app, value| {
                app.connect(&params::<IdParams>(value)?.id)?;
                Ok(Value::Null)
            }),
        )
        .on(
            "trust",
            handlers.with(|app, value| {
                app.trust(&params::<IdParams>(value)?.id)?;
                Ok(Value::Null)
            }),
        )
        .on(
            "disconnect",
            handlers.with(|app, value| {
                app.disconnect(&params::<IdParams>(value)?.id);
                Ok(Value::Null)
            }),
        )
        .on(
            "open",
            handlers.with(|app, value| {
                app.open(&params::<IdParams>(value)?.id)?;
                Ok(Value::Null)
            }),
        )
        .run()?;
    if let Some(app) = locked(&app).take() {
        app.stop_all();
    }
    Ok(())
}

type Getter = Arc<dyn Fn() -> Result<Arc<App>, String>>;

#[derive(Clone)]
struct Handlers(Getter);

impl Handlers {
    fn with(
        &self,
        handler: impl Fn(&Arc<App>, Value) -> Result<Value, String> + 'static,
    ) -> impl Fn(&Context, Value) -> Result<Value, String> + 'static {
        let get = Arc::clone(&self.0);
        move |_, value| handler(&get()?, value)
    }
}
