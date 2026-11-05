//! The system side of the Discord RPC module. It keeps the templates and holds the connection to
//! Discord, since a presence lasts only as long as the program that set it stays connected.

mod ipc;
mod link;
mod presence;
mod preview;
mod store;

use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use arthex_module::{Context, Module};
use link::{Link, Wanted};
use serde::Deserialize;
use serde_json::{Value, json};
use store::{Saved, Template};

struct Core {
    dir: PathBuf,
    saved: Saved,
    link: Link,
    /// When the active template was turned on, in milliseconds.
    since: i64,
}

impl Core {
    fn start(context: &Context) -> Self {
        let core = Self {
            dir: context.data_dir().to_path_buf(),
            saved: store::load(context.data_dir()),
            link: Link::start(context.events().clone()),
            since: now(),
        };
        core.apply();
        core
    }

    fn apply(&self) {
        let template = self.saved.active.as_ref().and_then(|id| {
            self.saved
                .templates
                .iter()
                .find(|template| &template.id == id)
        });
        self.link.want(template.map(|template| Wanted {
            template: template.clone(),
            since: self.since,
        }));
    }

    fn write(&self) -> Result<(), String> {
        store::save(&self.dir, &self.saved).map_err(|error| error.to_string())
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

#[derive(Deserialize)]
struct SaveParams {
    templates: Vec<Template>,
}

#[derive(Deserialize)]
struct ActivateParams {
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewParams {
    app_id: String,
    value: String,
}

fn state(core: &Core) -> Value {
    json!({
        "templates": core.saved.templates,
        "active": core.saved.active,
        "link": core.link.status(),
    })
}

fn save(core: &mut Core, params: Value) -> Result<Value, String> {
    let params: SaveParams = serde_json::from_value(params).map_err(|error| error.to_string())?;
    core.saved.templates = params.templates;
    let active_is_gone = core.saved.active.as_ref().is_some_and(|id| {
        !core
            .saved
            .templates
            .iter()
            .any(|template| &template.id == id)
    });
    if active_is_gone {
        core.saved.active = None;
    }
    core.write()?;
    core.apply();
    Ok(Value::Null)
}

fn activate(core: &mut Core, params: Value) -> Result<Value, String> {
    let params: ActivateParams =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    core.saved.active = params.id;
    core.since = now();
    core.write()?;
    core.apply();
    Ok(Value::Null)
}

/// The image arrives later as an event: a download must not hold up the calls that follow.
fn request_preview(context: &Context, params: Value) -> Result<Value, String> {
    let params: PreviewParams =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    let events = context.events().clone();
    thread::spawn(move || {
        let url = preview::fetch(&params.app_id, &params.value);
        let payload = json!({ "appId": params.app_id, "value": params.value, "url": url });
        // Arthex is closing if nobody listens any more.
        let _ = events.emit("image", payload);
    });
    Ok(Value::Null)
}

fn main() -> std::io::Result<()> {
    let core: Rc<RefCell<Option<Core>>> = Rc::default();
    let on_call = |handler: fn(&mut Core, Value) -> Result<Value, String>| {
        let core = Rc::clone(&core);
        move |_: &Context, params: Value| {
            let mut core = core.borrow_mut();
            let core = core.as_mut().ok_or("the program has not started")?;
            handler(core, params)
        }
    };
    let starting = Rc::clone(&core);
    Module::new()
        .on_start(move |context| *starting.borrow_mut() = Some(Core::start(context)))
        .on("state", on_call(|core, _| Ok(state(core))))
        .on("save", on_call(save))
        .on("activate", on_call(activate))
        .on("preview", request_preview)
        .run()
}
