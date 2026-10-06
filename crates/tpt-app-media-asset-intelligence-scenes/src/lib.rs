//! Deterministic scene-change detection scaffold (spec §9).
//! Full frame-difference/pHash analysis plugs in via tpt-visual; this
//! crate owns the deterministic boundary-splitting contract.

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
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
