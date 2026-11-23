//! A saved connection, as the interface edits it. Secrets are kept apart, never sent back.

use serde::{Deserialize, Serialize};

pub const MAX_NAME: usize = 40;
const MAX_FIELD: usize = 255;
const LETTERS: &str = "DEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Sftp,
    Ftp,
    Ftps,
}

impl Protocol {
    pub fn default_port(self) -> u16 {
        match self {
            Self::Sftp => 22,
            Self::Ftp | Self::Ftps => 21,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    #[default]
    Password,
    Key,
    Anonymous,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    /// The file name of the key the user picked, shown in the form. The key itself is a secret.
    pub key_name: String,
    /// The folder of the server shown as the drive, `/` for its root.
    pub folder: String,
    /// `Z`, or empty for no drive letter.
    pub letter: String,
    pub auto_connect: bool,
}

impl Connection {
    /// Fixes what the interface may have sent wrong, so that the program never acts on it.
    pub fn normalized(mut self) -> Self {
        self.name = clip(self.name.trim(), MAX_NAME);
        self.host = clip(self.host.trim(), MAX_FIELD);
        self.user = clip(&self.user, MAX_FIELD);
        self.key_name = clip(&self.key_name, MAX_FIELD);
        if self.port == 0 {
            self.port = self.protocol.default_port();
        }
        self.auth = match (self.protocol, self.auth) {
            (Protocol::Sftp, Auth::Anonymous) | (Protocol::Ftp | Protocol::Ftps, Auth::Key) => {
                Auth::Password
            }
            (_, auth) => auth,
        };
        self.folder = crate::paths::clean_folder(&self.folder);
        self.letter = drive_letter(&self.letter)
            .map(String::from)
            .unwrap_or_default();
        self
    }

    /// `true` once it names a server and who to log in as.
    pub fn is_complete(&self) -> bool {
        !self.host.is_empty() && (self.auth == Auth::Anonymous || !self.user.is_empty())
    }

    /// The name shown in Explorer: one never empty.
    pub fn shown_name(&self) -> &str {
        if self.name.is_empty() {
            &self.host
        } else {
            &self.name
        }
    }
}

/// Ids are made by the interface. They name the drive's folder and its Windows registration.
pub fn is_valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// `z:`, `Z` and `Z:\` all name `Z`. A and B are left to floppy drives, C to Windows.
pub fn drive_letter(value: &str) -> Option<&'static str> {
    let letter = value
        .trim()
        .trim_end_matches(['\\', '/'])
        .trim_end_matches(':');
    let upper = letter.to_ascii_uppercase();
    (upper.len() == 1)
        .then(|| LETTERS.find(upper.as_str()))
        .flatten()
        .map(|index| &LETTERS[index..=index])
}

fn clip(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_drive_letters() {
        assert_eq!(drive_letter("z:"), Some("Z"));
        assert_eq!(drive_letter("Y:\\"), Some("Y"));
        assert_eq!(drive_letter("X"), Some("X"));
        assert_eq!(drive_letter("C:"), None);
        assert_eq!(drive_letter("ZZ"), None);
        assert_eq!(drive_letter(""), None);
    }

    #[test]
    fn normalizes_what_the_interface_sends() {
        let connection = Connection {
            name: "  VPS\n ".into(),
            host: " 203.0.113.24 ".into(),
            protocol: Protocol::Ftp,
            auth: Auth::Key,
            folder: "www/".into(),
            letter: "z".into(),
            ..Connection::default()
        }
        .normalized();
        assert_eq!(connection.name, "VPS");
        assert_eq!(connection.host, "203.0.113.24");
        assert_eq!(connection.port, 21);
        assert_eq!(connection.auth, Auth::Password);
        assert_eq!(connection.folder, "/www");
        assert_eq!(connection.letter, "Z");
    }

    #[test]
    fn accepts_only_simple_ids() {
        assert!(is_valid_id("m3x9k2-ab"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("../x"));
        assert!(!is_valid_id("A!B"));
        assert!(!is_valid_id(&"a".repeat(33)));
    }

    #[test]
    fn knows_when_it_can_connect() {
        let mut connection = Connection {
            host: "example.com".into(),
            ..Connection::default()
        };
        assert!(!connection.is_complete());
        connection.auth = Auth::Anonymous;
        assert!(connection.is_complete());
        assert_eq!(connection.shown_name(), "example.com");
    }
}
