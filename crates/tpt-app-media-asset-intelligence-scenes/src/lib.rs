<<<<<<< HEAD
//! Deterministic scene-change detection (spec §9): score-threshold
//! splitting over frame differences, plus a frame-fed entry point that
//! decodes real RGBA frames via `VideoSource` (`--features tpt`).

#[cfg(feature = "tpt")]
pub mod frames;

use tpt_app_media_asset_intelligence_model::{AssetId, Scene};

/// Split `frame_scores` (one difference score per frame) into scenes:
/// a new scene starts wherever `score >= threshold`. Pure function,
/// deterministic for identical inputs.
pub fn split_scenes(
    asset_id: AssetId,
    frame_scores: &[f32],
    threshold: f32,
    fps: f64,
) -> Vec<Scene> {
    let mut scenes = Vec::new();
    let mut index = 0u32;
    let mut start_frame = 0usize;
    for (i, score) in frame_scores.iter().enumerate() {
        if i > 0 && *score >= threshold {
            scenes.push(Scene {
                asset_id,
                index,
                start_secs: start_frame as f64 / fps,
                end_secs: Some(i as f64 / fps),
            });
            index += 1;
            start_frame = i;
        }
    }
    scenes.push(Scene {
        asset_id,
        index,
        start_secs: start_frame as f64 / fps,
        end_secs: None,
    });
    scenes
=======
//! Deterministic scene-change detection (spec §9).
//!
//! Baseline MVP detection is technical and statistical — frame-difference and
//! perceptual-hash techniques via `tpt-visual`, in the spirit of TPT Media
//! Forensics' scene-change analysis. It is explicitly **not** a semantic
//! understanding of content: "this scene is an interview" is a tagging concern
//! (spec §11), not a scene-detection result.
//!
//! Determinism contract (spec §3.2): the same input bytes and the same engine
//! version produce the same [`tpt_app_media_asset_intelligence_model::Scene`]
//! boundaries. The detector configuration below is part of that contract —
//! changing a default is an engine-output change and must bump
//! [`tpt_app_media_asset_intelligence_core::ENGINE_VERSION`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// Configuration of the deterministic scene detector (spec §9).
///
/// Defaults are the values the first release validated against the golden
/// fixture archives (spec §19.2). They are serialized into index snapshots so a
/// future version can tell which detector produced stored boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneDetectorConfig {
    /// Distance threshold (in perceptual-hash units, 0–64) above which a frame
    /// pair counts as a scene change.
    pub hash_distance_threshold: u8,
    /// Minimum scene length in milliseconds; shorter cuts are merged into the
    /// previous scene to avoid strobing output on strobe/flash footage.
    pub min_scene_length_ms: u64,
    /// Sampling stride: analyse every Nth decoded frame.
    pub sample_every_nth_frame: u32,
}

impl Default for SceneDetectorConfig {
    fn default() -> Self {
        Self {
            hash_distance_threshold: 24,
            min_scene_length_ms: 1_000,
            sample_every_nth_frame: 1,
        }
    }
}

/// Re-exports of the consumed `tpt-visual` foundation surface (spec §5.2).
///
/// Scene-change detection compares decoded frames; these are the frame,
/// pixel-format and timing primitives the Phase 1 detector builds on. The
/// foundation's GPU compositor stack is deliberately not consumed — this
/// product's detection is deterministic CPU-side analysis (spec §3.2, §9).
pub mod foundation {
    pub use tpt_av_visual_utils::{FrameRate, PixelFormat, Resolution, Timecode, VideoFrame};
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;
<<<<<<< HEAD
    use uuid::Uuid;

    #[test]
    fn flat_scores_yield_single_scene() {
        let scenes = split_scenes(Uuid::new_v4(), &[0.1, 0.1, 0.1], 0.5, 25.0);
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].index, 0);
    }

    #[test]
    fn spike_starts_new_scene() {
        let scenes = split_scenes(Uuid::new_v4(), &[0.1, 0.9, 0.1], 0.5, 10.0);
        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[1].start_secs, 0.1);
    }

    #[test]
    fn empty_scores_yield_single_open_scene() {
        let scenes = split_scenes(Uuid::new_v4(), &[], 0.5, 25.0);
        assert_eq!(scenes.len(), 1);
        assert!(scenes[0].end_secs.is_none());
    }

    #[test]
    fn detection_is_deterministic() {
        let id = Uuid::new_v4();
        let scores = vec![0.1, 0.8, 0.2, 0.9];
        assert_eq!(
            split_scenes(id, &scores, 0.5, 25.0).len(),
            split_scenes(id, &scores, 0.5, 25.0).len()
        );
    }
=======

    #[test]
    fn defaults_are_stable() {
        let c = SceneDetectorConfig::default();
        assert_eq!(c.hash_distance_threshold, 24);
        assert_eq!(c.min_scene_length_ms, 1_000);
        assert_eq!(c.sample_every_nth_frame, 1);
    }

    #[test]
    fn config_round_trips_through_serde() {
        let c = SceneDetectorConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(
            serde_json::from_str::<SceneDetectorConfig>(&json).unwrap(),
            c
        );
    }

    /// Compile-time proof that the tpt-visual foundation resolves (Phase 0
    /// reachability check).
    #[test]
    fn tpt_visual_foundation_is_reachable() {
        use foundation::{FrameRate, PixelFormat, Resolution, Timecode};
        let rate = FrameRate::new(25, 1).unwrap();
        let _ = Resolution::new(1920, 1080).unwrap();
        let _ = PixelFormat::Yuv420p;
        let _ = Timecode::from_frames(50, rate);
    }
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}
