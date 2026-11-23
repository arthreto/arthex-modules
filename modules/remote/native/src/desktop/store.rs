//! The saved connections, in one file of the module's folder. Passwords and keys are encrypted for
//! the Windows account; the server keys the user trusted sit beside them.

use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use super::secret;
use crate::model::{Connection, is_valid_id};

const FILE: &str = "connections.json";
const TEMPORARY_FILE: &str = "connections.json.tmp";
const MAX_CONNECTIONS: usize = 100;
/// A private key is a few kilobytes; a bigger file is not one.
pub const MAX_KEY_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Secrets {
    pub password: String,
    pub passphrase: String,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Password,
    Passphrase,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    connections: Vec<Connection>,
    /// Encrypted `Secrets` by connection id.
    secrets: HashMap<String, String>,
    /// `host:port` → the SHA-256 fingerprint of the SSH key the user trusted.
    hosts: HashMap<String, String>,
}

pub struct Store {
    file: PathBuf,
    saved: Saved,
}

impl Store {
    pub fn load(dir: &Path) -> Self {
        let file = dir.join(FILE);
        let saved = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { file, saved }
    }

    pub fn connections(&self) -> &[Connection] {
        &self.saved.connections
    }

    pub fn connection(&self, id: &str) -> Option<&Connection> {
        self.saved
            .connections
            .iter()
            .find(|connection| connection.id == id)
    }

    /// Takes the list the interface edited. Secrets of removed connections go with them.
    pub fn replace(&mut self, connections: Vec<Connection>) -> io::Result<()> {
        let mut kept: Vec<Connection> = Vec::new();
        for connection in connections.into_iter().map(Connection::normalized) {
            if is_valid_id(&connection.id) && !kept.iter().any(|other| other.id == connection.id) {
                kept.push(connection);
            }
        }
        kept.truncate(MAX_CONNECTIONS);
        self.saved
            .secrets
            .retain(|id, _| kept.iter().any(|connection| &connection.id == id));
        self.saved.connections = kept;
        self.write()
    }

    pub fn secrets(&self, id: &str) -> Secrets {
        self.saved
            .secrets
            .get(id)
            .and_then(|encoded| STANDARD.decode(encoded).ok())
            .and_then(|bytes| secret::unprotect(&bytes))
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn set_secrets(&mut self, id: &str, secrets: &Secrets) -> io::Result<()> {
        if self.connection(id).is_none() {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        let bytes = serde_json::to_vec(secrets)?;
        let encrypted =
            secret::protect(&bytes).ok_or_else(|| io::Error::other("could not encrypt"))?;
        self.saved
            .secrets
            .insert(id.to_string(), STANDARD.encode(encrypted));
        self.write()
    }

    pub fn set_field(&mut self, id: &str, field: Field, value: String) -> io::Result<()> {
        let mut secrets = self.secrets(id);
        match field {
            Field::Password => secrets.password = value,
            Field::Passphrase => secrets.passphrase = value,
        }
        self.set_secrets(id, &secrets)
    }

    pub fn set_key(&mut self, id: &str, key: String) -> io::Result<()> {
        let mut secrets = self.secrets(id);
        secrets.key = key;
        self.set_secrets(id, &secrets)
    }

    pub fn trusted(&self, connection: &Connection) -> Option<&str> {
        self.saved
            .hosts
            .get(&host_key(connection))
            .map(String::as_str)
    }

    pub fn trust(&mut self, connection: &Connection, fingerprint: &str) -> io::Result<()> {
        self.saved
            .hosts
            .insert(host_key(connection), fingerprint.to_string());
        self.write()
    }

    fn write(&self) -> io::Result<()> {
        let temporary = self.file.with_file_name(TEMPORARY_FILE);
        fs::write(&temporary, serde_json::to_vec(&self.saved)?)?;
        fs::rename(temporary, &self.file)
    }
}

fn host_key(connection: &Connection) -> String {
    format!(
        "{}:{}",
        connection.host.to_ascii_lowercase(),
        connection.port
    )
}
