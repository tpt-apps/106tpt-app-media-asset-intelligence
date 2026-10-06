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
}
