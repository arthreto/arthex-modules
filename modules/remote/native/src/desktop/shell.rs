use std::{fs, path::Path, ptr};

use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

use super::store::MAX_KEY_BYTES;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Below this, `ShellExecuteW` reports an error rather than a handle.
const FIRST_SUCCESS: isize = 33;

/// Opens a drive or a folder in Explorer.
pub fn open(place: &Path) -> Result<(), String> {
    let (verb, target) = (wide("open"), wide(&place.to_string_lossy()));
    // SAFETY: both strings end with a zero and outlive the call; the other arguments may be null.
    let answer = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if answer as isize >= FIRST_SUCCESS {
        Ok(())
    } else {
        Err("Explorer did not open".to_string())
    }
}

/// A private key the user picks, read as text, with its file name. `Ok(None)` when they cancel.
pub fn pick_key() -> Result<Option<(String, String)>, String> {
    let start = std::env::var_os("USERPROFILE").map(|home| Path::new(&home).join(".ssh"));
    let mut dialog = rfd::FileDialog::new();
    if let Some(start) = start.filter(|folder| folder.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    let Some(file) = dialog.pick_file() else {
        return Ok(None);
    };
    let bytes = fs::read(&file).map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "notKey".to_string())?;
    if text.len() > MAX_KEY_BYTES || !looks_like_key(&text) {
        return Err("notKey".into());
    }
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Some((name, text)))
}

fn looks_like_key(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with("PuTTY-User-Key-File-")
        || (text.starts_with("-----BEGIN ") && text.contains("PRIVATE KEY-----"))
}
