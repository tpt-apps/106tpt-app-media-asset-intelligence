//! Domain model for TPT Media Asset Intelligence (spec §6).
//!
//! This crate is pure data plus invariants. It performs no I/O, no networking
//! and no clock reads, which is what lets the ingest engine, the CLI and the
//! desktop UI share exactly one implementation of the archive model
//! (spec §3.6).
//!
//! The model's defining invariant is the AI boundary (spec §3.3, §6.1): every
//! [`Archive`] is constructed with cloud AI settings disabled, and the model
//! offers no operation that enables them silently. Enabling cloud features is an
//! explicit, auditable act performed by the application layer after the §10.1
//! disclosure flow.
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//! use tpt_app_media_asset_intelligence_model::Archive;
//!
//! let archive = Archive::new(1, "Main NAS", vec![PathBuf::from("/mnt/media-nas/projects")]);
//!
//! // Cloud AI fields default to disabled/None (spec §6.1)…
//! assert!(!archive.ai_settings.cloud_tagging_enabled);
//! assert!(archive.ai_settings.cloud_provider.is_none());
//! assert!(!archive.ai_settings.cloud_semantic_search_enabled);
//!
//! // …and local tagging is a plain per-archive switch.
//! assert!(!archive.ai_settings.local_tagging_enabled);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod archive;
pub mod asset;
pub mod derivative;
pub mod duplicate;
pub mod scene;
pub mod search_entry;
pub mod tag;

pub use archive::{AiSettings, Archive, CloudAiProvider};
pub use asset::{Asset, StreamInfo, TechnicalMetadata};
pub use derivative::{Derivative, DerivativeKind};
pub use duplicate::{DuplicateGroup, DuplicateGroupError, MatchKind, ReviewStatus};
pub use scene::{Scene, SceneDetectionMethod};
pub use search_entry::SearchIndexEntry;
pub use tag::{RejectedTag, Tag, TagSource};

use std::path::{Path, PathBuf};

/// A validated, normalized path within an archive root.
///
/// Ingest stores workspace-relative, forward-slash-normalized paths so an index
/// stays meaningful when the archive root moves between mounts (spec §12,
/// broken/relinked path detection). This helper performs that normalization;
/// strict path validation for watch-folder and NAS roots happens at the ingest
/// boundary (spec §17).
pub fn normalize_archive_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_normalize_to_forward_slashes() {
        let p = PathBuf::from(r"projects\2024\interview.mov");
        assert_eq!(
            normalize_archive_path(&p),
            PathBuf::from("projects/2024/interview.mov")
        );
    }

    #[test]
    fn posix_paths_pass_through_unchanged() {
        let p = PathBuf::from("projects/2024/interview.mov");
        assert_eq!(normalize_archive_path(&p), p);
    }
}
