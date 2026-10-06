//! Frame-fed scene detection (`--features tpt`): decode RGBA frames via
//! `VideoSource`, score consecutive luma differences, split on threshold.
//! Deterministic for identical bytes.

use super::split_scenes;
use std::path::Path;
use tpt_app_media_asset_intelligence_model::{AssetId, Scene};

/// Mean absolute luma difference between two RGBA frames.
pub fn frame_difference(a: &[u8], b: &[u8]) -> f32 {
    let n = a.len().min(b.len()) / 4;
    if n == 0 {
        return 0.0;
    }
    let mut acc = 0u64;
    for i in 0..n {
        let la = (a[i * 4] as u32 * 30 + a[i * 4 + 1] as u32 * 59 + a[i * 4 + 2] as u32 * 11) / 100;
        let lb = (b[i * 4] as u32 * 30 + b[i * 4 + 1] as u32 * 59 + b[i * 4 + 2] as u32 * 11) / 100;
        acc += la.abs_diff(lb) as u64;
    }
    acc as f32 / (n as f32 * 255.0)
}

/// Detect scenes in `path` by decoding up to `max_frames` frames.
/// Returns `None` when the file has no decodable video.
pub fn detect_scenes(
    asset_id: AssetId,
    path: &Path,
    threshold: f32,
    max_frames: usize,
) -> Option<Vec<Scene>> {
    let mut src = tpt_av_asset_cache::video::open_video(path).ok()?;
    let info = src.info().clone();
    let fps = info.frame_rate.max(1.0);
    let mut scores = vec![0.0f32];
    let mut prev: Option<Vec<u8>> = None;
    for _ in 0..max_frames.max(1) {
        let frame = src.next_frame().ok()??;
        if let Some(p) = prev.replace(frame.data.clone()) {
            scores.push(frame_difference(&p, &frame.data));
        }
    }
    if scores.len() < 2 && prev.is_none() {
        return None;
    }
    Some(split_scenes(asset_id, &scores, threshold, fps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_frames_score_zero() {
        let a = vec![100u8; 16 * 4];
        assert_eq!(frame_difference(&a, &a), 0.0);
    }

    #[test]
    fn black_white_scores_one() {
        let a = vec![0u8; 4 * 4];
        let b = vec![255u8; 4 * 4];
        let d = frame_difference(&a, &b);
        assert!((d - 1.0).abs() < 0.01, "{d}");
    }
}
