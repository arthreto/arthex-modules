use std::{path::Path, ptr};

use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

use super::canvas::wide;

/// Below this, `ShellExecuteW` reports an error rather than a handle.
const FIRST_SUCCESS: isize = 33;

/// Opens an address or a file with the program the user chose for it.
pub fn open(target: &str) -> Result<(), String> {
    let (verb, target) = (wide("open"), wide(target));
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
        Err("nothing opens this".to_string())
    }
}

pub fn open_file(path: &Path) -> Result<(), String> {
    open(&path.display().to_string())
}
