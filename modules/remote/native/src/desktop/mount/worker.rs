//! Sends the user's edits to the server, on a thread and a connection of its own. A path is
//! checked when its turn comes, since the queue only says that something changed there.

use std::{
    collections::{BTreeSet, VecDeque},
    fs::{self, OpenOptions},
    io::{self, Seek, SeekFrom},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use cloud_filter::placeholder::ConvertOptions;
use windows_sys::Win32::{
    Foundation::ERROR_SHARING_VIOLATION, Storage::FileSystem::FILE_SHARE_READ,
};

use super::{link::Link, open_placeholder};
use crate::{desktop::remote, paths};

/// Lets a burst of saves settle before uploading.
const PASS: Duration = Duration::from_millis(700);

#[derive(Debug)]
pub enum Work {
    /// Upload this file, or create this folder, unless the server already has it.
    Sync(PathBuf),
    /// A server path whose local copy left the drive.
    Remove(String),
}

enum Message {
    Work(Work),
    /// Answers whether everything reached the server before the deadline.
    Flush(Instant, Sender<bool>),
}

pub struct Worker {
    sender: Option<Sender<Message>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(root: PathBuf, link: Link) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || Queue::new(root, link).run(&receiver));
        Self {
            sender: Some(sender),
            thread: Some(thread),
        }
    }

    pub fn enqueue(&self, work: Work) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(Message::Work(work));
        }
    }

    /// Waits until the queue is empty, or until `limit` has passed. `true` when nothing is left.
    pub fn flush(&self, limit: Duration) -> bool {
        let (answer, done) = mpsc::channel();
        let Some(sender) = &self.sender else {
            return true;
        };
        if sender
            .send(Message::Flush(Instant::now() + limit, answer))
            .is_err()
        {
            return false;
        }
        done.recv().unwrap_or(false)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Queue {
    root: PathBuf,
    link: Link,
    /// Sorted, so that a folder is created before what it holds.
    pending: BTreeSet<PathBuf>,
    removals: VecDeque<String>,
}

impl Queue {
    fn new(root: PathBuf, link: Link) -> Self {
        Self {
            root,
            link,
            pending: BTreeSet::new(),
            removals: VecDeque::new(),
        }
    }

    fn run(mut self, receiver: &Receiver<Message>) {
        let mut next = Instant::now() + PASS;
        loop {
            match receiver.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(Message::Work(Work::Sync(path))) => {
                    self.pending.insert(path);
                }
                Ok(Message::Work(Work::Remove(path))) => self.removals.push_back(path),
                Ok(Message::Flush(deadline, answer)) => {
                    while !self.is_empty() && Instant::now() < deadline {
                        self.pass();
                        if !self.is_empty() {
                            thread::sleep(PASS);
                        }
                    }
                    let _ = answer.send(self.is_empty());
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.pass();
                    next = Instant::now() + PASS;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.removals.is_empty()
    }

    /// One try at everything queued. What fails stays for the next pass.
    fn pass(&mut self) {
        while let Some(path) = self.removals.front() {
            let path = path.clone();
            let removed = self.link.run(|remote| match remote.stat(&path) {
                Ok(entry) if entry.is_dir => remote::remove_all(remote, &path),
                Ok(_) => remote.remove_file(&path),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            });
            if removed.is_err() {
                break;
            }
            self.removals.pop_front();
        }
        let mut paths = std::mem::take(&mut self.pending).into_iter();
        while let Some(path) = paths.next() {
            match self.sync(&path) {
                Ok(true) => {}
                // The server is out of reach: the rest waits for the next pass.
                Err(error) if error.kind() == io::ErrorKind::NotConnected => {
                    self.pending.insert(path);
                    self.pending.extend(paths);
                    return;
                }
                _ => {
                    self.pending.insert(path);
                }
            }
        }
    }

    /// `Ok(false)` when the file is still being written, or its folder not yet on the server.
    fn sync(&mut self, path: &Path) -> io::Result<bool> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(error) => return Err(error),
        };
        let Some(relative) = paths::strip_root(&self.root, path) else {
            return Ok(true);
        };
        let Some(target) = paths::remote_path(self.link.folder(), &relative) else {
            return Ok(true);
        };
        if metadata.file_type().is_symlink() || relative.as_os_str().is_empty() {
            return Ok(true);
        }
        if let Some(parent) = path.parent()
            && parent != self.root
            && !is_in_sync(parent)?
        {
            return Ok(false);
        }
        if metadata.is_dir() {
            self.sync_directory(path, &target)
        } else {
            self.upload(path, &target)
        }
    }

    fn sync_directory(&mut self, path: &Path, target: &str) -> io::Result<bool> {
        let mut placeholder = open_placeholder(path)?;
        let info = placeholder.info().map_err(io::Error::other)?;
        if info
            .as_ref()
            .is_some_and(cloud_filter::placeholder::PlaceholderInfo::is_in_sync)
        {
            return Ok(true);
        }
        self.link.run(|remote| match remote.create_dir(target) {
            Err(_) if remote.stat(target).is_ok_and(|entry| entry.is_dir) => Ok(()),
            result => result,
        })?;
        mark_in_sync(&mut placeholder, info.is_none())?;
        Ok(true)
    }

    fn upload(&mut self, path: &Path, target: &str) -> io::Result<bool> {
        if is_in_sync(path)? {
            return Ok(true);
        }
        // Refuses a file another program is still writing, and keeps writers out until the
        // uploaded version is marked in sync.
        let mut source = match OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(path)
        {
            Ok(source) => source,
            Err(error) if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION.cast_signed()) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        let mut placeholder = open_placeholder(path)?;
        let info = placeholder.info().map_err(io::Error::other)?;
        self.link.run(|remote| {
            source.seek(SeekFrom::Start(0))?;
            remote.write(target, &mut source)
        })?;
        mark_in_sync(&mut placeholder, info.is_none())?;
        drop(source);
        Ok(true)
    }
}

fn is_in_sync(path: &Path) -> io::Result<bool> {
    let info = open_placeholder(path)?.info().map_err(io::Error::other)?;
    Ok(info.is_some_and(|info| info.is_in_sync() && info.modified_data_size() == 0))
}

/// A file or folder made here becomes a placeholder, the drive's own kind of entry.
fn mark_in_sync(
    placeholder: &mut cloud_filter::placeholder::Placeholder,
    is_new: bool,
) -> io::Result<()> {
    if is_new {
        placeholder
            .convert_to_placeholder(ConvertOptions::default().mark_in_sync(), None)
            .map_err(io::Error::other)?;
    } else {
        placeholder
            .mark_in_sync(true, None)
            .map_err(io::Error::other)?;
    }
    Ok(())
}
