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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    }
}
