//! Generated derivatives: thumbnails, proxies and waveforms (spec §6.3, §16).
//!
//! Derivative files live in a separate cache directory, never inside the
//! archive and never inside the database (spec §16); the index records only
//! their paths.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tpt_app_media_asset_intelligence_core::{AssetId, DerivativeId};

/// The kind of derivative generated for an asset (spec §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DerivativeKind {
    /// A small still image used by grid/list views (spec §13.1).
    Thumbnail,
    /// An editing-friendly low-resolution copy of a video asset (spec §1.1).
    Proxy,
    /// An audio amplitude rendering used for audio browsing and audio-event tagging (spec §11).
    Waveform,
}

impl DerivativeKind {
    /// The kebab-case token used in paths, filters and JSON output.
    pub const fn as_str(self) -> &'static str {
        match self {
            DerivativeKind::Thumbnail => "thumbnail",
            DerivativeKind::Proxy => "proxy",
            DerivativeKind::Waveform => "waveform",
        }
    }
}

/// A generated derivative and where it was cached (spec §6.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Derivative {
    /// Stable identifier within the archive.
    pub id: DerivativeId,
    /// The source asset this derivative was generated from.
    pub asset: AssetId,
    /// Which kind of derivative this is.
    pub kind: DerivativeKind,
    /// Path to the cached file, relative to the derivative cache root (spec §16).
    pub path: PathBuf,
    /// Engine version that generated the file (spec §6.3); drives cache invalidation.
    pub generated_with_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_app_media_asset_intelligence_core::ENGINE_VERSION;

    #[test]
    fn kind_tokens_are_kebab_case() {
        assert_eq!(DerivativeKind::Thumbnail.as_str(), "thumbnail");
        assert_eq!(DerivativeKind::Proxy.as_str(), "proxy");
        assert_eq!(DerivativeKind::Waveform.as_str(), "waveform");
        assert_eq!(
            serde_json::to_string(&DerivativeKind::Waveform).unwrap(),
            "\"waveform\""
        );
    }

    #[test]
    fn derivative_records_generating_version() {
        let d = Derivative {
            id: DerivativeId::new(1),
            asset: AssetId::new(7),
            kind: DerivativeKind::Thumbnail,
            path: PathBuf::from("thumbnails/asset-7.jpg"),
            generated_with_version: ENGINE_VERSION.to_string(),
        };
        assert_eq!(d.generated_with_version, ENGINE_VERSION);
        assert_eq!(d.path, PathBuf::from("thumbnails/asset-7.jpg"));
    }
}
