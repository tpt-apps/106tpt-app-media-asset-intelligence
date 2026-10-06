//! Core primitives: codec allowlist, fingerprinting, error type (spec §3, §7, §17).
//!
//! MVP supports **open, royalty-free codecs only**. H.264/AAC are out of
//! scope (patent-encumbered). Out-of-scope files are still indexed by
//! path/fingerprint but flagged `unsupported` — never a crash.

pub mod codec;
pub mod error;
pub mod fingerprint;

pub use codec::{
    is_supported_codec, is_supported_extension, SUPPORTED_AUDIO_CODECS, SUPPORTED_CONTAINERS,
    SUPPORTED_VIDEO_CODECS,
};
pub use error::CoreError;
pub use fingerprint::fingerprint_bytes;
