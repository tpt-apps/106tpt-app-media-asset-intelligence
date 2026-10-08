<<<<<<< HEAD
//! Exact-hash duplicate detection; perceptual/audio matching via the
//! real TPT stack under `--features tpt` (spec §8).

#[cfg(feature = "tpt")]
pub mod perceptual;

use std::collections::HashMap;
use tpt_app_media_asset_intelligence_model::{AssetId, DuplicateGroup, MatchKind};
use uuid::Uuid;

/// Group asset ids by fingerprint; singletons are dropped.
pub fn exact_duplicates(fingerprints: &HashMap<AssetId, String>) -> Vec<DuplicateGroup> {
    let mut by_fp: HashMap<&str, Vec<AssetId>> = HashMap::new();
    for (id, fp) in fingerprints {
        by_fp.entry(fp.as_str()).or_default().push(*id);
    }
    by_fp
        .into_values()
        .filter(|ids| ids.len() > 1)
        .map(|asset_ids| DuplicateGroup {
            id: Uuid::new_v4(),
            asset_ids,
            match_kind: MatchKind::ExactHash,
            reviewed: false,
            keeper: None,
        })
        .collect()
=======
//! Duplicate and near-duplicate detection (spec §8).
//!
//! Detection runs in layers, cheapest and most certain first:
//!
//! 1. **Exact** — content-hash equality (deterministic, spec §8).
//! 2. **Perceptual** — frame-level similarity via `tpt-visual` (spec §8); this
//!    crate defines the groups, the detection itself is Phase 1.
//! 3. **Audio fingerprint** — `tpt-cadence`/DSP-based matching for audio-only
//!    or audio-dominant material (spec §8).
//!
//! Results are surfaced as reviewable
//! [`DuplicateGroup`](tpt_app_media_asset_intelligence_model::DuplicateGroup)s
//! with review status and keeper selection. The application **never**
//! automatically deletes a file from a duplicate group; dedupe actions are
//! always explicit and reviewable (spec §3.4, §8).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod report;

pub use report::{DedupeReport, WastedSummary};

use tpt_app_media_asset_intelligence_model::MatchKind;

/// The detection layers, in execution order (spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionLayer {
    /// Content-hash equality; deterministic (spec §8).
    ExactHash,
    /// Perceptual near-duplicate via frame similarity (spec §8).
    Perceptual,
    /// Audio fingerprint matching (spec §8).
    AudioFingerprint,
}

impl DetectionLayer {
    /// The [`MatchKind`] produced by this layer.
    pub const fn match_kind(self) -> MatchKind {
        match self {
            DetectionLayer::ExactHash => MatchKind::ExactHash,
            DetectionLayer::Perceptual => MatchKind::PerceptualNearDuplicate,
            DetectionLayer::AudioFingerprint => MatchKind::AudioFingerprint,
        }
    }

    /// All layers in execution order.
    pub const ALL: [DetectionLayer; 3] = [
        DetectionLayer::ExactHash,
        DetectionLayer::Perceptual,
        DetectionLayer::AudioFingerprint,
    ];
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
<<<<<<< HEAD
    fn exact_duplicates_groups_shared_fingerprints() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let map: HashMap<AssetId, String> =
            [(a, "fp1".into()), (b, "fp1".into()), (c, "fp2".into())].into();
        let groups = exact_duplicates(&map);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].asset_ids.len(), 2);
        assert!(!groups[0].reviewed);
        assert!(groups[0].keeper.is_none());
    }

    #[test]
    fn no_duplicates_yields_no_groups() {
        let map: HashMap<AssetId, String> = [(Uuid::new_v4(), "fp1".into())].into();
        assert!(exact_duplicates(&map).is_empty());
    }

    #[test]
    fn empty_input_yields_no_groups() {
        assert!(exact_duplicates(&HashMap::new()).is_empty());
=======
    fn layers_map_to_match_kinds() {
        assert_eq!(DetectionLayer::ExactHash.match_kind(), MatchKind::ExactHash);
        assert_eq!(
            DetectionLayer::Perceptual.match_kind(),
            MatchKind::PerceptualNearDuplicate
        );
        assert_eq!(
            DetectionLayer::AudioFingerprint.match_kind(),
            MatchKind::AudioFingerprint
        );
    }

    #[test]
    fn exact_hash_runs_first() {
        assert_eq!(DetectionLayer::ALL[0], DetectionLayer::ExactHash);
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
