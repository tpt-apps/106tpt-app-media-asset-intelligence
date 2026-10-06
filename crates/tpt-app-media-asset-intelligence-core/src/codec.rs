//! Open-codec allowlist (see todo.md Codec Scope decision).
//!
//! Video: AV1, VP9, VP8, Theora, FFV1.
//! Audio: Opus, Vorbis, FLAC, PCM/WAV.
//! Containers: Matroska/WebM, Ogg, WAV, FLAC (MP4/ISOBMFF only where it
//! carries an open codec, e.g. AV1).

/// Supported video codec identifiers (lowercase canonical names).
pub const SUPPORTED_VIDEO_CODECS: &[&str] = &["av1", "vp9", "vp8", "theora", "ffv1"];

/// Supported audio codec identifiers (lowercase canonical names).
pub const SUPPORTED_AUDIO_CODECS: &[&str] = &["opus", "vorbis", "flac", "pcm", "wav"];

/// Supported container identifiers (lowercase canonical names).
pub const SUPPORTED_CONTAINERS: &[&str] = &["matroska", "webm", "ogg", "wav", "flac", "mp4-av1"];

/// Returns true when `codec` (case-insensitive) is in the MVP allowlist.
pub fn is_supported_codec(codec: &str) -> bool {
    let lower = codec.to_ascii_lowercase();
    SUPPORTED_VIDEO_CODECS.contains(&lower.as_str())
        || SUPPORTED_AUDIO_CODECS.contains(&lower.as_str())
}

/// File extensions that *may* carry an allowlisted codec. Unknown
/// extensions are still indexed (as `unsupported`), never skipped.
pub fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mkv" | "webm" | "ogv" | "ogg" | "oga" | "opus" | "flac" | "wav" | "mp4"
    )
}

/// Human-readable allowlist summary for docs/CLI output.
pub fn allowlist_summary() -> &'static str {
    "video: AV1, VP9, VP8, Theora, FFV1; audio: Opus, Vorbis, FLAC, PCM/WAV; \
     containers: Matroska/WebM, Ogg, WAV, FLAC (MP4 only with AV1)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_open_codecs() {
        for c in [
            "av1", "vp9", "vp8", "theora", "ffv1", "opus", "vorbis", "flac", "pcm",
        ] {
            assert!(is_supported_codec(c), "{c} should be supported");
        }
    }

    #[test]
    fn allowlist_rejects_patent_encumbered_codecs() {
        for c in ["h264", "avc", "aac", "prores", "hevc", "h265"] {
            assert!(!is_supported_codec(c), "{c} must be out of scope");
        }
    }

    #[test]
    fn codec_check_is_case_insensitive() {
        assert!(is_supported_codec("AV1"));
        assert!(!is_supported_codec("H264"));
    }
}
