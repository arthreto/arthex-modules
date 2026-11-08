//! Password hashes in the formats that store them: PHC strings, and the `$2b$` of bcrypt.
//!
//! The settings are those OWASP recommends. A hash to check may carry other settings, within limits
//! that keep one check from taking the computer for minutes.

use std::collections::HashMap;

use argon2::{Algorithm, Argon2, Params, Version};
use sha2::Sha256;

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_PASSES: u32 = 2;
const BCRYPT_COST: u32 = 12;
const SCRYPT_LOG_N: u8 = 17;
const SCRYPT_R: u32 = 8;
const PBKDF2_ROUNDS: u32 = 600_000;

const MAX_ARGON2_MEMORY_KIB: u32 = 1 << 20;
const MAX_PASSES: u32 = 64;
const MAX_SCRYPT_LOG_N: u8 = 20;
const MAX_PBKDF2_ROUNDS: u32 = 10_000_000;
const MAX_KEY_LEN: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Argon2id,
    Bcrypt,
    Scrypt,
    Pbkdf2,
}

pub fn hash(kind: Kind, password: &str) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|error| error.to_string())?;
    hash_with_salt(kind, password, &salt)
}

fn hash_with_salt(kind: Kind, password: &str, salt: &[u8; SALT_LEN]) -> Result<String, String> {
    let password = password.as_bytes();
    let mut key = [0u8; KEY_LEN];
    match kind {
        Kind::Argon2id => {
            argon2id(
                password,
                salt,
                ARGON2_MEMORY_KIB,
                ARGON2_PASSES,
                1,
                &mut key,
            )?;
            Ok(format!(
                "$argon2id$v=19$m={ARGON2_MEMORY_KIB},t={ARGON2_PASSES},p=1${}${}",
                b64(salt),
                b64(&key)
            ))
        }
        Kind::Scrypt => {
            scrypt(password, salt, SCRYPT_LOG_N, SCRYPT_R, 1, &mut key)?;
            Ok(format!(
                "$scrypt$ln={SCRYPT_LOG_N},r={SCRYPT_R},p=1${}${}",
                b64(salt),
                b64(&key)
            ))
        }
        Kind::Pbkdf2 => {
            pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, PBKDF2_ROUNDS, &mut key);
            Ok(format!(
                "$pbkdf2-sha256$i={PBKDF2_ROUNDS},l={KEY_LEN}${}${}",
                b64(salt),
                b64(&key)
            ))
        }
        Kind::Bcrypt => bcrypt::hash_with_salt(password, BCRYPT_COST, *salt)
            .map(|parts| parts.format_for_version(bcrypt::Version::TwoB))
            .map_err(|error| error.to_string()),
    }
}

/// Whether `password` gives `stored`. An error says the hash is not one the tool knows.
pub fn verify(password: &str, stored: &str) -> Result<bool, String> {
    let stored = stored.trim();
    if stored.starts_with("$2") {
        return bcrypt::verify(password, stored).map_err(|error| error.to_string());
    }
    let phc = Phc::parse(stored).ok_or("unknown hash")?;
    if phc.key.is_empty() || phc.key.len() > MAX_KEY_LEN {
        return Err("unknown hash".to_string());
    }
    let password = password.as_bytes();
    let mut key = vec![0u8; phc.key.len()];
    match phc.id {
        "argon2id" => {
            let memory = phc.number("m")?;
            let passes = phc.number("t")?;
            let lanes = phc.number("p")?;
            if memory > MAX_ARGON2_MEMORY_KIB || passes > MAX_PASSES || lanes > MAX_PASSES {
                return Err("settings too high".to_string());
            }
            argon2id(password, &phc.salt, memory, passes, lanes, &mut key)?;
        }
        "scrypt" => {
            let log_n = u8::try_from(phc.number("ln")?).map_err(|error| error.to_string())?;
            let r = phc.number("r")?;
            let p = phc.number("p")?;
            if log_n > MAX_SCRYPT_LOG_N || r > MAX_PASSES || p > MAX_PASSES {
                return Err("settings too high".to_string());
            }
            scrypt(password, &phc.salt, log_n, r, p, &mut key)?;
        }
        "pbkdf2-sha256" => {
            let rounds = phc.number("i")?;
            if rounds == 0 || rounds > MAX_PBKDF2_ROUNDS {
                return Err("settings too high".to_string());
            }
            pbkdf2::pbkdf2_hmac::<Sha256>(password, &phc.salt, rounds, &mut key);
        }
        _ => return Err("unknown hash".to_string()),
    }
    Ok(same(&key, &phc.key))
}

fn argon2id(
    password: &[u8],
    salt: &[u8],
    memory: u32,
    passes: u32,
    lanes: u32,
    out: &mut [u8],
) -> Result<(), String> {
    let params =
        Params::new(memory, passes, lanes, Some(out.len())).map_err(|error| error.to_string())?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password, salt, out)
        .map_err(|error| error.to_string())
}

fn scrypt(
    password: &[u8],
    salt: &[u8],
    log_n: u8,
    r: u32,
    p: u32,
    out: &mut [u8],
) -> Result<(), String> {
    let params = scrypt::Params::new(log_n, r, p, out.len()).map_err(|error| error.to_string())?;
    scrypt::scrypt(password, salt, &params, out).map_err(|error| error.to_string())
}

