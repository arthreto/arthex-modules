//! What is said to yt-dlp and what it answers, away from the running of it.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A line of the progress of a download starts with this, and holds its fields.
const PROGRESS_MARK: &str = "progress:";
const PROCESSING_MARK: &str = "processing:";
const FILE_MARK: &str = "file:";
/// What it does not know, it writes as NA unless told otherwise, and that would not be JSON.
const PROGRESS_TEMPLATE: &str = concat!(
    "download:progress:",
    r#"{"file":%(progress.filename)j,"done":%(progress.downloaded_bytes|null)s,"#,
    r#""total":%(progress.total_bytes|null)s,"estimate":%(progress.total_bytes_estimate|null)s,"#,
    r#""speed":%(progress.speed|null)s,"eta":%(progress.eta|null)s}"#,
);
/// How much of the bar the video takes when a sound follows it: a bar never goes back.
const VIDEO_SHARE: f64 = 0.9;
/// The last percent is for what happens once everything is downloaded.
pub const LAST_PERCENT: f64 = 99.0;

pub const VIDEO_FORMATS: &[&str] = &["mp4", "mkv", "webm"];
pub const AUDIO_FORMATS: &[&str] = &["mp3", "m4a", "opus", "flac", "wav"];
const AUDIO_BITRATES: &[u32] = &[320, 192, 128];
/// Below this, a height is only offered when the site has nothing better.
const LOWEST_USUAL_HEIGHT: u32 = 360;

/// What can be asked of a link, as the tool shows it.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub title: String,
    pub author: String,
    pub site: String,
    pub duration: Option<u64>,
    #[serde(skip_serializing)]
    pub thumbnail: Option<String>,
    pub audio_only: bool,
    /// Heights from the highest, with how large they get if it is known.
    pub videos: Vec<VideoQuality>,
    pub audios: Vec<AudioQuality>,
    pub video_formats: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VideoQuality {
    pub height: u32,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioQuality {
    pub bitrate: u32,
    pub size: Option<u64>,
}

/// What the tool asks to download.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub url: String,
    pub audio: bool,
    pub format: String,
    /// A height, or a bitrate for sound. Ignored by the formats that lose nothing.
    pub quality: u32,
}

impl Request {
    pub fn check(&self) -> Result<(), String> {
        if !is_link(&self.url) {
            return Err("not a link".to_string());
        }
        let formats = if self.audio {
            AUDIO_FORMATS
        } else {
            VIDEO_FORMATS
        };
        if !formats.contains(&self.format.as_str()) {
            return Err("unknown format".to_string());
        }
        Ok(())
    }
}

/// Only addresses of the web: anything else could be taken for an option or a local file.
pub fn is_link(text: &str) -> bool {
    (text.starts_with("https://") || text.starts_with("http://"))
        && text.len() <= 2048
        && !text
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

/// Where the programs it needs are.
#[derive(Debug)]
pub struct Tools<'a> {
    pub quickjs: &'a Path,
    pub ffmpeg: Option<&'a Path>,
}

fn common(tools: &Tools) -> Vec<String> {
    let mut args: Vec<String> = [
        "--ignore-config",
        "--no-playlist",
        "--no-warnings",
        "--no-cache-dir",
        "--encoding",
        "utf-8",
        "--js-runtimes",
    ]
    .map(String::from)
    .to_vec();
    args.push(format!("quickjs:{}", tools.quickjs.display()));
    if let Some(ffmpeg) = tools.ffmpeg {
        args.push("--ffmpeg-location".to_string());
        args.push(ffmpeg.display().to_string());
    }
    args
}

pub fn analyse_args(url: &str, tools: &Tools) -> Vec<String> {
    let mut args = common(tools);
    args.push("--dump-single-json".to_string());
    // Nothing after this can be taken for an option.
    args.push("--".to_string());
    args.push(url.to_string());
    args
}

