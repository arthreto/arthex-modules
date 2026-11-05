use std::{ptr, slice, thread, time::Duration};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HGLOBAL, HWND},
    System::{
        DataExchange::{
            CloseClipboard, GetClipboardData, GetClipboardOwner, IsClipboardFormatAvailable,
            OpenClipboard, RegisterClipboardFormatW,
        },
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        Ole::CF_DIB,
        Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
    },
    UI::WindowsAndMessaging::GetWindowThreadProcessId,
};

use super::canvas::wide;
use crate::bitmap::Bitmap;

/// Programs that put a screenshot on the clipboard, in lowercase. Print Screen has no owner at all.
const SCREENSHOT_PROGRAMS: [&str; 6] = [
    "snippingtool.exe",
    "screenclippinghost.exe",
    "screensketch.exe",
    "sharex.exe",
    "greenshot.exe",
    "lightshot.exe",
];
const OPEN_ATTEMPTS: u32 = 5;
const OPEN_WAIT: Duration = Duration::from_millis(40);
const LONGEST_PATH: u32 = 32_768;

fn is_available(format: u32) -> bool {
    // SAFETY: only asks about a format, and works without opening the clipboard.
    unsafe { IsClipboardFormatAvailable(format) != 0 }
}

/// Password managers mark what they copy with this format, and it must be left alone.
fn is_excluded() -> bool {
    let name = wide("ExcludeClipboardContentFromMonitorProcessing");
    // SAFETY: the name outlives the call, and registering an existing format returns its number.
    is_available(unsafe { RegisterClipboardFormatW(name.as_ptr()) })
}

fn program_of(window: HWND) -> Option<String> {
    let mut process_id = 0;
    // SAFETY: writes into a local; a window that is gone leaves it at zero.
    unsafe { GetWindowThreadProcessId(window, &raw mut process_id) };
    if process_id == 0 {
        return None;
    }
    // SAFETY: the handle is closed below, and only asks for the process's name.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }
    let mut path = vec![0_u16; LONGEST_PATH as usize];
    let mut length = LONGEST_PATH;
    // SAFETY: the buffer holds `length` characters, and the handle was opened above.
    let found = unsafe {
        let found = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &raw mut length);
        CloseHandle(process);
        found != 0
    };
    found.then(|| String::from_utf16_lossy(&path[..length as usize]))
}

fn takes_screenshots(path: &str) -> bool {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    SCREENSHOT_PROGRAMS
        .iter()
        .any(|program| name.eq_ignore_ascii_case(program))
}

/// Whether the clipboard seems to have just received a screenshot. Its content is not read.
pub fn holds_screenshot() -> bool {
    if !is_available(u32::from(CF_DIB)) || is_excluded() {
        return false;
    }
    // SAFETY: only returns a handle, or null when nobody owns the clipboard.
    let owner = unsafe { GetClipboardOwner() };
    owner.is_null() || program_of(owner).is_some_and(|path| takes_screenshots(&path))
}

fn open() -> bool {
    for _ in 0..OPEN_ATTEMPTS {
        // SAFETY: closed by the caller. Another program may hold it for a moment.
        if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
            return true;
        }
        thread::sleep(OPEN_WAIT);
    }
    false
}

/// The image on the clipboard, only when the user asked for it.
pub fn image() -> Option<Bitmap> {
    if is_excluded() || !open() {
        return None;
    }
    // SAFETY: the clipboard is open until `CloseClipboard`, which keeps the data alive; the slice
    // covers exactly the memory Windows says the handle holds, and is only read while locked.
    unsafe {
        let data: HGLOBAL = GetClipboardData(u32::from(CF_DIB));
        let mut image = None;
        if !data.is_null() {
            let locked = GlobalLock(data);
            if !locked.is_null() {
                let bytes = slice::from_raw_parts(locked.cast::<u8>(), GlobalSize(data));
                image = Bitmap::from_dib(bytes);
                GlobalUnlock(data);
            }
        }
        CloseClipboard();
        image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_programs_are_found_by_their_name() {
        assert!(takes_screenshots(
            r"C:\Program Files\WindowsApps\Snip\SnippingTool.exe"
        ));
        assert!(takes_screenshots("ShareX.exe"));
    }

    #[test]
    fn other_programs_are_not_screenshots() {
        assert!(!takes_screenshots(r"C:\Program Files\Browser\browser.exe"));
    }
}
