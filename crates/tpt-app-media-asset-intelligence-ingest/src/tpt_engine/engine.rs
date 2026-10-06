//! Owned engine home: AssetDb + CacheStorage + background pipeline.

use super::direct::direct_probe;
use std::path::{Path, PathBuf};
use tpt_app_media_asset_intelligence_model::tpt_convert::wired as convert;
use tpt_app_media_asset_intelligence_model::Asset;
use tpt_av_asset_cache::CacheStorage;
use tpt_av_asset_db::AssetDb;
use tpt_av_asset_pipeline::{probe_media_info, AssetImporter, ProcessingPipeline};

/// Owned engine home: database + cache storage + background pipeline.
pub struct Engine {
    pub db: AssetDb,
    pub storage: CacheStorage,
    pub pipeline: ProcessingPipeline,
    pub home: PathBuf,
}

impl Engine {
    /// Opens (or creates) the engine home at `home`.
    pub fn open(home: &Path, workers: usize) -> Result<Self, String> {
        let db = AssetDb::open(&home.join("assets.redb")).map_err(|e| e.to_string())?;
        let storage = CacheStorage::new(home.join("derivatives"));
        storage.ensure_layout().map_err(|e| e.to_string())?;
        let mut pipeline = ProcessingPipeline::new(workers.max(1)).map_err(|e| e.to_string())?;
        pipeline.attach_db(db.clone());
        pipeline.start().map_err(|e| e.to_string())?;
        let _ = pipeline.recover_interrupted(&db, &storage);
        Ok(Self {
            db,
            storage,
            pipeline,
            home: home.to_path_buf(),
        })
    }

    /// Probe one file with the real decoder stacks, enforcing the
    /// open-codec allowlist. `None` = unrecognised (caller indexes by
    /// path/fingerprint as `unsupported`).
    pub fn probe(path: &Path) -> Option<(tpt_av_asset_utils::MediaInfo, bool)> {
        match probe_media_info(path) {
            Ok(info) => {
                let tech = convert::technical_from_media_info(&info);
                Some((info, tech.supported))
            }
            Err(_) => direct_probe(path),
        }
    }

    /// Import one file: probe → index → schedule background jobs.
    /// Unsupported files return `Err` (caller records them as
    /// `unsupported`; never a crash, spec §17).
    pub fn import(&self, path: &Path, archive: uuid::Uuid) -> Result<(Asset, Vec<u64>), String> {
        let (info, supported) =
            Self::probe(path).ok_or_else(|| format!("unrecognised file: {}", path.display()))?;
        if !supported {
            return Err(format!(
                "unsupported codec (indexed as unsupported): {}",
                path.display()
            ));
        }
        let importer =
            AssetImporter::new(self.pipeline.clone(), self.db.clone(), self.storage.clone());
        let (av_id, jobs) = importer.import_with_jobs(path).map_err(|e| e.to_string())?;
        let tech = convert::technical_from_media_info(&info);
        let asset = Asset {
            id: convert::asset_uuid_from_av_id(av_id),
            archive,
            path: path.to_path_buf(),
            fingerprint: crate::scan::scan_fingerprint(path),
            size_bytes: av_id.size(),
            modified_time: chrono::Utc::now(),
            media_type: convert::media_type_from(&info),
            technical_metadata: tech,
        };
        Ok((asset, jobs.into_iter().map(|j| j.0).collect()))
    }

    /// Derivative paths for an asset (paths only — bytes live under the
    /// engine home, separate from DB and source archive, spec §16).
    pub fn derivative_paths(&self, asset: &Asset) -> DerivativePaths {
        let av_id = av_id_for(asset);
        DerivativePaths {
            waveform: self.storage.waveform_path(av_id),
            thumbnails: self.storage.thumbnail_dir(av_id),
            proxy_video: self.storage.proxy_path(av_id, "mp4"),
            proxy_audio: self.storage.proxy_path(av_id, "flac"),
        }
    }
}

/// Derivative locations for one asset.
#[derive(Debug, Clone)]
pub struct DerivativePaths {
    pub waveform: PathBuf,
    pub thumbnails: PathBuf,
    pub proxy_video: PathBuf,
    pub proxy_audio: PathBuf,
}

fn av_id_for(asset: &Asset) -> tpt_av_asset_utils::AssetId {
    tpt_av_asset_utils::AssetId::from_path(&asset.path)
        .unwrap_or_else(|_| tpt_av_asset_utils::AssetId::from_parts(0, 0, asset.size_bytes))
}
