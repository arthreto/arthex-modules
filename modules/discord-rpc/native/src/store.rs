//! The templates, in one file of the module's folder.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

const FILE: &str = "templates.json";
const TEMPORARY_FILE: &str = "templates.json.tmp";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Playing,
    Listening,
    Watching,
    Competing,
}

/// What the short status of the member list names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Show {
    #[default]
    Name,
    Details,
    State,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Time {
    #[default]
    None,
    Elapsed,
    Left,
    Dates,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Button {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub app_id: String,
    pub title: String,
    pub kind: Kind,
    pub show: Show,
    pub details: String,
    pub details_url: String,
    pub state: String,
    pub state_url: String,
    pub large: String,
    pub large_text: String,
    pub large_url: String,
    pub small: String,
    pub small_text: String,
    pub small_url: String,
    pub time: Time,
    pub minutes: Option<u32>,
    /// Milliseconds.
    pub from: Option<i64>,
    /// Milliseconds.
    pub to: Option<i64>,
    pub size: Option<u32>,
    pub max: Option<u32>,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub templates: Vec<Template>,
    pub active: Option<String>,
}

pub fn load(dir: &Path) -> Saved {
    fs::read(dir.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// In one rename, so that a crash never leaves half a file.
pub fn save(dir: &Path, saved: &Saved) -> io::Result<()> {
    let temporary = dir.join(TEMPORARY_FILE);
    fs::write(&temporary, serde_json::to_vec_pretty(saved)?)?;
    fs::rename(temporary, dir.join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_what_it_saved() {
        let dir = std::env::temp_dir().join(format!("arthex-rpc-store-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let saved = Saved {
            templates: vec![Template {
                id: "a".into(),
                kind: Kind::Competing,
                minutes: Some(25),
                buttons: vec![Button {
                    label: "Site".into(),
                    url: "https://example.com".into(),
                }],
                ..Template::default()
            }],
            active: Some("a".into()),
        };
        save(&dir, &saved).unwrap();
        assert_eq!(load(&dir), saved);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn starts_empty_without_a_readable_file() {
        let dir = std::env::temp_dir().join("arthex-rpc-store-missing");
        assert_eq!(load(&dir), Saved::default());
    }

    #[test]
    fn fills_in_what_an_older_file_lacks() {
        let template: Template = serde_json::from_str(r#"{"id":"a","kind":"watching"}"#).unwrap();
        assert_eq!(template.kind, Kind::Watching);
        assert_eq!(template.show, Show::Name);
        assert!(template.buttons.is_empty());
    }
}
