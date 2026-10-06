use thiserror::Error;

/// Errors raised by the deterministic indexing core. Corrupt/malformed
/// inputs must surface as `Unparsable`/`Io`, never panic (spec §17).
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("i/o error: {0}")]
    Io(String),
    #[error("unparsable media: {0}")]
    Unparsable(String),
    #[error("unsupported codec/container: {0}")]
    Unsupported(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("internal error: {0}")]
    Internal(String),
}
