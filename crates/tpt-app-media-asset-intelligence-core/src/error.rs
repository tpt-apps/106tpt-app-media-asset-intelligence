//! Errors shared across the engine boundary (spec §14, §26 step 24).
//!
//! The engine reports failures with stable, machine-readable kinds so the CLI can
//! map them onto the stable exit-code contract (spec §14) without string matching.

use thiserror::Error;

/// Convenient result alias for engine-level fallible operations.
pub type Result<T, E = CoreError> = std::result::Result<T, E>;

/// Errors that can surface from the shared engine primitives.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A required input was empty, malformed or otherwise unusable.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A referenced entity does not exist in the archive index.
    #[error("not found: {0}")]
    NotFound(String),

    /// The operation would violate an invariant of the archive model.
    #[error("constraint violated: {0}")]
    ConstraintViolated(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_stable() {
        let err = CoreError::InvalidInput("empty query".to_string());
        assert_eq!(err.to_string(), "invalid input: empty query");

        let err = CoreError::NotFound("asset 7".to_string());
        assert_eq!(err.to_string(), "not found: asset 7");

        let err = CoreError::ConstraintViolated("duplicate keeper".to_string());
        assert_eq!(err.to_string(), "constraint violated: duplicate keeper");
    }
}