fn video_args(request: &Request, args: &mut Vec<String>) {
    let preferred = match request.format.as_str() {
        "webm" => "ext:webm:webm",
        _ => "ext:mp4:m4a",
    };
    // Some sites do not say their heights, and then there is no quality to ask for.
    let sorting = if request.quality > 0 {
        format!("res:{},{preferred}", request.quality)
    } else {
        preferred.to_string()
    };
    args.extend([
        "-f".to_string(),
        "bv*+ba/b".to_string(),
        "-S".to_string(),
        sorting,
        "--merge-output-format".to_string(),
        request.format.clone(),
        "--remux-video".to_string(),
        request.format.clone(),
    ]);
}

fn audio_args(request: &Request, args: &mut Vec<String>) {
    args.extend([
        "-f".to_string(),
        "ba/b".to_string(),
        "--extract-audio".to_string(),
        "--audio-format".to_string(),
        request.format.clone(),
    ]);
    if !is_lossless(&request.format) {
        args.push("--audio-quality".to_string());
        args.push(format!("{}K", request.quality));
    }
}

/// Keeping every sound, they have no quality to choose.
pub fn is_lossless(format: &str) -> bool {
    matches!(format, "flac" | "wav")
}

pub fn download_args(request: &Request, tools: &Tools, folder: &Path, work: &Path) -> Vec<String> {
    let mut args = common(tools);
    if request.audio {
        audio_args(request, &mut args);
    } else {
        video_args(request, &mut args);
    }
    args.extend([
        "--newline".to_string(),
        "--progress".to_string(),
        "--no-mtime".to_string(),
        "--progress-template".to_string(),
        PROGRESS_TEMPLATE.to_string(),
        "--progress-template".to_string(),
        format!("postprocess:{PROCESSING_MARK}%(progress.postprocessor)s"),
        "--print".to_string(),
        format!("after_move:{FILE_MARK}%(filepath)s"),
        "--no-simulate".to_string(),
        "--paths".to_string(),
        format!("home:{}", folder.display()),
        "--paths".to_string(),
        format!("temp:{}", work.display()),
        "--output".to_string(),
        "%(title).150B.%(ext)s".to_string(),
        "--".to_string(),
        request.url.clone(),
    ]);
    args
}

fn text_of(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value[*key].as_str().filter(|text| !text.is_empty()))
        .unwrap_or_default()
        .to_string()
}

fn size_of(format: &Value) -> Option<u64> {
    format["filesize"]
        .as_u64()
        .or_else(|| format["filesize_approx"].as_u64())
}

fn has_video(format: &Value) -> bool {
    format["vcodec"]
        .as_str()
        .is_some_and(|codec| codec != "none")
        && format["height"].is_u64()
}

fn is_sound_only(format: &Value) -> bool {
    format["vcodec"].as_str() == Some("none")
        && format["acodec"]
            .as_str()
            .is_some_and(|codec| codec != "none")
}

/// Sizes come from what would be picked: its list goes from the worst to the best of each kind.
fn video_qualities(formats: &[Value]) -> Vec<VideoQuality> {
    let sounds = || formats.iter().filter(|format| is_sound_only(format));
    let best_sound = sounds()
        .filter(|format| format["ext"].as_str() == Some("m4a"))
        .filter_map(size_of)
        .next_back()
        .or_else(|| sounds().filter_map(size_of).next_back());
    let mut heights: Vec<u32> = formats
        .iter()
        .filter(|format| has_video(format))
        .filter_map(|format| u32::try_from(format["height"].as_u64()?).ok())
        .collect();
    heights.sort_unstable_by(|a, b| b.cmp(a));
    heights.dedup();
    if heights.iter().any(|height| *height >= LOWEST_USUAL_HEIGHT) {
        heights.retain(|height| *height >= LOWEST_USUAL_HEIGHT);
    }
    heights
        .into_iter()
        .map(|height| {
            let same_height = || {
                formats
                    .iter()
                    .filter(|format| format["height"].as_u64() == Some(u64::from(height)))
                    .filter(|format| has_video(format))
            };
            let size = same_height()
                .filter(|format| format["ext"].as_str() == Some("mp4"))
                .filter_map(size_of)
                .next_back()
                .or_else(|| same_height().filter_map(size_of).next_back());
            VideoQuality {
                height,
                size: size.map(|size| size + best_sound.unwrap_or(0)),
            }
        })
        .collect()
}

