//! Scene segments for indexed video assets (spec §6.6, §9).
//!
//! Scene detection in the MVP is deterministic and technical
//! (frame-difference/perceptual-hash), not semantic: "this scene is an
//! interview" is a tagging concern (spec §11), not a scene-detection result.

use core::time::Duration;
use serde::{Deserialize, Serialize};
use tpt_app_media_asset_intelligence_core::{AssetId, SceneId};

/// How a scene boundary was detected (spec §6.6, §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SceneDetectionMethod {
    /// Frame-difference threshold over decoded frames (spec §9).
    FrameDifference,
    /// Perceptual-hash distance over sampled frames (spec §9).
    PerceptualHash,
}

impl SceneDetectionMethod {
    /// The kebab-case token used in JSON output.
    pub const fn as_str(self) -> &'static str {
        match self {
            SceneDetectionMethod::FrameDifference => "frame-difference",
            SceneDetectionMethod::PerceptualHash => "perceptual-hash",
        }
    }
}

/// A detected scene: a contiguous time range within one asset (spec §6.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    /// Stable identifier within the archive.
    pub id: SceneId,
    /// The asset the scene belongs to.
    pub asset: AssetId,
    /// Scene start offset from the start of the asset.
    pub start: Duration,
    /// Scene end offset from the start of the asset; strictly after `start`.
    pub end: Duration,
    /// How this scene was detected.
    pub detection_method: SceneDetectionMethod,
}

impl Scene {
    /// Creates a scene, enforcing `start < end`.
    ///
    /// # Errors
    ///
    /// Returns
    /// [`CoreError::ConstraintViolated`](tpt_app_media_asset_intelligence_core::CoreError::ConstraintViolated)
    /// if `end <= start`.
    pub fn new(
        id: u64,
        asset: AssetId,
        start: Duration,
        end: Duration,
        detection_method: SceneDetectionMethod,
    ) -> Result<Self, tpt_app_media_asset_intelligence_core::CoreError> {
        use tpt_app_media_asset_intelligence_core::CoreError;
        if end <= start {
            return Err(CoreError::ConstraintViolated(format!(
                "scene end ({end:?}) must be after start ({start:?})"
            )));
        }
        Ok(Self {
            id: SceneId::new(id),
            asset,
            start,
            end,
            detection_method,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_require_positive_length() {
        let ok = Scene::new(
            1,
            AssetId::new(1),
            Duration::from_secs(92),
            Duration::from_secs(258),
            SceneDetectionMethod::PerceptualHash,
        )
        .unwrap();
        assert_eq!(ok.detection_method.as_str(), "perceptual-hash");
        assert_eq!(ok.end - ok.start, Duration::from_secs(166));

        let zero = Scene::new(
            2,
            AssetId::new(1),
            Duration::from_secs(5),
            Duration::from_secs(5),
            SceneDetectionMethod::FrameDifference,
        );
        assert!(zero.is_err(), "zero-length scenes are not scenes");
    }

    #[test]
    fn serializes_with_kebab_case_method() {
        let scene = Scene::new(
            1,
            AssetId::new(3),
            Duration::ZERO,
            Duration::from_secs(10),
            SceneDetectionMethod::FrameDifference,
        )
        .unwrap();
        let json = serde_json::to_string(&scene).unwrap();
        assert!(
            json.contains("\"detection_method\":\"frame-difference\""),
            "got: {json}"
        );
        assert!(serde_json::from_str::<Scene>(&json).unwrap() == scene);
    }
}
