//! The in-memory archive index: the Pass 1 engine state (spec §6, §7.1, §18).
//!
//! The index maps archive-normalized paths to [`Asset`] records, assigns the
//! product's `AssetId` primary keys, and feeds both incremental rescans (the
//! size/mtime snapshot the scanner compares against, spec §18) and Pass 1
//! search (text fields per asset, spec §6.7, §10 Layer 1).
//!
//! Persistence lands with §26 step 14; this structure is the shape that gets
//! persisted, so the on-disk store will wrap — not replace — it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tpt_app_media_asset_intelligence_core::{ArchiveId, AssetFingerprint, AssetId, CoreError};
use tpt_app_media_asset_intelligence_model::Asset;

use crate::metadata::build_asset;
use crate::scan::ScannedFile;

/// The in-memory index of one archive's assets (spec §6.2).
#[derive(Debug)]
pub struct ArchiveIndex {
    archive: ArchiveId,
    assets: BTreeMap<AssetId, Asset>,
    by_path: BTreeMap<PathBuf, AssetId>,
    next_id: u64,
}

impl ArchiveIndex {
    /// Creates an empty index for an archive.
    pub fn new(archive: ArchiveId) -> Self {
        Self {
            archive,
            assets: BTreeMap::new(),
            by_path: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Number of indexed assets.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Returns `true` when nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// The archive this index belongs to.
    pub const fn archive(&self) -> ArchiveId {
        self.archive
    }

    /// All assets, in id order.
    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    /// Looks an asset up by archive-normalized path.
    pub fn get_by_path(&self, path: &Path) -> Option<&Asset> {
        let normalized = tpt_app_media_asset_intelligence_model::normalize_archive_path(path);
        self.by_path.get(&normalized).map(|id| &self.assets[id])
    }

    /// Looks an asset up by id.
    pub fn get(&self, id: AssetId) -> Option<&Asset> {
        self.assets.get(&id)
    }

    /// Indexes a scanned file: assigns the next `AssetId`, hashes the file's
    /// contents and stores the assembled [`Asset`].
    ///
    /// This is the Pass 1 core (spec §7.1): after it returns, the asset is
    /// searchable by filename/path and its technical metadata is recorded.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ConstraintViolated`] if the path is already
    /// indexed (rescans update through the incremental flow, not by
    /// re-inserting), or [`CoreError::InvalidInput`] if the file cannot be
    /// read for fingerprinting.
    pub fn index_scanned(
        &mut self,
        scanned: &ScannedFile,
        probed: &crate::metadata::Probed,
    ) -> Result<AssetId, CoreError> {
        use tpt_app_media_asset_intelligence_model::normalize_archive_path;

        let normalized = normalize_archive_path(&scanned.path);
        if self.by_path.contains_key(&normalized) {
            return Err(CoreError::ConstraintViolated(format!(
                "path already indexed: {}",
                normalized.display()
            )));
        }
        let fingerprint = AssetFingerprint::from_file(&scanned.path)
            .map_err(|e| CoreError::InvalidInput(format!("{}: {e}", scanned.path.display())))?;

        let id = AssetId::new(self.next_id);
        self.next_id += 1;
        let asset = build_asset(id, self.archive, scanned, fingerprint, probed);
        self.by_path.insert(normalized, id);
        self.assets.insert(id, asset);
        Ok(id)
    }

    /// Returns the indexed `(size, mtime)` snapshot the scanner compares
    /// against for incremental classification (spec §18).
    pub fn previous_record(&self, path: &Path) -> Option<(u64, SystemTime)> {
        self.get_by_path(path)
            .map(|a| (a.size_bytes, a.modified_time))
    }

    /// Exact-duplicate lookup: all assets whose fingerprint equals `fingerprint`.
    ///
    /// This is the input to Layer 1 duplicate-group formation (spec §8); the
    /// full detection pass (perceptual, audio) lands with the dedupe pipeline.
    pub fn assets_with_fingerprint(
        &self,
        fingerprint: &tpt_app_media_asset_intelligence_core::AssetFingerprint,
    ) -> Vec<AssetId> {
        self.assets
            .values()
            .filter(|a| &a.fingerprint == fingerprint)
            .map(|a| a.id)
            .collect()
    }

    /// Removes an asset (e.g. after the user deletes it from the archive via a
    /// reviewed action, spec §3.4).
    ///
    /// Returns `false` when the id is unknown.
    pub fn remove(&mut self, id: AssetId) -> bool {
        match self.assets.remove(&id) {
            Some(asset) => {
                self.by_path.remove(&asset.path);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// An empty probe result: enough for index mechanics; decoding is covered
    /// by `metadata.rs` unit tests and the engine integration test.
    fn probed_meta() -> crate::metadata::Probed {
        crate::metadata::Probed {
            media_type: tpt_app_media_asset_intelligence_core::MediaType::Video,
            technical_metadata: Default::default(),
        }
    }

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("tpt-mai-index-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}-{name}", std::process::id()));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents).unwrap();
        path
    }

    fn asset_by_path(index: &ArchiveIndex, id: AssetId) -> &Asset {
        index.get(id).unwrap()
    }

    #[test]
    fn indexes_scanned_files_with_monotonic_ids() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let p1 = temp_file("one.wav", b"one");
        let p2 = temp_file("two.wav", b"two");
        let s1 = ScannedFile::from_path(p1.clone()).unwrap();
        let s2 = ScannedFile::from_path(p2.clone()).unwrap();

        let id1 = index.index_scanned(&s1, &probed_meta()).unwrap();
        let id2 = index.index_scanned(&s2, &probed_meta()).unwrap();
        assert_eq!(id1.get(), 1);
        assert_eq!(id2.get(), 2);
        assert_eq!(index.len(), 2);

        let a = asset_by_path(&index, id1);
        assert_eq!(a.path, normalize(&p1));
        assert_eq!(a.archive, ArchiveId::new(1));
        assert_eq!(
            a.fingerprint,
            tpt_app_media_asset_intelligence_core::AssetFingerprint::from_slice(b"one")
        );
    }

    fn normalize(p: &Path) -> PathBuf {
        tpt_app_media_asset_intelligence_model::normalize_archive_path(p)
    }

    #[test]
    fn duplicate_paths_are_rejected_not_silently_overwritten() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let p = temp_file("dup.wav", b"data");
        let s = ScannedFile::from_path(p.clone()).unwrap();
        index.index_scanned(&s, &probed_meta()).unwrap();
        let err = index.index_scanned(&s, &probed_meta()).unwrap_err();
        assert!(err.to_string().contains("already indexed"), "got: {err}");
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn unreadable_files_are_input_errors_not_panics() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let missing = ScannedFile {
            path: PathBuf::from("Z:/definitely/missing/clip.wav"),
            size_bytes: 1,
            modified_time: SystemTime::UNIX_EPOCH,
            media_type: tpt_app_media_asset_intelligence_core::MediaType::Video,
        };
        let err = index.index_scanned(&missing, &probed_meta()).unwrap_err();
        assert!(matches!(err, CoreError::InvalidInput(_)), "got: {err:?}");
        assert!(index.is_empty());
    }

