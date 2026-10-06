//! Search index entries (spec §6.7, §10).
//!
//! Search is layered so the most reliable results never depend on optional AI
//! (spec §10): entries always carry Layer 1/2 text fields; the embedding field
//! is `None` unless a semantic-search layer (local, or cloud in Phase 2) has
//! been enabled for the archive.

use serde::{Deserialize, Serialize};
use tpt_app_media_asset_intelligence_core::{AssetId, TagId};

use crate::tag::TagSource;

/// A compact embedding vector for Layer 3 semantic/similarity search (spec §10).
pub type Embedding = Vec<f32>;

/// One entry of the search index for an asset (spec §6.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchIndexEntry {
    /// The asset this entry makes searchable.
    pub asset: AssetId,
    /// Indexable text: filename, path parts, technical metadata tokens, notes
    /// and transcript text (spec §10 Layers 1–2).
    pub text_fields: Vec<String>,
    /// Tags contributing to full-text matching (spec §10 Layer 2).
    pub tags: Vec<TagId>,
    /// Semantic embedding, present only when a Layer 3 source produced one (spec §10).
    pub embedding: Option<Embedding>,
    /// Which source produced [`SearchIndexEntry::embedding`].
    pub embedding_source: Option<TagSource>,
}

impl SearchIndexEntry {
    /// Creates a metadata/text-only entry (Layers 1–2, always available).
    pub fn text_only(asset: AssetId, text_fields: Vec<String>) -> Self {
        Self {
            asset,
            text_fields,
            tags: Vec::new(),
            embedding: None,
            embedding_source: None,
        }
    }

    /// Returns `true` if this entry carries a semantic embedding.
    pub fn has_embedding(&self) -> bool {
        self.embedding.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_only_entries_have_no_embedding() {
        let e = SearchIndexEntry::text_only(
            AssetId::new(1),
            vec!["interview_final.mov".to_string(), "prores".to_string()],
        );
        assert!(!e.has_embedding());
        assert!(e.embedding_source.is_none());
        assert!(e.tags.is_empty());
    }

    #[test]
    fn embedding_entries_record_their_source() {
        let e = SearchIndexEntry {
            asset: AssetId::new(2),
            text_fields: vec![],
            tags: vec![TagId::new(1)],
            embedding: Some(vec![0.1, -0.2, 0.3]),
            embedding_source: Some(TagSource::LocalModel {
                model: "tpt-embed-small".to_string(),
                version: "1.0.0".to_string(),
            }),
        };
        assert!(e.has_embedding());
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"kind\":\"local-model\""), "got: {json}");
        assert!(serde_json::from_str::<SearchIndexEntry>(&json).unwrap() == e);
    }
}
