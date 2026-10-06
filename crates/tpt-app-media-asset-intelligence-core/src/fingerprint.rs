use sha2::{Digest, Sha256};

/// Deterministic SHA-256 fingerprint of raw bytes, hex-encoded.
/// Same input bytes always yield the same fingerprint (spec §3.2).
pub fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
