//! Reviewable duplicate groups (spec §6.5, §8).
//!
//! Duplicate detection never deletes anything (spec §3.4, §8): a group carries
//! a [`ReviewStatus`] and an optional keeper, and every transition is an
//! explicit, undoable user action recorded in the persistence layer.

use serde::{Deserialize, Serialize};
use tpt_app_media_asset_intelligence_core::CoreError;
use tpt_app_media_asset_intelligence_core::{AssetId, DuplicateGroupId};

/// How the members of a group were matched (spec §6.5, §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchKind {
    /// Byte-identical content hashes (spec §8 layer 1).
    ExactHash,
    /// Frame-level perceptual similarity; catches re-exports/transcodes (spec §8 layer 2).
    PerceptualNearDuplicate,
    /// Audio fingerprint match for audio-only or audio-dominant material (spec §8 layer 3).
    AudioFingerprint,
}

impl MatchKind {
    /// The kebab-case token used in reports and JSON output (spec §14).
    pub const fn as_str(self) -> &'static str {
        match self {
            MatchKind::ExactHash => "exact-hash",
            MatchKind::PerceptualNearDuplicate => "perceptual-near-duplicate",
            MatchKind::AudioFingerprint => "audio-fingerprint",
        }
    }
}

/// Review progress of a duplicate group (spec §8, §13.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum ReviewStatus {
    /// Detected but not yet reviewed by a human.
    Unreviewed,
    /// A human reviewed the group and chose a keeper; the other members are
    /// recorded, nothing has been deleted (spec §8).
    Reviewed {
        /// The chosen keeper. All other members are non-keepers.
        keeper: AssetId,
    },
}

/// A group of assets detected as duplicates of each other (spec §6.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// Stable identifier within the archive.
    pub id: DuplicateGroupId,
    /// Member assets, in detection order.
    pub members: Vec<AssetId>,
    /// How the members were matched.
    pub match_kind: MatchKind,
    /// Similarity in `[0.0, 1.0]`; exactly `1.0` for exact-hash matches.
    pub similarity: f32,
    /// Review progress (spec §8).
    pub review_status: ReviewStatus,
}

impl DuplicateGroup {
    /// Creates an unreviewed group.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateGroupError`] if the group has fewer than two members
    /// (not a duplicate group), `similarity` is outside `[0.0, 1.0]`, or an
    /// exact-hash group's similarity is not exactly `1.0` (spec §8: an exact
    /// hash match is byte-identical, so any other value would be a lie in the
    /// review UI).
    pub fn new(
        id: u64,
        members: Vec<AssetId>,
        match_kind: MatchKind,
        similarity: f32,
    ) -> Result<Self, CoreError> {
        if members.len() < 2 {
            return Err(CoreError::ConstraintViolated(
                "a duplicate group needs at least two members".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&similarity) {
            return Err(CoreError::ConstraintViolated(format!(
                "similarity must be within [0.0, 1.0], got {similarity}"
            )));
        }
        if match_kind == MatchKind::ExactHash && similarity != 1.0 {
            return Err(CoreError::ConstraintViolated(
                "exact-hash groups always have similarity 1.0".to_string(),
            ));
        }
        if members
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != members.len()
        {
            return Err(CoreError::ConstraintViolated(
                "duplicate group members must be distinct assets".to_string(),
            ));
        }
        Ok(Self {
            id: DuplicateGroupId::new(id),
            members,
            match_kind,
            similarity,
            review_status: ReviewStatus::Unreviewed,
        })
    }

    /// Chooses a keeper for this group, marking it reviewed.
    ///
    /// The keeper must be a member of the group (spec §13.4: the review UI only
    /// offers group members). Nothing is deleted by this call (spec §3.4, §8).
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateGroupError`] if `keeper` is not a member.
    pub fn choose_keeper(&mut self, keeper: AssetId) -> Result<(), CoreError> {
        if !self.members.contains(&keeper) {
            return Err(CoreError::NotFound(format!(
                "asset {keeper} is not a member of group {}",
                self.id
            )));
        }
        self.review_status = ReviewStatus::Reviewed { keeper };
        Ok(())
    }

    /// Returns `true` if a human has reviewed this group.
    pub const fn is_reviewed(&self) -> bool {
        matches!(self.review_status, ReviewStatus::Reviewed { .. })
    }

    /// Returns the total number of member assets.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// Alias for errors returned by duplicate-group operations.
pub type DuplicateGroupError = CoreError;

#[cfg(test)]
mod tests {
    use super::*;

    fn a(n: u64) -> AssetId {
        AssetId::new(n)
    }

    #[test]
    fn exact_hash_groups_have_similarity_one() {
        let g = DuplicateGroup::new(1, vec![a(1), a(2)], MatchKind::ExactHash, 1.0).unwrap();
        assert_eq!(g.match_kind.as_str(), "exact-hash");
        assert!(!g.is_reviewed());
        assert_eq!(g.member_count(), 2);
    }

    #[test]
    fn exact_hash_similarity_below_one_is_rejected() {
        let err = DuplicateGroup::new(1, vec![a(1), a(2)], MatchKind::ExactHash, 0.99).unwrap_err();
        assert!(err.to_string().contains("similarity 1.0"), "got: {err}");
    }

    #[test]
    fn perceptual_groups_accept_partial_similarity() {
        let g = DuplicateGroup::new(
            14,
            vec![a(1), a(2), a(3)],
            MatchKind::PerceptualNearDuplicate,
            0.98,
        )
        .unwrap();
        assert_eq!(g.match_kind.as_str(), "perceptual-near-duplicate");
        assert_eq!(g.similarity, 0.98);
    }

    #[test]
    fn single_member_groups_are_rejected() {
        let err = DuplicateGroup::new(1, vec![a(1)], MatchKind::ExactHash, 1.0).unwrap_err();
        assert!(
            err.to_string().contains("at least two members"),
            "got: {err}"
        );
    }

    #[test]
    fn repeated_members_are_rejected() {
        let err = DuplicateGroup::new(1, vec![a(1), a(1)], MatchKind::ExactHash, 1.0).unwrap_err();
        assert!(err.to_string().contains("distinct"), "got: {err}");
    }

    #[test]
    fn out_of_range_similarity_is_rejected() {
        let err =
            DuplicateGroup::new(1, vec![a(1), a(2)], MatchKind::AudioFingerprint, 1.5).unwrap_err();
        assert!(err.to_string().contains("[0.0, 1.0]"), "got: {err}");
    }

    #[test]
    fn keeper_must_be_a_member_and_marks_reviewed() {
        let mut g = DuplicateGroup::new(1, vec![a(1), a(2)], MatchKind::ExactHash, 1.0).unwrap();
        assert!(g.choose_keeper(a(9)).is_err());
        g.choose_keeper(a(2)).unwrap();
        assert_eq!(g.review_status, ReviewStatus::Reviewed { keeper: a(2) });
        assert!(g.is_reviewed());
    }

    #[test]
    fn review_status_serializes_with_state_discriminant() {
        let mut g = DuplicateGroup::new(1, vec![a(1), a(2)], MatchKind::ExactHash, 1.0).unwrap();
        g.choose_keeper(a(1)).unwrap();
        let json = serde_json::to_string(&g.review_status).unwrap();
        assert!(json.contains("\"state\":\"reviewed\""), "got: {json}");
        assert!(json.contains("\"keeper\":1"), "got: {json}");
    }
}
