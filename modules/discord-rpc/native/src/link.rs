//! Keeps Discord showing the wanted template: connects, reconnects and respects its rate limit.

use std::{
    fs::File,
    sync::{
        Arc, Mutex, PoisonError,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use arthex_module::Events;
use serde::Serialize;

use crate::{
    ipc::{self, Connection, Error},
    presence,
    store::Template,
};

/// How often it looks for Discord, or checks it is still there, while a template is active.
const CHECK_EVERY: Duration = Duration::from_secs(15);
/// Discord takes 5 updates per 20 seconds.
const MIN_GAP: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Status {
    Idle,
    Connected,
    Offline,
    Refused { message: String },
}

pub struct Wanted {
    pub template: Template,
    /// Milliseconds.
    pub since: i64,
}

pub struct Link {
    sender: Sender<Option<Wanted>>,
    status: Arc<Mutex<Status>>,
}

impl Link {
    pub fn start(events: Events) -> Self {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status::Idle));
        let worker = Worker {
            events,
            status: Arc::clone(&status),
            wanted: None,
            connection: None,
            dirty: false,
            blocked: false,
            last_sent: None,
        };
        thread::spawn(move || worker.run(&receiver));
        Self { sender, status }
    }

    pub fn want(&self, wanted: Option<Wanted>) {
        // The worker only ends when the program does.
        let _ = self.sender.send(wanted);
    }

    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn is_app_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

struct Worker {
    events: Events,
    status: Arc<Mutex<Status>>,
    wanted: Option<Wanted>,
    connection: Option<Connection<File>>,
    /// Discord does not show the latest of `wanted` yet.
    dirty: bool,
    /// Discord refused to connect: trying again would change nothing until the template does.
    blocked: bool,
    last_sent: Option<Instant>,
}

impl Worker {
    fn run(mut self, receiver: &Receiver<Option<Wanted>>) {
        loop {
            self.step();
            let next = match self.wait() {
                Some(wait) => receiver.recv_timeout(wait),
                None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match next {
                Ok(wanted) => {
                    self.wanted = receiver.try_iter().last().unwrap_or(wanted);
                    self.dirty = true;
                    self.blocked = false;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// `None` when nothing can happen until the wanted template changes.
    fn wait(&self) -> Option<Duration> {
        if self.wanted.is_none() || self.blocked {
            return None;
        }
        if self.dirty && self.connection.is_some() {
            return Some(self.gap_left());
        }
        Some(CHECK_EVERY)
    }

    fn gap_left(&self) -> Duration {
        self.last_sent.map_or(Duration::ZERO, |sent| {
            MIN_GAP.saturating_sub(sent.elapsed())
        })
    }

    fn report(&self, status: Status) {
        let mut current = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        if *current != status {
            // Arthex is closing if nobody listens any more.
            let _ = self.events.emit("status", &status);
            *current = status;
        }
    }

    fn step(&mut self) {
        let Some(wanted) = &self.wanted else {
            self.stop();
            return;
        };
        if self.blocked {
            return;
        }
        let app_id = wanted.template.app_id.clone();
        let activity = presence::activity(&wanted.template, wanted.since);
        if !is_app_id(&app_id) {
            self.connection = None;
            self.blocked = true;
            self.report(Status::Refused {
                message: String::new(),
            });
            return;
        }
        if self
            .connection
            .as_ref()
            .is_some_and(|connection| connection.app_id() != app_id)
        {
            self.connection = None;
        }
        if self.connection.is_none() && !self.connect(&app_id) {
            return;
        }
        if self.dirty {
            if self.gap_left().is_zero() {
                self.send(&activity);
            }
        } else if self.ping_failed() {
            self.lose_connection();
        }
    }

    fn connect(&mut self, app_id: &str) -> bool {
        let opened = ipc::open()
            .map_err(Error::from)
            .and_then(|pipe| Connection::start(pipe, app_id));
        match opened {
            Ok(connection) => {
                self.connection = Some(connection);
                self.dirty = true;
                true
            }
            Err(Error::Lost) => {
                self.report(Status::Offline);
                false
            }
            Err(Error::Refused(message)) => {
                self.blocked = true;
                self.report(Status::Refused { message });
                false
            }
        }
    }

    fn send(&mut self, activity: &serde_json::Value) {
        let Some(connection) = &mut self.connection else {
            return;
        };
        match connection.set_activity(Some(activity)) {
            Ok(()) => {
                self.dirty = false;
                self.last_sent = Some(Instant::now());
                self.report(Status::Connected);
            }
            Err(Error::Refused(message)) => {
                self.dirty = false;
                self.last_sent = Some(Instant::now());
                self.report(Status::Refused { message });
            }
            Err(Error::Lost) => self.lose_connection(),
        }
    }

    fn ping_failed(&mut self) -> bool {
        self.connection
            .as_mut()
            .is_some_and(|connection| connection.ping().is_err())
    }

    fn lose_connection(&mut self) {
        self.connection = None;
        self.dirty = true;
        self.report(Status::Offline);
    }

    fn stop(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            // Closing the connection clears the presence anyway.
            let _ = connection.set_activity(None);
        }
        self.dirty = false;
        self.blocked = false;
        self.report(Status::Idle);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn an_application_id_is_only_digits() {
        assert!(is_app_id("1234567890123456789"));
        assert!(!is_app_id(""));
        assert!(!is_app_id("12a"));
        assert!(!is_app_id("../12"));
    }

    #[test]
    fn the_interface_reads_the_state_by_its_name() {
        assert_eq!(
            serde_json::to_value(Status::Offline).unwrap(),
            json!({ "state": "offline" })
        );
        assert_eq!(
            serde_json::to_value(Status::Refused {
                message: "No".into()
            })
            .unwrap(),
            json!({ "state": "refused", "message": "No" })
        );
    }
}