/// In constant time, so that a check says nothing of where the two differ.
fn same(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

/// `$id$settings$salt$key`, an optional `v=19` before the settings.
struct Phc<'a> {
    id: &'a str,
    settings: HashMap<&'a str, &'a str>,
    salt: Vec<u8>,
    key: Vec<u8>,
}

impl<'a> Phc<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let parts: Vec<&str> = text.strip_prefix('$')?.split('$').collect();
        let (id, rest) = parts.split_first()?;
        let [.., settings, salt, key] = rest else {
            return None;
        };
        let settings = settings
            .split(',')
            .map(|pair| pair.split_once('='))
            .collect::<Option<HashMap<_, _>>>()?;
        Some(Self {
            id,
            settings,
            salt: unb64(salt)?,
            key: unb64(key)?,
        })
    }

    fn number(&self, name: &str) -> Result<u32, String> {
        self.settings
            .get(name)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("no {name}"))
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 without padding, as PHC strings write it.
fn b64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &byte)| n | u32::from(byte) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(B64[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}

fn unb64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut bits, mut acc) = (0u32, 0u32);
    for byte in text.trim_end_matches('=').bytes() {
        let value = B64.iter().position(|&known| known == byte)?;
        acc = (acc << 6) | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_goes_both_ways_without_padding() {
        for length in 0u8..40 {
            let bytes: Vec<u8> = (0..length)
                .map(|i| i.wrapping_mul(37).wrapping_add(11))
                .collect();
            let text = b64(&bytes);
            assert!(!text.contains('='));
            assert_eq!(unb64(&text).unwrap(), bytes);
        }
    }

    #[test]
    fn checks_the_hashes_of_other_programs() {
        for stored in FOREIGN {
            assert_eq!(verify("motdepasse", stored), Ok(true), "{stored}");
            assert_eq!(verify("autre", stored), Ok(false), "{stored}");
        }
    }

    #[test]
    fn writes_the_same_strings_as_other_programs() {
        let salt: [u8; 16] = std::array::from_fn(|i| u8::try_from(i).unwrap());
        for kind in [Kind::Argon2id, Kind::Scrypt, Kind::Pbkdf2, Kind::Bcrypt] {
            let made = hash_with_salt(kind, "motdepasse", &salt).unwrap();
            let expected = SALTED.iter().find(|(k, _)| *k == kind).unwrap().1;
            assert_eq!(made, expected, "{kind:?}");
        }
    }

    #[test]
    fn a_new_hash_uses_a_new_salt_and_checks_out() {
        let first = hash(Kind::Pbkdf2, "motdepasse").unwrap();
        let second = hash(Kind::Pbkdf2, "motdepasse").unwrap();
        assert_ne!(first, second);
        assert_eq!(verify("motdepasse", &first), Ok(true));
    }

    #[test]
    fn refuses_unknown_hashes_and_settings_too_high() {
        assert!(verify("x", "not a hash").is_err());
        assert!(verify("x", "$md5$a$b$c").is_err());
        assert!(
            verify(
                "x",
                "$argon2id$v=19$m=99999999,t=1,p=1$c2FsdHNhbHQ$aGFzaGhhc2g"
            )
            .is_err()
        );
        assert!(
            verify(
                "x",
                "$pbkdf2-sha256$i=999999999,l=32$c2FsdHNhbHQ$aGFzaGhhc2g"
            )
            .is_err()
        );
    }

    const FOREIGN: [&str; 4] = [
        "$argon2id$v=19$m=4096,t=3,p=1$s9YfoedADLAiVlHZZqivYQ$MyP4CpZ+/j4YUvTCPna6SCyzkgue8K8kj7yHSev/nXY",
        "$2b$04$0xUyCAcTvnUfzllfz/Ekhedl81lVucRt0csEYo0rkvrw4DQ/Us/BC",
        "$scrypt$ln=14,r=8,p=1$Xt/5ONc7C0xdMG5iTPUuQg$3SIiJGAZhrS3r70iRlOMMvieIpvDsuvWJHBWFqT/UrI",
        "$pbkdf2-sha256$i=1000,l=32$QBC5hc/yOiNFajC5m6z/JQ$J0YN/q7td3ZQ9gCfHgQ8ercqT2ca5cegAqJi3n6iqe4",
    ];

    const SALTED: [(Kind, &str); 4] = [
        (
            Kind::Argon2id,
            "$argon2id$v=19$m=19456,t=2,p=1$AAECAwQFBgcICQoLDA0ODw$zuaVFsOmOlAgvNd7lGtncU+1LE42Px7+zdM5Wa/6hqM",
        ),
        (
            Kind::Scrypt,
            "$scrypt$ln=17,r=8,p=1$AAECAwQFBgcICQoLDA0ODw$5lDAE/u2iOsuZaQN6N0ZdTTHkSbWrpPqOmbElKznuEA",
        ),
        (
            Kind::Pbkdf2,
            "$pbkdf2-sha256$i=600000,l=32$AAECAwQFBgcICQoLDA0ODw$xULd0BOB0wH9+P4ovW/V4seLFtZAKjA23+ah1aOoVh8",
        ),
        (
            Kind::Bcrypt,
            "$2b$12$..CA.uOD/eaGAOmJB.yMBui4TzK1CGUIXkli.0B.t18dKPID5WYuG",
        ),
    ];
}
