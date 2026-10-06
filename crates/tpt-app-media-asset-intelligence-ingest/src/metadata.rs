//! Technical metadata extraction on the foundation (spec §7, §3.2, §5.2).
//!
//! Probing delegates to `tpt-av-asset-pipeline::probe_media_info`, which uses
//! the kinetix video stack and the cadence audio stack — the same deterministic
//! technical analysis used across the TPT media products (spec §3.2, §5.2).
//! This module maps the foundation's `MediaInfo` onto the product's
//! [`TechnicalMetadata`]
//! and assembles [`Asset`]
//! records: the "domain model on top of tpt-av-asset" of spec §26 step 3.
//!
//! Identity note: the foundation's `tpt_av_asset_utils::AssetId` (path+mtime+size
//! triple) is the *content identity* the derivative caches key on. The
//! product's [`AssetId`] is the
//! index primary key assigned by the archive index. Both exist; they answer
//! different questions and are never conflated.

use std::path::Path;

use tpt_app_media_asset_intelligence_core::{AssetFingerprint, AssetId, MediaType};
use tpt_app_media_asset_intelligence_model::{Asset, StreamInfo, TechnicalMetadata};
use tpt_av_asset_utils::MediaInfo;

/// Why a file could not be probed (spec §12 corrupt/unreadable; §17 safe handling).
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// No decoder in the foundation recognizes the file.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(std::path::PathBuf),
    /// The file could not be read or its metadata could not be determined.
    #[error("probe failed for {path}: {reason}")]
    Failed {
        /// The offending path.
        path: std::path::PathBuf,
        /// Foundation error text.
        reason: String,
    },
}

impl ProbeError {
    /// `true` when the foundation has no decoder for this file at all.
    ///
    /// Genuinely unknown formats (documents, project files) are normal archive
    /// content, not corruption; distinguishable from read/decode failures so
    /// archive health (spec §12) can classify correctly.
    pub fn is_unsupported_format(&self) -> bool {
        matches!(self, ProbeError::UnsupportedFormat(_))
    }
}

/// The Pass 1 probe result: refined media type plus technical metadata (spec §7).
///
/// The cheap extension classification ([`crate::classify_by_extension`]) is
/// refined by the decoders: a file named `.dat` that probes as video is video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    /// Refined broad media type from the decoders.
    pub media_type: MediaType,
    /// Deterministic technical metadata (spec §3.2).
    pub technical_metadata: TechnicalMetadata,
}

/// Probes a file's technical metadata via the foundation (spec §7, §26 step 5).
///
/// # Errors
///
/// Returns [`ProbeError::UnsupportedFormat`] when no decoder recognizes the
/// file, or [`ProbeError::Failed`] for read/decode problems. Neither is fatal
/// to an indexing run (spec §17).
pub fn probe(path: &Path) -> Result<Probed, ProbeError> {
    let info = tpt_av_asset_pipeline::probe_media_info(path).map_err(|e| match e {
        tpt_av_asset_utils::AssetError::UnsupportedFormat(p) => ProbeError::UnsupportedFormat(p),
        other => ProbeError::Failed {
            path: path.to_path_buf(),
            reason: other.to_string(),
        },
    })?;
    Ok(Probed {
        media_type: media_type_from(&info),
        technical_metadata: media_info_to_technical(path, &info),
    })
}

/// Maps the foundation's `MediaType` onto the product's.
fn media_type_from(info: &MediaInfo) -> MediaType {
    match info.media_type {
        tpt_av_asset_utils::MediaType::Video => MediaType::Video,
        tpt_av_asset_utils::MediaType::Audio => MediaType::Audio,
        tpt_av_asset_utils::MediaType::Image => MediaType::Image,
    }
}

/// Maps the foundation's `MediaInfo` onto the product's `TechnicalMetadata`.
///
/// Pure and deterministic (spec §3.2): the same `MediaInfo` always maps to the
/// same value.
///
/// Interim limitation, tracked for Phase 1: the foundation's probe does not yet
/// surface the *container* name, so it is derived from the file extension when
/// recognized and left `None` otherwise. Codec/resolution/frame-rate/duration
/// come from the decoders themselves.
pub fn media_info_to_technical(path: &Path, info: &MediaInfo) -> TechnicalMetadata {
    let mut meta = TechnicalMetadata {
        container: container_from_extension(path),
        ..TechnicalMetadata::default()
    };

    if let Some(video) = &info.video {
        meta.video_codec = Some(video.codec.to_ascii_lowercase());
        meta.width = Some(video.width);
        meta.height = Some(video.height);
        meta.frame_rate = rational_frame_rate(video.frame_rate);
        meta.streams.push(StreamInfo {
            index: meta.streams.len() as u32,
            kind: MediaType::Video,
            codec: Some(video.codec.to_ascii_lowercase()),
        });
    }
    if let Some(audio) = &info.audio {
        meta.audio_codec = Some(audio.codec.to_ascii_lowercase());
        meta.streams.push(StreamInfo {
            index: meta.streams.len() as u32,
            kind: MediaType::Audio,
            codec: Some(audio.codec.to_ascii_lowercase()),
        });
    }
    if meta.duration_ms.is_none() {
        meta.duration_ms = info
            .duration_secs
            .filter(|s| s.is_finite() && *s >= 0.0)
            .map(|s| (s * 1000.0).round() as u64);
    }
    meta
}

/// Converts a floating-point frame rate into the rational representation,
/// scaled to milli-fps (29.97 → 29970/1000) for deterministic storage (spec §3.2).
fn rational_frame_rate(rate: f64) -> Option<(u32, u32)> {
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let milli = (rate * 1000.0).round() as u64;
    u32::try_from(milli).ok().map(|num| (num, 1000))
}

