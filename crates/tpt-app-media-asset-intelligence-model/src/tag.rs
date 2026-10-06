//! Tags, their provenance and rejected-tag memory (spec §6.4, §11).
//!
//! Every tag records its source — manual, local-model or cloud-model — plus a
//! confidence value and the producing model/version where applicable
//! (spec §3.5). Rejected auto-tags are remembered so future re-tagging passes
//! flag them instead of silently reintroducing them (spec §11).

use serde::{Deserialize, Serialize};
use tpt_app_media_asset_intelligence_core::{AssetId, TagId};

/// Where a tag came from (spec §6.4, §3.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TagSource {
    /// Entered by the user; always trusted, never carries confidence.
    Manual,
    /// Produced by a bundled offline model (spec §11).
    LocalModel {
        /// Model identifier (e.g. "tpt-vision-labels").
        model: String,
        /// Model/version string stamped at tagging time.
        version: String,
    },
    /// Produced by an opt-in cloud model; impossible while cloud AI is disabled
    /// (spec §10.1, §17).
    CloudModel {
        /// Provider identifier (spec §6.1).
        provider: String,
        /// Model identifier as reported by the provider.
        model: String,
    },
}

impl TagSource {
    /// Returns `true` for any model-produced source (local or cloud).
    pub const fn is_model_generated(&self) -> bool {
        matches!(
            self,
            TagSource::LocalModel { .. } | TagSource::CloudModel { .. }
        )
    }
}

/// A label attached to an asset (spec §6.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// Stable identifier within the archive.
    pub id: TagId,
    /// The tagged asset.
    pub asset: AssetId,
    /// The label text, lowercase, e.g. "interview" (spec §11 examples).
    pub label: String,
    /// Provenance: who or what produced this tag (spec §3.5).
    pub source: TagSource,
    /// Confidence in `[0.0, 1.0]` for model-generated tags; `None` for manual tags.
    pub confidence: Option<f32>,
}

impl Tag {
    /// Creates a manual tag (no confidence).
    pub fn manual(id: u64, asset: AssetId, label: impl Into<String>) -> Self {
        Self {
            id: TagId::new(id),
            asset,
            label: label.into(),
            source: TagSource::Manual,
            confidence: None,
        }
    }

    /// Creates a local-model tag with confidence and model/version tracking.
    ///
    /// # Panics
    ///
    /// Panics if `confidence` is outside `[0.0, 1.0]`: a confidence value that
    /// cannot be displayed honestly breaks the §3.5 disclosure guarantee, so the
    /// model treats it as a programming error.
    pub fn local_model(
        id: u64,
        asset: AssetId,
        label: impl Into<String>,
        model: impl Into<String>,
        version: impl Into<String>,
        confidence: f32,
    ) -> Self {
        assert_confidence(confidence);
        Self {
            id: TagId::new(id),
            asset,
            label: label.into(),
            source: TagSource::LocalModel {
                model: model.into(),
                version: version.into(),
            },
            confidence: Some(confidence),
        }
    }

    /// Returns `true` if this exact label/source pair was previously rejected.
    ///
    /// A re-tagging pass must not silently reintroduce it; it may re-propose the
    /// tag only when flagged as previously rejected (spec §11).
    pub fn matches_rejection(&self, rejection: &RejectedTag) -> bool {
        self.asset == rejection.asset
            && self.label == rejection.label
            && source_key(&self.source) == source_key(&rejection.source)
    }
}

fn assert_confidence(confidence: f32) {
    assert!(
        (0.0..=1.0).contains(&confidence),
        "confidence must be within [0.0, 1.0], got {confidence}"
    );
}

/// A comparable key for a tag source: the kind plus the identifying model info.
fn source_key(source: &TagSource) -> String {
    match source {
        TagSource::Manual => "manual".to_string(),
        TagSource::LocalModel { model, version } => {
            format!("local-model:{model}:{version}")
        }
        TagSource::CloudModel { provider, model } => {
            format!("cloud-model:{provider}:{model}")
        }
    }
}

/// A user rejection of an auto-generated tag, remembered across re-tagging
/// passes (spec §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedTag {
    /// The asset whose tag was rejected.
    pub asset: AssetId,
    /// The rejected label.
    pub label: String,
    /// The source kind that produced the rejected tag.
    pub source: TagSource,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset() -> AssetId {
        AssetId::new(1)
    }

    #[test]
    fn manual_tags_have_no_confidence() {
        let tag = Tag::manual(1, asset(), "interview");
        assert_eq!(tag.source, TagSource::Manual);
        assert_eq!(tag.confidence, None);
        assert!(!tag.source.is_model_generated());
    }

    #[test]
    fn local_model_tags_track_model_and_version() {
        let tag = Tag::local_model(2, asset(), "person", "tpt-vision-labels", "0.3.0", 0.91);
        assert_eq!(
            tag.source,
            TagSource::LocalModel {
                model: "tpt-vision-labels".to_string(),
                version: "0.3.0".to_string()
            }
        );
        assert_eq!(tag.confidence, Some(0.91));
        assert!(tag.source.is_model_generated());
    }

    #[test]
    #[should_panic(expected = "confidence must be within")]
    fn out_of_range_confidence_is_a_programming_error() {
        Tag::local_model(3, asset(), "person", "m", "v", 1.5);
    }

    #[test]
    fn serializes_source_with_kind_discriminant() {
        let tag = Tag::local_model(2, asset(), "outdoor", "m", "v1", 0.88);
        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains("\"kind\":\"local-model\""), "got: {json}");
        assert!(serde_json::from_str::<Tag>(&json).unwrap() == tag);
    }

    #[test]
    fn rejection_matching_uses_label_and_source() {
        let rejected = RejectedTag {
            asset: asset(),
            label: "person".to_string(),
            source: TagSource::LocalModel {
                model: "tpt-vision-labels".to_string(),
                version: "0.3.0".to_string(),
            },
        };

        let same = Tag::local_model(4, asset(), "person", "tpt-vision-labels", "0.3.0", 0.9);
        let different_label =
            Tag::local_model(5, asset(), "crowd", "tpt-vision-labels", "0.3.0", 0.9);
        let different_version =
            Tag::local_model(6, asset(), "person", "tpt-vision-labels", "0.4.0", 0.9);

        assert!(same.matches_rejection(&rejected));
        assert!(!different_label.matches_rejection(&rejected));
        assert!(!different_version.matches_rejection(&rejected));
    }

    #[test]
    fn rejection_does_not_match_manual_tags_or_other_assets() {
        let rejected = RejectedTag {
            asset: asset(),
            label: "person".to_string(),
            source: TagSource::LocalModel {
                model: "m".to_string(),
                version: "v".to_string(),
            },
        };
        let manual_same_label = Tag::manual(7, asset(), "person");
        assert!(
            !manual_same_label.matches_rejection(&rejected),
            "a manual tag is a user statement, not a reintroduced auto-tag"
        );
        let other_asset = Tag::local_model(8, AssetId::new(2), "person", "m", "v", 0.5);
        assert!(!other_asset.matches_rejection(&rejected));
    }
}
