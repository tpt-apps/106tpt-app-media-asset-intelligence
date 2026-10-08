<<<<<<< HEAD
use sha2::{Digest, Sha256};

/// Deterministic SHA-256 fingerprint of raw bytes, hex-encoded.
/// Same input bytes always yield the same fingerprint (spec §3.2).
pub fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
=======
//! Content fingerprint for exact-duplicate detection (spec §6.2, §8).
//!
//! The fingerprint is a SHA-256 content hash over the file's bytes, computed by
//! the ingest pipeline. It is the basis of Layer 1 (exact-hash) duplicate
//! detection: two assets with equal fingerprints are byte-identical regardless
//! of filename or location (spec §8).
//!
//! Hashing is deterministic (spec §3.2) and streaming: files are read in fixed
//! chunks, so fingerprinting a multi-gigabyte master costs constant memory.
//!
//! The hex form is the stable external representation (CLI output, JSON
//! reports); comparisons use the raw 32-byte digest.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Read;

/// Number of bytes in the SHA-256 digest.
pub const FINGERPRINT_LEN: usize = 32;

/// Streaming read chunk size for [`AssetFingerprint::from_reader`].
const CHUNK_LEN: usize = 64 * 1024;

/// A SHA-256 content fingerprint over an asset's bytes.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssetFingerprint([u8; FINGERPRINT_LEN]);

impl AssetFingerprint {
    /// Creates a fingerprint from a raw 32-byte SHA-256 digest.
    pub const fn from_bytes(bytes: [u8; FINGERPRINT_LEN]) -> Self {
        Self(bytes)
    }

    /// Hashes an in-memory byte slice.
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Hashes a stream in fixed chunks, costing constant memory regardless of
    /// file size (spec §18: archives in the multiple terabytes).
    ///
    /// # Errors
    ///
    /// Propagates I/O errors from the reader; the ingest pipeline surfaces
    /// them as archive-health findings rather than crashing (spec §17).
    pub fn from_reader(mut reader: impl Read) -> std::io::Result<Self> {
        let mut hasher = Sha256::new();
        let mut chunk = vec![0u8; CHUNK_LEN];
        loop {
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            hasher.update(&chunk[..read]);
        }
        Ok(Self(hasher.finalize().into()))
    }

    /// Hashes the file at `path`.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors (missing file, permission denied, …).
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(std::io::BufReader::new(file))
    }

    /// Returns the raw 32-byte digest.
    pub const fn as_bytes(&self) -> &[u8; FINGERPRINT_LEN] {
        &self.0
    }

    /// Creates a fingerprint from its 64-character lowercase hex representation.
    ///
    /// Returns `None` when the string is not exactly 64 hex characters.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != FINGERPRINT_LEN * 2 {
            return None;
        }
        let mut bytes = [0u8; FINGERPRINT_LEN];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = hex_val(chunk[0])?;
            let lo = hex_val(chunk[1])?;
            bytes[i] = (hi << 4) | lo;
        }
        Some(Self(bytes))
    }

    /// Returns the 64-character lowercase hex representation.
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(FINGERPRINT_LEN * 2);
        for byte in &self.0 {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        // Uppercase is accepted on input for user convenience; output is always lowercase.
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl fmt::Debug for AssetFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AssetFingerprint({})", self.to_hex())
    }
}

impl fmt::Display for AssetFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for AssetFingerprint {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for AssetFingerprint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Self::from_hex(&hex)
            .ok_or_else(|| serde::de::Error::custom("fingerprint must be 64 hex characters"))
    }
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;

<<<<<<< HEAD
    #[test]
    fn fingerprint_is_deterministic() {
        assert_eq!(fingerprint_bytes(b"abc"), fingerprint_bytes(b"abc"));
    }

    #[test]
    fn fingerprint_differs_per_input() {
        assert_ne!(fingerprint_bytes(b"a"), fingerprint_bytes(b"b"));
    }

    #[test]
    fn fingerprint_handles_empty_input() {
        assert_eq!(fingerprint_bytes(b"").len(), 64);
    }

    #[test]
    fn fingerprint_handles_boundary_single_byte() {
        assert_eq!(fingerprint_bytes(&[0x00]).len(), 64);
=======
    fn sample_digest() -> [u8; FINGERPRINT_LEN] {
        let mut bytes = [0u8; FINGERPRINT_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        bytes
    }

    #[test]
    fn empty_input_matches_the_sha256_empty_digest() {
        // Well-known SHA-256 of the empty string.
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(AssetFingerprint::from_slice(b"").to_hex(), expected);
    }

    #[test]
    fn abc_vector_matches_the_sha256_test_vector() {
        // Well-known SHA-256 of "abc".
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(AssetFingerprint::from_slice(b"abc").to_hex(), expected);
    }

    #[test]
    fn streaming_hash_equals_single_shot_regardless_of_chunking() {
        // 3×CHUNK_LEN + 17 forces partial chunks at both ends.
        let payload: Vec<u8> = (0..3 * CHUNK_LEN + 17).map(|i| (i % 251) as u8).collect();
        let single = AssetFingerprint::from_slice(&payload);
        let streamed = AssetFingerprint::from_reader(std::io::Cursor::new(&payload)).unwrap();
        assert_eq!(single, streamed);
    }

    #[test]
    fn one_byte_difference_changes_the_fingerprint() {
        let a = AssetFingerprint::from_slice(b"interview final");
        let b = AssetFingerprint::from_slice(b"interview finam");
        assert_ne!(a, b);
    }

    #[test]
    fn from_file_hashes_file_contents() {
        let dir = std::env::temp_dir().join("tpt-mai-fingerprint-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("payload.bin");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            AssetFingerprint::from_file(&path).unwrap(),
            AssetFingerprint::from_slice(b"abc")
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn hex_round_trip_is_lowercase() {
        let fp = AssetFingerprint::from_bytes(sample_digest());
        let hex = fp.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(AssetFingerprint::from_hex(&hex), Some(fp));
    }

    #[test]
    fn hex_input_accepts_uppercase() {
        let fp = AssetFingerprint::from_bytes(sample_digest());
        let upper = fp.to_hex().to_uppercase();
        assert_eq!(AssetFingerprint::from_hex(&upper), Some(fp));
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert_eq!(AssetFingerprint::from_hex(""), None);
        assert_eq!(AssetFingerprint::from_hex("zz"), None);
        assert_eq!(AssetFingerprint::from_hex(&"a".repeat(63)), None);
        assert_eq!(AssetFingerprint::from_hex(&"a".repeat(65)), None);
        assert_eq!(
            AssetFingerprint::from_hex(&format!("{}g", "0".repeat(63))),
            None
        );
    }

    #[test]
    fn serializes_as_hex_string() {
        let fp = AssetFingerprint::from_bytes(sample_digest());
        let json = serde_json::to_string(&fp).unwrap();
        assert_eq!(json, format!("\"{}\"", fp.to_hex()));
        assert_eq!(serde_json::from_str::<AssetFingerprint>(&json).unwrap(), fp);
    }

    #[test]
    fn invalid_json_shape_is_rejected() {
        assert!(serde_json::from_str::<AssetFingerprint>("\"tooshort\"").is_err());
        assert!(serde_json::from_str::<AssetFingerprint>("42").is_err());
    }

    #[test]
    fn empty_digest_is_a_valid_value() {
        let fp = AssetFingerprint::from_bytes([0u8; FINGERPRINT_LEN]);
        assert_eq!(
            fp.to_hex(),
            "0".repeat(64),
            "the all-zero digest must remain representable"
        );
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
