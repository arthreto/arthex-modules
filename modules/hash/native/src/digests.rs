//! The digests the tool lists, all fed in one pass over the data.

use std::sync::LazyLock;

use crc::{CRC_32_ISCSI, CRC_32_ISO_HDLC, Crc};
use digest::{Digest, ExtendableOutput, Update, core_api::BlockSizeUser};
use hmac::{Mac, SimpleHmac, digest::KeyInit};

/// In the order of the tool, which groups them.
pub const ALGORITHMS: [&str; 22] = [
    "md5",
    "sha1",
    "ripemd160",
    "sha224",
    "sha256",
    "sha384",
    "sha512",
    "sha512-256",
    "sha3-224",
    "sha3-256",
    "sha3-384",
    "sha3-512",
    "keccak256",
    "shake128",
    "shake256",
    "blake2b",
    "blake2s",
    "blake3",
    "crc32",
    "crc32c",
    "adler32",
    "xxh64",
];

static CRC32: LazyLock<Crc<u32>> = LazyLock::new(|| Crc::<u32>::new(&CRC_32_ISO_HDLC));
static CRC32C: LazyLock<Crc<u32>> = LazyLock::new(|| Crc::<u32>::new(&CRC_32_ISCSI));

/// One algorithm being fed.
trait Running: Send {
    fn feed(&mut self, data: &[u8]);
    fn finish(self: Box<Self>) -> Vec<u8>;
}

struct Plain<D>(D);

