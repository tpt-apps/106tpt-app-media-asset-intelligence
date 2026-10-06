//! Ingestion pipeline for TPT Media Asset Intelligence (spec §7).
//!
//! The pipeline mirrors the two-pass strategy used across the TPT media stack:
//! a fast Pass 1 (fingerprint + technical metadata + metadata search index) that
//! makes an archive searchable within seconds of a scan starting, followed by a
//! slower Pass 2 (derivatives, duplicates, scenes, tagging) that progressively
//! enriches the index (spec §7.1).
//!
//! ```text
//! Filesystem/watch-folder scan
//!   → Fingerprint
//!   → Technical metadata extraction        (Pass 1: fast, always runs)
//!   → Search index (metadata/filename)
//!   → Derivative generation                (Pass 2)
//!   → Duplicate detection → Scene detection → Tagging
//!   → Search index update
//! ```
//!
//! This crate builds directly on the `tpt-av-asset` foundation (spec §5.1): the
//! [`foundation`] module maps each foundation capability to its product
//! responsibility here. Pass 1 is implemented: scanning ([`scanner`]),
//! fingerprinting (in `-core`), technical metadata extraction ([`metadata`])
//! and the in-memory [`ArchiveIndex`] the search layer
//! reads.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod foundation;
pub mod index;
pub mod metadata;
pub mod pipeline_stage;
pub mod scan;
pub mod scanner;

pub use index::ArchiveIndex;
pub use metadata::{build_asset, media_info_to_technical, probe, ProbeError, Probed};
pub use pipeline_stage::{PipelinePass, PipelineStage};
pub use scan::{ScanOutcome, ScannedFile};
pub use scanner::{scan_roots, validate_root, ScanConfigError, ScanIssue, ScanRun, ScannedEntry};

use std::path::Path;

/// Classifies a file's broad
/// [`MediaType`](tpt_app_media_asset_intelligence_core::MediaType) from its
/// extension.
///
/// Pass 1 needs a cheap, deterministic classification before any decoder runs;
/// it is refined by real probing during metadata extraction (spec §7).
pub fn classify_by_extension(path: &Path) -> tpt_app_media_asset_intelligence_core::MediaType {
    use tpt_app_media_asset_intelligence_core::MediaType;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mov" | "mp4" | "m4v" | "mkv" | "avi" | "webm" | "mts" | "m2ts" | "mxf" | "wmv" | "flv"
        | "mpg" | "mpeg" | "dv" => MediaType::Video,
        "wav" | "aiff" | "aif" | "mp3" | "aac" | "m4a" | "flac" | "ogg" | "opus" | "wma" => {
            MediaType::Audio
        }
        "jpg" | "jpeg" | "png" | "tif" | "tiff" | "bmp" | "gif" | "webp" | "dpx" | "exr" => {
            MediaType::Image
        }
        _ => MediaType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn video_extensions_classify_as_video() {
        for name in ["interview.mov", "CLIP01.MP4", "master.mkv", "archive.mxf"] {
            assert_eq!(
                classify_by_extension(&PathBuf::from(name)),
                tpt_app_media_asset_intelligence_core::MediaType::Video,
                "{name}"
            );
        }
    }

    #[test]
    fn audio_image_and_unknown_extensions_classify() {
        assert_eq!(
            classify_by_extension(&PathBuf::from("room_tone.wav")),
            tpt_app_media_asset_intelligence_core::MediaType::Audio
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("poster.JPG")),
            tpt_app_media_asset_intelligence_core::MediaType::Image
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("notes.txt")),
            tpt_app_media_asset_intelligence_core::MediaType::Other
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("no_extension")),
            tpt_app_media_asset_intelligence_core::MediaType::Other
        );
    }

    #[test]
    fn classification_is_deterministic() {
        let p = PathBuf::from("a/b/c.mov");
        assert_eq!(classify_by_extension(&p), classify_by_extension(&p));
    }
}
