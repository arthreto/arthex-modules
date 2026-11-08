//! What a code may ask to open: a web page, an email, a call. Never a program or a file.

const SCHEMES: [&str; 4] = ["https://", "http://", "mailto:", "tel:"];

/// The address when its scheme is one the user expects a code to open, and it holds nothing else.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn checked(address: &str) -> Result<&str, String> {
    let lower = address.to_ascii_lowercase();
    let known = SCHEMES
        .iter()
        .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len());
    if !known
        || address
            .chars()
            .any(|char| char.is_control() || char.is_whitespace())
    {
        return Err("this address cannot be opened".to_string());
    }
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_pages_emails_and_calls_open() {
        for address in [
            "https://github.com",
            "HTTP://example.com/a?b=c",
            "mailto:a@b.fr?subject=Hi%20there",
            "tel:+3361234",
        ] {
            assert_eq!(checked(address), Ok(address));
        }
    }

    #[test]
    fn programs_files_and_odd_addresses_do_not() {
        for address in [
            "file:///C:/Windows/System32/calc.exe",
            "C:\\Windows\\notepad.exe",
            "ms-settings:",
            "javascript:alert(1)",
            "https://",
            "https://a.com b",
            "https://a.com\n",
        ] {
            assert!(checked(address).is_err(), "{address}");
        }
    }
}
