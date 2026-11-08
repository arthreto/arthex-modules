//! What the interface may ask TMDB for: a closed list of addresses, checked before any request.

use serde_json::{Map, Value};

pub const API: &str = "https://api.themoviedb.org/3";
const IMAGES: &str = "https://image.tmdb.org/t/p/";
const PARAMS: [&str; 6] = [
    "query",
    "language",
    "append_to_response",
    "include_video_language",
    "page",
    "region",
];
const SIZES: [&str; 5] = ["w92", "w185", "w300", "w342", "w500"];
const LONGEST_VALUE: usize = 200;

fn is_number(part: &str) -> bool {
    !part.is_empty() && part.len() <= 10 && part.bytes().all(|byte| byte.is_ascii_digit())
}

/// `/search/multi`, `/trending/all/week`, `/configuration`, `/movie/1`, `/tv/1`,
/// `/tv/1/season/2` or `/person/1`.
fn is_known_path(path: &str) -> bool {
    let parts: Vec<&str> = path.strip_prefix('/').unwrap_or("").split('/').collect();
    match parts.as_slice() {
        ["search", "multi"] | ["trending", "all", "week"] | ["configuration"] => true,
        ["movie" | "tv" | "person", id] => is_number(id),
        ["tv", id, "season", season] => is_number(id) && is_number(season),
        _ => false,
    }
}

/// Every byte but the unreserved ones of RFC 3986, as `%XX`.
fn encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The address of an API call, or why it is refused.
pub fn api_url(path: &str, params: &Map<String, Value>) -> Result<String, String> {
    if !is_known_path(path) {
        return Err(format!("{path} is not asked from TMDB"));
    }
    let mut query = Vec::new();
    for (name, value) in params {
        let Some(value) = value.as_str() else {
            return Err(format!("{name} is not a text"));
        };
        if !PARAMS.contains(&name.as_str()) || value.len() > LONGEST_VALUE {
            return Err(format!("{name} is not asked from TMDB"));
        }
        query.push(format!("{name}={}", encoded(value)));
    }
    let separator = if query.is_empty() { "" } else { "?" };
    Ok(format!("{API}{path}{separator}{}", query.join("&")))
}

/// The address of an image TMDB gave the path of, in one of its sizes.
pub fn image_url(path: &str, size: &str) -> Result<String, String> {
    let name = path.strip_prefix('/').unwrap_or("");
    let (stem, extension) = name.rsplit_once('.').unwrap_or(("", ""));
    let valid = SIZES.contains(&size)
        && !stem.is_empty()
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        && matches!(extension, "jpg" | "png" | "svg");
    if valid {
        Ok(format!("{IMAGES}{size}{path}"))
    } else {
        Err(format!("{path} is not a TMDB image"))
    }
}

/// The media type of an image, from its name.
pub fn image_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        _ => "image/jpeg",
    }
}

/// What may open in the browser: a page of `YouTube` or of TMDB, nothing else.
pub fn is_openable(address: &str) -> bool {
    let rest = address.strip_prefix("https://").unwrap_or("");
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    matches!(host, "www.youtube.com" | "www.themoviedb.org")
        && !address
            .chars()
            .any(|char| char.is_control() || char.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn known_calls_get_their_parameters_encoded() {
        assert_eq!(
            api_url("/search/multi", &params(&json!({ "query": "Amélie & co" }))),
            Ok(format!("{API}/search/multi?query=Am%C3%A9lie%20%26%20co"))
        );
        assert_eq!(
            api_url("/tv/90228/season/2", &Map::new()),
            Ok(format!("{API}/tv/90228/season/2"))
        );
    }

    #[test]
    fn other_calls_and_parameters_are_refused() {
        for path in [
            "/account",
            "/movie/abc",
            "/movie/1/rating",
            "/../movie/1",
            "movie/1",
        ] {
            assert!(api_url(path, &Map::new()).is_err(), "{path}");
        }
        assert!(api_url("/movie/1", &params(&json!({ "session_id": "x" }))).is_err());
        assert!(api_url("/movie/1", &params(&json!({ "page": 2 }))).is_err());
        assert!(
            api_url(
                "/search/multi",
                &params(&json!({ "query": "x".repeat(201) }))
            )
            .is_err()
        );
    }

    #[test]
    fn images_are_paths_of_tmdb_in_its_sizes() {
        assert_eq!(
            image_url("/abc_D-1.jpg", "w185"),
            Ok("https://image.tmdb.org/t/p/w185/abc_D-1.jpg".to_string())
        );
        assert!(image_url("/abc.jpg", "original").is_err());
        assert!(image_url("/../x.jpg", "w185").is_err());
        assert!(image_url("abc.jpg", "w185").is_err());
        assert!(image_url("/abc.exe", "w185").is_err());
        assert_eq!(image_type("/logo.png"), "image/png");
    }

    #[test]
    fn only_youtube_and_tmdb_pages_open() {
        assert!(is_openable("https://www.youtube.com/watch?v=Way9Dexny3w"));
        assert!(is_openable("https://www.themoviedb.org/movie/693134"));
        assert!(!is_openable("http://www.youtube.com/watch?v=x"));
        assert!(!is_openable("https://www.youtube.com.evil.com/"));
        assert!(!is_openable("https://evil.com/?https://www.youtube.com/"));
        assert!(!is_openable("https://www.youtube.com/ x"));
    }
}
