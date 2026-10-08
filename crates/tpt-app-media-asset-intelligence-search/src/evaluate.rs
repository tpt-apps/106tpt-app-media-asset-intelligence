//! Pass 1 query evaluation with per-result explanations (spec §10, §3.5, §13.2).
//!
//! Layer 1 (deterministic metadata) evaluation: every match explains *why* it
//! matched — which field, which value — never a binary verdict (spec §3.5).
//! Evaluation is deterministic: results sort by asset id, so the same index
//! and query always produce the same order.

use std::collections::BTreeMap;

use tpt_app_media_asset_intelligence_core::AssetId;
use tpt_app_media_asset_intelligence_model::Asset;

use crate::query::{parse_query, Query, QueryTerm};

/// The field names recognized in `field:value` terms (spec §10 Layer 1).
///
/// Matching semantics per field:
/// - `filename`, `path`: case-insensitive substring.
/// - `ext`, `media`, `codec`, `container`, `resolution`: case-insensitive equality.
/// - `tag`: case-insensitive equality against tag labels (manual tags for
///   Layer 1; local-model labels join once tagging has run, spec §10 Layer 2).
pub mod field {
    /// The file's name (`interview_final.mov`).
    pub const FILENAME: &str = "filename";
    /// The archive-normalized path.
    pub const PATH: &str = "path";
    /// Lowercased extension (`mov`).
    pub const EXT: &str = "ext";
    /// Broad media type token (`video`/`audio`/`image`/`other`).
    pub const MEDIA: &str = "media";
    /// Video or audio codec (`prores`, `h264`, `pcm`).
    pub const CODEC: &str = "codec";
    /// Container token (`mov`, `wav`).
    pub const CONTAINER: &str = "container";
    /// Resolution token (`1920x1080`).
    pub const RESOLUTION: &str = "resolution";
    /// A tag label (`interview`).
    pub const TAG: &str = "tag";

    /// All recognized field names.
    pub const ALL: [&str; 8] = [
        FILENAME, PATH, EXT, MEDIA, CODEC, CONTAINER, RESOLUTION, TAG,
    ];
}

/// One asset's searchable document (spec §6.7, §10 Layer 1).
///
/// Field values keep their original casing for explanations; matching is
/// case-insensitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDoc {
    /// The asset this document makes searchable.
    pub asset: AssetId,
    /// Field values by field name.
    pub fields: BTreeMap<String, Vec<String>>,
}

impl SearchDoc {
    /// Builds the Layer 1 document for an asset from its indexed metadata.
    ///
    /// Deterministic (spec §3.2): the same asset always yields the same
    /// document.
    pub fn from_asset(asset: &Asset) -> Self {
        let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();

        let path_text = asset.path.to_string_lossy().replace('\\', "/");
        let filename = asset
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = asset
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        fields.insert(field::FILENAME.to_string(), vec![filename]);
        fields.insert(field::PATH.to_string(), vec![path_text]);
        if !ext.is_empty() {
            fields.insert(field::EXT.to_string(), vec![ext]);
        }
        fields.insert(
            field::MEDIA.to_string(),
            vec![asset.media_type.as_str().to_string()],
        );

        let meta = &asset.technical_metadata;
        let mut codecs: Vec<String> = Vec::new();
        if let Some(codec) = &meta.video_codec {
            codecs.push(codec.clone());
        }
        if let Some(codec) = &meta.audio_codec {
            codecs.push(codec.clone());
        }
        if !codecs.is_empty() {
            fields.insert(field::CODEC.to_string(), codecs);
        }
        if let Some(container) = &meta.container {
            fields.insert(field::CONTAINER.to_string(), vec![container.clone()]);
        }
        if let (Some(w), Some(h)) = (meta.width, meta.height) {
            fields.insert(field::RESOLUTION.to_string(), vec![format!("{w}x{h}")]);
        }

        Self {
            asset: asset.id,
            fields,
        }
    }

    /// Attaches tag labels (manual for Layer 1; model labels join in Layer 2).
    pub fn with_tags(mut self, labels: impl IntoIterator<Item = String>) -> Self {
        self.fields
            .entry(field::TAG.to_string())
            .or_default()
            .extend(labels);
        self
    }

