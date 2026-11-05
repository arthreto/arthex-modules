//! The images of the preview. The interface cannot load them itself: its content rules forbid
//! remote images, so they reach it as data addresses.

use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;

use crate::presence::is_web_address;

const MAX_IMAGE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LIST_BYTES: u64 = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Asset {
    id: String,
    name: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(concat!("Arthex Discord RPC/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn asset_id(assets: &[u8], name: &str) -> Option<String> {
    serde_json::from_slice::<Vec<Asset>>(assets)
        .ok()?
        .into_iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.id)
}

fn data_address(content_type: &str, bytes: &[u8]) -> Option<String> {
    let content_type = content_type.split(';').next()?.trim();
    content_type
        .starts_with("image/")
        .then(|| format!("data:{content_type};base64,{}", STANDARD.encode(bytes)))
}

fn download(agent: &ureq::Agent, address: &str, limit: u64) -> Option<(String, Vec<u8>)> {
    let mut response = agent.get(address).call().ok()?;
    let content_type = response
        .headers()
        .get("content-type")?
        .to_str()
        .ok()?
        .to_string();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .ok()?;
    Some((content_type, bytes))
}

/// The address of an image imported on the application, which Discord lists by name.
fn imported_image(agent: &ureq::Agent, app_id: &str, name: &str) -> Option<String> {
    if app_id.is_empty() || !app_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let list = format!("https://discord.com/api/v10/oauth2/applications/{app_id}/assets");
    let (_, assets) = download(agent, &list, MAX_LIST_BYTES)?;
    let id = asset_id(&assets, name)?;
    Some(format!(
        "https://cdn.discordapp.com/app-assets/{app_id}/{id}.png?size=160"
    ))
}

/// `value` is what the user typed: an address, or the name of an imported image.
pub fn fetch(app_id: &str, value: &str) -> Option<String> {
    let value = value.trim();
    let agent = agent();
    let address = if is_web_address(value) {
        value.to_string()
    } else {
        imported_image(&agent, app_id, value)?
    };
    let (content_type, bytes) = download(&agent, &address, MAX_IMAGE_BYTES)?;
    data_address(&content_type, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_an_imported_image_by_its_exact_name() {
        let assets =
            br#"[{"id":"1","type":1,"name":"logo"},{"id":"2","type":1,"name":"logo-big"}]"#;
        assert_eq!(asset_id(assets, "logo-big").as_deref(), Some("2"));
        assert_eq!(asset_id(assets, "LOGO"), None);
        assert_eq!(asset_id(b"not json", "logo"), None);
    }

    #[test]
    fn makes_a_data_address_only_from_an_image() {
        assert_eq!(
            data_address("image/png; charset=binary", b"abc").as_deref(),
            Some("data:image/png;base64,YWJj")
        );
        assert_eq!(data_address("text/html", b"abc"), None);
    }

    #[test]
    fn never_asks_for_the_images_of_a_malformed_application() {
        let agent = agent();
        assert_eq!(imported_image(&agent, "12/../34", "logo"), None);
        assert_eq!(imported_image(&agent, "", "logo"), None);
    }
}
