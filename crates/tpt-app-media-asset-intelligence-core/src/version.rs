//! The engine version stamped into derivatives and index records (spec §6.3, §3.2).
//!
//! Derivatives record `generated_with_version` so a later application version can
//! decide which cached derivatives are stale and must be regenerated (cache
//! invalidation, spec §5.1). The version changes with any change that can alter
//! engine output — including the deterministic technical-analysis results
//! (spec §3.2).

/// Version of the indexing/search engine, stamped into derivatives and index records.
///
/// This is the engine's own version, independent of the workspace package
/// version: only changes to engine *output* bump it.
pub const ENGINE_VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_version_is_semver_shaped() {
        let parts: Vec<&str> = ENGINE_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "major.minor.patch");
        assert!(
            parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())),
            "numeric components only, got {ENGINE_VERSION}"
        );
    }
}
