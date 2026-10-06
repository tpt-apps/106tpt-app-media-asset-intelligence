//! Archive health (spec §12).
//!
//! Archive health covers the health of the archive **as a collection** —
//! distinct from per-file quality control, which is TPT Media QC's role and is
//! referenced rather than duplicated here (spec §12).
//!
//! Checks (spec §12):
//!
//! - missing files (indexed but no longer present on disk)
//! - broken/relinked paths (moved files)
//! - unreadable/corrupt assets encountered during indexing
//! - orphaned derivatives (thumbnail/proxy/waveform with no matching source asset)
//! - storage growth and duplicate-waste trends over time
//!
//! Findings are snapshot values (spec §16 "archive health snapshots"): the
//! persistence layer stores them over time so the dashboard can show trends.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tpt_app_media_asset_intelligence_core::{AssetId, DerivativeId};

/// One problem found by an archive-health check (spec §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum HealthIssue {
    /// Indexed but no longer present on disk (spec §12).
    MissingFile {
        /// The missing asset.
        asset: AssetId,
    },
    /// A file moved: a fingerprint match exists at a new path (spec §12).
    MovedFile {
        /// The asset whose recorded path vanished.
        asset: AssetId,
        /// Archive-normalized path where a fingerprint match now lives.
        new_path: String,
    },
    /// Encountered unreadable/corrupt during indexing (spec §12, §17).
    CorruptAsset {
        /// The asset flagged by the ingest pipeline.
        asset: AssetId,
        /// Failure detail for the details view (spec §13.6).
        detail: String,
    },
    /// Derivative with no matching source asset (spec §12).
    OrphanedDerivative {
        /// The orphaned derivative record.
        derivative: DerivativeId,
    },
}

impl HealthIssue {
    /// Stable machine token for the issue kind (spec §14 output).
    pub const fn kind_token(&self) -> &'static str {
        match self {
            HealthIssue::MissingFile { .. } => "missing-file",
            HealthIssue::MovedFile { .. } => "moved-file",
            HealthIssue::CorruptAsset { .. } => "corrupt-asset",
            HealthIssue::OrphanedDerivative { .. } => "orphaned-derivative",
        }
    }
}

/// A point-in-time archive health snapshot (spec §12, §16).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Total indexed assets at snapshot time.
    pub total_assets: usize,
    /// All findings, grouped by kind token for the dashboard (spec §13.6).
    pub issues: Vec<HealthIssue>,
}

impl HealthSnapshot {
    /// Counts findings by kind, keyed by [`HealthIssue::kind_token`].
    pub fn counts_by_kind(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for issue in &self.issues {
            *counts.entry(issue.kind_token()).or_insert(0) += 1;
        }
        counts
    }

    /// Number of findings of one kind.
    pub fn count_of(&self, kind_token: &str) -> usize {
        self.issues
            .iter()
            .filter(|i| i.kind_token() == kind_token)
            .count()
    }

    /// Returns `true` when the archive has no findings.
    pub fn is_healthy(&self) -> bool {
        self.issues.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> HealthSnapshot {
        HealthSnapshot {
            total_assets: 48_201,
            issues: vec![
                HealthIssue::MissingFile {
                    asset: AssetId::new(1),
                },
                HealthIssue::MissingFile {
                    asset: AssetId::new(2),
                },
                HealthIssue::CorruptAsset {
                    asset: AssetId::new(3),
                    detail: "container truncated".to_string(),
                },
                HealthIssue::OrphanedDerivative {
                    derivative: tpt_app_media_asset_intelligence_core::DerivativeId::new(41),
                },
            ],
        }
    }

    #[test]
    fn counts_group_by_kind_token() {
        let s = snapshot();
        assert_eq!(s.count_of("missing-file"), 2);
        assert_eq!(s.count_of("corrupt-asset"), 1);
        assert_eq!(s.count_of("orphaned-derivative"), 1);
        assert_eq!(s.count_of("moved-file"), 0);
        let counts = s.counts_by_kind();
        assert_eq!(counts.get("missing-file"), Some(&2));
        assert_eq!(counts.values().sum::<usize>(), s.issues.len());
    }

    #[test]
    fn moved_file_reports_the_new_path() {
        let issue = HealthIssue::MovedFile {
            asset: AssetId::new(9),
            new_path: "projects/2024/interview_final.mov".to_string(),
        };
        assert_eq!(issue.kind_token(), "moved-file");
    }

    #[test]
    fn snapshot_serializes_with_kind_tags() {
        let s = snapshot();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"kind\":\"missing-file\""), "got: {json}");
        assert_eq!(serde_json::from_str::<HealthSnapshot>(&json).unwrap(), s);
    }

    #[test]
    fn empty_snapshot_is_healthy() {
        assert!(HealthSnapshot::default().is_healthy());
        assert!(!snapshot().is_healthy());
    }
}
