<<<<<<< HEAD
//! Local-model tagging with source/confidence tracking and rejected-tag memory (spec §11).
//! Cloud tagging is Phase 2 only; this crate refuses cloud work unless
//! explicitly enabled via [`CloudTaggingGate`].

use tpt_app_media_asset_intelligence_model::{AssetId, Tag, TagSource};

/// Gate that makes cloud tagging impossible to invoke accidentally.
/// Local tagging is always available; cloud requires an explicit flag.
#[derive(Debug, Clone, Copy)]
pub struct CloudTaggingGate {
    pub cloud_allowed: bool,
}

impl CloudTaggingGate {
    pub fn local_only() -> Self {
        Self {
            cloud_allowed: false,
        }
    }

    pub fn require_cloud(&self) -> Result<(), String> {
        if self.cloud_allowed {
            Ok(())
        } else {
            Err("cloud tagging requires an explicit opt-in flag".to_string())
        }
    }
}

/// Record a manual tag (create/edit path, spec §11).
pub fn manual_tag(asset_id: AssetId, label: &str) -> Tag {
    Tag {
        asset_id,
        label: label.to_string(),
        source: TagSource::Manual,
        confidence: None,
        model_version: None,
        rejected: false,
    }
}

/// Record a local-model tag with confidence + model version.
pub fn local_model_tag(
    asset_id: AssetId,
    label: &str,
    confidence: f32,
    model_version: &str,
) -> Tag {
    Tag {
        asset_id,
        label: label.to_string(),
        source: TagSource::LocalModel,
        confidence: Some(confidence),
        model_version: Some(model_version.to_string()),
        rejected: false,
    }
}

/// Re-tagging filter: drop candidates whose label was previously rejected,
/// returning them separately so the UI can flag the reintroduction (§11).
pub fn filter_rejected(tags: Vec<Tag>, rejected_labels: &[String]) -> (Vec<Tag>, Vec<Tag>) {
    let mut kept = Vec::new();
    let mut flagged = Vec::new();
    for t in tags {
        if rejected_labels.iter().any(|r| r == &t.label) {
            flagged.push(t);
        } else {
            kept.push(t);
        }
    }
    (kept, flagged)
=======
//! Tagging: offline local-model auto-tagging plus manual tagging (spec §11).
//!
//! Default (local): a bundled local model set produces object/scene/label tags
//! from thumbnails and sampled frames, and basic audio-event tags from
//! waveforms — entirely offline (spec §11, §20). Every tag carries its source
//! and confidence, and every model run is traceable through the
//! model/version pair on [`tpt_app_media_asset_intelligence_model::TagSource::LocalModel`].
//!
//! Rejected-tag memory (spec §11): when the user rejects an auto-tag, the
//! [`RejectedTagMemory`] below records it so future re-tagging passes flag the
//! tag instead of silently reintroducing it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tpt_app_media_asset_intelligence_core::AssetId;
use tpt_app_media_asset_intelligence_model::{RejectedTag, Tag};

/// Outcome of proposing an auto-tag against the rejected-tag memory (spec §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalDecision {
    /// Not previously rejected; propose normally.
    Propose,
    /// Previously rejected by the user; propose only flagged as such (spec §11).
    ProposeFlaggedRejected,
}

/// The set of user-rejected auto-tags for one archive (spec §11).
///
/// Persistence maps this set to the database; the memory itself is an ordinary
/// value so review screens can diff and display it (spec §13.5).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedTagMemory {
    entries: BTreeSet<MemoryKey>,
}

/// A hashable key for one rejection: asset, label and source kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct MemoryKey {
    asset: AssetId,
    label: String,
    source_kind: SourceKind,
}

/// Coarse source kind for memory keys: model identity but not per-run version.
///
/// A model *update* may legitimately re-propose a previously rejected label —
/// the spec requires flagged re-proposal, not permanent suppression (spec §11).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SourceKind {
    Local,
    Cloud,
}

impl RejectedTagMemory {
    /// Records a user rejection of an auto-generated tag (spec §11).
    pub fn reject(&mut self, tag: &Tag) {
        let Some(kind) = model_kind(&tag.source) else {
            return; // manual tags cannot be "auto-reintroduced"
        };
        self.entries.insert(MemoryKey {
            asset: tag.asset,
            label: normalize_label(&tag.label),
            source_kind: kind,
        });
    }

    /// Decides how a proposed auto-tag should be surfaced (spec §11).
    pub fn decision_for(&self, tag: &Tag) -> ProposalDecision {
        match model_kind(&tag.source) {
            None => ProposalDecision::Propose,
            Some(kind) => {
                if self.entries.contains(&MemoryKey {
                    asset: tag.asset,
                    label: normalize_label(&tag.label),
                    source_kind: kind,
                }) {
                    ProposalDecision::ProposeFlaggedRejected
                } else {
                    ProposalDecision::Propose
                }
            }
        }
    }

    /// All recorded rejections, for the tagging review screen (spec §13.5).
    pub fn entries(&self) -> Vec<RejectedTag> {
        self.entries
            .iter()
            .map(|key| RejectedTag {
                asset: key.asset,
                label: key.label.clone(),
                source: match key.source_kind {
                    SourceKind::Local => {
                        tpt_app_media_asset_intelligence_model::TagSource::LocalModel {
                            model: "*".to_string(),
                            version: "*".to_string(),
                        }
                    }
                    SourceKind::Cloud => {
                        tpt_app_media_asset_intelligence_model::TagSource::CloudModel {
                            provider: "*".to_string(),
                            model: "*".to_string(),
                        }
                    }
                },
            })
            .collect()
    }

