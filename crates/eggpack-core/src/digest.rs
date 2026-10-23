//! The single producer-side file-digest authority.
//!
//! `eggpack-core` owns exactly one way to turn a file into a byte count and a
//! lowercase hex SHA-256 digest. Manifest aggregation, local finalization, and
//! candidate qualification all route through [`file_digest`], so a recorded
//! digest always describes the bytes of the exact regular file that was
//! inspected. `eggpack-ci` and `eggpack-github` consume the same helper rather
//! than re-implementing one.
//!
//! A digest is an integrity fact about bytes only. Nothing here produces or
//! asserts a signature, a provenance chain, or a publisher identity.

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

/// Fixed streaming buffer size for file digests.
///
/// The digest never holds a whole artifact in memory: a finalized release
/// bundle can be far larger than the process should grow to accommodate.
const DIGEST_BUFFER_BYTES: usize = 64 * 1024;

/// Why a file digest was refused.
///
/// Every variant is a hard failure of the enclosing operation. There is no
/// sentinel digest, no partial record, and no retry against the same path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestError {
    /// The path could not be inspected.
    Unavailable,
    /// The path is not a regular non-symlink file.
    NotRegularFile,
    /// The file is empty; a zero-length input never produces a digest record.
    Empty,
    /// The file could not be opened for reading.
    OpenFailed,
    /// The file could not be read to its end.
    ReadFailed,
    /// The accumulated byte count overflowed `u64`.
    SizeOverflow,
    /// The path changed while it was being read.
    ChangedDuringRead,
}

impl DigestError {
    /// Bounded, single-line reason phrase for this refusal.
    ///
    /// The phrase is a fragment: a caller prefixes the subject it was
    /// digesting, so no diagnostic loses the context it had before the
    /// producer converged on one helper.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Unavailable => "is unavailable",
            Self::NotRegularFile => "is not a regular non-symlink file",
            Self::Empty => "must be non-empty",
            Self::OpenFailed => "cannot be opened",
            Self::ReadFailed => "read failed",
            Self::SizeOverflow => "size overflow",
            Self::ChangedDuringRead => "changed during read",
        }
    }
}

impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "digest input {}", self.reason())
    }
}

impl std::error::Error for DigestError {}

/// Digest one regular file into its byte count and its lowercase hex SHA-256.
///
/// This is the **single** producer-side file-digest authority. It refuses, in
/// this order:
///
/// 1. the path is inspected with `symlink_metadata`, so a symlink is never
///    followed and a non-regular file is refused;
/// 2. an empty file is refused **before** it is read, so a zero-length input
///    can never produce a digest record;
/// 3. the bytes are streamed through a fixed 64 KiB buffer and the byte count
///    is accumulated with overflow checking, so a large artifact never grows
///    the process to accommodate it;
/// 4. the path is inspected again with `symlink_metadata` after the read, and
///    the digest is refused if the path became a symlink, stopped being a
///    regular file, or changed length while it was read.
///
/// Step 4 is the time-of-check/time-of-use guard. The symlink rejection in
/// step 1 is a property of the path policy, so it stays a path observation;
/// the read then happens through the handle that was opened, and the evidence
/// recorded is of those bytes. Without the post-read re-check, a path
/// replaced between the check and the open could make the digest describe
/// some other file than the one that was inspected, and a file rewritten
/// mid-read could produce a size and a digest that disagree with each other.
///
/// The hashed bytes are unchanged by this helper: the hash function, the
/// encoding, and the lowercase hex representation are exactly what the
/// producer has always recorded.
pub fn file_digest(path: &Path) -> Result<(u64, String), DigestError> {
    let before = fs::symlink_metadata(path).map_err(|_| DigestError::Unavailable)?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(DigestError::NotRegularFile);
    }
    if before.len() == 0 {
        return Err(DigestError::Empty);
    }
    let mut file = File::open(path).map_err(|_| DigestError::OpenFailed)?;
    let (size, digest) = hash_stream(&mut file)?;
    post_read_check(path, size)?;
    Ok((size, digest))
}

/// Stream one open file through the digest, accumulating the byte count.
fn hash_stream(file: &mut File) -> Result<(u64, String), DigestError> {
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; DIGEST_BUFFER_BYTES];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| DigestError::ReadFailed)?;
        if count == 0 {
            break;
        }
        size = add_size(size, count as u64)?;
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
    Ok((
        size,
        digest.iter().map(|byte| format!("{byte:02x}")).collect(),
    ))
}

