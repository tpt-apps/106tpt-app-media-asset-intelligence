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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        }
    }

    #[test]
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
    }
}
