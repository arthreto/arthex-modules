//! A connection to the server that opens again by itself when the network drops it.

use std::io;

use crate::{
    desktop::{
        remote::{self, Failure, Remote},
        store::Secrets,
    },
    model::Connection,
};

pub struct Link {
    connection: Connection,
    secrets: Secrets,
    trusted: Option<String>,
    remote: Option<Box<dyn Remote>>,
}

impl Link {
    pub fn new(connection: Connection, secrets: Secrets, trusted: Option<String>) -> Self {
        Self {
            connection,
            secrets,
            trusted,
            remote: None,
        }
    }

    /// Opens it now, to tell the user at once what is wrong.
    pub fn open(&mut self) -> Result<(), Failure> {
        self.remote = Some(remote::open(
            &self.connection,
            &self.secrets,
            self.trusted.as_deref(),
        )?);
        Ok(())
    }

    pub fn folder(&self) -> &str {
        &self.connection.folder
    }

    /// Runs `work`, once more on a new connection if the old one was lost.
    pub fn run<T>(
        &mut self,
        mut work: impl FnMut(&mut dyn Remote) -> io::Result<T>,
    ) -> io::Result<T> {
        for attempt in 0..2 {
            if self.remote.is_none() {
                self.open().map_err(|failure| {
                    io::Error::new(io::ErrorKind::NotConnected, format!("{failure:?}"))
                })?;
            }
            let Some(remote) = self.remote.as_deref_mut() else {
                continue;
            };
            match work(remote) {
                Err(error) if attempt == 0 && remote::is_link_lost(&error) => self.remote = None,
                result => return result,
            }
        }
        Err(io::Error::from(io::ErrorKind::NotConnected))
    }
}
