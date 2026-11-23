//! SFTP, over SSH. The server's key is checked against the one the user trusted.

use std::{
    io::{self, Read},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use russh::{
    Disconnect,
    client::{self, AuthResult, Handle, KeyboardInteractiveAuthResponse},
    keys::{self, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate},
};
use russh_sftp::{
    client::{SftpSession, error::Error as SftpError},
    protocol::{FileType, StatusCode},
};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    runtime::Runtime,
};

use super::{Entry, Failure, Remote};
use crate::{
    desktop::store::Secrets,
    model::{Auth, Connection},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const CHUNK: usize = 256 * 1024;

/// The key the server showed, kept to ask the user about it.
struct Shown {
    algorithm: String,
    fingerprint: String,
}

struct Client {
    trusted: Option<String>,
    shown: Arc<Mutex<Option<Shown>>>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        // Certificates need an authority the user never chose: they are not offered.
        let accepted = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => {
                let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
                let accepted = self.trusted.as_deref() == Some(fingerprint.as_str());
                *self.shown.lock().unwrap_or_else(PoisonError::into_inner) = Some(Shown {
                    algorithm: algorithm_name(key.algorithm().as_str()),
                    fingerprint,
                });
                accepted
            }
            PublicKeyOrCertificate::Certificate(_) => false,
        };
        async move { Ok(accepted) }
    }
}

pub struct Sftp {
    runtime: Runtime,
    handle: Handle<Client>,
    session: SftpSession,
}

impl Sftp {
    pub fn open(
        connection: &Connection,
        secrets: &Secrets,
        trusted: Option<&str>,
    ) -> Result<Self, Failure> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| Failure::Other(error.to_string()))?;
        let shown = Arc::new(Mutex::new(None));
        let client = Client {
            trusted: trusted.map(String::from),
            shown: Arc::clone(&shown),
        };
        let config = Arc::new(client::Config {
            keepalive_interval: Some(Duration::from_secs(30)),
            ..client::Config::default()
        });
        let address = (connection.host.clone(), connection.port);
        let (handle, session) = runtime.block_on(async {
            let connected =
                tokio::time::timeout(CONNECT_TIMEOUT, client::connect(config, address, client))
                    .await
                    .map_err(|_| Failure::Unreachable("timeout".into()))?;
            let mut handle = connected.map_err(|error| {
                host_failure(&shown, trusted)
                    .unwrap_or_else(|| Failure::Unreachable(error.to_string()))
            })?;
            authenticate(&mut handle, connection, secrets).await?;
            let channel = handle.channel_open_session().await.map_err(other)?;
            channel
                .request_subsystem(true, "sftp")
                .await
                .map_err(other)?;
            let session = SftpSession::new(channel.into_stream())
                .await
                .map_err(other)?;
            Ok::<_, Failure>((handle, session))
        })?;
        Ok(Self {
            runtime,
            handle,
            session,
        })
    }
}

fn host_failure(shown: &Mutex<Option<Shown>>, trusted: Option<&str>) -> Option<Failure> {
    let shown = shown.lock().unwrap_or_else(PoisonError::into_inner);
    let shown = shown.as_ref()?;
    match trusted {
        Some(trusted) if trusted == shown.fingerprint => None,
        Some(_) => Some(Failure::ChangedHost {
            fingerprint: shown.fingerprint.clone(),
        }),
        None => Some(Failure::UnknownHost {
            algorithm: shown.algorithm.clone(),
            fingerprint: shown.fingerprint.clone(),
        }),
    }
}

async fn authenticate(
    handle: &mut Handle<Client>,
    connection: &Connection,
    secrets: &Secrets,
) -> Result<(), Failure> {
    let user = connection.user.as_str();
    let accepted = if connection.auth == Auth::Key {
        let passphrase = (!secrets.passphrase.is_empty()).then_some(secrets.passphrase.as_str());
        let key = keys::decode_secret_key(&secrets.key, passphrase).map_err(|_| Failure::BadKey)?;
        let hash = handle
            .best_supported_rsa_hash()
            .await
            .map_err(other)?
            .flatten();
        let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
        handle
            .authenticate_publickey(user, key)
            .await
            .map_err(other)?
            .success()
    } else {
        let result = handle
            .authenticate_password(user, secrets.password.as_str())
            .await
            .map_err(other)?;
        matches!(result, AuthResult::Success)
            || keyboard_interactive(handle, user, &secrets.password).await?
    };
    if accepted {
        Ok(())
    } else {
        Err(Failure::Refused)
    }
}

/// Servers that only ask for the password through prompts, as some Linux defaults do.
async fn keyboard_interactive(
    handle: &mut Handle<Client>,
    user: &str,
    password: &str,
) -> Result<bool, Failure> {
    let mut reply = handle
        .authenticate_keyboard_interactive_start(user, None)
        .await
        .map_err(other)?;
    for _ in 0..3 {
        match reply {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = prompts.iter().map(|_| password.to_string()).collect();
                reply = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(other)?;
            }
        }
    }
    Ok(false)
}

