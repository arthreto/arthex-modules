//! The drive letter: a name of the current session for the drive's folder, as `subst` makes,
//! with the connection's name and a network drive icon in Explorer. No administrator rights.

use std::{path::Path, ptr};

use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    Storage::FileSystem::{
        DDD_REMOVE_DEFINITION, DefineDosDeviceW, GetLogicalDrives, QueryDosDeviceW,
    },
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW,
        RegSetValueExW,
    },
    UI::Shell::{SHCNE_DRIVEADD, SHCNE_DRIVEREMOVED, SHCNF_PATHW, SHChangeNotify},
};

pub const ICON: &str = "%SystemRoot%\\System32\\imageres.dll,-33";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn device(letter: &str) -> Vec<u16> {
    wide(&format!("{letter}:"))
}

/// Explorer reads a drive's label and icon here, for the current user only.
fn labels_key(letter: &str) -> String {
    format!("Software\\Classes\\Applications\\Explorer.exe\\Drives\\{letter}")
}

pub fn is_taken(letter: &str) -> bool {
    let Some(index) = letter.bytes().next().map(|byte| byte.wrapping_sub(b'A')) else {
        return true;
    };
    // SAFETY: no arguments; it returns a bit per drive letter in use.
    let drives = unsafe { GetLogicalDrives() };
    index >= 26 || drives & (1 << index) != 0
}

/// What `letter` points to, as Windows stores it (`\??\C:\…`).
fn target_of(letter: &str) -> Option<String> {
    let name = device(letter);
    let mut buffer = vec![0u16; 1024];
    // SAFETY: `name` ends with a zero; Windows writes at most `buffer.len()` characters.
    let length = unsafe {
        QueryDosDeviceW(
            name.as_ptr(),
            buffer.as_mut_ptr(),
            u32::try_from(buffer.len()).unwrap_or(0),
        )
    };
    if length == 0 {
        return None;
    }
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

fn points_to(letter: &str, folder: &Path) -> bool {
    target_of(letter).is_some_and(|target| {
        target
            .trim_start_matches("\\??\\")
            .eq_ignore_ascii_case(&folder.to_string_lossy())
    })
}

pub fn assign(letter: &str, folder: &Path, label: &str) -> bool {
    let (name, target) = (device(letter), wide(&folder.to_string_lossy()));
    // SAFETY: both strings end with a zero and outlive the call.
    if unsafe { DefineDosDeviceW(0, name.as_ptr(), target.as_ptr()) } == 0 {
        return false;
    }
    write_labels(letter, label);
    notify(letter, SHCNE_DRIVEADD);
    true
}

/// Removes the letter only if it still points to `folder`, as a letter may have been reused.
pub fn release(letter: &str, folder: &Path) {
    if !points_to(letter, folder) {
        return;
    }
    let name = device(letter);
    // SAFETY: `name` ends with a zero; a null target removes the latest definition, checked above.
    unsafe { DefineDosDeviceW(DDD_REMOVE_DEFINITION, name.as_ptr(), ptr::null()) };
    let key = wide(&labels_key(letter));
    // SAFETY: `key` ends with a zero; deleting a missing key only fails.
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
    notify(letter, SHCNE_DRIVEREMOVED);
}

fn write_labels(letter: &str, label: &str) {
    for (sub, value) in [("DefaultLabel", label), ("DefaultIcon", ICON)] {
        let path = wide(&format!("{}\\{sub}", labels_key(letter)));
        let mut key: HKEY = ptr::null_mut();
        // SAFETY: `path` ends with a zero; Windows fills `key`, closed below.
        let opened = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                path.as_ptr(),
                0,
                ptr::null(),
                0,
                KEY_WRITE,
                ptr::null(),
                &raw mut key,
                ptr::null_mut(),
            )
        };
        if opened != ERROR_SUCCESS {
            continue;
        }
        let data = wide(value);
        let bytes = u32::try_from(data.len() * 2).unwrap_or(0);
        // SAFETY: `data` holds `bytes` bytes ending with a zero; `key` is open.
        unsafe {
            RegSetValueExW(key, ptr::null(), 0, REG_SZ, data.as_ptr().cast(), bytes);
            RegCloseKey(key);
        }
    }
}

fn notify(letter: &str, event: u32) {
    let path = wide(&format!("{letter}:\\"));
    // SAFETY: `path` ends with a zero and outlives the call.
    unsafe {
        SHChangeNotify(
            event.cast_signed(),
            SHCNF_PATHW,
            path.as_ptr().cast(),
            ptr::null(),
        );
    };
}