    /// Number of recorded rejections.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no rejections are recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn model_kind(source: &tpt_app_media_asset_intelligence_model::TagSource) -> Option<SourceKind> {
    use tpt_app_media_asset_intelligence_model::TagSource;
    match source {
        TagSource::Manual => None,
        TagSource::LocalModel { .. } => Some(SourceKind::Local),
        TagSource::CloudModel { .. } => Some(SourceKind::Cloud),
    }
}

fn normalize_label(label: &str) -> String {
    label.trim().to_ascii_lowercase()
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;
<<<<<<< HEAD
    use uuid::Uuid;

    #[test]
    fn cloud_gate_blocks_by_default() {
        assert!(CloudTaggingGate::local_only().require_cloud().is_err());
    }

    #[test]
    fn cloud_gate_passes_when_explicit() {
        let g = CloudTaggingGate {
            cloud_allowed: true,
        };
        assert!(g.require_cloud().is_ok());
    }

    #[test]
    fn rejected_tags_are_flagged_not_silently_reintroduced() {
        let id = Uuid::new_v4();
        let tags = vec![
            local_model_tag(id, "outdoor", 0.9, "local-1"),
            local_model_tag(id, "indoor", 0.8, "local-1"),
        ];
        let (kept, flagged) = filter_rejected(tags, &["indoor".to_string()]);
        assert_eq!(kept.len(), 1);
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].label, "indoor");
    }

    #[test]
    fn manual_tags_carry_no_confidence() {
        let t = manual_tag(Uuid::new_v4(), "interview");
        assert_eq!(t.source, TagSource::Manual);
        assert!(t.confidence.is_none());
=======
    use tpt_app_media_asset_intelligence_core::TagId;

    fn local_tag(label: &str, version: &str) -> Tag {
        Tag::local_model(1, AssetId::new(7), label, "tpt-vision-labels", version, 0.9)
    }

    #[test]
    fn rejected_local_tag_is_flagged_on_reproposal() {
        let mut memory = RejectedTagMemory::default();
        let tag = local_tag("person", "0.3.0");
        memory.reject(&tag);
        assert_eq!(memory.len(), 1);
        assert_eq!(
            memory.decision_for(&local_tag("person", "0.3.0")),
            ProposalDecision::ProposeFlaggedRejected
        );
    }

    #[test]
    fn rejection_survives_model_update_but_is_matchable() {
        // A model update re-proposing a rejected label is still flagged (spec §11:
        // "at least flagging that it was previously rejected").
        let mut memory = RejectedTagMemory::default();
        memory.reject(&local_tag("person", "0.3.0"));
        assert_eq!(
            memory.decision_for(&local_tag("person", "0.9.0")),
            ProposalDecision::ProposeFlaggedRejected
        );
    }

    #[test]
    fn other_labels_and_assets_are_unaffected() {
        let mut memory = RejectedTagMemory::default();
        memory.reject(&local_tag("person", "0.3.0"));
        assert_eq!(
            memory.decision_for(&local_tag("outdoor", "0.3.0")),
            ProposalDecision::Propose
        );
        let other_asset = Tag::local_model(
            2,
            AssetId::new(8),
            "person",
            "tpt-vision-labels",
            "0.3.0",
            0.9,
        );
        assert_eq!(memory.decision_for(&other_asset), ProposalDecision::Propose);
    }

    #[test]
    fn manual_tags_are_never_memorized() {
        let mut memory = RejectedTagMemory::default();
        let manual = Tag::manual(3, AssetId::new(7), "person");
        memory.reject(&manual);
        assert!(
            memory.is_empty(),
            "manual tags have no model to reintroduce them"
        );
        assert_eq!(memory.decision_for(&manual), ProposalDecision::Propose);
    }

    #[test]
    fn labels_normalize_for_matching() {
        let mut memory = RejectedTagMemory::default();
        memory.reject(&local_tag("Person", "0.3.0"));
        assert_eq!(
            memory.decision_for(&local_tag("person", "0.3.0")),
            ProposalDecision::ProposeFlaggedRejected
        );
    }

    #[test]
    fn entries_round_trip_through_serde() {
        let mut memory = RejectedTagMemory::default();
        memory.reject(&local_tag("person", "0.3.0"));
        let json = serde_json::to_string(&memory).unwrap();
        let restored: RejectedTagMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, memory);
        assert_eq!(restored.entries().len(), 1);
    }

    #[test]
    fn cloud_rejections_are_tracked_separately() {
        use tpt_app_media_asset_intelligence_model::TagSource;
        let mut memory = RejectedTagMemory::default();
        let cloud_tag = Tag {
            id: TagId::new(9),
            asset: AssetId::new(7),
            label: "person".to_string(),
            source: TagSource::CloudModel {
                provider: "example".to_string(),
                model: "big-labeler".to_string(),
            },
            confidence: Some(0.95),
        };
        memory.reject(&cloud_tag);
        assert_eq!(memory.len(), 1);
        assert_eq!(
            memory.decision_for(&local_tag("person", "0.3.0")),
            ProposalDecision::Propose
        );
        assert_eq!(
            memory.decision_for(&cloud_tag),
            ProposalDecision::ProposeFlaggedRejected
        );
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
