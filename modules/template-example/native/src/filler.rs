//! Files of an exact size, and the zip of sample files.

use std::{
    fs::File,
    io::{self, Cursor, Write},
    path::Path,
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub const MIN_BYTES: u64 = 1024;
pub const MAX_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const CHUNK: usize = 1024 * 1024;

/// Fills `file` with `bytes` zeros, at once: NTFS gives zeros without writing them. Random content
/// is written chunk by chunk, telling `progress` each new percent.
pub fn sized(file: &mut File, bytes: u64, random: bool, progress: impl Fn(u8)) -> io::Result<()> {
    if !random {
        return file.set_len(bytes);
    }
    // SplitMix64: fast, and plenty for bytes that must not compress.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15 ^ bytes;
    let mut chunk = vec![0u8; CHUNK];
    let mut written = 0;
    let mut shown = u8::MAX;
    while written < bytes {
        for word in chunk.as_chunks_mut::<8>().0 {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *word = (z ^ (z >> 31)).to_le_bytes();
        }
        let size = usize::try_from((bytes - written).min(CHUNK as u64)).unwrap_or(CHUNK);
        file.write_all(&chunk[..size])?;
        written += size as u64;
        let percent = u8::try_from(written * 100 / bytes).unwrap_or(100);
        if percent != shown {
            shown = percent;
            progress(percent);
        }
    }
    file.flush()
}

/// What goes in a zip of samples, made once for every file of that kind.
pub struct Samples {
    pub text: String,
    pub csv: String,
    pub markdown: String,
    pub jpeg: Vec<u8>,
    pub pdf: Vec<u8>,
}

/// `files` sample files in two folders, `documents/` and `images/`.
pub fn archive(files: usize, samples: &Samples, folders: (&str, &str)) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // A JPEG is already compressed.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let (documents, images) = folders;
    for index in 1..=files {
        let (name, bytes, options) = match index % 5 {
            1 => (
                format!("{documents}/note-{index:03}.txt"),
                samples.text.as_bytes(),
                deflated,
            ),
            2 => (
                format!("{documents}/table-{index:03}.csv"),
                samples.csv.as_bytes(),
                deflated,
            ),
            3 => (
                format!("{documents}/readme-{index:03}.md"),
                samples.markdown.as_bytes(),
                deflated,
            ),
            4 => (
                format!("{images}/photo-{index:03}.jpg"),
                samples.jpeg.as_slice(),
                stored,
            ),
            _ => (
                format!("{documents}/document-{index:03}.pdf"),
                samples.pdf.as_slice(),
                deflated,
            ),
        };
        writer
            .start_file(name, options)
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    Ok(writer
        .finish()
        .map_err(|error| error.to_string())?
        .into_inner())
}

/// The free space of the drive of `folder`, when Windows tells it.
#[cfg(windows)]
pub fn free_space(folder: &Path) -> Option<u64> {
    use std::{iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = folder
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let mut free = 0u64;
    // SAFETY: a NUL-terminated path and a u64 for the one value asked for.
    let read = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &raw mut free,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    (read != 0).then_some(free)
}

#[cfg(not(windows))]
pub fn free_space(_: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn writes_the_exact_size() {
        let folder = tempfile::tempdir().unwrap();
        for random in [false, true] {
            let path = folder.path().join(format!("{random}.bin"));
            let mut file = File::create(&path).unwrap();
            sized(&mut file, 3 * 1024 * 1024 + 5, random, |_| ()).unwrap();
            assert_eq!(std::fs::metadata(&path).unwrap().len(), 3 * 1024 * 1024 + 5);
        }
        let random = std::fs::read(folder.path().join("true.bin")).unwrap();
        assert!(
            random
                .chunks(64)
                .all(|chunk| chunk.iter().any(|byte| *byte != 0))
        );
    }

    #[test]
    fn puts_the_files_in_two_folders() {
        let samples = Samples {
            text: "t".into(),
            csv: "c".into(),
            markdown: "m".into(),
            jpeg: vec![1, 2],
            pdf: vec![3],
        };
        let bytes = archive(50, &samples, ("documents", "images")).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(zip.len(), 50);
        assert_eq!(
            zip.file_names()
                .filter(|name| name.starts_with("images/"))
                .count(),
            10
        );
        let mut text = String::new();
        zip.by_name("documents/note-001.txt")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "t");
    }
}
