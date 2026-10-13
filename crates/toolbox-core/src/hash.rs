//! Hashes of text or files with MD5, SHA-1, SHA-2 and BLAKE3, computed in one pass.

use crate::encoding::hex_encode;
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

impl Algorithm {
    pub const ALL: [Algorithm; 6] = [
        Algorithm::Md5,
        Algorithm::Sha1,
        Algorithm::Sha256,
        Algorithm::Sha384,
        Algorithm::Sha512,
        Algorithm::Blake3,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Algorithm::Md5 => "MD5",
            Algorithm::Sha1 => "SHA-1",
            Algorithm::Sha256 => "SHA-256",
            Algorithm::Sha384 => "SHA-384",
            Algorithm::Sha512 => "SHA-512",
            Algorithm::Blake3 => "BLAKE3",
        }
    }

    /// MD5 and SHA-1 are broken for security purposes; fine for checksums only.
    pub fn is_insecure(self) -> bool {
        matches!(self, Algorithm::Md5 | Algorithm::Sha1)
    }
}

/// Lowercase hex digests, indexed like [`Algorithm::ALL`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digests([String; 6]);

impl Digests {
    pub fn get(&self, algorithm: Algorithm) -> &str {
        &self.0[Algorithm::ALL
            .iter()
            .position(|&a| a == algorithm)
            .expect("every algorithm is listed")]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Algorithm, &str)> {
        Algorithm::ALL
            .into_iter()
            .zip(self.0.iter().map(String::as_str))
    }

    /// The algorithm whose digest equals `expected`, ignoring case, surrounding whitespace and
    /// a `sha256:`-style prefix.
    pub fn find_match(&self, expected: &str) -> Option<Algorithm> {
        let expected = expected.trim();
        let expected = expected
            .rsplit_once(':')
            .map_or(expected, |(_, digest)| digest)
            .trim()
            .to_lowercase();
        if expected.is_empty() {
            return None;
        }
        self.iter()
            .find(|(_, digest)| *digest == expected)
            .map(|(algorithm, _)| algorithm)
    }
}

/// Feeds data to every algorithm at once.
pub struct Hasher {
    md5: Md5,
    sha1: Sha1,
    sha256: Sha256,
    sha384: Sha384,
    sha512: Sha512,
    blake3: Box<blake3::Hasher>,
}

impl Default for Hasher {
    fn default() -> Self {
        Self {
            md5: Md5::new(),
            sha1: Sha1::new(),
            sha256: Sha256::new(),
            sha384: Sha384::new(),
            sha512: Sha512::new(),
            blake3: Box::new(blake3::Hasher::new()),
        }
    }
}

impl Hasher {
    pub fn update(&mut self, data: &[u8]) {
        self.md5.update(data);
        self.sha1.update(data);
        self.sha256.update(data);
        self.sha384.update(data);
        self.sha512.update(data);
        self.blake3.update(data);
    }

    pub fn finalize(self) -> Digests {
        Digests([
            hex_encode(&self.md5.finalize()),
            hex_encode(&self.sha1.finalize()),
            hex_encode(&self.sha256.finalize()),
            hex_encode(&self.sha384.finalize()),
            hex_encode(&self.sha512.finalize()),
            hex_encode(self.blake3.finalize().as_bytes()),
        ])
    }
}

pub fn hash_bytes(data: &[u8]) -> Digests {
    let mut hasher = Hasher::default();
    hasher.update(data);
    hasher.finalize()
}

/// Hashes a file in chunks, adding the bytes read so far to `progress`. Returns `Ok(None)` if
/// `cancel` was set before it finished.
pub fn hash_file(
    path: &Path,
    progress: &AtomicU64,
    cancel: &AtomicBool,
) -> io::Result<Option<Digests>> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::default();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let read = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        hasher.update(&buffer[..read]);
        progress.fetch_add(read as u64, Ordering::Relaxed);
    }
    Ok(Some(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        let digests = hash_bytes(b"abc");
        assert_eq!(
            digests.get(Algorithm::Md5),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            digests.get(Algorithm::Sha1),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            digests.get(Algorithm::Sha256),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digests.get(Algorithm::Sha384),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            digests.get(Algorithm::Sha512),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            digests.get(Algorithm::Blake3),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert_eq!(
            hash_bytes(b"").get(Algorithm::Blake3),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn file_matches_bytes_and_reports_progress() {
        let path = std::env::temp_dir().join(format!("toolbox-hash-test-{}", std::process::id()));
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        let progress = AtomicU64::new(0);
        let digests = hash_file(&path, &progress, &AtomicBool::new(false))
            .unwrap()
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(digests, hash_bytes(&data));
        assert_eq!(progress.load(Ordering::Relaxed), data.len() as u64);
    }

    #[test]
    fn cancelled_file_hash_returns_none() {
        let path = std::env::temp_dir().join(format!("toolbox-hash-cancel-{}", std::process::id()));
        std::fs::write(&path, b"data").unwrap();
        let result = hash_file(&path, &AtomicU64::new(0), &AtomicBool::new(true)).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn finds_matching_checksum() {
        let digests = hash_bytes(b"abc");
        let sha256 = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert_eq!(digests.find_match(sha256), Some(Algorithm::Sha256));
        assert_eq!(
            digests.find_match(&format!("  sha256:{sha256}\n")),
            Some(Algorithm::Sha256)
        );
        assert_eq!(
            digests.find_match("900150983cd24fb0d6963f7d28e17f72"),
            Some(Algorithm::Md5)
        );
        assert_eq!(digests.find_match("deadbeef"), None);
        assert_eq!(digests.find_match("  "), None);
    }
}