    #[test]
    fn previous_record_feeds_incremental_rescans() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let p = temp_file("inc.wav", b"payload");
        let s = ScannedFile::from_path(p.clone()).unwrap();
        let id = index.index_scanned(&s, &probed_meta()).unwrap();

        let (size, mtime) = index.previous_record(&p).expect("indexed");
        assert_eq!(size, 7);
        let a = asset_by_path(&index, id);
        assert_eq!(mtime, a.modified_time);
        assert_eq!(index.previous_record(Path::new("/not/indexed.wav")), None);
    }

    #[test]
    fn exact_duplicate_lookup_finds_content_copies() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let a = temp_file("master.wav", b"same bytes");
        let b = temp_file("copy.wav", b"same bytes");
        for p in [&a, &b] {
            let s = ScannedFile::from_path(p.clone()).unwrap();
            index.index_scanned(&s, &probed_meta()).unwrap();
        }
        let fp = tpt_app_media_asset_intelligence_core::AssetFingerprint::from_slice(b"same bytes");
        let members = index.assets_with_fingerprint(&fp);
        assert_eq!(members.len(), 2, "both copies share the fingerprint");
    }

    #[test]
    fn remove_drops_both_lookup_paths() {
        let mut index = ArchiveIndex::new(ArchiveId::new(1));
        let p = temp_file("gone.wav", b"x");
        let s = ScannedFile::from_path(p.clone()).unwrap();
        let id = index.index_scanned(&s, &probed_meta()).unwrap();

        assert!(index.remove(id));
        assert!(!index.remove(id), "second remove reports unknown");
        assert_eq!(index.get(id), None);
        assert_eq!(index.get_by_path(&p), None);
        // The id counter is not reused: the next asset gets a fresh id.
        let s2 = ScannedFile::from_path(temp_file("after.wav", b"y")).unwrap();
        let next = index.index_scanned(&s2, &probed_meta()).unwrap();
        assert_eq!(next.get(), 2);
    }
}
