//! The arguments of `FFmpeg`, and what it reports. Its LGPL build has no x264: H.264 goes through
//! `OpenH264`, which takes a bitrate instead of a quality.

use crate::format::is_audio;

const FALLBACK_VIDEO_KBPS: u32 = 4000;
const MIN_VIDEO_KBPS: u32 = 300;
const SOUND_OF_VIDEO_KBPS: u32 = 160;
const PALETTE: &str = "split[a][b];[a]palettegen[p];[b][p]paletteuse";
/// H.264 and VP9 need even dimensions, which a photo or a cropped video does not always have.
const EVEN_SIZE: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";
const THUMBNAIL_SIZE: &str = "scale=96:96:force_original_aspect_ratio=increase,crop=96:96";
const MAX_ICON_SIZE: &str =
    "scale='min(256,iw)':'min(256,ih)':force_original_aspect_ratio=decrease";

/// What `FFmpeg` says about a file when it opens it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Probe {
    pub duration_us: Option<u64>,
    pub bitrate_kbps: Option<u32>,
}

pub struct Job<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub target: &'a str,
    /// From 10 to 100.
    pub quality: u32,
    /// The height of a video, never enlarged.
    pub height: Option<u32>,
    pub strip_metadata: bool,
    pub probe: &'a Probe,
}

fn clock_us(text: &str) -> Option<u64> {
    let mut parts = text.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let (seconds, fraction) = parts.next()?.split_once('.')?;
    let seconds: u64 = seconds.parse().ok()?;
    let hundredths: u64 = fraction.get(..2)?.parse().ok()?;
    Some(((hours * 3600 + minutes * 60 + seconds) * 100 + hundredths) * 10_000)
}

/// Reads the `Duration: 00:01:02.33, start: 0.0, bitrate: 1234 kb/s` of `ffmpeg -i`.
pub fn parse_probe(log: &str) -> Probe {
    let Some(line) = log
        .lines()
        .find(|line| line.trim_start().starts_with("Duration:"))
    else {
        return Probe::default();
    };
    let duration_us = line
        .trim_start()
        .strip_prefix("Duration:")
        .and_then(|rest| rest.split(',').next())
        .and_then(|text| clock_us(text.trim()));
    let bitrate_kbps = line
        .split("bitrate:")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|text| text.parse().ok());
    Probe {
        duration_us,
        bitrate_kbps,
    }
}

/// A line of `-progress pipe:1`, as a percentage of `duration_us`.
pub fn progress_percent(line: &str, duration_us: u64) -> Option<u8> {
    let done: u64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    if duration_us == 0 {
        return None;
    }
    // Never 100 here: only the end of the program says the file is complete.
    Some(u8::try_from((done.saturating_mul(100) / duration_us).min(99)).unwrap_or(99))
}

fn jpeg_scale(quality: u32) -> u32 {
    2 + (100 - quality.clamp(10, 100)) * 29 / 90
}

fn avif_crf(quality: u32) -> u32 {
    10 + (100 - quality.clamp(10, 100)) * 40 / 90
}

fn audio_kbps(quality: u32) -> u32 {
    96 + (quality.clamp(10, 100) - 10) * 224 / 90
}

fn video_kbps(probe: &Probe, quality: u32) -> u32 {
    probe.bitrate_kbps.map_or(FALLBACK_VIDEO_KBPS, |source| {
        (source.saturating_mul(quality.clamp(10, 100)) / 100).max(MIN_VIDEO_KBPS)
    })
}

fn push(args: &mut Vec<String>, values: &[&str]) {
    args.extend(values.iter().map(ToString::to_string));
}

fn image_args(job: &Job, args: &mut Vec<String>) {
    let quality = job.quality;
    match job.target {
        "jpeg" => push(args, &["-q:v", &jpeg_scale(quality).to_string()]),
        "webp" => push(args, &["-c:v", "libwebp", "-quality", &quality.to_string()]),
        "avif" => push(
            args,
            &[
                "-c:v",
                "libaom-av1",
                "-still-picture",
                "1",
                "-cpu-used",
                "6",
                "-crf",
                &avif_crf(quality).to_string(),
            ],
        ),
        "ico" => push(
            args,
            &["-vf", MAX_ICON_SIZE, "-c:v", "png", "-pix_fmt", "rgba"],
        ),
        "tiff" => push(args, &["-compression_algo", "deflate"]),
        "png" if quality < 50 => push(args, &["-vf", PALETTE, "-compression_level", "9"]),
        "png" => push(args, &["-compression_level", "9"]),
        "gif" => push(args, &["-vf", PALETTE]),
        _ => {}
    }
    push(args, &["-frames:v", "1"]);
    if job.target != "webp" && job.target != "avif" && job.target != "ico" {
        // One picture, not a numbered series, whatever the name of the file.
        push(args, &["-update", "1"]);
    }
}