/// Recognized container tokens by extension; `None` for unknown extensions.
fn container_from_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let container = match ext.as_str() {
        "mov" | "m4v" => "mov",
        "mp4" | "m4a" => "mp4",
        "mkv" => "mkv",
        "avi" => "avi",
        "webm" => "webm",
        "wav" => "wav",
        "aiff" | "aif" => "aiff",
        "flac" => "flac",
        "mp3" => "mp3",
        "ogg" | "opus" => "ogg",
        _ => return None,
    };
    Some(container.to_string())
}

/// Assembles a domain [`Asset`] from the Pass 1 stage outputs (spec §6.2, §26
/// step 3): scan record, content fingerprint, refined media type and probed
/// technical metadata.
pub fn build_asset(
    id: AssetId,
    archive: tpt_app_media_asset_intelligence_core::ArchiveId,
    scanned: &crate::scan::ScannedFile,
    fingerprint: AssetFingerprint,
    probed: &Probed,
) -> Asset {
    Asset {
        id,
        archive,
        path: tpt_app_media_asset_intelligence_model::normalize_archive_path(&scanned.path),
        fingerprint,
        size_bytes: scanned.size_bytes,
        modified_time: scanned.modified_time,
        media_type: probed.media_type,
        technical_metadata: probed.technical_metadata.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn info(media_type: tpt_av_asset_utils::MediaType) -> MediaInfo {
        MediaInfo::new(
            tpt_av_asset_utils::AssetId::from_parts(1, 2, 3),
            Path::new("/media/clip.mov"),
            media_type,
        )
    }

    #[test]
    fn unsupported_extensions_leave_container_none() {
        let meta = media_info_to_technical(
            &PathBuf::from("/x/notes.txt"),
            &info(tpt_av_asset_utils::MediaType::Image),
        );
        assert_eq!(meta.container, None);
        assert!(meta.is_empty());
    }

    #[test]
    fn known_extensions_map_to_containers() {
        let meta = media_info_to_technical(
            &PathBuf::from("/x/clip.mov"),
            &info(tpt_av_asset_utils::MediaType::Video),
        );
        assert_eq!(meta.container.as_deref(), Some("mov"));
        let meta = media_info_to_technical(
            &PathBuf::from("/x/ROOM.MOV"),
            &info(tpt_av_asset_utils::MediaType::Video),
        );
        assert_eq!(meta.container.as_deref(), Some("mov"), "case-insensitive");
    }

    #[test]
    fn video_info_maps_to_codecs_resolution_and_rational_frame_rate() {
        let mut i = info(tpt_av_asset_utils::MediaType::Video);
        i.duration_secs = Some(3.5);
        i.video = Some(tpt_av_asset_utils::VideoInfo {
            width: 1920,
            height: 1080,
            frame_rate: 29.97,
            codec: "H264".to_string(),
            pixel_format: "yuv420p".to_string(),
            bit_rate: None,
            frame_count: 105,
            duration_secs: 3.5,
        });
        let meta = media_info_to_technical(&PathBuf::from("/x/clip.mov"), &i);
        assert_eq!(
            meta.video_codec.as_deref(),
            Some("h264"),
            "codecs lowercase"
        );
        assert_eq!(meta.width, Some(1920));
        assert_eq!(meta.height, Some(1080));
        assert_eq!(meta.frame_rate, Some((29970, 1000)), "milli-fps rational");
        assert_eq!(meta.duration_ms, Some(3500));
        assert_eq!(meta.streams.len(), 1);
        assert_eq!(meta.streams[0].kind, MediaType::Video);
        assert_eq!(meta.streams[0].index, 0);
    }

    #[test]
    fn audio_streams_are_indexed_after_video() {
        let mut i = info(tpt_av_asset_utils::MediaType::Video);
        i.video = Some(tpt_av_asset_utils::VideoInfo {
            width: 640,
            height: 360,
            frame_rate: 25.0,
            codec: "h264".to_string(),
            pixel_format: "yuv420p".to_string(),
            bit_rate: None,
            frame_count: 250,
            duration_secs: 10.0,
        });
        i.audio = Some(tpt_av_asset_utils::AudioInfo {
            sample_rate: 48_000,
            channels: 2,
            bit_depth: 16,
            codec: "aac".to_string(),
            bit_rate: None,
            duration_secs: 10.0,
        });
        let meta = media_info_to_technical(&PathBuf::from("/x/clip.mov"), &i);
        assert_eq!(meta.audio_codec.as_deref(), Some("aac"));
        assert_eq!(meta.streams.len(), 2);
        assert_eq!(meta.streams[1].kind, MediaType::Audio);
        assert_eq!(meta.streams[1].index, 1);
    }

    #[test]
    fn non_finite_and_negative_durations_are_rejected() {
        let mut i = info(tpt_av_asset_utils::MediaType::Audio);
        i.duration_secs = Some(-1.0);
        let meta = media_info_to_technical(&PathBuf::from("/x/a.wav"), &i);
        assert_eq!(meta.duration_ms, None);

        i.duration_secs = Some(f64::NAN);
        let meta = media_info_to_technical(&PathBuf::from("/x/a.wav"), &i);
        assert_eq!(meta.duration_ms, None);
    }

    #[test]
    fn degenerate_frame_rates_are_rejected() {
        assert_eq!(rational_frame_rate(0.0), None);
        assert_eq!(rational_frame_rate(-25.0), None);
        assert_eq!(rational_frame_rate(f64::NAN), None);
        assert_eq!(rational_frame_rate(25.0), Some((25000, 1000)));
    }
}
