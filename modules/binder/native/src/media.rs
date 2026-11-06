//! The images, videos and documents copied into a document, in `media/`, which Arthex serves to the
//! interface for what it can show. Only known kinds of file ever get in or are opened.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

const FOLDER: &str = "media";
const IMAGES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];
const VIDEOS: &[&str] = &["mp4", "m4v", "mov", "webm"];
const FILES: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf", "txt", "csv",
    "md", "epub", "zip", "7z", "rar",
];
static NEXT_NAME: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Video,
    File,
}

impl Kind {
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Image => IMAGES,
            Self::Video => VIDEOS,
            Self::File => FILES,
        }
    }
}

/// A file once copied: how the document refers to it, and what the user called it.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub file: String,
    pub name: String,
    pub size: u64,
}

fn known_extension(name: &str) -> Option<String> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    [IMAGES, VIDEOS, FILES]
        .iter()
        .any(|list| list.contains(&extension.as_str()))
        .then_some(extension)
}

/// A name in the folder, for what a document may ask of it.
fn is_media_name(name: &str) -> bool {
    !name.starts_with('.')
        && name.len() <= 80
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
        && known_extension(name).is_some()
}

fn new_name(extension: &str) -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_millis());
    let count = NEXT_NAME.fetch_add(1, Ordering::Relaxed);
    format!("{time:x}-{count:x}.{extension}")
}

pub struct Media {
    folder: PathBuf,
}

impl Media {
    pub fn new(data: &Path) -> Self {
        Self {
            folder: data.join(FOLDER),
        }
    }

    pub fn import(&self, source: &Path, kind: Kind) -> Result<Imported, String> {
        let file_name = source.to_string_lossy();
        let extension = known_extension(&file_name)
            .filter(|extension| kind.extensions().contains(&extension.as_str()))
            .ok_or("unknown kind of file")?;
        fs::create_dir_all(&self.folder).map_err(|error| error.to_string())?;
        let file = new_name(&extension);
        let size = fs::copy(source, self.folder.join(&file)).map_err(|error| error.to_string())?;
        let shown = if kind == Kind::File {
            source.file_name()
        } else {
            source.file_stem()
        };
        Ok(Imported {
            file,
            name: shown.map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            size,
        })
    }

    /// An image pasted from the clipboard.
    pub fn save_image(&self, bytes: &[u8], extension: &str) -> Result<Imported, String> {
        let extension = extension.to_ascii_lowercase();
        if !IMAGES.contains(&extension.as_str()) {
            return Err("unknown kind of image".to_string());
        }
        fs::create_dir_all(&self.folder).map_err(|error| error.to_string())?;
        let file = new_name(&extension);
        fs::write(self.folder.join(&file), bytes).map_err(|error| error.to_string())?;
        Ok(Imported {
            file,
            name: String::new(),
            size: bytes.len() as u64,
        })
    }

    /// The path of a file a document refers to, if it is really one of them.
    pub fn path_of(&self, name: &str) -> Option<PathBuf> {
        let path = self.folder.join(name);
        (is_media_name(name) && path.is_file()).then_some(path)
    }

    pub fn sizes(&self) -> BTreeMap<String, u64> {
        fs::read_dir(&self.folder)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                Some((
                    entry.file_name().into_string().ok()?,
                    entry.metadata().ok()?.len(),
                ))
            })
            .collect()
    }

    /// Takes away what no document refers to any more, at the start, not while one can be undone.
    pub fn prune(&self, texts: &[&str]) {
        for name in self.sizes().keys() {
            let wanted = format!("{FOLDER}/{name}");
            if !texts.iter().any(|text| text.contains(&wanted)) {
                let _ = fs::remove_file(self.folder.join(name));
            }
        }
    }
}

/// Only addresses of the web are opened from a link.
pub fn is_web_address(address: &str) -> bool {
    (address.starts_with("http://") || address.starts_with("https://"))
        && !address
            .chars()
            .any(|character| character.is_control() || character == ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media() -> (tempfile::TempDir, Media) {
        let folder = tempfile::tempdir().unwrap();
        let media = Media::new(folder.path());
        (folder, media)
    }

    #[test]
    fn a_file_is_copied_under_a_name_of_its_own() {
        let (folder, media) = media();
        let source = folder.path().join("Cours de maths.PDF");
        fs::write(&source, "contenu").unwrap();
        let imported = media.import(&source, Kind::File).unwrap();
        assert_eq!(known_extension(&imported.file).as_deref(), Some("pdf"));
        assert!(is_media_name(&imported.file));
        assert_eq!(imported.name, "Cours de maths.PDF");
        assert_eq!(imported.size, 7);
        assert_eq!(
            fs::read_to_string(media.path_of(&imported.file).unwrap()).unwrap(),
            "contenu"
        );
    }

    #[test]
    fn two_copies_never_share_a_name() {
        let (folder, media) = media();
        let source = folder.path().join("photo.png");
        fs::write(&source, "x").unwrap();
        let first = media.import(&source, Kind::Image).unwrap();
        let second = media.import(&source, Kind::Image).unwrap();
        assert_ne!(first.file, second.file);
        assert_eq!(first.name, "photo");
    }

    #[test]
    fn only_the_kind_asked_for_gets_in() {
        let (folder, media) = media();
        for (name, kind) in [
            ("a.exe", Kind::File),
            ("a.pdf", Kind::Image),
            ("a.png", Kind::Video),
        ] {
            let source = folder.path().join(name);
            fs::write(&source, "x").unwrap();
            assert!(media.import(&source, kind).is_err(), "{name}");
        }
        assert!(media.save_image(b"x", "exe").is_err());
    }

    #[test]
    fn a_document_cannot_reach_outside_the_folder_or_a_program() {
        let (folder, media) = media();
        fs::write(folder.path().join("secret.pdf"), "x").unwrap();
        fs::create_dir_all(&media.folder).unwrap();
        fs::write(media.folder.join("a.exe"), "x").unwrap();
        for name in [
            "../secret.pdf",
            "..\\secret.pdf",
            "a.exe",
            ".pdf",
            "absent.pdf",
        ] {
            assert_eq!(media.path_of(name), None, "{name}");
        }
    }

    #[test]
    fn what_no_document_refers_to_goes_away() {
        let (folder, media) = media();
        let source = folder.path().join("photo.png");
        fs::write(&source, "x").unwrap();
        let kept = media.import(&source, Kind::Image).unwrap();
        let lost = media.import(&source, Kind::Image).unwrap();
        let text = format!("![Photo](media/{})", kept.file);
        media.prune(&[text.as_str()]);
        assert!(media.path_of(&kept.file).is_some());
        assert!(media.path_of(&lost.file).is_none());
        assert_eq!(media.sizes().len(), 1);
    }

    #[test]
    fn only_the_web_is_opened_from_a_link() {
        assert!(is_web_address("https://fasttext.cc/docs"));
        assert!(is_web_address("http://exemple.fr"));
        for address in [
            "file:///c:/a.exe",
            "ms-settings:",
            "https://a b",
            "calc.exe",
            "https://a\nb",
        ] {
            assert!(!is_web_address(address), "{address}");
        }
    }
}