    fn values_for(&self, field_name: &str) -> &[String] {
        self.fields
            .get(field_name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// One matching result with its §3.5 explanation (spec §13.2: every result
/// shows why it matched).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// The matching asset.
    pub asset: AssetId,
    /// Why this asset matched, one entry per contributing term.
    pub explanations: Vec<String>,
}

/// Evaluates a parsed query over documents (spec §10 Layer 1).
///
/// For convenience, `query_text` may be passed raw via [`evaluate_text`].
/// Results sort by asset id; ties are impossible (ids are unique).
pub fn evaluate(query: &Query, docs: &[SearchDoc]) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = docs
        .iter()
        .filter_map(|doc| {
            eval_query(doc, query).map(|explanations| SearchHit {
                asset: doc.asset,
                explanations,
            })
        })
        .collect();
    hits.sort_by_key(|h| h.asset);
    hits
}

/// Parses then evaluates (spec §14): the CLI/GUI hand-over convenience.
///
/// # Errors
///
/// Returns the query parse error for malformed input.
pub fn evaluate_text(
    query_text: &str,
    docs: &[SearchDoc],
) -> Result<Vec<SearchHit>, crate::query::QueryError> {
    let query = parse_query(query_text)?;
    Ok(evaluate(&query, docs))
}

fn eval_query(doc: &SearchDoc, query: &Query) -> Option<Vec<String>> {
    match query {
        Query::MatchAll => Some(vec!["matches all assets".to_string()]),
        Query::Term(term) => eval_term(doc, term),
        Query::And(a, b) => {
            let left = eval_query(doc, a)?;
            let right = eval_query(doc, b)?;
            Some([left, right].concat())
        }
        Query::Or(a, b) => match (eval_query(doc, a), eval_query(doc, b)) {
            (Some(l), Some(r)) => Some([l, r].concat()),
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        },
        // `a NOT b` and the desugared prefix `NOT b` (`* NOT b`).
        Query::Not(a, b) => {
            let left = eval_query(doc, a)?;
            if eval_query(doc, b).is_some() {
                None
            } else {
                Some(left)
            }
        }
    }
}

fn eval_term(doc: &SearchDoc, term: &QueryTerm) -> Option<Vec<String>> {
    match &term.field {
        None => eval_bare_term(doc, term),
        Some(field_name) => eval_field_term(doc, field_name, term),
    }
}

fn eval_field_term(doc: &SearchDoc, field_name: &str, term: &QueryTerm) -> Option<Vec<String>> {
    let values = doc.values_for(field_name);
    if values.is_empty() {
        return None;
    }
    let needle = term.value.to_ascii_lowercase();
    for value in values {
        let value_lc = value.to_ascii_lowercase();
        let matched = if field_name == field::FILENAME || field_name == field::PATH {
            value_lc.contains(&needle)
        } else {
            value_lc == needle
        };
        if matched {
            return Some(vec![format!(
                "{field_name} matches \"{}\" ({value})",
                term.value
            )]);
        }
    }
    None
}

fn eval_bare_term(doc: &SearchDoc, term: &QueryTerm) -> Option<Vec<String>> {
    // Bare terms search filename and path by substring, then the technical
    // fields and tags by equality (spec §10 Layer 1).
    for name in [field::FILENAME, field::PATH] {
        if let Some(hit) = eval_field_term(doc, name, term) {
            return Some(hit);
        }
    }
    for name in [
        field::EXT,
        field::MEDIA,
        field::CODEC,
        field::CONTAINER,
        field::RESOLUTION,
        field::TAG,
    ] {
        if let Some(hit) = eval_field_term(doc, name, term) {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tpt_app_media_asset_intelligence_core::MediaType;
    use tpt_app_media_asset_intelligence_model::TechnicalMetadata;

    fn asset(id: u64, path: &str, media_type: MediaType, meta: TechnicalMetadata) -> Asset {
        Asset {
            id: AssetId::new(id),
            archive: tpt_app_media_asset_intelligence_core::ArchiveId::new(1),
            path: PathBuf::from(path),
            fingerprint: tpt_app_media_asset_intelligence_core::AssetFingerprint::from_slice(
                path.as_bytes(),
            ),
            size_bytes: 100,
            modified_time: std::time::SystemTime::UNIX_EPOCH,
            media_type,
            technical_metadata: meta,
        }
    }

    fn prores_1080() -> TechnicalMetadata {
        TechnicalMetadata {
            container: Some("mov".to_string()),
            video_codec: Some("prores".to_string()),
            audio_codec: Some("pcm".to_string()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some((25000, 1000)),
            duration_ms: Some(120_000),
            streams: vec![],
        }
    }

    fn docs() -> Vec<SearchDoc> {
        vec![
            SearchDoc::from_asset(&asset(
                1,
                "projects/2024/interview_final.mov",
                MediaType::Video,
                prores_1080(),
            ))
            .with_tags(["interview".to_string()]),
            SearchDoc::from_asset(&asset(
                2,
                "projects/2024/b_roll.mov",
                MediaType::Video,
                prores_1080(),
            )),
            SearchDoc::from_asset(&asset(
                3,
                "archive/room_tone.wav",
                MediaType::Audio,
                TechnicalMetadata {
                    container: Some("wav".to_string()),
                    audio_codec: Some("wav".to_string()),
                    ..TechnicalMetadata::default()
                },
            )),
        ]
    }

    #[test]
    fn spec_example_query_finds_the_prores_interview() {
        // `codec:prores AND tag:interview` (spec §14): asset 1 only.
        let hits = evaluate_text("codec:prores AND tag:interview", &docs()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].asset, AssetId::new(1));
        assert!(
            hits[0].explanations.iter().any(|e| e.contains("codec")),
            "explains the codec match: {:?}",
            hits[0].explanations
        );
        assert!(
            hits[0].explanations.iter().any(|e| e.contains("tag")),
            "explains the tag match: {:?}",
            hits[0].explanations
        );
    }

    #[test]
    fn bare_terms_search_filename_and_path_by_substring() {
        let hits = evaluate_text("interview", &docs()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].asset, AssetId::new(1));

        let hits = evaluate_text("2024", &docs()).unwrap();
        assert_eq!(hits.len(), 2, "path substring matches both 2024 assets");
    }

    #[test]
    fn field_terms_are_case_insensitive_and_equality_based() {
        assert_eq!(evaluate_text("CODEC:ProRes", &docs()).unwrap().len(), 2);
        assert_eq!(evaluate_text("media:video", &docs()).unwrap().len(), 2);
        assert_eq!(evaluate_text("media:audio", &docs()).unwrap().len(), 1);
        // Substring must NOT match for equality fields: "pro" is not a codec.
        assert_eq!(evaluate_text("codec:pro", &docs()).unwrap().len(), 0);
        // Resolution token matches.
        assert_eq!(
            evaluate_text("resolution:1920x1080", &docs())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn filename_and_path_use_substring_matching() {
        assert_eq!(evaluate_text("filename:final", &docs()).unwrap().len(), 1);
        assert_eq!(evaluate_text("path:archive", &docs()).unwrap().len(), 1);
    }

    #[test]
    fn unknown_fields_match_nothing_rather_than_panicking() {
        assert_eq!(evaluate_text("nosuchfield:x", &docs()).unwrap().len(), 0);
    }

    #[test]
    fn or_and_not_compose() {
        // (b_roll OR room_tone) → assets 2, 3.
        let hits = evaluate_text("b_roll OR room_tone", &docs()).unwrap();
        let ids: Vec<u64> = hits.iter().map(|h| h.asset.get()).collect();
        assert_eq!(ids, vec![2, 3]);

        // prores NOT tag:interview → asset 2.
        let hits = evaluate_text("prores NOT tag:interview", &docs()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].asset, AssetId::new(2));

        // Prefix NOT: NOT media:video → asset 3.
        let hits = evaluate_text("NOT media:video", &docs()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].asset, AssetId::new(3));
    }

    #[test]
    fn match_all_returns_everything_with_an_honest_explanation() {
        let hits = evaluate_text("", &docs()).unwrap();
        assert_eq!(hits.len(), 3);
        assert!(hits
            .iter()
            .all(|h| h.explanations.iter().any(|e| e.contains("all"))));
    }

    #[test]
    fn phrases_match_their_text_across_fields() {
        let hits = evaluate_text("\"interview_final.mov\"", &docs()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].asset, AssetId::new(1));
    }

    #[test]
    fn results_are_deterministic_and_sorted_by_asset_id() {
        let a = evaluate_text("media:video", &docs()).unwrap();
        let b = evaluate_text("media:video", &docs()).unwrap();
        assert_eq!(a, b, "same inputs, same results");
        let ids: Vec<u64> = a.iter().map(|h| h.asset.get()).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn malformed_queries_report_errors() {
        assert!(evaluate_text("(unclosed", &docs()).is_err());
    }

    #[test]
    fn tags_join_the_document_through_with_tags() {
        let docs = vec![SearchDoc::from_asset(&asset(
            9,
            "x.mov",
            MediaType::Video,
            TechnicalMetadata::default(),
        ))
        .with_tags(["outdoor".to_string(), "press-event".to_string()])];
        let hits = evaluate_text("tag:press-event", &docs).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0]
                .explanations
                .iter()
                .any(|e| e.contains("press-event")),
            "explains the tag: {:?}",
            hits[0].explanations
        );
    }
}
