//! Content fingerprint for exact-duplicate detection (spec §6.2, §8).
//!
//! The fingerprint is a SHA-256 content hash over the file's bytes, computed by
//! the ingest pipeline. It is the basis of Layer 1 (exact-hash) duplicate
//! detection: two assets with equal fingerprints are byte-identical regardless
//! of filename or location (spec §8).
//!
//! The hex form is the stable external representation (CLI output, JSON
//! reports); comparisons use the raw 32-byte digest.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Number of bytes in the SHA-256 digest.
pub const FINGERPRINT_LEN: usize = 32;

/// A SHA-256 content fingerprint over an asset's bytes.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssetFingerprint([u8; FINGERPRINT_LEN]);

impl AssetFingerprint {
    /// Creates a fingerprint from a raw 32-byte SHA-256 digest.
    pub const fn from_bytes(bytes: [u8; FINGERPRINT_LEN]) -> Self {
        Self(bytes)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_digest() -> [u8; FINGERPRINT_LEN] {
        let mut bytes = [0u8; FINGERPRINT_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        bytes
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
    }
}
