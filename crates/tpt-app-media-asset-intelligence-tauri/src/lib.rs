//! Desktop command surface for the Tauri shell (spec §13). Real window/menu
//! wiring and the §13.1–§13.7 screens land with the Tauri frontend; this crate
//! owns the command contracts so CLI/service/GUI share one engine. The
//! commands call the same [`handlers`] as the local HTTP service; the upcoming
//! frontend exposes them as `#[tauri::command]`s via `tauri::generate_handler`.

use std::path::Path;

use tpt_app_media_asset_intelligence_health::HealthTrend;
use tpt_app_media_asset_intelligence_model::Asset;
use tpt_app_media_asset_intelligence_persistence::Store;
use tpt_app_media_asset_intelligence_queue::JobQueue;
use tpt_app_media_asset_intelligence_service::handlers;
use uuid::Uuid;

/// Database filename shared with the CLI, so GUI and CLI talk to one archive.
pub const DB_FILENAME: &str = "tptmai.sqlite";

pub type CmdResult<T> = Result<T, String>;

/// In-process engine handle for the desktop shell (spec §13).
pub struct TauriCommands {
    queue: JobQueue,
    data_dir: std::path::PathBuf,
}

impl TauriCommands {
    /// Open (creating if needed) the persistent engine under `data_dir`.
    pub fn open_local(data_dir: impl AsRef<Path>) -> CmdResult<Self> {
        let dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let queue = JobQueue::open(&dir.join(DB_FILENAME)).map_err(|e| e.to_string())?;
        Ok(Self {
            queue,
            data_dir: dir,
        })
    }

    /// Borrow the engine from an existing handle (e.g. the worker runner).
    pub fn from_job_queue(queue: JobQueue, data_dir: impl AsRef<Path>) -> Self {
        Self {
            queue,
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    pub fn go(&self) -> CmdResult<handlers::HealthResponse> {
        handlers::health(&self.queue).map_err(|e| e.to_string())
    }

    pub fn available_archives(
        &self,
    ) -> CmdResult<Vec<tpt_app_media_asset_intelligence_model::Archive>> {
        let guard = handlers::lock(self.queue.store()).map_err(|e| e.to_string())?;
        guard.list_archives().map_err(|e| e.to_string())
    }

    pub fn search(&self, archive_id: Uuid, query: &str) -> CmdResult<Vec<handlers::MatchHit>> {
        handlers::search(&self.queue, &archive_id.to_string(), query).map_err(|e| e.to_string())
    }

    pub fn asset(&self, asset_id: Uuid) -> CmdResult<Asset> {
        handlers::asset(&self.queue, &asset_id.to_string()).map_err(|e| e.to_string())
    }

    pub fn reindex(&self, archive_id: Uuid) -> CmdResult<Uuid> {
        handlers::reindex(&self.queue, &archive_id.to_string())
            .map_err(|e| e.to_string())
            .map(|r| r.job_id)
    }

    pub fn job(&self, job_id: Uuid) -> CmdResult<handlers::JobView> {
        handlers::job(&self.queue, &job_id.to_string()).map_err(|e| e.to_string())
    }

    /// Run queued jobs to completion (the GUI's background worker runner).
    pub fn run_queue(&self) -> CmdResult<()> {
        self.queue.run_until_idle().map_err(|e| e.to_string())
    }

    pub fn health_trend(
        &self,
        archive_id: Option<Uuid>,
        limit: i64,
    ) -> CmdResult<Vec<HealthTrend>> {
        handlers::health_trend(
            &self.queue,
            archive_id.as_ref().map(|id| id.to_string()).as_deref(),
            limit,
        )
        .map_err(|e| e.to_string())
    }

    /// Backing store, for admin/power-user flows not yet surfaced.
    pub fn store(&self) -> &std::sync::Mutex<Store> {
        self.queue.store()
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

/// Frontend navigation commands (spec §13.1 routes).
#[derive(Debug, Clone)]
pub enum UiCommand {
    OpenArchive { archive_id: String },
    InspectAsset { asset_id: Uuid },
    RunSearch { query: String },
    ReviewDuplicates { group_id: String },
    OpenHealth,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands() -> (TauriCommands, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mai-tauri-{}", Uuid::new_v4()));
        let cmd = TauriCommands::open_local(&dir).unwrap();
        (cmd, dir)
    }

    #[test]
    fn open_local_creates_an_engine() {
        let (cmd, dir) = commands();
        assert!(dir.join(DB_FILENAME).exists());
        let health = cmd.go().unwrap();
        assert!(health.ok);
        assert_eq!(health.archives, 0);
        assert!(cmd.search(Uuid::nil(), "x").unwrap().is_empty());
        drop(cmd);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn full_desktop_lifecycle_index_search_trend() {
        let (cmd, dir) = commands();
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.mkv"), [0u8; 32]).unwrap();

        let archive_id = {
            let store = cmd.queue.store();
            let guard = store.lock().unwrap();
            let a = tpt_app_media_asset_intelligence_model::Archive {
                id: Uuid::new_v4(),
                name: "news".into(),
                roots: vec![root],
                watch_enabled: false,
                ai_settings: Default::default(),
            };
            guard.upsert_archive(&a).unwrap();
            a.id
        };

        let job_id = cmd.reindex(archive_id).unwrap();
        cmd.run_queue().unwrap();
        let job = cmd.job(job_id).unwrap();
        assert_eq!(job.status, "Done");
        assert_eq!(job.progress, 1.0);

        let hits = cmd.search(archive_id, "a.mkv").unwrap();
        assert!(hits.iter().any(|h| h.path.contains("a.mkv")));

        let trend = cmd.health_trend(Some(archive_id), 10).unwrap();
        assert_eq!(trend.len(), 1);
        assert_eq!(trend[0].total_assets, 1);
        assert_eq!(trend[0].delta, 0);

        let archived = cmd.available_archives().unwrap();
        assert_eq!(archived.len(), 1);

        let nav = UiCommand::InspectAsset {
            asset_id: hits[0].asset_id,
        };
        assert!(matches!(nav, UiCommand::InspectAsset { .. }));
        drop(cmd);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
