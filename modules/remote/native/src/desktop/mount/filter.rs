//! What Windows asks while the user browses the drive: the folders' contents, the files' bytes,
//! and leave to delete or rename. Edits are queued for the worker, which uploads them.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
};

use cloud_filter::{
    error::{CResult, CloudErrorKind},
    filter::{Request, StateWatchEvent, SyncFilter, info, ticket},
    metadata::Metadata,
    placeholder_create_info::PlaceholderCreateInfo,
    utility::WriteAt,
};
use nt_time::FileTime;

use super::{
    link::Link,
    open_placeholder,
    worker::{Work, Worker},
};
use crate::{
    desktop::remote::{self, Entry},
    paths,
};

/// Transfers stay 4 KiB aligned until the end of the file, as Windows requires.
const CHUNK: usize = 1024 * 1024;

pub struct Filter {
    root: PathBuf,
    folder: String,
    /// Folder listings, deletions and renames.
    browse: Mutex<Link>,
    /// File contents, on a connection of their own so that a long download never blocks browsing.
    transfer: Mutex<Link>,
    pub worker: Worker,
}

impl Filter {
    pub fn new(root: PathBuf, browse: Link, transfer: Link, worker: Worker) -> Self {
        Self {
            root,
            folder: browse.folder().to_string(),
            browse: Mutex::new(browse),
            transfer: Mutex::new(transfer),
            worker,
        }
    }

    fn remote_of(&self, path: &Path) -> CResult<String> {
        paths::strip_root(&self.root, path)
            .and_then(|relative| paths::remote_path(&self.folder, &relative))
            .ok_or(CloudErrorKind::InvalidRequest)
    }

    fn browse(&self) -> MutexGuard<'_, Link> {
        self.browse.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn cloud_error(error: &io::Error) -> CloudErrorKind {
    match error.kind() {
        io::ErrorKind::PermissionDenied => CloudErrorKind::AccessDenied,
        io::ErrorKind::NotFound | io::ErrorKind::UnexpectedEof => CloudErrorKind::NotInSync,
        io::ErrorKind::NotConnected => CloudErrorKind::NetworkUnavailable,
        _ => CloudErrorKind::Unsuccessful,
    }
}

fn api_error<E>(_: E) -> CloudErrorKind {
    CloudErrorKind::Unsuccessful
}

fn metadata(entry: &Entry) -> Metadata {
    let shown = if entry.is_dir {
        Metadata::directory()
    } else {
        Metadata::file().size(entry.size)
    };
    let time = entry
        .modified
        .and_then(|seconds| FileTime::from_unix_time_secs(seconds).ok())
        .unwrap_or_else(FileTime::now);
    shown
        .created(time)
        .written(time)
        .changed(time)
        .accessed(time)
}

/// Whether a file or folder moved out of the drive holds all its contents, so that the server
/// copy can go. An online-only one keeps it: the server holds the only bytes.
fn is_whole(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    let Ok(placeholder) = open_placeholder(path) else {
        return false;
    };
    if metadata.is_dir() {
        let partial = placeholder
            .state()
            .ok()
            .flatten()
            .is_some_and(|state| state.partial());
        !partial
            && fs::read_dir(path)
                .is_ok_and(|entries| entries.flatten().all(|entry| is_whole(&entry.path())))
    } else {
        match placeholder.info() {
            Ok(None) => true,
            Ok(Some(info)) => {
                u64::try_from(info.on_disk_data_size()).is_ok_and(|size| size >= metadata.len())
            }
            Err(_) => false,
        }
    }
}

fn ignore_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

impl SyncFilter for Filter {
    fn fetch_data(
        &self,
        request: Request,
        ticket: ticket::FetchData,
        info: info::FetchData,
    ) -> CResult<()> {
        let range = info.required_file_range();
        let path = self.remote_of(&request.path())?;
        let mut transfer = self.transfer.lock().unwrap_or_else(PoisonError::into_inner);
        let mut pending = Vec::with_capacity(CHUNK);
        let mut position = range.start;
        transfer
            .run(|remote| {
                pending.clear();
                position = range.start;
                remote.read(&path, range.start, range.end, &mut |bytes| {
                    pending.extend_from_slice(bytes);
                    while pending.len() >= CHUNK {
                        ticket
                            .write_at(&pending[..CHUNK], position)
                            .map_err(io::Error::other)?;
                        position += CHUNK as u64;
                        pending.drain(..CHUNK);
                        // Progress is only shown to the user; a failed report changes nothing.
                        let _ =
                            ticket.report_progress(range.end - range.start, position - range.start);
                    }
                    Ok(())
                })
            })
            .map_err(|error| cloud_error(&error))?;
        if !pending.is_empty() {
            ticket.write_at(&pending, position).map_err(api_error)?;
        }
        Ok(())
    }

