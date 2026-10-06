//! Conversions between this crate's domain model and the real TPT AV
//! foundation types (`tpt-av-asset-utils::MediaInfo`, kinetix/visual
//! frame metadata). Available with `--features tpt`.
//!
//! The mapping rules:
//! - `MediaType::Audio/Video` map 1:1; av-asset's `Image` folds into our
//!   `Unknown` (out of MVP scope, still indexed).
//! - `VideoInfo.codec` is checked against the open-codec allowlist in
//!   `-core`; anything else marks `TechnicalMetadata.supported = false`
//!   (indexed by path/fingerprint, flagged `unsupported` — never a crash).

#[cfg(not(feature = "tpt"))]
use crate::MediaType;
#[cfg(feature = "tpt")]
use crate::{Asset, MediaType, TechnicalMetadata};

/// Map an av-asset media-type into ours (pure, no `tpt` feature needed
/// for the enum shape — but kept here so the rule lives in one place).
pub fn map_media_type_name(name: &str) -> MediaType {
    match name {
        "audio" => MediaType::Audio,
        "video" => MediaType::Video,
        _ => MediaType::Unknown,
    }
}

#[cfg(feature = "tpt")]
pub mod wired {
    use super::*;
    use tpt_app_media_asset_intelligence_core::is_supported_codec;

    /// Convert a real `tpt-av-asset` `MediaInfo` into our `TechnicalMetadata`.
    pub fn technical_from_media_info(info: &tpt_av_asset_utils::MediaInfo) -> TechnicalMetadata {
        use tpt_av_asset_utils::MediaType as AvMediaType;
        match info.media_type {
            AvMediaType::Video => {
                let v = info.video.as_ref();
                let codec = v.map(|v| v.codec.clone()).unwrap_or_default();
                TechnicalMetadata {
                    codec: codec.clone(),
                    container: String::new(),
                    supported: is_supported_codec(&codec),
                    width: v.map(|v| v.width),
                    height: v.map(|v| v.height),
                    duration_secs: info.duration_secs,
                }
            }
            AvMediaType::Audio => {
                let a = info.audio.as_ref();
                let codec = a.map(|a| a.codec.clone()).unwrap_or_default();
                TechnicalMetadata {
                    codec: codec.clone(),
                    container: String::new(),
                    supported: is_supported_codec(&codec),
                    width: None,
                    height: None,
                    duration_secs: info.duration_secs,
                }
            }
            AvMediaType::Image => TechnicalMetadata {
                codec: "image".to_string(),
                container: String::new(),
                supported: false,
                width: None,
                height: None,
                duration_secs: info.duration_secs,
            },
        }
    }

    /// Map an av-asset `MediaInfo` media-type into our `Asset.media_type`.
    pub fn media_type_from(info: &tpt_av_asset_utils::MediaInfo) -> MediaType {
        use tpt_av_asset_utils::MediaType as AvMediaType;
        match info.media_type {
            AvMediaType::Audio => MediaType::Audio,
            AvMediaType::Video => MediaType::Video,
            AvMediaType::Image => MediaType::Unknown,
        }
    }

    /// Build the deterministic asset UUID from an av-asset `AssetId`
    /// (path-hash/mtime/size triple → UUID v5, stable for the same file
    /// identity).
    pub fn asset_uuid_from_av_id(id: tpt_av_asset_utils::AssetId) -> uuid::Uuid {
        uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            format!(
                "tpt-av-asset:{}:{}:{}",
                id.path_hash(),
                id.mtime_ms(),
                id.size()
            )
            .as_bytes(),
        )
    }

    /// Lightweight check that the kinetix + visual frame types link:
    /// returns the byte length of a visual RGBA frame descriptor.
    pub fn visual_frame_probe(width: u32, height: u32) -> usize {
        let frame = tpt_av_visual_utils::frame::VideoFrame::rgba(width, height, 0);
        frame.data.len()
    }

    #[allow(dead_code)]
    fn _use_asset(_: &Asset) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_names_map() {
        assert_eq!(map_media_type_name("audio"), MediaType::Audio);
        assert_eq!(map_media_type_name("video"), MediaType::Video);
        assert_eq!(map_media_type_name("image"), MediaType::Unknown);
        assert_eq!(map_media_type_name("xyz"), MediaType::Unknown);
    }
}
