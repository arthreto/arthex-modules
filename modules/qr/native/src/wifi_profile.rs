//! The profile Windows needs to join a network, written from what a code holds.

use std::fmt::Write;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(windows, derive(serde::Deserialize))]
pub enum Security {
    #[cfg_attr(windows, serde(rename = "WPA"))]
    Wpa,
    #[cfg_attr(windows, serde(rename = "WEP"))]
    Wep,
    #[cfg_attr(windows, serde(rename = "nopass"))]
    Open,
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The name in hexadecimal too: Windows matches a network by its bytes, whatever their encoding.
fn hex(text: &str) -> String {
    text.bytes().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02X}");
        out
    })
}

/// A WLAN profile named after the network. A WPA code does not say which version: WPA2 is the
/// one every router offers, alone or next to WPA3.
pub fn profile(ssid: &str, password: &str, security: Security, hidden: bool) -> String {
    let (authentication, encryption, key) = match security {
        Security::Wpa => ("WPA2PSK", "AES", Some("passPhrase")),
        Security::Wep => ("open", "WEP", Some("networkKey")),
        Security::Open => ("open", "none", None),
    };
    let shared_key = key.map_or_else(String::new, |key_type| {
        format!(
            "<sharedKey><keyType>{key_type}</keyType><protected>false</protected><keyMaterial>{}</keyMaterial></sharedKey>",
            escaped(password)
        )
    });
    format!(
        concat!(
            r#"<?xml version="1.0"?><WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">"#,
            "<name>{name}</name>",
            "<SSIDConfig><SSID><hex>{hex}</hex><name>{name}</name></SSID><nonBroadcast>{hidden}</nonBroadcast></SSIDConfig>",
            "<connectionType>ESS</connectionType><connectionMode>auto</connectionMode>",
            "<MSM><security><authEncryption><authentication>{authentication}</authentication>",
            "<encryption>{encryption}</encryption><useOneX>false</useOneX></authEncryption>{shared_key}</security></MSM>",
            "</WLANProfile>"
        ),
        name = escaped(ssid),
        hex = hex(ssid),
        hidden = hidden,
        authentication = authentication,
        encryption = encryption,
        shared_key = shared_key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wpa_network_has_its_passphrase_escaped() {
        let xml = profile("Café & co", "a<b", Security::Wpa, false);
        assert!(xml.contains("<name>Café &amp; co</name>"));
        assert!(xml.contains("<hex>436166C3A9202620636F</hex>"));
        assert!(
            xml.contains("<authentication>WPA2PSK</authentication><encryption>AES</encryption>")
        );
        assert!(xml.contains("<keyType>passPhrase</keyType><protected>false</protected><keyMaterial>a&lt;b</keyMaterial>"));
        assert!(xml.contains("<nonBroadcast>false</nonBroadcast>"));
    }

    #[test]
    fn an_open_network_has_no_key_and_a_hidden_one_says_so() {
        let xml = profile("Cafe", "", Security::Open, true);
        assert!(xml.contains("<authentication>open</authentication><encryption>none</encryption>"));
        assert!(!xml.contains("sharedKey"));
        assert!(xml.contains("<nonBroadcast>true</nonBroadcast>"));
    }

    #[test]
    fn a_wep_network_has_a_network_key() {
        let xml = profile("Old", "12345", Security::Wep, false);
        assert!(xml.contains("<encryption>WEP</encryption>"));
        assert!(xml.contains("<keyType>networkKey</keyType>"));
    }
}
