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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_settings_default_to_local_only() {
        let s = AiSettings::default();
        assert!(s.local_tagging_enabled);
        assert!(!s.cloud_tagging_enabled);
        assert!(!s.cloud_semantic_search_enabled);
        assert!(s.cloud_provider.is_none());
    }
}
