//! The name of the file to create: checked as Windows would, and never one that exists.

use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED: [&str; 4] = ["con", "prn", "aux", "nul"];
const MAX_NAME: usize = 200;

/// Why a name or a format is refused, as the interface shows it.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    Empty,
    Characters,
    Reserved,
    Format,
}

impl Refused {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty => "name-empty",
            Self::Characters => "name-characters",
            Self::Reserved => "name-reserved",
            Self::Format => "format",
        }
    }
}

fn is_reserved(stem: &str) -> bool {
    let lower = stem.to_lowercase();
    RESERVED.contains(&lower.as_str())
        || ["com", "lpt"].iter().any(|prefix| {
            lower
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.len() == 1 && rest.chars().all(|c| c.is_ascii_digit()))
        })
}

/// The name without the spaces around it, if Windows accepts it.
pub fn check_name(name: &str) -> Result<&str, Refused> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Refused::Empty);
    }
    if name.chars().count() > MAX_NAME
        || name
            .chars()
            .any(|c| FORBIDDEN.contains(&c) || c.is_control())
    {
        return Err(Refused::Characters);
    }
    if is_reserved(name) || name.ends_with('.') {
        return Err(Refused::Reserved);
    }
    Ok(name)
}

/// The format without its dot, in lowercase: letters and digits only.
pub fn check_format(format: &str) -> Result<String, Refused> {
    let format = format.trim().trim_start_matches('.').to_lowercase();
    if (1..=12).contains(&format.len()) && format.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(format)
    } else {
        Err(Refused::Format)
    }
}

fn file_name(name: &str, format: &str, number: u32) -> String {
    if number == 1 {
        format!("{name}.{format}")
    } else {
        format!("{name} ({number}).{format}")
    }
}

/// The name the file would get now: `name.format`, or `name (2).format`… when it exists.
pub fn free_name(folder: &Path, name: &str, format: &str) -> String {
    (1..10_000)
        .map(|number| file_name(name, format, number))
        .find(|candidate| !folder.join(candidate).exists())
        .unwrap_or_else(|| file_name(name, format, 1))
}

/// Creates the file under its free name and gives its path. It is created empty at once with
/// `create_new`, so that two creations never take the same name and a file that appears meanwhile
/// is never replaced.
pub fn reserve(folder: &Path, name: &str, format: &str) -> io::Result<(PathBuf, File)> {
    for number in 1..10_000 {
        let path = folder.join(file_name(name, format, number));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::from(io::ErrorKind::AlreadyExists))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_what_windows_accepts() {
        assert_eq!(check_name("  exemple  "), Ok("exemple"));
        assert_eq!(
            check_name("rapport 2026 (final)"),
            Ok("rapport 2026 (final)")
        );
        assert_eq!(check_name("Réunion 日本"), Ok("Réunion 日本"));
        assert_eq!(check_name("console"), Ok("console"));
        assert_eq!(check_name("com10"), Ok("com10"));
    }

    #[test]
    fn refuses_what_windows_refuses() {
        assert_eq!(check_name("   "), Err(Refused::Empty));
        assert_eq!(check_name("a/b"), Err(Refused::Characters));
        assert_eq!(check_name("quoi?"), Err(Refused::Characters));
        assert_eq!(check_name("tab\there"), Err(Refused::Characters));
        assert_eq!(check_name("CON"), Err(Refused::Reserved));
        assert_eq!(check_name("lpt1"), Err(Refused::Reserved));
        assert_eq!(check_name("fin."), Err(Refused::Reserved));
        assert_eq!(check_name(&"a".repeat(201)), Err(Refused::Characters));
    }

    #[test]
    fn checks_the_format() {
        assert_eq!(check_format(".PNG"), Ok("png".to_string()));
        assert_eq!(check_format("mp4"), Ok("mp4".to_string()));
        assert_eq!(check_format(""), Err(Refused::Format));
        assert_eq!(check_format("tar.gz"), Err(Refused::Format));
        assert_eq!(check_format("abcdefghijklm"), Err(Refused::Format));
    }

    #[test]
    fn never_takes_a_name_in_use() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("exemple.png"), "mine").unwrap();
        assert_eq!(
            free_name(folder.path(), "exemple", "png"),
            "exemple (2).png"
        );
        let (first, _) = reserve(folder.path(), "exemple", "png").unwrap();
        let (second, _) = reserve(folder.path(), "exemple", "png").unwrap();
        assert_eq!(first, folder.path().join("exemple (2).png"));
        assert_eq!(second, folder.path().join("exemple (3).png"));
        assert_eq!(
            std::fs::read_to_string(folder.path().join("exemple.png")).unwrap(),
            "mine"
        );
    }
}
