//! Exact-hash duplicate detection; perceptual/audio matching via the
//! real TPT stack under `--features tpt` (spec §8).

#[cfg(feature = "tpt")]
pub mod perceptual;

use std::collections::HashMap;
use tpt_app_media_asset_intelligence_model::{AssetId, DuplicateGroup, MatchKind};
use uuid::Uuid;

/// Group asset ids by fingerprint; singletons are dropped.
pub fn exact_duplicates(fingerprints: &HashMap<AssetId, String>) -> Vec<DuplicateGroup> {
    let mut by_fp: HashMap<&str, Vec<AssetId>> = HashMap::new();
    for (id, fp) in fingerprints {
        by_fp.entry(fp.as_str()).or_default().push(*id);
    }
    by_fp
        .into_values()
        .filter(|ids| ids.len() > 1)
        .map(|asset_ids| DuplicateGroup {
            id: Uuid::new_v4(),
            asset_ids,
            match_kind: MatchKind::ExactHash,
            reviewed: false,
            keeper: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_duplicates_groups_shared_fingerprints() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let map: HashMap<AssetId, String> =
            [(a, "fp1".into()), (b, "fp1".into()), (c, "fp2".into())].into();
        let groups = exact_duplicates(&map);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].asset_ids.len(), 2);
        assert!(!groups[0].reviewed);
        assert!(groups[0].keeper.is_none());
    }

    #[test]
    fn no_duplicates_yields_no_groups() {
        let map: HashMap<AssetId, String> = [(Uuid::new_v4(), "fp1".into())].into();
        assert!(exact_duplicates(&map).is_empty());
    }

    #[test]
    fn empty_input_yields_no_groups() {
        assert!(exact_duplicates(&HashMap::new()).is_empty());
    }
}