    fn fetch_placeholders(
        &self,
        request: Request,
        ticket: ticket::FetchPlaceholders,
        _info: info::FetchPlaceholders,
    ) -> CResult<()> {
        let directory = request.path();
        let path = self.remote_of(&directory)?;
        let entries = self
            .browse()
            .run(|remote| remote.list(&path))
            .map_err(|error| cloud_error(&error))?;
        let mut placeholders: Vec<PlaceholderCreateInfo> = entries
            .iter()
            // What the user made here meanwhile stays as it is.
            .filter(|entry| {
                paths::is_shown_name(&entry.name) && !directory.join(&entry.name).exists()
            })
            .map(|entry| {
                // Windows wants an identity for each entry: its path on the server.
                PlaceholderCreateInfo::new(&entry.name)
                    .metadata(metadata(entry))
                    .blob(format!("{}/{}", path.trim_end_matches('/'), entry.name).into_bytes())
                    .mark_in_sync()
            })
            .collect();
        ticket
            .pass_with_placeholders(&mut placeholders)
            .map_err(api_error)?;
        Ok(())
    }

    fn closed(&self, request: Request, info: info::Closed) {
        if !info.deleted() {
            self.worker.enqueue(Work::Sync(request.path()));
        }
    }

    fn dehydrate(
        &self,
        _request: Request,
        ticket: ticket::Dehydrate,
        _info: info::Dehydrate,
    ) -> CResult<()> {
        // Windows frees the space only of files in sync, so local edits are never lost.
        ticket.pass().map_err(api_error)
    }

    fn delete(&self, request: Request, ticket: ticket::Delete, info: info::Delete) -> CResult<()> {
        if !info.is_undelete() {
            let path = self.remote_of(&request.path())?;
            let directory = info.is_directory();
            self.browse()
                .run(|remote| {
                    ignore_missing(if directory {
                        remote::remove_all(remote, &path)
                    } else {
                        remote.remove_file(&path)
                    })
                })
                .map_err(|error| cloud_error(&error))?;
        }
        ticket.pass().map_err(api_error)
    }

    fn rename(&self, request: Request, ticket: ticket::Rename, info: info::Rename) -> CResult<()> {
        match (info.source_in_scope(), info.target_in_scope()) {
            (true, true) => {
                let from = self.remote_of(&request.path())?;
                let to = self.remote_of(&info.target_path())?;
                self.browse()
                    .run(|remote| remote.rename(&from, &to))
                    .map_err(|error| cloud_error(&error))?;
            }
            // Out of the drive, such as to the Recycle Bin: the server copy goes once it is done.
            (true, false) => {}
            _ => return Err(CloudErrorKind::NotSupported),
        }
        ticket.pass().map_err(api_error)
    }

    fn renamed(&self, request: Request, info: info::Renamed) {
        let target = request.path();
        if paths::strip_root(&self.root, &target).is_some() || !is_whole(&target) {
            return;
        }
        if let Ok(path) = self.remote_of(&info.source_path()) {
            self.worker.enqueue(Work::Remove(path));
        }
    }

    fn state_changed(&self, event: StateWatchEvent) {
        let StateWatchEvent::Changed(mut found) = event else {
            return;
        };
        while let Some(path) = found.pop() {
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_dir() {
                // Only what is already here: listing the server is not this thread's job.
                if let Ok(entries) = fs::read_dir(&path) {
                    found.extend(entries.flatten().map(|entry| entry.path()));
                }
            }
            if path != self.root {
                self.worker.enqueue(Work::Sync(path));
            }
        }
    }
}
