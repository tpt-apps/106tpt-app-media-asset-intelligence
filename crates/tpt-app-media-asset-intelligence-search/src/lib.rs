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

pub mod query;

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
}
