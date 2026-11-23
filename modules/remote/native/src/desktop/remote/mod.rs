//! The servers, behind one trait: what the drive asks of them, whatever their protocol.

mod ftp;
mod sftp;

use std::io::{self, Read};

use crate::{
    desktop::store::Secrets,
    model::{Connection, Protocol},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// Seconds since 1970.
    pub modified: Option<i64>,
}

/// Why a connection did not open, as the interface explains it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Unreachable(String),
    Refused,
    /// First connection to this SSH server: the user is asked to trust its key.
    UnknownHost {
        algorithm: String,
        fingerprint: String,
    },
    /// The server presents another key than the one trusted.
    ChangedHost {
        fingerprint: String,
    },
    Certificate(String),
    BadKey,
    NoFolder,
    Other(String),
}

pub trait Remote: Send {
    fn list(&mut self, path: &str) -> io::Result<Vec<Entry>>;
    fn stat(&mut self, path: &str) -> io::Result<Entry>;
    /// Hands `sink` the bytes of `start..end`, in order.
    fn read(
        &mut self,
        path: &str,
        start: u64,
        end: u64,
        sink: &mut dyn FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()>;
    /// Replaces the file with everything `source` gives.
    fn write(&mut self, path: &str, source: &mut dyn Read) -> io::Result<()>;
    fn create_dir(&mut self, path: &str) -> io::Result<()>;
    fn remove_file(&mut self, path: &str) -> io::Result<()>;
    /// An empty folder.
    fn remove_dir(&mut self, path: &str) -> io::Result<()>;
    fn rename(&mut self, from: &str, to: &str) -> io::Result<()>;
}

/// The server's key already trusted for this connection, if any (SFTP only).
pub fn open(
    connection: &Connection,
    secrets: &Secrets,
    trusted: Option<&str>,
) -> Result<Box<dyn Remote>, Failure> {
    let mut remote: Box<dyn Remote> = match connection.protocol {
        Protocol::Sftp => Box::new(sftp::Sftp::open(connection, secrets, trusted)?),
        Protocol::Ftp | Protocol::Ftps => Box::new(ftp::Ftp::open(connection, secrets)?),
    };
    match remote.stat(&connection.folder) {
        Ok(entry) if entry.is_dir => Ok(remote),
        Ok(_) => Err(Failure::NoFolder),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(Failure::NoFolder),
        Err(error) => Err(Failure::Other(error.to_string())),
    }
}

/// A folder and all it holds, which servers only delete once empty.
pub fn remove_all(remote: &mut dyn Remote, path: &str) -> io::Result<()> {
    for entry in remote.list(path)? {
        let child = format!("{}/{}", path.trim_end_matches('/'), entry.name);
        if entry.is_dir {
            remove_all(remote, &child)?;
        } else {
            remote.remove_file(&child)?;
        }
    }
    remote.remove_dir(path)
}

/// Errors that a new connection may fix, unlike a missing file or a refusal.
pub fn is_link_lost(error: &io::Error) -> bool {
    !matches!(
        error.kind(),
        io::ErrorKind::NotFound
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::AlreadyExists
            | io::ErrorKind::InvalidInput
            | io::ErrorKind::Unsupported
            | io::ErrorKind::DirectoryNotEmpty
    )
}
