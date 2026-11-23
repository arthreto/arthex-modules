//! Paths of the server, always with `/`, from the paths Windows gives under the drive's folder.

use std::path::{Component, Path, PathBuf};

/// `path` relative to `root`, ignoring case as Windows does, or `None` when it lies outside.
pub fn strip_root(root: &Path, path: &Path) -> Option<PathBuf> {
    let mut parts = path.components();
    for expected in root.components() {
        let part = parts.next()?;
        if !part
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy())
        {
            return None;
        }
    }
    Some(parts.collect())
}

/// `www/`, `\www` and `/www/./` all become `/www`; `..` is dropped, so the root stays the root.
pub fn clean_folder(folder: &str) -> String {
    let parts: Vec<&str> = folder
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect();
    format!("/{}", parts.join("/"))
}

/// The server path of `relative`, a path under the drive's folder, or `None` when it tries to
/// leave it or holds a name the server could misread.
pub fn remote_path(folder: &str, relative: &Path) -> Option<String> {
    let mut path = folder.trim_end_matches('/').to_string();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        let name = name.to_str()?;
        if name.contains(['/', '\\']) || name.chars().any(char::is_control) {
            return None;
        }
        path.push('/');
        path.push_str(name);
    }
    Some(if path.is_empty() { "/".into() } else { path })
}

/// A name of a server listing that is safe to create on Windows. Others are not shown.
pub fn is_shown_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name);
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        && !RESERVED
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_root_whatever_its_case() {
        let root: PathBuf = ["Users", "Me", "Remote", "vps"].iter().collect();
        let inside: PathBuf = ["users", "me", "remote", "VPS", "www", "a.txt"]
            .iter()
            .collect();
        let outside: PathBuf = ["Users", "Me", "Desktop", "a.txt"].iter().collect();
        assert_eq!(
            strip_root(&root, &inside),
            Some(["www", "a.txt"].iter().collect())
        );
        assert_eq!(strip_root(&root, &root), Some(PathBuf::new()));
        assert_eq!(strip_root(&root, &outside), None);
    }

    #[test]
    fn cleans_folders() {
        assert_eq!(clean_folder(""), "/");
        assert_eq!(clean_folder("/"), "/");
        assert_eq!(clean_folder("www/"), "/www");
        assert_eq!(clean_folder("\\home\\ubuntu"), "/home/ubuntu");
        assert_eq!(clean_folder("/a/../b/./c"), "/a/b/c");
    }

    #[test]
    fn joins_under_the_folder() {
        let relative: PathBuf = ["site", "index.html"].iter().collect();
        assert_eq!(
            remote_path("/", &relative).as_deref(),
            Some("/site/index.html")
        );
        assert_eq!(
            remote_path("/home/ubuntu", &relative).as_deref(),
            Some("/home/ubuntu/site/index.html")
        );
        assert_eq!(remote_path("/", Path::new("")).as_deref(), Some("/"));
        assert_eq!(remote_path("/www", Path::new("")).as_deref(), Some("/www"));
    }

    #[test]
    fn refuses_to_leave_the_folder() {
        assert_eq!(remote_path("/www", Path::new("..")), None);
        assert_eq!(remote_path("/www", Path::new("a/../../etc")), None);
    }

    #[test]
    fn hides_names_windows_cannot_hold() {
        assert!(is_shown_name("index.html"));
        assert!(is_shown_name(".bashrc"));
        assert!(!is_shown_name("a:b"));
        assert!(!is_shown_name("con.txt"));
        assert!(!is_shown_name("trailing."));
        assert!(!is_shown_name(".."));
    }
}
