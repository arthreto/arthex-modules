//! Sound and video made by the shared `FFmpeg`: a beep, or a test pattern with a counter and a beep
//! each second.

use std::{ffi::OsString, path::Path};

pub const AUDIO: [&str; 4] = ["wav", "mp3", "ogg", "flac"];
pub const VIDEO: [&str; 3] = ["mp4", "webm", "mkv"];
/// The heights offered, as in `720p`.
pub const HEIGHTS: [u32; 4] = [480, 720, 1080, 2160];

fn video_size(height: u32) -> Option<(u32, u32, &'static str)> {
    match height {
        480 => Some((854, 480, "1M")),
        720 => Some((1280, 720, "2500k")),
        1080 => Some((1920, 1080, "5M")),
        2160 => Some((3840, 2160, "16M")),
        _ => None,
    }
}

/// The command line of `FFmpeg` for `format`, writing to `output` and its progress to stdout.
pub fn args(format: &str, seconds: u32, height: u32, output: &Path) -> Option<Vec<OsString>> {
    let mut args: Vec<String> = ["-nostdin", "-hide_banner", "-loglevel", "error", "-y"]
        .map(String::from)
        .to_vec();
    let audio_args = |codec: &[&str]| codec.iter().map(ToString::to_string).collect::<Vec<_>>();
    if AUDIO.contains(&format) {
        // One second of a 1 kHz beep, then silence.
        args.extend(audio_args(&["-f", "lavfi", "-i"]));
        args.push(format!(
            "aevalsrc=exprs=if(lt(t\\,1)\\,0.5*sin(2*PI*1000*t)\\,0):s=44100:d={seconds}"
        ));
        args.extend(audio_args(&["-ac", "1"]));
        args.extend(audio_args(match format {
            "wav" => &["-c:a", "pcm_s16le", "-f", "wav"],
            "mp3" => &["-c:a", "libmp3lame", "-b:a", "128k", "-f", "mp3"],
            "ogg" => &["-c:a", "libvorbis", "-q:a", "4", "-f", "ogg"],
            _ => &["-c:a", "flac", "-f", "flac"],
        }));
    } else if VIDEO.contains(&format) {
        let (width, height, bitrate) = video_size(height)?;
        args.extend(audio_args(&["-f", "lavfi", "-i"]));
        args.push(format!(
            "testsrc=size={width}x{height}:rate=30:duration={seconds}"
        ));
        args.extend(audio_args(&["-f", "lavfi", "-i"]));
        // A tenth of a second of beep at the start of each second, under the counter.
        args.push(format!(
            "aevalsrc=exprs=if(lt(mod(t\\,1)\\,0.1)\\,0.4*sin(2*PI*1000*t)\\,0):s=48000:d={seconds}"
        ));
        args.extend(audio_args(&[
            "-shortest",
            "-pix_fmt",
            "yuv420p",
            "-b:v",
            bitrate,
        ]));
        args.extend(audio_args(match format {
            "webm" => &[
                "-c:v",
                "libvpx-vp9",
                "-deadline",
                "realtime",
                "-cpu-used",
                "8",
                "-row-mt",
                "1",
                "-c:a",
                "libopus",
                "-b:a",
                "96k",
                "-f",
                "webm",
            ],
            "mkv" => &[
                "-c:v",
                "libopenh264",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-f",
                "matroska",
            ],
            _ => &[
                "-c:v",
                "libopenh264",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            ],
        }));
    } else {
        return None;
    }
    args.extend(audio_args(&["-progress", "pipe:1", "-nostats"]));
    let mut args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
    args.push(output.into());
    Some(args)
}

/// A line of `-progress pipe:1`, as a percentage of `seconds`, never 100 before the end.
pub fn progress(line: &str, seconds: u32) -> Option<u8> {
    let done: u64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    let total = u64::from(seconds) * 1_000_000;
    (total > 0).then(|| u8::try_from(done * 100 / total).unwrap_or(99).min(99))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(format: &str, height: u32) -> String {
        args(format, 30, height, Path::new("out"))
            .unwrap()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn makes_each_format_with_its_codecs() {
        assert!(line("mp3", 0).contains("-c:a libmp3lame"));
        assert!(line("flac", 0).contains("-f flac"));
        assert!(line("mp4", 720).contains("testsrc=size=1280x720:rate=30:duration=30"));
        assert!(line("webm", 1080).contains("-c:v libvpx-vp9"));
        assert!(line("mkv", 2160).contains("-f matroska"));
        assert!(line("mp4", 720).ends_with("-progress pipe:1 -nostats out"));
    }

    #[test]
    fn refuses_what_it_does_not_make() {
        assert!(args("mp4", 30, 999, Path::new("out")).is_none());
        assert!(args("png", 30, 720, Path::new("out")).is_none());
    }

    #[test]
    fn follows_the_progress() {
        assert_eq!(progress("out_time_us=15000000", 30), Some(50));
        assert_eq!(progress("out_time_us=30000000", 30), Some(99));
        assert_eq!(progress("out_time_us=N/A", 30), None);
        assert_eq!(progress("frame=12", 30), None);
    }
}
