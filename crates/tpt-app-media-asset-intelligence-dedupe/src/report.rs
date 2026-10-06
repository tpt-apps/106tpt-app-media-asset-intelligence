//! Machine-readable dedupe reports for the CLI (spec §8, §14).
//!
//! `tpt-media-asset-intel dedupe --report duplicates.json` writes this report.
//! It is a *review aid*: it names groups and the reclaimable bytes, it does not
//! delete anything (spec §3.4, §8).

use serde::{Deserialize, Serialize};
use tpt_app_media_asset_intelligence_model::DuplicateGroup;

/// Summary of storage that duplicate groups waste (spec §8, §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WastedSummary {
    /// Number of duplicate groups found.
    pub groups: usize,
    /// Total assets sitting in duplicate groups (members, not keepers).
    pub assets_in_groups: usize,
    /// Sum of member bytes that a keeper choice could reclaim, in bytes.
    ///
    /// Exact definition: for each group, all member sizes minus the size of
    /// that group's current keeper (or, for unreviewed groups, its largest
    /// member). Phase 1 fills the size input; the arithmetic is fixed here so
    /// the CLI report and the Archive Health dashboard (spec §12) agree.
    pub reclaimable_bytes: u64,
}

/// A complete dedupe pass result (spec §14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DedupeReport {
    /// The archive that was scanned.
    pub archive: String,
    /// Duplicate groups, review state included (spec §8).
    pub groups: Vec<DuplicateGroup>,
    /// Aggregate waste summary (spec §12).
    pub wasted: WastedSummary,
}

impl DedupeReport {
    /// Builds a report from groups plus a member-size lookup.
    ///
    /// The lookup returns `None` for assets whose size is unknown (e.g. a
    /// vanished file, spec §12); unknown sizes contribute zero to the
    /// reclaimable total rather than failing the whole report.
    ///
    /// Definition of `reclaimable_bytes`: per group, the sum of all member
    /// sizes minus exactly one baseline copy — the chosen keeper for reviewed
    /// groups, otherwise the largest member.
    pub fn build(
        archive: impl Into<String>,
        groups: Vec<DuplicateGroup>,
        size_of: impl Fn(tpt_app_media_asset_intelligence_core::AssetId) -> Option<u64>,
    ) -> Self {
        let mut wasted = WastedSummary {
            groups: groups.len(),
            ..WastedSummary::default()
        };
        for group in &groups {
            let known: Vec<u64> = group
                .members
                .iter()
                .filter_map(|asset| size_of(*asset))
                .collect();
            wasted.assets_in_groups += known.len();
            if known.is_empty() {
                continue;
            }
            let keeper_size = match group.review_status {
                tpt_app_media_asset_intelligence_model::ReviewStatus::Reviewed { keeper } => {
                    size_of(keeper)
                }
                tpt_app_media_asset_intelligence_model::ReviewStatus::Unreviewed => None,
            };
            let baseline = keeper_size
                .or_else(|| known.iter().copied().max())
                .unwrap_or_default();
            wasted.reclaimable_bytes += known.iter().sum::<u64>().saturating_sub(baseline);
        }
        Self {
            archive: archive.into(),
            groups,
            wasted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tpt_app_media_asset_intelligence_core::AssetId;
    use tpt_app_media_asset_intelligence_model::{DuplicateGroup, MatchKind};

    fn sizes(pairs: &[(u64, u64)]) -> BTreeMap<AssetId, u64> {
        pairs
            .iter()
            .map(|(id, s)| (AssetId::new(*id), *s))
            .collect()
    }

    #[test]
    fn empty_archive_produces_empty_report() {
        let report = DedupeReport::build("main", vec![], |_| None);
        assert_eq!(report.archive, "main");
        assert_eq!(report.wasted, WastedSummary::default());
    }

    #[test]
    fn unreviewed_group_uses_largest_member_as_keeper_baseline() {
        // Sizes: 4.1 GB, 4.1 GB, 3.9 GB (spec §8 example, scaled down).
        let table = sizes(&[(1, 4100), (2, 4100), (3, 3900)]);
        let group = DuplicateGroup::new(
            14,
            vec![AssetId::new(1), AssetId::new(2), AssetId::new(3)],
            MatchKind::PerceptualNearDuplicate,
            0.98,
        )
        .unwrap();
        let report = DedupeReport::build("main", vec![group], |a| table.get(&a).copied());
        assert_eq!(report.wasted.groups, 1);
        assert_eq!(report.wasted.assets_in_groups, 3);
        // Reclaimable = total (12100) minus the largest member (4100).
        assert_eq!(report.wasted.reclaimable_bytes, 8000);
    }

    #[test]
    fn equal_size_duplicates_all_count_as_reclaimable() {
        let table = sizes(&[(1, 4100), (2, 4100)]);
        let group = DuplicateGroup::new(
            1,
            vec![AssetId::new(1), AssetId::new(2)],
            MatchKind::ExactHash,
            1.0,
        )
        .unwrap();
        let report = DedupeReport::build("main", vec![group], |a| table.get(&a).copied());
        assert_eq!(
            report.wasted.reclaimable_bytes, 4100,
            "one keeper copy retained"
        );
    }

    #[test]
    fn reviewed_group_baselines_on_the_chosen_keeper() {
        let table = sizes(&[(1, 4100), (2, 4100), (3, 3900)]);
        let mut group = DuplicateGroup::new(
            14,
            vec![AssetId::new(1), AssetId::new(2), AssetId::new(3)],
            MatchKind::PerceptualNearDuplicate,
            0.98,
        )
        .unwrap();
        // Keeper is not the largest member: reclaimable includes the larger copies too.
        group.choose_keeper(AssetId::new(3)).unwrap();
        let report = DedupeReport::build("main", vec![group], |a| table.get(&a).copied());
        assert_eq!(report.wasted.reclaimable_bytes, 8200, "4100 + 4100");
    }

    #[test]
    fn unknown_sizes_contribute_zero_not_failure() {
        let group = DuplicateGroup::new(
            1,
            vec![AssetId::new(1), AssetId::new(2)],
            MatchKind::ExactHash,
            1.0,
        )
        .unwrap();
        let report = DedupeReport::build("main", vec![group], |_| None);
        assert_eq!(report.wasted.assets_in_groups, 0);
        assert_eq!(report.wasted.reclaimable_bytes, 0);
    }

    #[test]
    fn report_serializes_with_group_review_state() {
        let mut group = DuplicateGroup::new(
            1,
            vec![AssetId::new(1), AssetId::new(2)],
            MatchKind::ExactHash,
            1.0,
        )
        .unwrap();
        group.choose_keeper(AssetId::new(1)).unwrap();
        let report = DedupeReport::build("main", vec![group], |_| Some(100));
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"state\":\"reviewed\""), "got: {json}");
        assert!(serde_json::from_str::<DedupeReport>(&json).unwrap() == report);
    }
}
