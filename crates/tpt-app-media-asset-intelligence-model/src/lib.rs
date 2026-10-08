<<<<<<< HEAD
//! Domain model (spec §6): Archive, Asset, Derivative, Tag, DuplicateGroup, Scene.

pub mod tpt_convert;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub type ArchiveId = Uuid;
pub type AssetId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archive {
    pub id: ArchiveId,
    pub name: String,
    pub roots: Vec<PathBuf>,
    pub watch_enabled: bool,
    pub ai_settings: AiSettings,
}

/// Cloud AI fields default to disabled/None (spec §6.1, §3.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub local_tagging_enabled: bool,
    pub cloud_tagging_enabled: bool,
    pub cloud_provider: Option<String>,
    pub cloud_semantic_search_enabled: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            local_tagging_enabled: true,
            cloud_tagging_enabled: false,
            cloud_provider: None,
            cloud_semantic_search_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub archive: ArchiveId,
    pub path: PathBuf,
    pub fingerprint: String,
    pub size_bytes: u64,
    pub modified_time: DateTime<Utc>,
    pub media_type: MediaType,
    pub technical_metadata: TechnicalMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Video,
    Audio,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalMetadata {
    pub codec: String,
    pub container: String,
    pub supported: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Derivative {
    pub asset_id: AssetId,
    pub kind: DerivativeKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivativeKind {
    Thumbnail,
    Proxy,
    Waveform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub asset_id: AssetId,
    pub label: String,
    pub source: TagSource,
    pub confidence: Option<f32>,
    pub model_version: Option<String>,
    pub rejected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagSource {
    Manual,
    LocalModel,
    CloudModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: Uuid,
    pub asset_ids: Vec<AssetId>,
    pub match_kind: MatchKind,
    pub reviewed: bool,
    pub keeper: Option<AssetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchKind {
    ExactHash,
    Perceptual,
    AudioFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub asset_id: AssetId,
    pub index: u32,
    pub start_secs: f64,
    pub end_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexEntry {
    pub asset_id: AssetId,
    pub filename: String,
    pub codec: String,
    pub tags: Vec<String>,
    pub notes: String,
=======
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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
<<<<<<< HEAD
    fn ai_settings_default_to_local_only() {
        let s = AiSettings::default();
        assert!(s.local_tagging_enabled);
        assert!(!s.cloud_tagging_enabled);
        assert!(!s.cloud_semantic_search_enabled);
        assert!(s.cloud_provider.is_none());
=======
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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
