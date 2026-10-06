//! localhost-only service surface (spec §15). Disabled by default,
//! binds 127.0.0.1 only — never externally.

/// Routes exposed when the operator explicitly enables the API.
pub const ROUTES: &[&str] = &[
    "GET /archives/:id/search",
    "GET /assets/:id",
    "POST /archives/:id/reindex",
    "GET /jobs/:id",
    "GET /health",
];

/// Service is always disabled unless explicitly enabled.
pub fn is_enabled(explicit_flag: bool) -> bool {
    explicit_flag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        assert!(!is_enabled(false));
    }

    #[test]
    fn routes_cover_spec_endpoints() {
        assert_eq!(ROUTES.len(), 5);
    }
}