#[allow(clippy::needless_pass_by_value)]
fn other(error: impl ToString) -> Failure {
    Failure::Other(error.to_string())
}

/// `ssh-ed25519` reads `ED25519`, as the mockup shows it.
fn algorithm_name(algorithm: &str) -> String {
    let name = algorithm.strip_prefix("ssh-").unwrap_or(algorithm);
    let name = if name.starts_with("ecdsa") {
        "ecdsa"
    } else {
        name
    };
    name.to_ascii_uppercase()
}

fn io_error(error: SftpError) -> io::Error {
    match error {
        SftpError::Status(status) => {
            let kind = match status.status_code {
                StatusCode::NoSuchFile => io::ErrorKind::NotFound,
                StatusCode::PermissionDenied => io::ErrorKind::PermissionDenied,
                StatusCode::OpUnsupported => io::ErrorKind::Unsupported,
                StatusCode::NoConnection | StatusCode::ConnectionLost => {
                    io::ErrorKind::ConnectionAborted
                }
                _ => io::ErrorKind::Other,
            };
            io::Error::new(kind, status.error_message)
        }
        error => io::Error::new(io::ErrorKind::ConnectionAborted, error.to_string()),
    }
}

impl Remote for Sftp {
    fn list(&mut self, path: &str) -> io::Result<Vec<Entry>> {
        let session = &self.session;
        self.runtime.block_on(async {
            let mut entries = Vec::new();
            for entry in session.read_dir(path).await.map_err(io_error)? {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                let metadata = if entry.file_type() == FileType::Symlink {
                    // A link is shown as what it points to; a broken one is left out.
                    match session
                        .metadata(format!("{}/{name}", path.trim_end_matches('/')))
                        .await
                    {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    }
                } else {
                    entry.metadata()
                };
                entries.push(Entry {
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.mtime.map(i64::from),
                    name,
                });
            }
            Ok(entries)
        })
    }

    fn stat(&mut self, path: &str) -> io::Result<Entry> {
        let metadata = self
            .runtime
            .block_on(self.session.metadata(path))
            .map_err(io_error)?;
        Ok(Entry {
            name: path.rsplit('/').next().unwrap_or_default().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.mtime.map(i64::from),
        })
    }

    fn read(
        &mut self,
        path: &str,
        start: u64,
        end: u64,
        sink: &mut dyn FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let session = &self.session;
        self.runtime.block_on(async {
            let mut file = session.open(path).await.map_err(io_error)?;
            file.seek(io::SeekFrom::Start(start)).await?;
            let mut buffer = vec![0; CHUNK];
            let mut position = start;
            while position < end {
                let wanted = usize::try_from(end - position).unwrap_or(CHUNK).min(CHUNK);
                let read = file.read(&mut buffer[..wanted]).await?;
                if read == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                sink(&buffer[..read])?;
                position += read as u64;
            }
            Ok(())
        })
    }

    fn write(&mut self, path: &str, source: &mut dyn Read) -> io::Result<()> {
        let session = &self.session;
        self.runtime.block_on(async {
            let mut file = session.create(path).await.map_err(io_error)?;
            let mut buffer = vec![0; CHUNK];
            loop {
                let read = source.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                file.write_all(&buffer[..read]).await?;
            }
            file.shutdown().await
        })
    }

    fn create_dir(&mut self, path: &str) -> io::Result<()> {
        self.runtime
            .block_on(self.session.create_dir(path))
            .map_err(io_error)
    }

    fn remove_file(&mut self, path: &str) -> io::Result<()> {
        self.runtime
            .block_on(self.session.remove_file(path))
            .map_err(io_error)
    }

    fn remove_dir(&mut self, path: &str) -> io::Result<()> {
        self.runtime
            .block_on(self.session.remove_dir(path))
            .map_err(io_error)
    }

    fn rename(&mut self, from: &str, to: &str) -> io::Result<()> {
        self.runtime
            .block_on(self.session.rename(from, to))
            .map_err(io_error)
    }
}

impl Drop for Sftp {
    fn drop(&mut self) {
        let session = &self.session;
        let handle = &self.handle;
        self.runtime.block_on(async {
            let _ = session.close().await;
            let _ = handle.disconnect(Disconnect::ByApplication, "", "en").await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::algorithm_name;

    #[test]
    fn names_key_algorithms() {
        assert_eq!(algorithm_name("ssh-ed25519"), "ED25519");
        assert_eq!(algorithm_name("ssh-rsa"), "RSA");
        assert_eq!(algorithm_name("ecdsa-sha2-nistp256"), "ECDSA");
    }
}