fn video_args(job: &Job, args: &mut Vec<String>) {
    let bitrate = format!("{}k", video_kbps(job.probe, job.quality));
    let scale = job.height.map_or_else(
        || EVEN_SIZE.to_string(),
        |height| format!("scale=-2:'min({height},ih)'"),
    );
    match job.target {
        "gif" => {
            let width = job.height.map_or(480, |height| height * 16 / 9);
            push(
                args,
                &[
                    "-an",
                    "-vf",
                    &format!("fps=12,scale={width}:-1:flags=lanczos,{PALETTE}"),
                ],
            );
            return;
        }
        "webm" => push(
            args,
            &[
                "-c:v",
                "libvpx-vp9",
                "-b:v",
                &bitrate,
                "-row-mt",
                "1",
                "-deadline",
                "good",
                "-cpu-used",
                "4",
                "-c:a",
                "libopus",
            ],
        ),
        "avi" => push(
            args,
            &["-c:v", "mpeg4", "-b:v", &bitrate, "-c:a", "libmp3lame"],
        ),
        _ => push(
            args,
            &[
                "-c:v",
                "libopenh264",
                "-b:v",
                &bitrate,
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ],
        ),
    }
    push(
        args,
        &["-b:a", &format!("{SOUND_OF_VIDEO_KBPS}k"), "-vf", &scale],
    );
}

fn audio_args(job: &Job, args: &mut Vec<String>) {
    let bitrate = format!("{}k", audio_kbps(job.quality));
    push(args, &["-vn"]);
    match job.target {
        "mp3" => push(args, &["-c:a", "libmp3lame", "-b:a", &bitrate]),
        "m4a" => push(args, &["-c:a", "aac", "-b:a", &bitrate]),
        "ogg" => push(args, &["-c:a", "libvorbis", "-b:a", &bitrate]),
        "flac" => push(args, &["-c:a", "flac", "-compression_level", "8"]),
        _ => push(args, &["-c:a", "pcm_s16le"]),
    }
}

/// A small square picture of an image, for the list.
pub fn thumbnail_arguments(input: &str, output: &str) -> Vec<String> {
    let mut args = Vec::new();
    push(
        &mut args,
        &[
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-y",
            "-i",
            input,
        ],
    );
    push(
        &mut args,
        &[
            "-vf",
            THUMBNAIL_SIZE,
            "-frames:v",
            "1",
            "-update",
            "1",
            "-q:v",
            "5",
            output,
        ],
    );
    args
}

