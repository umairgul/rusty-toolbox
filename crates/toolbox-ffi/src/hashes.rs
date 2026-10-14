use crate::ToolError;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use toolbox_core::hash::{self, Algorithm, Digests};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

impl From<Algorithm> for HashAlgorithm {
    fn from(algorithm: Algorithm) -> Self {
        match algorithm {
            Algorithm::Md5 => HashAlgorithm::Md5,
            Algorithm::Sha1 => HashAlgorithm::Sha1,
            Algorithm::Sha256 => HashAlgorithm::Sha256,
            Algorithm::Sha384 => HashAlgorithm::Sha384,
            Algorithm::Sha512 => HashAlgorithm::Sha512,
            Algorithm::Blake3 => HashAlgorithm::Blake3,
        }
    }
}

impl From<HashAlgorithm> for Algorithm {
    fn from(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Md5 => Algorithm::Md5,
            HashAlgorithm::Sha1 => Algorithm::Sha1,
            HashAlgorithm::Sha256 => Algorithm::Sha256,
            HashAlgorithm::Sha384 => Algorithm::Sha384,
            HashAlgorithm::Sha512 => Algorithm::Sha512,
            HashAlgorithm::Blake3 => Algorithm::Blake3,
        }
    }
}

/// e.g. "SHA-256".
#[uniffi::export]
pub fn hash_algorithm_label(algorithm: HashAlgorithm) -> String {
    Algorithm::from(algorithm).label().into()
}

/// One algorithm's digest.
#[derive(Clone, Debug, PartialEq, Eq, Hash, uniffi::Record)]
pub struct HashDigest {
    pub algorithm: HashAlgorithm,
    /// e.g. "SHA-256".
    pub label: String,
    /// Lowercase hex.
    pub digest: String,
    /// MD5 and SHA-1: fine for checksums, broken for security uses.
    pub insecure: bool,
}

/// The digests of one input with every algorithm.
#[derive(uniffi::Object)]
pub struct HashDigests(Digests);

#[uniffi::export]
impl HashDigests {
    /// Every digest, in display order.
    pub fn entries(&self) -> Vec<HashDigest> {
        self.0
            .iter()
            .map(|(algorithm, digest)| HashDigest {
                algorithm: algorithm.into(),
                label: algorithm.label().into(),
                digest: digest.into(),
                insecure: algorithm.is_insecure(),
            })
            .collect()
    }

    /// The algorithm whose digest equals `expected`, ignoring case, surrounding whitespace and
    /// a `sha256:`-style prefix.
    pub fn find_match(&self, expected: &str) -> Option<HashAlgorithm> {
        self.0.find_match(expected).map(Into::into)
    }
}

/// Hashes the UTF-8 bytes of `text` with every algorithm. Slow for very large text: call it
/// off the main thread.
#[uniffi::export]
pub fn hashes_text(text: &str) -> Arc<HashDigests> {
    Arc::new(HashDigests(hash::hash_bytes(text.as_bytes())))
}

/// Hashes one file on a background thread. Create a job, call [`Self::run`] off the main
/// thread, and meanwhile read [`Self::bytes_read`] for progress or call [`Self::cancel`].
#[derive(Default, uniffi::Object)]
pub struct HashFileJob {
    progress: AtomicU64,
    cancel: AtomicBool,
}

#[uniffi::export]
impl HashFileJob {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Reads and hashes the whole file, blocking until done. Returns nil if cancelled.
    pub fn run(&self, path: &str) -> Result<Option<Arc<HashDigests>>, ToolError> {
        let path = Path::new(path);
        hash::hash_file(path, &self.progress, &self.cancel)
            .map(|digests| digests.map(|digests| Arc::new(HashDigests(digests))))
            .map_err(|error| {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                ToolError::new(format!("Couldn't read {name}: {error}"))
            })
    }

    /// Bytes hashed so far. Safe to call from any thread while [`Self::run`] is going.
    pub fn bytes_read(&self) -> u64 {
        self.progress.load(Ordering::Relaxed)
    }

    /// Stops [`Self::run`] at the next chunk; it then returns nil.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_digests_and_matching() {
        let digests = hashes_text("abc");
        let entries = digests.entries();
        assert_eq!(entries.len(), 6);
        assert_eq!(entries[0].label, "MD5");
        assert!(entries[0].insecure && !entries[2].insecure);
        assert_eq!(
            digests.find_match(&format!("sha256:{}", entries[2].digest.to_uppercase())),
            Some(HashAlgorithm::Sha256)
        );
        assert_eq!(digests.find_match("deadbeef"), None);
    }

    #[test]
    fn file_job_hashes_reports_progress_and_cancels() {
        let path = std::env::temp_dir().join(format!("toolbox-ffi-hash-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let path_text = path.to_string_lossy().into_owned();

        let job = HashFileJob::new();
        let digests = job.run(&path_text).unwrap().unwrap();
        assert_eq!(digests.entries(), hashes_text("abc").entries());
        assert_eq!(job.bytes_read(), 3);

        let cancelled = HashFileJob::new();
        cancelled.cancel();
        assert!(cancelled.run(&path_text).unwrap().is_none());

        std::fs::remove_file(&path).unwrap();
        let error = HashFileJob::new().run(&path_text).err().unwrap();
        assert!(
            error
                .to_string()
                .starts_with("Couldn't read toolbox-ffi-hash-")
        );
    }
}
