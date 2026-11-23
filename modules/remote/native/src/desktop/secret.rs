use std::{ptr, slice};

use windows_sys::Win32::{
    Foundation::{HLOCAL, LocalFree},
    Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
};

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(data.len()).unwrap_or(u32::MAX),
        pbData: data.as_ptr().cast_mut(),
    }
}

/// Copies what Windows allocated, then frees it.
fn taken(output: &CRYPT_INTEGER_BLOB) -> Vec<u8> {
    // SAFETY: Windows wrote `cbData` bytes at `pbData`, freed here once and never used again.
    unsafe {
        let bytes = slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as HLOCAL);
        bytes
    }
}

/// Encrypted for the current Windows account: another account, or another computer, cannot read it.
pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
    let input = blob(data);
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    // SAFETY: `input` points into `data`, which outlives the call; Windows fills `output`.
    let done = unsafe {
        CryptProtectData(
            &raw const input,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut output,
        )
    };
    (done != 0).then(|| taken(&output))
}

pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    let input = blob(data);
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    // SAFETY: as in `protect`.
    let done = unsafe {
        CryptUnprotectData(
            &raw const input,
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut output,
        )
    };
    (done != 0).then(|| taken(&output))
}
