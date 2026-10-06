//! Extension-sniffing probe: maps a scanned file to codec/container/
//! supported metadata (spec §7). The `tpt` feature replaces the sniffed
//! codec with real `probe_media_info` output (see `tpt_engine`).

use std::path::Path;
use tpt_app_media_asset_intelligence_core::{is_supported_codec, is_supported_extension};
use tpt_app_media_asset_intelligence_model::{MediaType, TechnicalMetadata};

/// Result of probing one file: media type + technical metadata.
#[derive(Debug, Clone)]
pub struct ProbedFile {
    pub media_type: MediaType,
    pub technical: TechnicalMetadata,
}

/// Probe by extension/container sniffing. Unknown extensions are indexed
/// as `Unknown`/`unsupported` — never an error (spec §17).
pub fn probe_file(path: &Path) -> ProbedFile {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (media_type, codec, container) = match ext.as_str() {
        "mkv" | "webm" | "ogv" | "mp4" => (MediaType::Video, ext.clone(), ext.clone()),
        "ogg" | "oga" | "opus" | "flac" | "wav" => (MediaType::Audio, ext.clone(), ext.clone()),
        _ => (MediaType::Unknown, ext.clone(), ext.clone()),
    };
    let supported = is_supported_codec(&codec)
        || (is_supported_extension(&ext)
            && matches!(
                ext.as_str(),
                "mkv" | "webm" | "ogg" | "wav" | "flac" | "opus"
            ));
    ProbedFile {
        media_type,
        technical: TechnicalMetadata {
            codec,
            container,
            supported,
            width: None,
            height: None,
            duration_secs: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn open_containers_probe_supported() {
        let p = probe_file(&PathBuf::from("clip.mkv"));
        assert_eq!(p.media_type, MediaType::Video);
    }

    #[test]
    fn unknown_extension_is_unsupported_not_error() {
        let p = probe_file(&PathBuf::from("clip.prores.mov"));
        assert!(!p.technical.supported);
        assert_eq!(p.media_type, MediaType::Unknown);
    }

    #[test]
    fn audio_extensions_probe_audio() {
        let p = probe_file(&PathBuf::from("take.opus"));
        assert_eq!(p.media_type, MediaType::Audio);
    }

    #[test]
    fn empty_extension_is_unknown() {
        let p = probe_file(&PathBuf::from("README"));
        assert_eq!(p.media_type, MediaType::Unknown);
        assert!(!p.technical.supported);
    }
}