impl<D: Digest + Send> Running for Plain<D> {
    fn feed(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

struct Keyed<M>(M);

impl<M: Mac + Send> Running for Keyed<M> {
    fn feed(&mut self, data: &[u8]) {
        Mac::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().into_bytes().to_vec()
    }
}

/// SHAKE gives as many bytes as asked: 256 bits for SHAKE128 and 512 for SHAKE256, as is usual.
struct Shake<X>(X, usize);

impl<X: Update + ExtendableOutput + Send> Running for Shake<X> {
    fn feed(&mut self, data: &[u8]) {
        Update::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        let mut out = vec![0; self.1];
        self.0.finalize_xof_into(&mut out);
        out
    }
}

struct Blake3(blake3::Hasher);

impl Running for Blake3 {
    fn feed(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().as_bytes().to_vec()
    }
}

struct Checksum(crc::Digest<'static, u32>);

impl Running for Checksum {
    fn feed(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_be_bytes().to_vec()
    }
}

struct Adler(adler2::Adler32);

impl Running for Adler {
    fn feed(&mut self, data: &[u8]) {
        self.0.write_slice(data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.checksum().to_be_bytes().to_vec()
    }
}

struct Xxh64(xxhash_rust::xxh64::Xxh64);

impl Running for Xxh64 {
    fn feed(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.digest().to_be_bytes().to_vec()
    }
}

fn plain(id: &str) -> Option<Box<dyn Running>> {
    Some(match id {
        "md5" => Box::new(Plain(md5::Md5::new())),
        "sha1" => Box::new(Plain(sha1::Sha1::new())),
        "ripemd160" => Box::new(Plain(ripemd::Ripemd160::new())),
        "sha224" => Box::new(Plain(sha2::Sha224::new())),
        "sha256" => Box::new(Plain(sha2::Sha256::new())),
        "sha384" => Box::new(Plain(sha2::Sha384::new())),
        "sha512" => Box::new(Plain(sha2::Sha512::new())),
        "sha512-256" => Box::new(Plain(sha2::Sha512_256::new())),
        "sha3-224" => Box::new(Plain(sha3::Sha3_224::new())),
        "sha3-256" => Box::new(Plain(sha3::Sha3_256::new())),
        "sha3-384" => Box::new(Plain(sha3::Sha3_384::new())),
        "sha3-512" => Box::new(Plain(sha3::Sha3_512::new())),
        "keccak256" => Box::new(Plain(sha3::Keccak256::new())),
        "shake128" => Box::new(Shake(sha3::Shake128::default(), 32)),
        "shake256" => Box::new(Shake(sha3::Shake256::default(), 64)),
        "blake2b" => Box::new(Plain(blake2::Blake2b512::new())),
        "blake2s" => Box::new(Plain(blake2::Blake2s256::new())),
        "blake3" => Box::new(Blake3(blake3::Hasher::new())),
        "crc32" => Box::new(Checksum(CRC32.digest())),
        "crc32c" => Box::new(Checksum(CRC32C.digest())),
        "adler32" => Box::new(Adler(adler2::Adler32::new())),
        "xxh64" => Box::new(Xxh64(xxhash_rust::xxh64::Xxh64::new(0))),
        _ => return None,
    })
}

fn hmac<D>(key: &[u8]) -> Option<Box<dyn Running>>
where
    D: Digest + BlockSizeUser + Send + 'static,
{
    let mac = <SimpleHmac<D> as KeyInit>::new_from_slice(key).ok()?;
    Some(Box::new(Keyed(mac)))
}

/// HMAC exists for the hashes built on a compression function, not for the others.
fn keyed(id: &str, key: &[u8]) -> Option<Box<dyn Running>> {
    match id {
        "md5" => hmac::<md5::Md5>(key),
        "sha1" => hmac::<sha1::Sha1>(key),
        "ripemd160" => hmac::<ripemd::Ripemd160>(key),
        "sha224" => hmac::<sha2::Sha224>(key),
        "sha256" => hmac::<sha2::Sha256>(key),
        "sha384" => hmac::<sha2::Sha384>(key),
        "sha512" => hmac::<sha2::Sha512>(key),
        "sha512-256" => hmac::<sha2::Sha512_256>(key),
        "sha3-224" => hmac::<sha3::Sha3_224>(key),
        "sha3-256" => hmac::<sha3::Sha3_256>(key),
        "sha3-384" => hmac::<sha3::Sha3_384>(key),
        "sha3-512" => hmac::<sha3::Sha3_512>(key),
        _ => None,
    }
}

/// Every algorithm at once, so that a file is read only one time.
pub struct Digests(Vec<(&'static str, Box<dyn Running>)>);

impl Digests {
    /// With a key, only the algorithms that have an HMAC.
    pub fn new(key: Option<&[u8]>) -> Self {
        Self(
            ALGORITHMS
                .iter()
                .filter_map(|&id| {
                    let running = match key {
                        Some(key) => keyed(id, key),
                        None => plain(id),
                    };
                    running.map(|running| (id, running))
                })
                .collect(),
        )
    }

    pub fn feed(&mut self, data: &[u8]) {
        for (_, running) in &mut self.0 {
            running.feed(data);
        }
    }

    /// The same, the algorithms shared out between `threads`, for the blocks of a large file.
    pub fn feed_in_parallel(&mut self, data: &[u8], threads: usize) {
        let per_thread = self.0.len().div_ceil(threads.max(1));
        std::thread::scope(|scope| {
            for group in self.0.chunks_mut(per_thread) {
                scope.spawn(move || {
                    for (_, running) in group {
                        running.feed(data);
                    }
                });
            }
        });
    }

    pub fn finish(self) -> Vec<(&'static str, String)> {
        self.0
            .into_iter()
            .map(|(id, running)| (id, hex(&running.finish())))
            .collect()
    }
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digests_of(text: &str, key: Option<&str>) -> Vec<(&'static str, String)> {
        let mut digests = Digests::new(key.map(str::as_bytes));
        digests.feed(text.as_bytes());
        digests.finish()
    }

    fn find(list: &[(&'static str, String)], id: &str) -> String {
        list.iter()
            .find(|(found, _)| *found == id)
            .map(|(_, hex)| hex.clone())
            .unwrap()
    }

    #[test]
    fn every_algorithm_gives_the_reference_digest_of_abc() {
        let list = digests_of("abc", None);
        assert_eq!(list.len(), ALGORITHMS.len());
        for (id, expected) in REFERENCE_ABC {
            assert_eq!(find(&list, id), expected, "{id}");
        }
    }

    #[test]
    fn feeding_in_pieces_changes_nothing() {
        let text = "x".repeat(5000);
        let whole = digests_of(&text, None);
        let mut pieces = Digests::new(None);
        for chunk in text.as_bytes().chunks(777) {
            pieces.feed(chunk);
        }
        assert_eq!(pieces.finish(), whole);
    }

    #[test]
    fn feeding_in_parallel_changes_nothing() {
        let text = "y".repeat(10_000);
        let mut parallel = Digests::new(None);
        parallel.feed_in_parallel(text.as_bytes(), 5);
        assert_eq!(parallel.finish(), digests_of(&text, None));
    }

    #[test]
    fn a_key_gives_the_hmac_of_the_algorithms_that_have_one() {
        let list = digests_of("bonjour", Some("secret"));
        assert_eq!(list.len(), 12);
        assert_eq!(
            find(&list, "sha256"),
            "ea583e6f2fa1f714ea389e448a56bbdbc287a6e51b3bce4d78045d0c109a2fb9"
        );
        assert_eq!(
            find(&list, "sha3-256"),
            REFERENCE_HMAC_SHA3_256,
            "HMAC-SHA3-256"
        );
        assert!(list.iter().all(|(id, _)| !id.starts_with("blake")));
    }

    const REFERENCE_HMAC_SHA3_256: &str =
        "41bd68d6cb6161a55f55daec2adcf04a6bfcdf65e170546dd833e49c8ed18a03";

    const REFERENCE_ABC: [(&str, &str); 22] = [
        ("md5", "900150983cd24fb0d6963f7d28e17f72"),
        ("sha1", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        ("ripemd160", "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"),
        (
            "sha224",
            "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
        ),
        (
            "sha256",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "sha384",
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
        ),
        (
            "sha512",
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        ),
        (
            "sha512-256",
            "53048e2681941ef99b2e29b76b4c7dabe4c2d0c634fc6d46e0e2f13107e7af23",
        ),
        (
            "sha3-224",
            "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf",
        ),
        (
            "sha3-256",
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
        ),
        (
            "sha3-384",
            "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25",
        ),
        (
            "sha3-512",
            "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
        ),
        (
            "keccak256",
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45",
        ),
        (
            "shake128",
            "5881092dd818bf5cf8a3ddb793fbcba74097d5c526a6d35f97b83351940f2cc8",
        ),
        (
            "shake256",
            "483366601360a8771c6863080cc4114d8db44530f8f1e1ee4f94ea37e78b5739d5a15bef186a5386c75744c0527e1faa9f8726e462a12a4feb06bd8801e751e4",
        ),
        (
            "blake2b",
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
        ),
        (
            "blake2s",
            "508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982",
        ),
        (
            "blake3",
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
        ),
        ("crc32", "352441c2"),
        ("crc32c", "364b3fb7"),
        ("adler32", "024d0127"),
        ("xxh64", "44bc2cf5ad770999"),
    ];
}
