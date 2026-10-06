//! Indexed assets and their technical metadata (spec §6.2, §7).
//!
//! Technical metadata extraction is deterministic: the same input bytes and the
//! same engine version produce the same [`TechnicalMetadata`] (spec §3.2), so
//! the type is plain data with no behaviour beyond normalization.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use tpt_app_media_asset_intelligence_core::{ArchiveId, AssetFingerprint, AssetId, MediaType};

/// A single indexed media file (spec §6.2).
///
/// The index stores paths and fingerprints, never raw media (spec §16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// Stable identifier within the archive.
    pub id: AssetId,
    /// The archive this asset belongs to.
    pub archive: ArchiveId,
    /// Archive-normalized path (see the crate-level path normalization helper).
    pub path: PathBuf,
    /// SHA-256 content fingerprint; equal fingerprints mean byte-identical files (spec §8).
    pub fingerprint: AssetFingerprint,
    /// File size in bytes at last index time.
    pub size_bytes: u64,
    /// File modification time at last index time; drives incremental re-index decisions (spec §18).
    pub modified_time: SystemTime,
    /// Broad media classification.
    pub media_type: MediaType,
    /// Deterministic technical metadata extracted during Pass 1 (spec §7).
    pub technical_metadata: TechnicalMetadata,
}

/// Deterministic technical metadata for an asset (spec §3.2, §6.2, §7).
///
/// Fields the foundation decoders could not determine stay `None`; a decoder
/// failure is not the same as "unknown value" and is recorded by the archive
/// health pipeline (spec §12) rather than by a fake value here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnicalMetadata {
    /// Container format (e.g. "mov", "mkv", "wav"), lowercase, if determined.
    pub container: Option<String>,
    /// Primary video codec (e.g. "prores", "h264"), lowercase, if determined.
    pub video_codec: Option<String>,
    /// Primary audio codec (e.g. "pcm-s16le", "aac"), lowercase, if determined.
    pub audio_codec: Option<String>,
    /// Encoded width in pixels, if video.
    pub width: Option<u32>,
    /// Encoded height in pixels, if video.
    pub height: Option<u32>,
    /// Frame rate as a rational (numerator, denominator), if video.
    pub frame_rate: Option<(u32, u32)>,
    /// Duration in milliseconds, if determined.
    pub duration_ms: Option<u64>,
    /// Per-stream layout, in stream order (spec §7 "stream layout").
    pub streams: Vec<StreamInfo>,
}

impl TechnicalMetadata {
    /// Returns `true` if every optional field is unset.
    ///
    /// Ingest treats this as "extraction produced nothing usable" and surfaces
    /// the file to archive health as potentially corrupt/unreadable (spec §12).
    pub fn is_empty(&self) -> bool {
        self.container.is_none()
            && self.video_codec.is_none()
            && self.audio_codec.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.frame_rate.is_none()
            && self.duration_ms.is_none()
            && self.streams.is_empty()
    }
}

/// One stream within a container, in stream order (spec §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Zero-based position of the stream within the container.
    pub index: u32,
    /// Stream kind.
    pub kind: MediaType,
    /// Codec identifier, lowercase, if determined.
    pub codec: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint() -> AssetFingerprint {
        AssetFingerprint::from_hex(&"ab".repeat(32)).unwrap()
    }

    #[test]
    fn default_technical_metadata_is_empty_and_detected_as_such() {
        let meta = TechnicalMetadata::default();
        assert!(meta.is_empty());
        assert_eq!(meta, TechnicalMetadata::default());
    }

    #[test]
    fn populated_metadata_is_not_empty() {
        let meta = TechnicalMetadata {
            container: Some("mov".to_string()),
            ..TechnicalMetadata::default()
        };
        assert!(!meta.is_empty());
    }

    #[test]
    fn asset_serializes_fingerprint_as_hex() {
        let asset = Asset {
            id: AssetId::new(1),
            archive: ArchiveId::new(1),
            path: PathBuf::from("projects/interview.mov"),
            fingerprint: fingerprint(),
            size_bytes: 4_100_000_000,
            modified_time: SystemTime::UNIX_EPOCH,
            media_type: MediaType::Video,
            technical_metadata: TechnicalMetadata::default(),
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(
            json.contains("\"fingerprint\":\"abababab"),
            "hex string, got: {json}"
        );
        assert!(serde_json::from_str::<Asset>(&json).unwrap() == asset);
    }

    #[test]
    fn stream_info_keeps_container_order() {
        let meta = TechnicalMetadata {
            streams: vec![
                StreamInfo {
                    index: 0,
                    kind: MediaType::Video,
                    codec: Some("h264".to_string()),
                },
                StreamInfo {
                    index: 1,
                    kind: MediaType::Audio,
                    codec: Some("aac".to_string()),
                },
            ],
            ..TechnicalMetadata::default()
        };
        assert_eq!(meta.streams[0].kind, MediaType::Video);
        assert_eq!(meta.streams[1].kind, MediaType::Audio);
    }
}
