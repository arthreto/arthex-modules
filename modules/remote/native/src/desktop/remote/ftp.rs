//! FTP, and FTPS with explicit TLS. The server's certificate is checked by Windows.

use std::{
    io::{self, Read},
    net::ToSocketAddrs,
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};

use suppaftp::{
    FtpError, NativeTlsConnector, NativeTlsFtpStream, Status, list::File, native_tls::TlsConnector,
    types::FileType,
};

use super::{Entry, Failure, Remote};
use crate::{
    desktop::store::Secrets,
    model::{Auth, Connection, Protocol},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const CHUNK: usize = 256 * 1024;

pub struct Ftp {
    stream: NativeTlsFtpStream,
    /// Whether the server lists with `MLSD`, known after the first listing.
    machine_listing: Option<bool>,
}

impl Ftp {
    pub fn open(connection: &Connection, secrets: &Secrets) -> Result<Self, Failure> {
        let address = (connection.host.as_str(), connection.port)
            .to_socket_addrs()
            .map_err(|error| Failure::Unreachable(error.to_string()))?
            .next()
            .ok_or_else(|| Failure::Unreachable("no address".into()))?;
        let stream = NativeTlsFtpStream::connect_timeout(address, CONNECT_TIMEOUT)
            .map_err(|error| Failure::Unreachable(error.to_string()))?;
        let mut stream = if connection.protocol == Protocol::Ftps {
            let tls = TlsConnector::new().map_err(|error| Failure::Other(error.to_string()))?;
            stream
                .into_secure(NativeTlsConnector::from(tls), &connection.host)
                .map_err(|error| Failure::Certificate(error.to_string()))?
        } else {
            stream
        };
        // Servers behind a router often announce their private address for data connections.
        stream.set_passive_nat_workaround(true);
        let (user, password) = if connection.auth == Auth::Anonymous {
            ("anonymous", "anonymous@")
        } else {
            (connection.user.as_str(), secrets.password.as_str())
        };
        stream.login(user, password).map_err(|error| match error {
            FtpError::UnexpectedResponse(_) => Failure::Refused,
            error => Failure::Unreachable(error.to_string()),
        })?;
        stream
            .transfer_type(FileType::Binary)
            .map_err(|error| Failure::Other(error.to_string()))?;
        Ok(Self {
            stream,
            machine_listing: None,
        })
    }

    fn lines(&mut self, path: &str) -> io::Result<Vec<String>> {
        if self.machine_listing != Some(false) {
            match self.stream.mlsd(Some(path)) {
                Ok(lines) => {
                    self.machine_listing = Some(true);
                    return Ok(lines);
                }
                Err(FtpError::UnexpectedResponse(response))
                    if self.machine_listing.is_none() && is_unknown_command(response.status) =>
                {
                    self.machine_listing = Some(false);
                }
                Err(error) => return Err(io_error(error)),
            }
        }
        self.stream.list(Some(path)).map_err(io_error)
    }

    fn is_dir(&mut self, path: &str) -> bool {
        self.stream.cwd(path).is_ok()
    }
}

fn is_unknown_command(status: Status) -> bool {
    matches!(
        status,
        Status::NotImplemented
            | Status::BadCommand
            | Status::NotImplementedParameter
            | Status::BadArguments
    )
}

fn io_error(error: FtpError) -> io::Error {
    match error {
        FtpError::UnexpectedResponse(response) => {
            let kind = match response.status {
                Status::FileUnavailable | Status::BadFilename => io::ErrorKind::NotFound,
                Status::NotLoggedIn | Status::LoggedIn => io::ErrorKind::PermissionDenied,
                Status::NotImplemented | Status::NotImplementedParameter => {
                    io::ErrorKind::Unsupported
                }
                _ => io::ErrorKind::Other,
            };
            io::Error::new(
                kind,
                String::from_utf8_lossy(&response.body).trim().to_string(),
            )
        }
        FtpError::ConnectionError(error) => io::Error::new(io::ErrorKind::ConnectionAborted, error),
        error => io::Error::new(io::ErrorKind::ConnectionAborted, error.to_string()),
    }
}

fn seconds(file: &File) -> Option<i64> {
    let since = file.modified().duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(since.as_secs())
        .ok()
        .filter(|seconds| *seconds > 0)
}

impl Remote for Ftp {
    fn list(&mut self, path: &str) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        for line in self.lines(path)? {
            // Machine listings and Unix or DOS ones alike.
            let Ok(file) = File::from_str(&line) else {
                continue;
            };
            let name = file.name().to_string();
            if name == "." || name == ".." {
                continue;
            }
            let is_dir = if file.is_symlink() {
                self.is_dir(&format!("{}/{name}", path.trim_end_matches('/')))
            } else {
                file.is_directory()
            };
            entries.push(Entry {
                is_dir,
                size: file.size() as u64,
                modified: seconds(&file),
                name,
            });
        }
        Ok(entries)
    }

    fn stat(&mut self, path: &str) -> io::Result<Entry> {
        let name = path.rsplit('/').next().unwrap_or_default().to_string();
        if self.is_dir(path) {
            return Ok(Entry {
                name,
                is_dir: true,
                size: 0,
                modified: None,
            });
        }
        let size = self.stream.size(path).map_err(io_error)?;
        Ok(Entry {
            name,
            is_dir: false,
            size: size as u64,
            modified: None,
        })
    }

    fn read(
        &mut self,
        path: &str,
        start: u64,
        end: u64,
        sink: &mut dyn FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let offset =
            usize::try_from(start).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        self.stream.resume_transfer(offset).map_err(io_error)?;
        let mut transfer = self.stream.retr_as_stream(path).map_err(io_error)?;
        let mut buffer = vec![0; CHUNK];
        let mut position = start;
        let result = (|| {
            while position < end {
                let wanted = usize::try_from(end - position).unwrap_or(CHUNK).min(CHUNK);
                let read = transfer.read(&mut buffer[..wanted])?;
                if read == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                sink(&buffer[..read])?;
                position += read as u64;
            }
            // One more byte tells whether the server is done sending, or must be stopped.
            transfer.read(&mut buffer[..1])
        })();
        match result {
            Ok(0) => transfer.finish().map_err(io_error),
            Ok(_) => self.stream.abort(transfer).map_err(io_error),
            Err(error) => {
                let _ = self.stream.abort(transfer);
                Err(error)
            }
        }
    }

    fn write(&mut self, path: &str, source: &mut dyn Read) -> io::Result<()> {
        self.stream
            .put_file(path, &mut { source })
            .map(|_| ())
            .map_err(io_error)
    }

    fn create_dir(&mut self, path: &str) -> io::Result<()> {
        self.stream.mkdir(path).map_err(io_error)
    }

    fn remove_file(&mut self, path: &str) -> io::Result<()> {
        self.stream.rm(path).map_err(io_error)
    }

    fn remove_dir(&mut self, path: &str) -> io::Result<()> {
        self.stream.rmdir(path).map_err(io_error)
    }

    fn rename(&mut self, from: &str, to: &str) -> io::Result<()> {
        self.stream.rename(from, to).map_err(io_error)
    }
}

impl Drop for Ftp {
    fn drop(&mut self) {
        let _ = self.stream.quit();
    }
}
