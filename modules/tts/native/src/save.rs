//! Keeps a voice in the Music folder, as WAV or as MP3 made by the shared `FFmpeg`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::wav;

/// The first name of the form `voice.wav`, `voice (2).wav`… that is free in `folder`.
pub fn free_name(folder: &Path, base: &str, extension: &str) -> PathBuf {
    (2..10_000)
        .map(|number| folder.join(format!("{base} ({number}).{extension}")))
        .find(|path| !path.exists())
        .filter(|_| folder.join(format!("{base}.{extension}")).exists())
        .unwrap_or_else(|| folder.join(format!("{base}.{extension}")))
}

/// Writes the voice as `format` in `folder`, and gives the file it made.
pub fn save(
    samples: &[f32],
    rate: u32,
    format: &str,
    folder: &Path,
    base: &str,
    ffmpeg: &Path,
    scratch: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(folder).map_err(|_| "folder".to_string())?;
    let wave = wav::encode(samples, rate);
    match format {
        "wav" => {
            let path = free_name(folder, base, "wav");
            fs::write(&path, wave).map_err(|_| "write".to_string())?;
            Ok(path)
        }
        "mp3" => {
            let input = scratch.join("saving.wav");
            fs::write(&input, wave).map_err(|_| "write".to_string())?;
            let path = free_name(folder, base, "mp3");
            let made = encode_mp3(ffmpeg, &input, &path);
            let _ = fs::remove_file(&input);
            made.map(|()| path)
        }
        _ => Err("format".to_string()),
    }
}

fn encode_mp3(ffmpeg: &Path, input: &Path, output: &Path) -> Result<(), String> {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-c:a", "libmp3lame", "-q:a", "2", "-f", "mp3"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let status = command.status().map_err(|_| "ffmpeg".to_string())?;
    if status.success() {
        Ok(())
    } else {
        let _ = fs::remove_file(output);
        Err("ffmpeg".to_string())
    }
}

/// The Music folder of the user, even when it was moved (to `OneDrive`, for instance).
#[cfg(windows)]
pub fn music_folder() -> Option<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, ptr, slice};
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Music, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    };

    let mut path = ptr::null_mut();
    // SAFETY: Windows allocates the path, which is read up to its NUL and then freed.
    unsafe {
        let found = SHGetKnownFolderPath(
            &FOLDERID_Music,
            KF_FLAG_DEFAULT as u32,
            ptr::null_mut(),
            &raw mut path,
        ) == 0;
        let music = found.then(|| {
            let mut length = 0;
            while *path.add(length) != 0 {
                length += 1;
            }
            PathBuf::from(OsString::from_wide(slice::from_raw_parts(path, length)))
        });
        CoTaskMemFree(path.cast());
        music
    }
}

#[cfg(not(windows))]
pub fn music_folder() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Music"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_free_name() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(
            free_name(folder.path(), "voix", "wav"),
            folder.path().join("voix.wav")
        );
        fs::write(folder.path().join("voix.wav"), b"").unwrap();
        fs::write(folder.path().join("voix (2).wav"), b"").unwrap();
        assert_eq!(
            free_name(folder.path(), "voix", "wav"),
            folder.path().join("voix (3).wav")
        );
        assert_eq!(
            free_name(folder.path(), "voix", "mp3"),
            folder.path().join("voix.mp3")
        );
    }

    #[test]
    fn saves_a_wave_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = save(
            &[0.0, 0.5],
            44_100,
            "wav",
            folder.path(),
            "voice",
            Path::new("ffmpeg"),
            folder.path(),
        )
        .unwrap();
        assert_eq!(path, folder.path().join("voice.wav"));
        assert_eq!(fs::read(path).unwrap().len(), 48);
    }

    #[test]
    fn refuses_another_format() {
        let folder = tempfile::tempdir().unwrap();
        let saved = save(
            &[],
            44_100,
            "ogg",
            folder.path(),
            "voice",
            Path::new("ffmpeg"),
            folder.path(),
        );
        assert_eq!(saved.err().unwrap(), "format");
    }
}
