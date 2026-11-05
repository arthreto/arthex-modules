//! What each type of file can become.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Video,
    Audio,
    Document,
}

const IMAGES: [&str; 8] = ["jpeg", "png", "webp", "avif", "ico", "gif", "tiff", "bmp"];
const VIDEOS: [&str; 6] = ["mp4", "webm", "mov", "mkv", "avi", "gif"];
const AUDIOS: [&str; 5] = ["mp3", "m4a", "wav", "flac", "ogg"];
const DOCUMENTS: [&str; 5] = ["pdf", "docx", "odt", "txt", "html"];

pub fn kind_of(extension: &str) -> Option<Kind> {
    match extension.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "bmp" | "tif" | "tiff" | "ico"
        | "heic" => Some(Kind::Image),
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "wmv" | "flv" | "mpg" | "mpeg" => {
            Some(Kind::Video)
        }
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus" | "wma" => Some(Kind::Audio),
        "pdf" | "docx" | "doc" | "odt" | "rtf" | "txt" | "html" | "htm" | "pptx" | "xlsx" => {
            Some(Kind::Document)
        }
        _ => None,
    }
}

pub fn is_audio(target: &str) -> bool {
    AUDIOS.contains(&target)
}

/// A video can also become its sound.
pub fn is_allowed(kind: Kind, target: &str) -> bool {
    match kind {
        Kind::Image => IMAGES.contains(&target),
        Kind::Video => VIDEOS.contains(&target) || AUDIOS.contains(&target),
        Kind::Audio => AUDIOS.contains(&target),
        Kind::Document => DOCUMENTS.contains(&target),
    }
}

/// The extension of a format's files.
pub fn extension_of(target: &str) -> &str {
    if target == "jpeg" { "jpg" } else { target }
}

fn same_format(extension: &str, target: &str) -> bool {
    let extension = extension.to_ascii_lowercase();
    extension == target
        || (target == "jpeg" && extension == "jpg")
        || (target == "tiff" && extension == "tif")
}

/// The first format that is not the file's own, so a new file never starts as a useless copy.
pub fn default_target(kind: Kind, extension: &str) -> &'static str {
    let order: &[&'static str] = match kind {
        Kind::Image => &["jpeg", "png"],
        Kind::Video => &["mp4", "webm"],
        Kind::Audio => &["mp3", "m4a"],
        Kind::Document => &["pdf", "docx"],
    };
    order
        .iter()
        .find(|target| !same_format(extension, target))
        .copied()
        .unwrap_or(order[0])
}

/// The format a compressed file keeps. Uncompressed sound has to change format to get smaller.
pub fn compressed_target(kind: Kind, extension: &str) -> String {
    let extension = extension.to_ascii_lowercase();
    let known = |list: &[&str]| list.contains(&extension.as_str());
    match kind {
        Kind::Image if extension == "jpg" || extension == "jpeg" => "jpeg".to_string(),
        Kind::Image if extension == "tif" => "tiff".to_string(),
        Kind::Image if known(&IMAGES) => extension,
        Kind::Image => "jpeg".to_string(),
        Kind::Video if known(&VIDEOS) => extension,
        Kind::Video => "mp4".to_string(),
        Kind::Audio if extension == "wav" => "flac".to_string(),
        Kind::Audio if known(&AUDIOS) => extension,
        Kind::Audio => "m4a".to_string(),
        Kind::Document => extension,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_files_whatever_their_case() {
        assert_eq!(kind_of("PNG"), Some(Kind::Image));
        assert_eq!(kind_of("mov"), Some(Kind::Video));
        assert_eq!(kind_of("wav"), Some(Kind::Audio));
        assert_eq!(kind_of("pdf"), Some(Kind::Document));
        assert_eq!(kind_of("exe"), None);
    }

    #[test]
    fn a_video_can_become_sound_but_not_the_reverse() {
        assert!(is_allowed(Kind::Video, "mp3"));
        assert!(!is_allowed(Kind::Audio, "mp4"));
        assert!(is_allowed(Kind::Image, "ico"));
        assert!(!is_allowed(Kind::Document, "jpeg"));
    }

    #[test]
    fn never_starts_on_the_format_a_file_already_has() {
        assert_eq!(default_target(Kind::Image, "JPG"), "png");
        assert_eq!(default_target(Kind::Image, "png"), "jpeg");
        assert_eq!(default_target(Kind::Video, "mp4"), "webm");
        assert_eq!(default_target(Kind::Audio, "wav"), "mp3");
    }

    #[test]
    fn a_compressed_file_keeps_its_format_when_it_can() {
        assert_eq!(compressed_target(Kind::Image, "JPG"), "jpeg");
        assert_eq!(compressed_target(Kind::Image, "heic"), "jpeg");
        assert_eq!(compressed_target(Kind::Video, "mov"), "mov");
        assert_eq!(compressed_target(Kind::Video, "wmv"), "mp4");
        assert_eq!(compressed_target(Kind::Audio, "wav"), "flac");
        assert_eq!(compressed_target(Kind::Audio, "mp3"), "mp3");
    }
}
