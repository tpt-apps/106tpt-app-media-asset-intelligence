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
}
