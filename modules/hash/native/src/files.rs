//! A file hashed on a thread of its own, read once for every algorithm, its progress sent to the
//! interface as `hash://file` with the whole state, so that it never depends on one missed event.

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use arthex_module::Events;
use serde::Serialize;

use crate::digests::Digests;

const BLOCK: usize = 4 << 20;
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub id: &'static str,
    pub hex: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileState {
    job: u64,
    name: String,
    size: u64,
    done: u64,
    digests: Option<Vec<Entry>>,
    failed: bool,
}

pub struct Files {
    state: Arc<Mutex<Option<FileState>>>,
    cancel: Mutex<Arc<AtomicBool>>,
    events: Events,
}

impl Files {
    pub fn new(events: Events) -> Self {
        Self {
            state: Arc::default(),
            cancel: Mutex::default(),
            events,
        }
    }

    /// Stops the file in progress, if any, and hashes this one instead.
    pub fn start(&self, path: &Path) -> Result<FileState, String> {
        let file = File::open(path).map_err(|error| error.to_string())?;
        let size = file.metadata().map_err(|error| error.to_string())?.len();
        let stop = Arc::new(AtomicBool::new(false));
        let previous = std::mem::replace(&mut *self.lock_cancel(), Arc::clone(&stop));
        previous.store(true, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let started = FileState {
            job: state.as_ref().map_or(1, |state| state.job + 1),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size,
            ..FileState::default()
        };
        *state = Some(started.clone());
        drop(state);
        let shared = Arc::clone(&self.state);
        let events = self.events.clone();
        let job = started.job;
        thread::spawn(move || {
            let outcome = hash(file, &stop, |done| {
                update(&shared, &events, job, |state| state.done = done);
            });
            if stop.load(Ordering::Relaxed) {
                return;
            }
            update(&shared, &events, job, |state| match outcome {
                Ok(digests) => {
                    state.done = state.size;
                    state.digests = Some(digests);
                }
                Err(_) => state.failed = true,
            });
        });
        Ok(started)
    }

    pub fn current(&self) -> Option<FileState> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn cancel(&self) {
        self.lock_cancel().store(true, Ordering::Relaxed);
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    fn lock_cancel(&self) -> std::sync::MutexGuard<'_, Arc<AtomicBool>> {
        self.cancel.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Changes the state if it is still this job's, and tells the interface.
fn update(
    shared: &Mutex<Option<FileState>>,
    events: &Events,
    job: u64,
    change: impl FnOnce(&mut FileState),
) {
    let mut state = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(current) = state.as_mut().filter(|state| state.job == job) else {
        return;
    };
    change(current);
    let snapshot = current.clone();
    drop(state);
    // Nobody listens while the overlay is closed: the interface asks for the state when it opens.
    let _ = events.emit("file", snapshot);
}

fn hash(
    mut file: impl Read,
    stop: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> io::Result<Vec<Entry>> {
    let threads = thread::available_parallelism().map_or(1, usize::from);
    let mut digests = Digests::new(None);
    let mut buffer = vec![0; BLOCK];
    let mut done = 0u64;
    let mut told = Instant::now();
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let read = fill(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        digests.feed_in_parallel(&buffer[..read], threads);
        done += read as u64;
        if told.elapsed() >= PROGRESS_EVERY {
            progress(done);
            told = Instant::now();
        }
    }
    Ok(digests
        .finish()
        .into_iter()
        .map(|(id, hex)| Entry { id, hex })
        .collect())
}

/// A whole block when the file has one, so that the threads get enough work each time.
fn fill(file: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_gives_the_digests_of_its_content() {
        let content: Vec<u8> = (0..(BLOCK + 12_345))
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let mut reported = Vec::new();
        let entries = hash(content.as_slice(), &AtomicBool::new(false), |done| {
            reported.push(done);
        })
        .unwrap();
        let mut expected = Digests::new(None);
        expected.feed(&content);
        let expected = expected.finish();
        assert_eq!(entries.len(), expected.len());
        for (entry, (id, hex)) in entries.iter().zip(expected) {
            assert_eq!((entry.id, entry.hex.as_str()), (id, hex.as_str()));
        }
        assert!(reported.iter().all(|&done| done <= content.len() as u64));
    }

    #[test]
    fn a_stopped_file_gives_nothing() {
        let stopped = hash(&[1u8, 2, 3][..], &AtomicBool::new(true), |_| {});
        assert!(stopped.is_err());
    }
}