fn audio_qualities(duration: Option<u64>) -> Vec<AudioQuality> {
    AUDIO_BITRATES
        .iter()
        .map(|bitrate| AudioQuality {
            bitrate: *bitrate,
            size: duration.map(|seconds| seconds * u64::from(*bitrate) * 1000 / 8),
        })
        .collect()
}

/// Only when it says so of everything: some sites do not name their codecs at all, and storyboards
/// have neither picture nor sound.
fn only_sound(formats: &[Value]) -> bool {
    let mut real = formats.iter().filter(|format| {
        format["vcodec"].as_str() != Some("none") || format["acodec"].as_str() != Some("none")
    });
    real.clone().next().is_some() && real.all(is_sound_only)
}

fn video_formats(formats: &[Value]) -> Vec<String> {
    let has_webm = formats
        .iter()
        .any(|format| has_video(format) && format["ext"].as_str() == Some("webm"));
    VIDEO_FORMATS
        .iter()
        .filter(|format| **format != "webm" || has_webm)
        .map(ToString::to_string)
        .collect()
}

/// Reads what yt-dlp says of a link. A live stream never ends, so it cannot be downloaded.
pub fn parse_info(json: &str) -> Result<Info, String> {
    let value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if value["is_live"].as_bool() == Some(true) {
        return Err("live".to_string());
    }
    let empty = Vec::new();
    let formats = value["formats"].as_array().unwrap_or(&empty);
    let duration = value["duration"].as_f64().map(|seconds| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = seconds.round() as u64;
        whole
    });
    let videos = video_qualities(formats);
    Ok(Info {
        title: text_of(&value, &["title", "fulltitle"]),
        author: text_of(&value, &["uploader", "channel", "creator", "artist"]),
        site: text_of(&value, &["webpage_url_domain", "extractor_key"]),
        duration,
        thumbnail: value["thumbnail"]
            .as_str()
            .filter(|address| is_link(address))
            .map(str::to_string),
        audio_only: only_sound(formats),
        videos,
        audios: audio_qualities(duration),
        video_formats: video_formats(formats),
    })
}

#[derive(Debug, Deserialize)]
struct RawProgress {
    file: Option<String>,
    done: Option<f64>,
    total: Option<f64>,
    estimate: Option<f64>,
    speed: Option<f64>,
    eta: Option<f64>,
}

/// What a line of yt-dlp says.
#[derive(Debug, PartialEq)]
pub enum Line {
    Progress {
        file: String,
        fraction: f64,
        speed: Option<f64>,
        eta: Option<f64>,
    },
    Processing,
    File(String),
    Other,
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim();
    if let Some(json) = line.strip_prefix(PROGRESS_MARK) {
        let Ok(raw) = serde_json::from_str::<RawProgress>(json) else {
            return Line::Other;
        };
        let total = raw.total.or(raw.estimate).filter(|total| *total > 0.0);
        return Line::Progress {
            file: raw.file.unwrap_or_default(),
            fraction: match (raw.done, total) {
                (Some(done), Some(total)) => (done / total).clamp(0.0, 1.0),
                _ => 0.0,
            },
            speed: raw.speed,
            eta: raw.eta,
        };
    }
    if line.starts_with(PROCESSING_MARK) {
        return Line::Processing;
    }
    if let Some(file) = line.strip_prefix(FILE_MARK) {
        return Line::File(file.to_string());
    }
    Line::Other
}

/// Turns the downloads of a job into one bar that never goes back: a video then its sound are two
/// downloads, each going from 0 to 100 %.
#[derive(Debug, Default)]
pub struct Bar {
    audio: bool,
    file: Option<String>,
    stream: u32,
    percent: f64,
}

impl Bar {
    pub fn new(audio: bool) -> Self {
        Self {
            audio,
            ..Self::default()
        }
    }

