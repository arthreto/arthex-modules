//! A connection shown as a drive, through the Cloud Files API of Windows (the one of OneDrive):
//! no driver to install, no administrator rights. Its folder holds placeholders that fill when
//! opened; the letter points to that folder.

mod drive;
mod filter;
mod link;
mod worker;

use std::{
    fs,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use cloud_filter::{
    filter::{StateWatchEvent, SyncFilter},
    placeholder::Placeholder,
    root::{
        Connection as Attached, HydrationType, PopulationType, SecurityId, Session, SyncRootId,
        SyncRootIdBuilder, SyncRootInfo,
    },
};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;

use self::{filter::Filter, link::Link, worker::Worker};
use crate::{
    desktop::{remote::Failure, store::Secrets},
    model::Connection,
};

const PROVIDER: &str = "ArthexRemote";
/// How long a disconnection waits for the last edits to reach the server.
const FLUSH_LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum MountError {
    Remote(Failure),
    LetterTaken,
    System(String),
}

pub struct Mount {
    attached: Option<Attached<Filter>>,
    root: PathBuf,
    sync_root: SyncRootId,
    letter: Option<String>,
}

fn system(error: impl std::fmt::Display) -> MountError {
    MountError::System(error.to_string())
}

/// Opens without reading the contents, which would download a file just to look at it.
pub fn open_placeholder(path: &Path) -> std::io::Result<Placeholder> {
    fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map(Placeholder::from)
}

fn sync_root_id(id: &str) -> Result<SyncRootId, MountError> {
    let user = SecurityId::current_user().map_err(system)?;
    Ok(SyncRootIdBuilder::new(PROVIDER)
        .user_security_id(user)
        .account_name(id)
        .build())
}

fn root_info(connection: &Connection, root: &Path) -> Result<SyncRootInfo, MountError> {
    SyncRootInfo::default()
        .with_display_name(connection.shown_name())
        .with_hydration_type(HydrationType::Full)
        .with_population_type(PopulationType::Full)
        .with_icon(drive::ICON)
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_path(root)
        .map_err(system)
}

/// The real path of a folder, in the form Windows gives to the callbacks (`C:\…`), whatever
/// junction, short name or redirection of the profile led to it.
fn real_path(path: &Path) -> std::io::Result<PathBuf> {
    let real = fs::canonicalize(path)?;
    let text = real.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\UNC\") {
        Some(share) => PathBuf::from(format!(r"\\{share}")),
        None => PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text)),
    })
}

/// Whether a drive's folder was left, by a crash or by edits that could not be sent: a clean
/// disconnection removes it. Windows' own registry of drives is not asked, as it can answer
/// that a registered folder is unknown.
pub fn is_left(id: &str, drives: &Path) -> bool {
    fs::read_dir(drives.join(id)).is_ok_and(|mut entries| entries.next().is_some())
}

impl Mount {
    pub fn start(
        connection: &Connection,
        secrets: &Secrets,
        trusted: Option<&str>,
        drives: &Path,
    ) -> Result<Self, MountError> {
        let link = || {
            Link::new(
                connection.clone(),
                secrets.clone(),
                trusted.map(String::from),
            )
        };
        let mut browse = link();
        browse.open().map_err(MountError::Remote)?;
        let letter = (!connection.letter.is_empty()).then(|| connection.letter.clone());
        let sync_root = sync_root_id(&connection.id)?;
        let registered = sync_root.is_registered().unwrap_or(false);
        let folder = drives.join(&connection.id);
        // A folder left from before is kept as it is: what it holds that was not sent leaves now.
        fs::create_dir_all(&folder).map_err(system)?;
        let root = real_path(&folder).map_err(system)?;
        if let Some(letter) = &letter
            && drive::is_taken(letter)
        {
            drive::release(letter, &root);
            if drive::is_taken(letter) {
                // Only an empty folder goes: one left from before keeps its edits.
                let _ = fs::remove_dir(&root);
                return Err(MountError::LetterTaken);
            }
        }
        if !registered
            || sync_root
                .info()
                .is_ok_and(|info| info.display_name() != connection.shown_name())
        {
            sync_root
                .register(root_info(connection, &root)?)
                .map_err(system)?;
        }
        let worker = Worker::start(root.clone(), link());
        let filter = Filter::new(root.clone(), browse, link(), worker);
        let attached = Session::new().connect(&root, filter).map_err(system)?;
        // Edits made while it was not connected are sent now.
        attached
            .filter()
            .state_changed(StateWatchEvent::Changed(vec![root.clone()]));
        let mount = Self {
            attached: Some(attached),
            root,
            sync_root,
            letter,
        };
        if let Some(letter) = &mount.letter
            && !drive::assign(letter, &mount.root, connection.shown_name())
        {
            mount.stop();
            return Err(MountError::LetterTaken);
        }
        Ok(mount)
    }

    /// The letter if it has one, else the folder.
    pub fn place(&self) -> PathBuf {
        self.letter.as_ref().map_or_else(
            || self.root.clone(),
            |letter| PathBuf::from(format!("{letter}:\\")),
        )
    }

    /// Sends what is left, then removes the drive. Its folder stays, still registered, if some
    /// edits could not be sent: they leave on the next connection.
    pub fn stop(mut self) {
        let sent = self
            .attached
            .as_ref()
            .is_none_or(|attached| attached.filter().worker.flush(FLUSH_LIMIT));
        if let Some(letter) = &self.letter {
            drive::release(letter, &self.root);
        }
        drop(self.attached.take());
        if sent {
            let _ = self.sync_root.unregister();
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// Removes a drive of a deleted connection, or one left by a crash, with nothing to send.
pub fn forget(id: &str, letter: &str, drives: &Path) {
    let folder = drives.join(id);
    let root = real_path(&folder).unwrap_or(folder);
    if !letter.is_empty() {
        drive::release(letter, &root);
    }
    if let Ok(sync_root) = sync_root_id(id)
        && sync_root.is_registered().unwrap_or(false)
    {
        let _ = sync_root.unregister();
    }
    let _ = fs::remove_dir_all(root);
}
