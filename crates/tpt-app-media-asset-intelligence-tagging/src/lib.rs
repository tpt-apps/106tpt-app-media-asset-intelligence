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
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
