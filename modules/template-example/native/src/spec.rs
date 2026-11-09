//! What the interface asks for, checked against the closed lists of choices it offers.

use std::path::PathBuf;

use serde::Deserialize;

use crate::{filler, media, picture, text};

pub const DOCUMENTS: [&str; 5] = ["pdf", "docx", "xlsx", "pptx", "odt"];
const ROWS: [usize; 4] = [10, 100, 1_000, 100_000];
const PAGES: [usize; 4] = [1, 5, 20, 100];
const SECONDS: [u32; 4] = [5, 30, 60, 600];
const FILES: [usize; 3] = [5, 50, 500];
const SIZES: [(u32, u32); 4] = [(512, 512), (1280, 720), (1920, 1080), (3840, 2160)];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub folder: PathBuf,
    pub name: String,
    pub format: String,
    pub picture: Option<String>,
    pub size: Option<(u32, u32)>,
    pub rows: Option<usize>,
    pub pages: Option<usize>,
    pub seconds: Option<u32>,
    pub height: Option<u32>,
    pub files: Option<usize>,
    pub bytes: Option<u64>,
    #[serde(default)]
    pub random: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Spec {
    Text {
        rows: usize,
    },
    Image {
        picture: String,
        size: Option<(u32, u32)>,
    },
    Document {
        pages: usize,
    },
    Media {
        seconds: u32,
        height: u32,
    },
    Archive {
        files: usize,
    },
    Sized {
        bytes: u64,
        random: bool,
    },
}

fn one_of<T: PartialEq + Copy>(value: Option<T>, allowed: &[T], default: T) -> Result<T, String> {
    match value {
        None => Ok(default),
        Some(value) if allowed.contains(&value) => Ok(value),
        Some(_) => Err("option".to_string()),
    }
}

/// What to make of `format` (already checked), with the options of the request.
pub fn spec(format: &str, request: &Request) -> Result<Spec, String> {
    if picture::FORMATS.contains(&format) {
        let picture = request
            .picture
            .clone()
            .unwrap_or_else(|| "landscape".to_string());
        if !picture::PICTURES.contains(&picture.as_str()) {
            return Err("option".to_string());
        }
        if request.size.is_some_and(|size| !SIZES.contains(&size)) {
            return Err("option".to_string());
        }
        return Ok(Spec::Image {
            picture,
            size: request.size,
        });
    }
    if text::content(format, 0, text::Language::English).is_some() {
        let rows = if text::WITH_ROWS.contains(&format) {
            one_of(request.rows, &ROWS, 100)?
        } else {
            0
        };
        return Ok(Spec::Text { rows });
    }
    if DOCUMENTS.contains(&format) {
        return Ok(Spec::Document {
            pages: one_of(request.pages, &PAGES, 5)?,
        });
    }
    if media::AUDIO.contains(&format) || media::VIDEO.contains(&format) {
        return Ok(Spec::Media {
            seconds: one_of(request.seconds, &SECONDS, 30)?,
            height: one_of(request.height, &media::HEIGHTS, 720)?,
        });
    }
    if format == "zip" {
        return Ok(Spec::Archive {
            files: one_of(request.files, &FILES, 5)?,
        });
    }
    let bytes = request.bytes.unwrap_or(100 * 1024 * 1024);
    if !(filler::MIN_BYTES..=filler::MAX_BYTES).contains(&bytes) {
        return Err("option".to_string());
    }
    Ok(Spec::Sized {
        bytes,
        random: request.random,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(json: &str) -> Request {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn sorts_each_format_into_its_kind() {
        let plain = request(r#"{"folder":"C:\\","name":"a","format":"x"}"#);
        assert_eq!(
            spec("png", &plain),
            Ok(Spec::Image {
                picture: "landscape".into(),
                size: None
            })
        );
        assert_eq!(spec("csv", &plain), Ok(Spec::Text { rows: 100 }));
        assert_eq!(spec("py", &plain), Ok(Spec::Text { rows: 0 }));
        assert_eq!(spec("xlsx", &plain), Ok(Spec::Document { pages: 5 }));
        assert_eq!(
            spec("mp3", &plain),
            Ok(Spec::Media {
                seconds: 30,
                height: 720
            })
        );
        assert_eq!(spec("zip", &plain), Ok(Spec::Archive { files: 5 }));
        assert_eq!(
            spec("iso", &plain),
            Ok(Spec::Sized {
                bytes: 100 * 1024 * 1024,
                random: false
            })
        );
    }

    #[test]
    fn takes_only_the_choices_offered() {
        let image = request(
            r#"{"folder":"C:\\","name":"a","format":"png","picture":"city","size":[1920,1080]}"#,
        );
        assert_eq!(
            spec("png", &image),
            Ok(Spec::Image {
                picture: "city".into(),
                size: Some((1920, 1080))
            })
        );
        for json in [
            r#"{"folder":"C:\\","name":"a","format":"png","picture":"../x"}"#,
            r#"{"folder":"C:\\","name":"a","format":"png","size":[9000,9000]}"#,
            r#"{"folder":"C:\\","name":"a","format":"csv","rows":5000000}"#,
            r#"{"folder":"C:\\","name":"a","format":"mp4","height":4320}"#,
            r#"{"folder":"C:\\","name":"a","format":"bin","bytes":20000000000}"#,
        ] {
            let asked = request(json);
            let format = asked.format.clone();
            assert!(spec(&format, &asked).is_err(), "{json}");
        }
    }
}
