//! Headless service host for the indexing/search engine (spec §3.6, §15).
//!
//! Two responsibilities land here:
//!
//! 1. **Background jobs** (spec §13.7): indexing, derivative-generation and
//!    tagging jobs with pause/resume/cancel, built on the foundation's
//!    resumable pipeline (spec §5.1) — Phase 1.
//! 2. **Optional localhost-only API** (spec §15): `127.0.0.1`, never bound
//!    externally, **disabled by default**, endpoints `/archives/:id/search`,
//!    `/assets/:id`, `/archives/:id/reindex`, `/jobs/:id`, `/health`.
//!
//! The API configuration type below encodes the §15 safety invariants now, so
//! the Phase 1 HTTP wiring cannot accidentally violate them: the default
//! binding is loopback, and `enabled` defaults to `false`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// Configuration of the optional local API (spec §15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalApiConfig {
    /// Whether the API server starts at all. `false` by default (spec §15).
    pub enabled: bool,
    /// Bind address. Defaults to loopback; binding any non-loopback address is
    /// rejected by [`LocalApiConfig::validate`] (spec §15: "Never bind
    /// externally by default").
    pub bind: String,
    /// TCP port.
    pub port: u16,
}

impl Default for LocalApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: "127.0.0.1".to_string(),
            port: 8420,
        }
    }
}

impl LocalApiConfig {
    /// Checks the §15 invariants: loopback-only binding.
    ///
    /// # Errors
    ///
    /// Returns a message naming the violation when the bind address is not a
    /// loopback address.
    pub fn validate(&self) -> Result<(), String> {
        if !loopback_host(host_of(&self.bind)) {
            return Err(format!(
                "the local API must bind a loopback address (127.0.0.1), got `{}`",
                self.bind
            ));
        }
        Ok(())
    }
}

/// Extracts the host portion of a `host` or `host:port` bind string,
/// understanding bracketed IPv6 literals (`[::1]:8420`) and bare IPv6 (`::1`).
fn host_of(bind: &str) -> &str {
    if let Some(rest) = bind.strip_prefix('[') {
        match rest.find(']') {
            Some(end) => &bind[..end + 2], // include both brackets
            None => bind,                  // malformed; rejected as non-loopback
        }
    } else if bind.matches(':').count() == 1 {
        &bind[..bind.rfind(':').unwrap()]
    } else {
        bind
    }
}

fn loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]") || host.starts_with("127.")
}

/// The local API endpoints (spec §15), enumerated for tests and docs.
pub const API_ENDPOINTS: [(&str, &str); 5] = [
    ("GET", "/archives/:id/search"),
    ("GET", "/assets/:id"),
    ("POST", "/archives/:id/reindex"),
    ("GET", "/jobs/:id"),
    ("GET", "/health"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_is_disabled_on_loopback_by_default() {
        let cfg = LocalApiConfig::default();
        assert!(!cfg.enabled, "disabled by default (spec §15)");
        assert_eq!(cfg.bind, "127.0.0.1");
        cfg.validate().unwrap();
    }

    #[test]
    fn external_binds_are_rejected() {
        for bind in ["0.0.0.0", "192.168.1.10", "10.0.0.7:8420"] {
            let cfg = LocalApiConfig {
                enabled: true,
                bind: bind.to_string(),
                port: 8420,
            };
            assert!(cfg.validate().is_err(), "{bind} must be rejected");
        }
    }

    #[test]
    fn loopback_variants_are_accepted() {
        for bind in ["127.0.0.1", "localhost", "::1", "[::1]:8420"] {
            let cfg = LocalApiConfig {
                enabled: true,
                bind: bind.to_string(),
                port: 8420,
            };
            cfg.validate()
                .unwrap_or_else(|e| panic!("{bind} rejected: {e}"));
        }
    }

    #[test]
    fn endpoint_list_matches_the_spec() {
        let paths: Vec<&str> = API_ENDPOINTS.iter().map(|(_, p)| *p).collect();
        assert_eq!(
            paths,
            vec![
                "/archives/:id/search",
                "/assets/:id",
                "/archives/:id/reindex",
                "/jobs/:id",
                "/health",
            ]
        );
    }
}
