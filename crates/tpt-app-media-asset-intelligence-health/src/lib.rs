<<<<<<< HEAD
//! Archive health checks: missing, corrupt, orphaned derivatives, waste (spec §12).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tpt_app_media_asset_intelligence_model::{Asset, AssetId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub taken_at: DateTime<Utc>,
    pub total_assets: usize,
    pub missing_files: Vec<AssetId>,
    pub corrupt_assets: Vec<AssetId>,
    pub orphaned_derivatives: Vec<PathBuf>,
    pub duplicate_waste_bytes: u64,
}

impl HealthSnapshot {
    pub fn missing_count(&self) -> usize {
        self.missing_files.len()
    }
}

/// Assets whose path no longer exists on disk.
pub fn find_missing(assets: &[Asset]) -> Vec<&Asset> {
    assets.iter().filter(|a| !a.path.exists()).collect()
}

/// Assets flagged unsupported/corrupt during indexing.
pub fn find_corrupt(assets: &[Asset]) -> Vec<&Asset> {
    assets
        .iter()
        .filter(|a| !a.technical_metadata.supported)
        .collect()
}

/// Derivatives with no matching source asset id.
pub fn find_orphaned_derivatives(
    derivatives: &[(AssetId, PathBuf)],
    live_ids: &[AssetId],
) -> Vec<PathBuf> {
    derivatives
        .iter()
        .filter(|(id, _)| !live_ids.contains(id))
        .map(|(_, p)| p.clone())
        .collect()
}

/// Total count of things a user must act on in a snapshot.
pub fn intervention_count(snapshot: &HealthSnapshot) -> usize {
    snapshot.missing_count() + snapshot.corrupt_assets.len() + snapshot.orphaned_derivatives.len()
}

/// A per-snapshot trend row for the health dashboard (§21): the total
/// intervention count plus how it changed since the previous snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrend {
    pub taken_at: DateTime<Utc>,
    pub total_assets: usize,
    pub interventions: usize,
    /// Interventions vs. the previous snapshot (`0` for the first row).
    pub delta: i64,
    pub duplicate_waste_bytes: u64,
}

/// Turn a chronological list of snapshots (oldest first) into trend rows.
/// Snapshots are sorted defensively by `taken_at` so callers can pass
/// `recent_health_snapshots` output in either order.
pub fn health_trend(snapshots: &[HealthSnapshot]) -> Vec<HealthTrend> {
    let mut sorted: Vec<&HealthSnapshot> = snapshots.iter().collect();
    sorted.sort_by_key(|s| s.taken_at);
    let mut out = Vec::with_capacity(sorted.len());
    for (i, s) in sorted.iter().enumerate() {
        let interventions = intervention_count(s);
        let delta = if i == 0 {
            0
        } else {
            interventions as i64 - intervention_count(sorted[i - 1]) as i64
        };
        out.push(HealthTrend {
            taken_at: s.taken_at,
            total_assets: s.total_assets,
            interventions,
            delta,
            duplicate_waste_bytes: s.duplicate_waste_bytes,
        });
    }
    out
=======
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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;
<<<<<<< HEAD
    use tpt_app_media_asset_intelligence_model::{MediaType, TechnicalMetadata};
    use uuid::Uuid;

    fn asset(path: PathBuf, supported: bool) -> Asset {
        Asset {
            id: Uuid::new_v4(),
            archive: Uuid::new_v4(),
            path,
            fingerprint: "fp".into(),
            size_bytes: 10,
            modified_time: Utc::now(),
            media_type: MediaType::Video,
            technical_metadata: TechnicalMetadata {
                codec: "av1".into(),
                container: "matroska".into(),
                supported,
                width: None,
                height: None,
                duration_secs: None,
            },
        }
    }

    #[test]
    fn missing_detection() {
        let a = asset(PathBuf::from("__no_such_file__.mkv"), true);
        assert_eq!(find_missing(&[a]).len(), 1);
    }

    #[test]
    fn corrupt_detection_flags_unsupported() {
        let a = asset(PathBuf::from("x.mkv"), false);
        assert_eq!(find_corrupt(&[a]).len(), 1);
    }

    #[test]
    fn orphaned_derivative_detection() {
        let live = Uuid::new_v4();
        let dead = Uuid::new_v4();
        let derivs = vec![
            (live, PathBuf::from("a.thumb")),
            (dead, PathBuf::from("b.thumb")),
        ];
        let orphans = find_orphaned_derivatives(&derivs, &[live]);
        assert_eq!(orphans, vec![PathBuf::from("b.thumb")]);
    }

    #[test]
    fn empty_archive_is_healthy() {
        assert!(find_missing(&[]).is_empty());
        assert!(find_corrupt(&[]).is_empty());
    }

    fn snapshot(
        taken_at: DateTime<Utc>,
        total: usize,
        missing: usize,
        corrupt: usize,
        orphans: usize,
    ) -> HealthSnapshot {
        HealthSnapshot {
            taken_at,
            total_assets: total,
            missing_files: vec![Uuid::new_v4(); missing],
            corrupt_assets: vec![Uuid::new_v4(); corrupt],
            orphaned_derivatives: vec![PathBuf::from("o"); orphans],
            duplicate_waste_bytes: 0,
=======

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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
        }
    }

    #[test]
<<<<<<< HEAD
    fn trend_reports_interventions_and_deltas() {
        let runs = vec![
            snapshot(Utc::now() - chrono::Duration::hours(2), 100, 1, 0, 0),
            snapshot(Utc::now() - chrono::Duration::hours(1), 100, 1, 1, 0),
            snapshot(Utc::now(), 98, 2, 1, 1),
        ];
        let trend = health_trend(&runs);
        assert_eq!(trend.len(), 3);
        assert_eq!(trend[0].interventions, 1);
        assert_eq!(trend[0].delta, 0);
        assert_eq!(trend[1].interventions, 2);
        assert_eq!(trend[1].delta, 1);
        assert_eq!(trend[2].interventions, 4);
        assert_eq!(trend[2].delta, 2);
        assert!(trend[0].taken_at < trend[1].taken_at);
    }

    #[test]
    fn trend_sorts_input_defensively() {
        let later = snapshot(Utc::now(), 10, 1, 0, 0);
        let earlier = snapshot(Utc::now() - chrono::Duration::days(1), 12, 0, 0, 0);
        let trend = health_trend(&[later, earlier]);
        assert_eq!(trend.len(), 2);
        assert!(trend[0].taken_at < trend[1].taken_at);
        assert_eq!(trend[1].delta, 1);
=======
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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