/// Accumulate the byte count without wrapping.
fn add_size(size: u64, count: u64) -> Result<u64, DigestError> {
    size.checked_add(count).ok_or(DigestError::SizeOverflow)
}

/// Re-inspect the path after the read and refuse a path that changed.
fn post_read_check(path: &Path, size: u64) -> Result<(), DigestError> {
    let after = fs::symlink_metadata(path).map_err(|_| DigestError::ChangedDuringRead)?;
    if !post_read_stable(&after, size) {
        return Err(DigestError::ChangedDuringRead);
    }
    Ok(())
}

/// The post-read stability rule: the same kind of regular file, same length.
fn post_read_stable(after: &fs::Metadata, size: u64) -> bool {
    !after.file_type().is_symlink() && after.is_file() && after.len() == size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_temp_dir;
    use std::path::PathBuf;

    fn write_file(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn regular_file_digest_is_the_lowercase_hex_sha256_of_its_bytes() {
        let root = test_temp_dir("digest-regular");
        let path = write_file(&root, "artifact", b"abc");
        let (size, sha256) = file_digest(&path).unwrap();
        assert_eq!(size, 3);
        assert_eq!(
            sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(sha256.len(), 64);
        assert_eq!(sha256, sha256.to_lowercase());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn identical_bytes_digest_identically_and_different_bytes_differ() {
        let root = test_temp_dir("digest-determinism");
        let first = write_file(&root, "first", b"payload");
        let second = write_file(&root, "second", b"payload");
        let third = write_file(&root, "third", b"payload!");
        let first_digest = file_digest(&first).unwrap();
        let second_digest = file_digest(&second).unwrap();
        let third_digest = file_digest(&third).unwrap();
        assert_eq!(first_digest, second_digest);
        assert_ne!(first_digest, third_digest);
        assert_ne!(first_digest.0, third_digest.0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_file_is_refused_before_it_is_read() {
        let root = test_temp_dir("digest-empty");
        let path = write_file(&root, "empty", b"");
        assert_eq!(file_digest(&path).unwrap_err(), DigestError::Empty);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_is_refused() {
        let root = test_temp_dir("digest-directory");
        assert_eq!(file_digest(&root).unwrap_err(), DigestError::NotRegularFile);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_path_is_refused() {
        let root = test_temp_dir("digest-missing");
        let path = root.join("absent");
        assert_eq!(file_digest(&path).unwrap_err(), DigestError::Unavailable);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_is_refused() {
        let root = test_temp_dir("digest-symlink");
        let target = write_file(&root, "target", b"payload");
        let link = root.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(file_digest(&link).unwrap_err(), DigestError::NotRegularFile);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_read_length_change_is_refused() {
        // The read race cannot be staged deterministically on the supported
        // platforms: nothing guarantees that a path changes between the
        // helper's read and its post-read re-check. The plan therefore allows
        // asserting the post-read comparison directly instead of racing a
        // thread, and `post_read_check` is the helper's own post-read step,
        // exercised here on real files.
        let root = test_temp_dir("digest-length-change");
        let path = write_file(&root, "artifact", b"0123456789");
        let (size, _) = hash_stream(&mut File::open(&path).unwrap()).unwrap();
        assert_eq!(size, 10);
        fs::write(&path, b"0123456789abcd").unwrap();
        assert_eq!(
            post_read_check(&path, size).unwrap_err(),
            DigestError::ChangedDuringRead
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_read_type_change_is_refused() {
        // Same deterministic-staging note as the length-change test: the
        // helper's post-read step is asserted directly on real files.
        let root = test_temp_dir("digest-type-change");
        let path = write_file(&root, "artifact", b"0123456789");
        let (size, _) = hash_stream(&mut File::open(&path).unwrap()).unwrap();
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(
            post_read_check(&path, size).unwrap_err(),
            DigestError::ChangedDuringRead
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn post_read_symlink_change_is_refused() {
        // Same deterministic-staging note as the length-change test.
        let root = test_temp_dir("digest-symlink-change");
        let path = write_file(&root, "artifact", b"0123456789");
        let (size, _) = hash_stream(&mut File::open(&path).unwrap()).unwrap();
        fs::remove_file(&path).unwrap();
        let target = write_file(&root, "target", b"0123456789");
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert_eq!(
            post_read_check(&path, size).unwrap_err(),
            DigestError::ChangedDuringRead
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn size_overflow_is_an_error_not_a_wrapped_value() {
        assert_eq!(
            add_size(u64::MAX, 1).unwrap_err(),
            DigestError::SizeOverflow
        );
        assert_eq!(add_size(10, 5).unwrap(), 15);
    }
}