    pub fn advance(&mut self, file: &str, fraction: f64) -> f64 {
        if self.file.as_deref() != Some(file) {
            if self.file.is_some() {
                self.stream += 1;
            }
            self.file = Some(file.to_string());
        }
        let overall = if self.audio {
            fraction
        } else if self.stream == 0 {
            fraction * VIDEO_SHARE
        } else {
            VIDEO_SHARE + fraction * (1.0 - VIDEO_SHARE)
        };
        self.percent = self.percent.max(overall * LAST_PERCENT);
        self.percent
    }
}

/// What the tool says when yt-dlp gives up, from the last thing it wrote on its errors.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Failure {
    Unsupported,
    Protected,
    Unavailable,
    Network,
    Other,
}

pub fn failure_of(errors: &str) -> Failure {
    let text = errors.to_lowercase();
    if text.contains("unsupported url") {
        Failure::Unsupported
    } else if text.contains("drm") {
        Failure::Protected
    } else if [
        "private",
        "unavailable",
        "not available",
        "removed",
        "sign in",
        "login",
        "logged-in",
        "authentication",
        "blocked",
        "http error 4",
        "log in",
        "live",
    ]
    .iter()
    .any(|word| text.contains(word))
    {
        Failure::Unavailable
    } else if [
        "timed out",
        "unable to download",
        "connection",
        "getaddrinfo",
        "network",
    ]
    .iter()
    .any(|word| text.contains(word))
    {
        Failure::Network
    } else {
        Failure::Other
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn tools() -> Tools<'static> {
        Tools {
            quickjs: Path::new("C:/tools/qjs.exe"),
            ffmpeg: Some(Path::new("C:/shared/ffmpeg")),
        }
    }

    fn request(audio: bool, format: &str, quality: u32) -> Request {
        Request {
            url: "https://www.youtube.com/watch?v=aqz-KE-bpKQ".to_string(),
            audio,
            format: format.to_string(),
            quality,
        }
    }

    #[test]
    fn only_a_link_of_the_web_gets_to_the_program() {
        assert!(is_link("https://www.tiktok.com/@a/video/1"));
        assert!(is_link("http://exemple.fr/a?b=c"));
        for bad in [
            "--exec=calc",
            "file:///c:/a",
            "C:/video.mp4",
            "https://a b",
            "https://a\nb",
            "ftp://a",
            "",
        ] {
            assert!(!is_link(bad), "{bad}");
        }
        assert!(!is_link(&format!("https://{}", "a".repeat(3000))));
    }

    #[test]
    fn a_request_is_checked_before_anything_runs() {
        assert!(request(false, "mp4", 1080).check().is_ok());
        assert!(request(true, "mp3", 192).check().is_ok());
        assert!(request(false, "mp3", 1080).check().is_err());
        assert!(request(true, "exe", 192).check().is_err());
        let mut bad = request(false, "mp4", 1080);
        bad.url = "--exec=calc".to_string();
        assert!(bad.check().is_err());
    }

    #[test]
    fn nothing_after_the_options_can_be_taken_for_one() {
        let args = analyse_args("https://a.fr/b", &tools());
        assert_eq!(&args[args.len() - 2..], ["--", "https://a.fr/b"]);
        let args = download_args(
            &request(false, "mp4", 720),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert_eq!(args[args.len() - 2], "--");
        assert_eq!(args.last(), Some(&request(false, "mp4", 720).url));
        assert!(args.contains(&"--ignore-config".to_string()));
        assert!(args.contains(&"--no-playlist".to_string()));
    }

    #[test]
    fn a_video_asks_for_its_height_and_container() {
        let args = download_args(
            &request(false, "mkv", 1080),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert!(args.contains(&"res:1080,ext:mp4:m4a".to_string()));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--merge-output-format", "mkv"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--remux-video", "mkv"]));
        assert!(args.contains(&"home:D:/v".to_string()));
        let args = download_args(
            &request(false, "webm", 720),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert!(args.contains(&"res:720,ext:webm:webm".to_string()));
    }

    #[test]
    fn a_sound_asks_for_its_bitrate_unless_it_loses_nothing() {
        let args = download_args(
            &request(true, "mp3", 192),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert!(args.contains(&"--extract-audio".to_string()));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--audio-quality", "192K"])
        );
        let args = download_args(
            &request(true, "flac", 192),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert!(!args.contains(&"--audio-quality".to_string()));
    }

    fn sample() -> String {
        json!({
            "title": "Big Buck Bunny",
            "uploader": "Blender",
            "extractor_key": "Youtube",
            "duration": 635.2,
            "thumbnail": "https://i.ytimg.com/a.webp",
            "formats": [
                { "format_id": "sb0", "ext": "mhtml", "vcodec": "none", "acodec": "none" },
                { "ext": "m4a", "vcodec": "none", "acodec": "mp4a", "filesize": 10 },
                { "ext": "webm", "vcodec": "opus", "acodec": "none", "height": 1080, "filesize": 500 },
                { "ext": "mp4", "vcodec": "avc1", "acodec": "none", "height": 1080, "filesize_approx": 400 },
                { "ext": "webm", "vcodec": "vp9", "acodec": "none", "height": 720 },
                { "ext": "mp4", "vcodec": "avc1", "acodec": "none", "height": 1080 }
            ]
        })
        .to_string()
    }

    #[test]
    fn reads_what_can_be_asked_of_a_link() {
        let info = parse_info(&sample()).unwrap();
        assert_eq!(info.title, "Big Buck Bunny");
        assert_eq!(info.author, "Blender");
        assert_eq!(info.site, "Youtube");
        assert_eq!(info.duration, Some(635));
        assert!(!info.audio_only);
        assert_eq!(
            info.videos,
            [
                VideoQuality {
                    height: 1080,
                    size: Some(410)
                },
                VideoQuality {
                    height: 720,
                    size: None
                },
            ]
        );
        assert_eq!(info.video_formats, ["mp4", "mkv", "webm"]);
        assert_eq!(
            info.audios[1],
            AudioQuality {
                bitrate: 192,
                size: Some(635 * 192 * 1000 / 8)
            }
        );
    }

    #[test]
    fn a_link_with_only_sound_offers_no_video() {
        let json = json!({
            "title": "Morceau",
            "duration": 60,
            "formats": [{ "ext": "mp3", "vcodec": "none", "acodec": "mp3" }]
        });
        let info = parse_info(&json.to_string()).unwrap();
        assert!(info.audio_only);
        assert_eq!(info.videos.len(), 0);
    }

    #[test]
    fn very_low_heights_are_only_offered_when_nothing_is_better() {
        let format = |height: u32| json!({ "ext": "mp4", "vcodec": "h264", "height": height });
        let heights = |list: Vec<Value>| -> Vec<u32> {
            video_qualities(&list)
                .iter()
                .map(|video| video.height)
                .collect()
        };
        assert_eq!(
            heights(vec![format(144), format(360), format(720)]),
            [720, 360]
        );
        assert_eq!(heights(vec![format(144), format(240)]), [240, 144]);
    }

    #[test]
    fn a_site_that_does_not_name_its_codecs_still_has_a_video() {
        let json = json!({ "formats": [{ "ext": "mp4", "vcodec": null, "acodec": null, "height": null }] });
        let info = parse_info(&json.to_string()).unwrap();
        assert!(!info.audio_only);
        assert_eq!(info.videos.len(), 0);
        let args = download_args(
            &request(false, "mp4", 0),
            &tools(),
            Path::new("D:/v"),
            Path::new("D:/t"),
        );
        assert!(args.contains(&"ext:mp4:m4a".to_string()));
    }

    #[test]
    fn webm_is_only_offered_when_the_site_has_it() {
        let json = json!({
            "formats": [{ "ext": "mp4", "vcodec": "h264", "acodec": "aac", "height": 720 }]
        });
        assert_eq!(
            parse_info(&json.to_string()).unwrap().video_formats,
            ["mp4", "mkv"]
        );
    }

    #[test]
    fn a_live_stream_is_refused_and_a_bad_answer_too() {
        assert!(parse_info(&json!({ "is_live": true }).to_string()).is_err());
        assert!(parse_info("pas du json").is_err());
    }

    #[test]
    fn a_strange_thumbnail_is_left_out() {
        let json = json!({ "thumbnail": "file:///c:/a.png", "formats": [] });
        assert_eq!(parse_info(&json.to_string()).unwrap().thumbnail, None);
    }

    #[test]
    fn reads_the_lines_of_a_download() {
        let line = r#"progress:{"file":"a.mp4","done":50,"total":100,"estimate":null,"speed":12.5,"eta":4}"#;
        assert_eq!(
            parse_line(line),
            Line::Progress {
                file: "a.mp4".to_string(),
                fraction: 0.5,
                speed: Some(12.5),
                eta: Some(4.0),
            }
        );
        let estimated =
            r#"progress:{"file":"a","done":1,"total":null,"estimate":4,"speed":null,"eta":null}"#;
        assert!(
            matches!(parse_line(estimated), Line::Progress { fraction, .. } if (fraction - 0.25).abs() < 1e-9)
        );
        let unknown = r#"progress:{"file":"a","done":1,"total":null,"estimate":null,"speed":null,"eta":null}"#;
        assert!(matches!(parse_line(unknown), Line::Progress { fraction, .. } if fraction == 0.0));
        let missing =
            r#"progress:{"file":"a","done":1,"total":4,"estimate":null,"speed":2.5,"eta":0}"#;
        assert!(matches!(
            parse_line(missing),
            Line::Progress { eta: Some(_), .. }
        ));
        assert_eq!(parse_line("processing:Merger"), Line::Processing);
        assert_eq!(
            parse_line("file:D:/v/a.mp4\r\n"),
            Line::File("D:/v/a.mp4".to_string())
        );
        assert_eq!(parse_line("[youtube] Extracting"), Line::Other);
        assert_eq!(parse_line("progress:pas du json"), Line::Other);
    }

    #[test]
    fn the_template_of_progress_is_read_back() {
        assert!(PROGRESS_TEMPLATE.starts_with("download:progress:"));
        assert_eq!(PROGRESS_TEMPLATE.matches("|null").count(), 5);
    }

    #[test]
    fn the_bar_never_goes_back_when_the_sound_starts() {
        let mut bar = Bar::new(false);
        let end_of_video = bar.advance("a.f137.mp4", 1.0);
        let start_of_sound = bar.advance("a.f140.m4a", 0.0);
        assert!(start_of_sound >= end_of_video);
        assert!(bar.advance("a.f140.m4a", 0.5) > start_of_sound);
        assert!(bar.advance("a.f140.m4a", 0.2) >= bar.percent - f64::EPSILON);
    }

    #[test]
    fn a_sound_alone_takes_the_whole_bar() {
        let mut bar = Bar::new(true);
        assert!((bar.advance("a.m4a", 0.5) - LAST_PERCENT / 2.0).abs() < 1e-9);
    }

    #[test]
    fn tells_why_it_gave_up() {
        assert_eq!(
            failure_of("ERROR: Unable to download webpage: HTTP Error 404: Not Found"),
            Failure::Unavailable
        );
        assert_eq!(
            failure_of("ERROR: Your IP address is blocked from accessing this post"),
            Failure::Unavailable
        );
        assert_eq!(
            failure_of("ERROR: Unsupported URL: https://a"),
            Failure::Unsupported
        );
        assert_eq!(
            failure_of("ERROR: This video is DRM protected"),
            Failure::Protected
        );
        assert_eq!(
            failure_of("ERROR: Private video. Sign in"),
            Failure::Unavailable
        );
        assert_eq!(
            failure_of("ERROR: Unable to download webpage: timed out"),
            Failure::Network
        );
        assert_eq!(failure_of("ERROR: something else"), Failure::Other);
    }
}
