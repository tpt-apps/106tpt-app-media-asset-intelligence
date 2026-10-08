<<<<<<< HEAD
//! Layer 1 deterministic + Layer 2 full-text search (spec §10).
//! Layer 3 semantic search is Phase 2 and not implemented here.

pub mod query;

pub use query::{parse_query, Query, QueryError};
use tpt_app_media_asset_intelligence_model::SearchIndexEntry;

/// Case-insensitive substring match over filename, codec, tags, notes.
pub fn matches(entry: &SearchIndexEntry, query: &Query) -> bool {
    query.clauses.iter().all(|c| {
        let needle = c.value.to_ascii_lowercase();
        match c.field.as_str() {
            "codec" => entry.codec.to_ascii_lowercase().contains(&needle),
            "tag" => entry
                .tags
                .iter()
                .any(|t| t.to_ascii_lowercase().contains(&needle)),
            "note" | "notes" => entry.notes.to_ascii_lowercase().contains(&needle),
            _ => {
                entry.filename.to_ascii_lowercase().contains(&needle)
                    || entry
                        .tags
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains(&needle))
                    || entry.notes.to_ascii_lowercase().contains(&needle)
            }
        }
    })
}

/// Explain why an entry matched (spec §3.5: explain every match).
pub fn explain_match(entry: &SearchIndexEntry, query: &Query) -> Vec<String> {
    let mut reasons = Vec::new();
    for c in &query.clauses {
        let needle = c.value.to_ascii_lowercase();
        if entry.filename.to_ascii_lowercase().contains(&needle) {
            reasons.push(format!("filename contains '{}'", c.value));
        }
        if entry.codec.to_ascii_lowercase().contains(&needle) {
            reasons.push(format!("codec '{}' matches", entry.codec));
        }
        for t in &entry.tags {
            if t.to_ascii_lowercase().contains(&needle) {
                reasons.push(format!("tag '{t}' matches"));
            }
        }
        if entry.notes.to_ascii_lowercase().contains(&needle) {
            reasons.push("notes match".to_string());
        }
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn entry() -> SearchIndexEntry {
        SearchIndexEntry {
            asset_id: Uuid::new_v4(),
            filename: "interview_av1.mkv".into(),
            codec: "av1".into(),
            tags: vec!["interview".into()],
            notes: "press event".into(),
        }
    }

    #[test]
    fn codec_query_matches() {
        let q = parse_query("codec:av1").unwrap();
        assert!(matches(&entry(), &q));
    }

    #[test]
    fn codec_query_rejects_mismatch() {
        let q = parse_query("codec:prores").unwrap();
        assert!(!matches(&entry(), &q));
    }

    #[test]
    fn free_text_searches_filename_tags_notes() {
        let q = parse_query("interview").unwrap();
        assert!(matches(&entry(), &q));
        assert!(!explain_match(&entry(), &q).is_empty());
    }

    #[test]
    fn malformed_query_is_an_error() {
        assert!(parse_query("codec:").is_err());
    }
=======
//! Layered search for TPT Media Asset Intelligence (spec §10).
//!
//! Search is layered so the most reliable results never depend on optional AI:
//!
//! - **Layer 1** — deterministic metadata search: filename, path, extension,
//!   codec, container, resolution, frame rate, duration, dates, manual tags
//!   (always available).
//! - **Layer 2** — full-text over local-model tag labels, transcripts and user
//!   notes (available once processed).
//! - **Layer 3** — semantic/similarity search: local embedding models by
//!   default, cloud embeddings strictly opt-in (Phase 2, spec §10.1, §21).
//!
//! This crate defines the query language shared by the GUI search screen and
//! the CLI (`codec:prores AND tag:interview`, spec §14). The query parser is a
//! fuzz target (spec §19.5): it must reject malformed input with a positioned
//! error, never panic.
//!
//! # Grammar
//!
//! ```text
//! query   := or_expr
//! or_expr := and_expr ( OR and_expr )*
//! and_expr:= not_expr ( (AND)? not_expr )*
//! not_expr:= NOT not_expr | primary
//! primary := '(' or_expr ')' | phrase | term
//! phrase  := '"' text '"'
//! term    := (field ':')? value
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod evaluate;
pub mod query;

pub use evaluate::{evaluate, evaluate_text, SearchDoc, SearchHit};
pub use query::{parse_query, Query, QueryError, QueryTerm};

/// The search layers of a result, for per-result match explanations (spec §10, §13.2).
///
/// Every result must be explainable in terms of *why* it matched — matched
/// field, matched tag, or similarity score — never as an unambiguous yes/no
/// (spec §3.5, §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchLayer {
    /// Deterministic metadata match (spec §10 Layer 1).
    Metadata,
    /// Full-text match over tags/notes/transcripts (spec §10 Layer 2).
    FullText,
    /// Semantic/similarity match; carries a 0.0–1.0 similarity score, not a
    /// binary verdict (spec §10 Layer 3, §3.5).
    Semantic,
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}