/// Everything after the program's name.
pub fn arguments(job: &Job, still_image: bool) -> Vec<String> {
    let mut args = Vec::new();
    push(
        &mut args,
        &[
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-y",
            "-i",
            job.input,
        ],
    );
    push(&mut args, &["-progress", "pipe:1", "-nostats"]);
    if job.strip_metadata {
        push(&mut args, &["-map_metadata", "-1"]);
    }
    if is_audio(job.target) {
        audio_args(job, &mut args);
    } else if still_image {
        image_args(job, &mut args);
    } else {
        video_args(job, &mut args);
    }
    push(&mut args, &[job.output]);
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE: Probe = Probe {
        duration_us: Some(10_000_000),
        bitrate_kbps: Some(2000),
    };

    fn job(target: &str) -> Job<'_> {
        Job {
            input: "in",
            output: "out",
            target,
            quality: 50,
            height: None,
            strip_metadata: false,
            probe: &PROBE,
        }
    }

    fn has(args: &[String], pair: [&str; 2]) -> bool {
        args.windows(2)
            .any(|window| window[0] == pair[0] && window[1] == pair[1])
    }

    #[test]
    fn reads_what_ffmpeg_says_about_a_file() {
        let log = "Input #0, mov\n  Duration: 00:01:02.33, start: 0.000000, bitrate: 1234 kb/s\n  Stream #0";
        assert_eq!(
            parse_probe(log),
            Probe {
                duration_us: Some(62_330_000),
                bitrate_kbps: Some(1234)
            }
        );
        assert_eq!(
            parse_probe("  Duration: N/A, bitrate: N/A"),
            Probe::default()
        );
        assert_eq!(parse_probe("nothing"), Probe::default());
    }

    #[test]
    fn follows_the_progress_without_ever_saying_it_is_finished() {
        assert_eq!(
            progress_percent("out_time_us=5000000", 10_000_000),
            Some(50)
        );
        assert_eq!(
            progress_percent("out_time_us=99000000", 10_000_000),
            Some(99)
        );
        assert_eq!(progress_percent("out_time_us=N/A", 10_000_000), None);
        assert_eq!(progress_percent("frame=12", 10_000_000), None);
        assert_eq!(progress_percent("out_time_us=5", 0), None);
    }

    #[test]
    fn output_and_input_stay_whole_arguments() {
        let mut job = job("mp3");
        job.input = r"C:\mes fichiers\a b.wav";
        job.output = r"C:\mes fichiers\a b.mp3";
        let args = arguments(&job, false);
        assert!(has(&args, ["-i", r"C:\mes fichiers\a b.wav"]));
        assert_eq!(
            args.last().map(String::as_str),
            Some(r"C:\mes fichiers\a b.mp3")
        );
    }

    #[test]
    fn the_best_quality_gives_the_best_settings() {
        assert_eq!(jpeg_scale(100), 2);
        assert_eq!(jpeg_scale(10), 31);
        assert_eq!(avif_crf(100), 10);
        assert_eq!(audio_kbps(10), 96);
        assert_eq!(audio_kbps(100), 320);
    }

    #[test]
    fn a_video_gets_a_share_of_its_own_bitrate() {
        let args = arguments(&job("mp4"), false);
        assert!(has(&args, ["-c:v", "libopenh264"]));
        assert!(has(&args, ["-b:v", "1000k"]));
        assert!(has(&args, ["-vf", EVEN_SIZE]));
        let unknown = Probe::default();
        assert_eq!(video_kbps(&unknown, 50), FALLBACK_VIDEO_KBPS);
        assert_eq!(
            video_kbps(
                &Probe {
                    bitrate_kbps: Some(100),
                    ..Probe::default()
                },
                10
            ),
            MIN_VIDEO_KBPS
        );
    }

    #[test]
    fn a_video_is_never_enlarged() {
        let mut job = job("mp4");
        job.height = Some(720);
        assert!(has(
            &arguments(&job, false),
            ["-vf", "scale=-2:'min(720,ih)'"]
        ));
    }

    #[test]
    fn a_video_can_become_its_sound() {
        let args = arguments(&job("mp3"), false);
        assert!(args.iter().any(|arg| arg == "-vn"));
        assert!(has(&args, ["-c:a", "libmp3lame"]));
        assert!(!args.iter().any(|arg| arg == "-c:v"));
    }

    #[test]
    fn an_image_stays_one_picture() {
        let args = arguments(&job("jpeg"), true);
        assert!(has(&args, ["-frames:v", "1"]));
        assert!(has(&args, ["-update", "1"]));
        let icon = arguments(&job("ico"), true);
        assert!(has(&icon, ["-vf", MAX_ICON_SIZE]));
        assert!(has(&icon, ["-c:v", "png"]));
    }

    #[test]
    fn a_thumbnail_is_one_small_picture() {
        let args = thumbnail_arguments("in", "out");
        assert!(has(&args, ["-i", "in"]));
        assert!(has(&args, ["-vf", THUMBNAIL_SIZE]));
        assert_eq!(args.last().map(String::as_str), Some("out"));
    }

    #[test]
    fn removes_metadata_only_when_asked() {
        assert!(
            !arguments(&job("png"), true)
                .iter()
                .any(|arg| arg == "-map_metadata")
        );
        let mut asked = job("png");
        asked.strip_metadata = true;
        assert!(has(&arguments(&asked, true), ["-map_metadata", "-1"]));
    }
}
