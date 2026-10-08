//! Job queue executor (spec §13.7, §18).
//!
//! The queue runs indexing, derivative-generation and tagging jobs with
//! pause/resume/cancel, up to `available_parallelism()` concurrent workers,
//! while search keeps running in the foreground process. Every transition is
//! persisted through the [`Store`](tpt_app_media_asset_intelligence_persistence::Store);
//! an interrupted run is recovered on the next start: `Active` jobs left by a
//! crashed/restarted process are requeued and re-run idempotently (unchanged
//! assets re-upsert, never duplicate — spec §18).
//!
//! Built-in workers (resolved from the persisted `JobKind` at run time, so
//! they survive restarts):
//! - [`JobKind::Index`] — real recursive scan of the archive roots
//!   ([`scan_roots`]), asset fingerprinting + probe, idempotent upsert, and a
//!   per-run [`HealthSnapshot`] (`§12`). Corrupt/unreadable files never fail
//!   the job (`§17`).
//! - [`JobKind::Derivatives`] / [`JobKind::Tagging`] — pause/resume/cancel
//!   mechanics are wired end-to-end; real work plugs in when `tpt-av-asset` /
//!   local model weights land (see `docs/INTEGRATION_RISKS.md`, §26 steps
//!   7/11). Jobs complete with a "skipped — not wired" note so the queue
//!   screen stays honest.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use thiserror::Error;
use uuid::Uuid;

pub use tpt_app_media_asset_intelligence_persistence::job::{Job, JobKind, JobStatus};
use tpt_app_media_asset_intelligence_persistence::Store;

use tpt_app_media_asset_intelligence_health::{find_corrupt, find_missing, HealthSnapshot};
use tpt_app_media_asset_intelligence_ingest::{probe_file, scan_roots};
use tpt_app_media_asset_intelligence_model::{Archive, Asset, AssetId};

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("store error: {0}")]
    Store(#[from] tpt_app_media_asset_intelligence_persistence::StoreError),
    #[error("job {0} not found")]
    NotFound(Uuid),
    #[error("archive {0} not found")]
    ArchiveNotFound(Uuid),
    #[error("no work registered for job {0}")]
    NoWork(Uuid),
    #[error("control operation failed: {0}")]
    Control(String),
    #[error("shared state lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Acquire a shared lock, mapping poison (a panicked holder) to an error so
/// one bad worker never panics the whole process (§26 step 24).
fn guard<'a, T>(
    m: &'a Mutex<T>,
    what: &'a str,
) -> Result<std::sync::MutexGuard<'a, T>, QueueError> {
    m.lock()
        .map_err(|_| QueueError::LockPoisoned(what.to_string()))
}

/// Cooperative context handed to a job's work function. Poll the flags
/// often; report progress through [`JobContext::report_progress`]
/// (0.0..=1.0).
pub struct JobContext<'a> {
    pub is_cancelled: &'a dyn Fn() -> bool,
    pub is_paused: &'a dyn Fn() -> bool,
    pub progress: &'a mut dyn FnMut(f64),
}

impl JobContext<'_> {
    pub fn is_cancelled(&self) -> bool {
        (self.is_cancelled)()
    }

    pub fn is_paused(&self) -> bool {
        (self.is_paused)()
    }

    pub fn report_progress(&mut self, p: f64) {
        (self.progress)(p.clamp(0.0, 1.0));
    }
}

/// A job's work. Returns `Ok(note)` on completion or `Err(message)` on
/// failure (persisted as `Failed`).
pub type JobWork = Box<dyn Fn(&mut JobContext<'_>) -> Result<String, String> + Send>;

#[derive(Debug, Default)]
struct JobControl {
    paused: AtomicBool,
    cancelled: AtomicBool,
}

impl JobControl {
    fn should_stop(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

type SharedStore = Arc<Mutex<Store>>;

/// The job queue: durable job records in the store plus a bounded worker
/// pool that executes them concurrently.
pub struct JobQueue {
    store: SharedStore,
    /// Max concurrently-running workers (§18: `available_parallelism()`).
    parallel: usize,
    controls: Arc<Mutex<HashMap<Uuid, Arc<JobControl>>>>,
    /// In-process custom work by job id (built-ins are resolved at run time
    /// and do not need this).
    work: Arc<Mutex<HashMap<Uuid, JobWork>>>,
}

impl JobQueue {
    pub fn open(path: &Path) -> Result<Self, QueueError> {
        Self::from_store(Store::open(path)?)
    }

    pub fn in_memory() -> Result<Self, QueueError> {
        Self::from_store(Store::in_memory()?)
    }

    /// Build a queue; interrupted `Active` jobs are requeued so a restart
    /// resumes the scan instead of dropping work on the floor (§18).
    pub fn from_store(store: Store) -> Result<Self, QueueError> {
        let store = Arc::new(Mutex::new(store));
        let parallel = std::thread::available_parallelism()
            .map(|n| n.get().clamp(1, 8))
            .unwrap_or(2);
        let queue = Self {
            store,
            parallel,
            controls: Arc::new(Mutex::new(HashMap::new())),
            work: Arc::new(Mutex::new(HashMap::new())),
        };
        queue.recover_interrupted()?;
        Ok(queue)
    }

    pub fn store(&self) -> &Mutex<Store> {
        &self.store
    }

    pub fn parallel(&self) -> usize {
        self.parallel
    }

    /// Requeue jobs an earlier run left `Active` (crash/restart).
    pub fn recover_interrupted(&self) -> Result<usize, QueueError> {
        let store = guard(&*self.store, "store")?;
        let mut recovered = 0;
        for job in store.list_jobs()? {
            if job.status == JobStatus::Active {
                let requeued = Job {
                    id: job.id,
                    status: JobStatus::Queued,
                    progress: 0.0,
                    error: None,
                    updated_at: chrono::Utc::now(),
                    ..job
                };
                store.put_job(&requeued)?;
                recovered += 1;
            }
        }
        Ok(recovered)
    }

    /// Persist a new queued job; work is resolved from `JobKind` at run time.
    pub fn enqueue(&self, archive_id: Option<Uuid>, kind: JobKind) -> Result<Uuid, QueueError> {
        if let Some(id) = archive_id {
            let store = guard(&*self.store, "store")?;
            if store.get_archive(id)?.is_none() {
                return Err(QueueError::ArchiveNotFound(id));
            }
        }
        let job = Job::new(archive_id, kind);
        guard(&*self.store, "store")?.put_job(&job)?;
        guard(&*self.controls, "controls")?.insert(job.id, Arc::new(JobControl::default()));
        Ok(job.id)
    }

    /// Like [`JobQueue::enqueue`] but with caller-supplied work (used by
    /// shells that have a concrete pipeline and by tests).
    pub fn enqueue_custom(
        &self,
        archive_id: Option<Uuid>,
        kind: JobKind,
        work: JobWork,
    ) -> Result<Uuid, QueueError> {
        let id = self.enqueue(archive_id, kind)?;
        guard(&*self.work, "work")?.insert(id, work);
        Ok(id)
    }

    /// A durable `Index` job over the archive's roots (spec §7, §13.7).
    pub fn enqueue_index(&self, archive_id: Uuid) -> Result<Uuid, QueueError> {
        self.enqueue(Some(archive_id), JobKind::Index)
    }

    /// A derivative-generation job. Not wired until `tpt-av-asset` resolves
    /// (§26 step 7); completes with a skip note so the queue is observable.
    pub fn enqueue_derivatives(&self, archive_id: Option<Uuid>) -> Result<Uuid, QueueError> {
        self.enqueue(archive_id, JobKind::Derivatives)
    }

    /// A tagging job; runs after local model weights land (§11, §26 step 11).
    pub fn enqueue_tagging(&self, archive_id: Option<Uuid>) -> Result<Uuid, QueueError> {
        self.enqueue(archive_id, JobKind::Tagging)
    }

    pub fn list(&self) -> Result<Vec<Job>, QueueError> {
        Ok(guard(&*self.store, "store")?.list_jobs()?)
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Job>, QueueError> {
        Ok(guard(&*self.store, "store")?.get_job(id)?)
    }

    /// Run every queued job (in batches of `parallel`) until the queue is
    /// empty, then return. Runs synchronously on the caller's thread.
    pub fn run_until_idle(&self) -> Result<(), QueueError> {
        loop {
            let queued: Vec<Uuid> = self
                .list()?
                .into_iter()
                .filter(|j| j.status == JobStatus::Queued)
                .map(|j| j.id)
                .collect();
            if queued.is_empty() {
                return Ok(());
            }
            let mut handles = Vec::new();
            for id in queued.into_iter().take(self.parallel) {
                handles.push(self.spawn_worker(id));
            }
            for h in handles {
                h.join()
                    .map_err(|_| QueueError::Control("worker panicked".into()))?;
            }
        }
    }

    /// Run the queue on a background thread (used by shells and tests).
    pub fn run_in_background(&self) -> JoinHandle<Result<(), QueueError>> {
        let queue = self.clone_ref();
        thread::spawn(move || queue.run_until_idle())
    }

    pub fn cancel(&self, id: Uuid) -> Result<(), QueueError> {
        let store = guard(&*self.store, "store")?;
        let Some(job) = store.get_job(id)? else {
            return Err(QueueError::NotFound(id));
        };
        if matches!(
            job.status,
            JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled
        ) {
            return Err(QueueError::Control(format!("job {id} already finished")));
        }
        if let Some(control) = guard(&*self.controls, "controls")?.get(&id) {
            control.cancelled.store(true, Ordering::SeqCst);
        }
        store.set_status(id, JobStatus::Cancelled, Some("cancelled by user".into()))?;
        Ok(())
    }

    pub fn pause(&self, id: Uuid) -> Result<(), QueueError> {
        let store = guard(&*self.store, "store")?;
        let Some(job) = store.get_job(id)? else {
            return Err(QueueError::NotFound(id));
        };
        if !matches!(job.status, JobStatus::Active | JobStatus::Queued) {
            return Err(QueueError::Control(format!("job {id} is not runnable")));
        }
        if let Some(control) = guard(&*self.controls, "controls")?.get(&id) {
            control.paused.store(true, Ordering::SeqCst);
        }
        store.set_status(id, JobStatus::Paused, None)?;
        Ok(())
    }

    pub fn resume(&self, id: Uuid) -> Result<(), QueueError> {
        let store = guard(&*self.store, "store")?;
        let Some(job) = store.get_job(id)? else {
            return Err(QueueError::NotFound(id));
        };
        if job.status != JobStatus::Paused {
            return Err(QueueError::Control(format!("job {id} is not paused")));
        }
        if let Some(control) = guard(&*self.controls, "controls")?.get(&id) {
            control.paused.store(false, Ordering::SeqCst);
        }
        store.set_status(id, JobStatus::Active, None)?;
        Ok(())
    }

    /// A second handle onto the same queue (shared store, controls and work).
    pub fn clone_ref(&self) -> Self {
        Self {
            store: self.store.clone(),
            parallel: self.parallel,
            controls: self.controls.clone(),
            work: self.work.clone(),
        }
    }

    fn spawn_worker(&self, id: Uuid) -> JoinHandle<()> {
        let store = self.store.clone();
        let control = self
            .controls
            .lock()
            .map(|g| {
                g.get(&id)
                    .cloned()
                    .unwrap_or_else(|| Arc::new(JobControl::default()))
            })
            .unwrap_or_else(|_| Arc::new(JobControl::default()));
        let custom = self
            .work
            .lock()
            .map(|mut g| g.remove(&id))
            .unwrap_or_else(|_| {
                eprintln!("queue worker {id}: work map poisoned; falling back to built-in");
                None
            });
        thread::spawn(move || {
            if let Err(e) = run_job(&store, id, control, custom) {
                eprintln!("queue worker {id}: {e}");
            }
        })
    }
}

fn run_job(
    store: &SharedStore,
    id: Uuid,
    control: Arc<JobControl>,
    custom: Option<JobWork>,
) -> Result<(), QueueError> {
    guard(store, "store")?.set_status(id, JobStatus::Active, None)?;
    let work = custom
        .map(Ok)
        .unwrap_or_else(|| resolve_builtin_work(store, id))?;
    let mut last_percent = -1.0;
    let mut report_progress = |p: f64| {
        let pct = ((p.clamp(0.0, 1.0)) * 100.0).round();
        if pct != last_percent {
            last_percent = pct;
            if let Ok(lock) = guard(store, "store") {
                let _ = lock.set_progress(id, p);
            }
        }
    };
    let is_cancelled = || control.should_stop();
    let is_paused = || control.paused.load(Ordering::SeqCst);
    let mut ctx = JobContext {
        is_cancelled: &is_cancelled,
        is_paused: &is_paused,
        progress: &mut report_progress,
    };
    match work(&mut ctx) {
        Ok(note) => {
            let lock = guard(store, "store")?;
            let mut job = lock.get_job(id)?.ok_or(QueueError::NotFound(id))?;
            if control.should_stop() {
                job.status = JobStatus::Cancelled;
                job.error = Some("cancelled by user".into());
            } else {
                job.status = JobStatus::Done;
                job.progress = 1.0;
                job.note = Some(note);
                job.error = None;
            }
            job.updated_at = chrono::Utc::now();
            lock.put_job(&job)?;
            Ok(())
        }
        Err(message) => {
            let lock = guard(store, "store")?;
            let mut job = lock.get_job(id)?.ok_or(QueueError::NotFound(id))?;
            job.status = if control.should_stop() {
                JobStatus::Cancelled
            } else {
                JobStatus::Failed
            };
            job.error = Some(message);
            job.updated_at = chrono::Utc::now();
            lock.put_job(&job)?;
            Ok(())
        }
    }
}

/// Map a persisted `JobKind` back to a worker so restarted jobs re-run
/// without in-process registration.
fn resolve_builtin_work(store: &SharedStore, id: Uuid) -> Result<JobWork, QueueError> {
    let job = guard(store, "store")?
        .get_job(id)?
        .ok_or(QueueError::NotFound(id))?;
    let store = store.clone();
    match job.kind {
        JobKind::Index => {
            let archive_id = job
                .archive_id
                .ok_or_else(|| QueueError::Control("index job has no archive".into()))?;
            Ok(Box::new(move |ctx: &mut JobContext<'_>| {
                let archive = {
                    let store = guard(&*store, "store").map_err(|e| e.to_string())?;
                    store
                        .get_archive(archive_id)
                        .map_err(|e| format!("archive lookup: {e}"))?
                        .ok_or_else(|| format!("archive {archive_id} not found"))?
                };
                run_index_worker(&store, &archive, ctx)
            }))
        }
        JobKind::Derivatives => Ok(Box::new(|ctx: &mut JobContext<'_>| {
            ctx.report_progress(1.0);
            Ok("skipped: derivative generation requires tpt-av-asset wiring (see INTEGRATION_RISKS)".to_string())
        })),
        JobKind::Tagging => Ok(Box::new(|ctx: &mut JobContext<'_>| {
            ctx.report_progress(1.0);
            Ok("skipped: local tagging models not wired yet (see INTEGRATION_RISKS)".to_string())
        })),
    }
}

fn run_index_worker(
    store: &SharedStore,
    archive: &Archive,
    ctx: &mut JobContext<'_>,
) -> Result<String, String> {
    let roots = archive.roots.clone();
    let (found, errors) = scan_roots(&roots);
    let assets: Vec<Asset> = found
        .iter()
        .filter(|f| !f.fingerprint.is_empty())
        .map(|f| {
            let probe = probe_file(&f.path);
            Asset {
                id: asset_id_for(&archive.id, &f.path),
                archive: archive.id,
                path: f.path.clone(),
                fingerprint: f.fingerprint.clone(),
                size_bytes: f.size_bytes,
                modified_time: archive_modified(&f.path),
                media_type: probe.media_type,
                technical_metadata: probe.technical,
            }
        })
        .collect();
    let total = assets.len().max(1) as f64;
    for (i, asset) in assets.iter().enumerate() {
        if ctx.is_cancelled() {
            return Err("cancelled".to_string());
        }
        while ctx.is_paused() {
            thread::sleep(std::time::Duration::from_millis(10));
        }
        let lock = guard(store, "store").map_err(|e| e.to_string())?;
        lock.upsert_asset(asset).map_err(|e| e.to_string())?;
        drop(lock);
        ctx.report_progress((i as f64 + 1.0) / total);
    }
    // Per-run archive-health snapshot: reconcile the full archive set so a
    // moved/deleted file surfaces as missing right after a rescan (§12).
    let snapshot = {
        let lock = guard(store, "store").map_err(|e| e.to_string())?;
        let archive_assets = lock
            .assets_for_archive(archive.id)
            .map_err(|e| e.to_string())?;
        let missing: Vec<AssetId> = find_missing(&archive_assets)
            .into_iter()
            .map(|a| a.id)
            .collect();
        let corrupt: Vec<AssetId> = find_corrupt(&archive_assets)
            .into_iter()
            .map(|a| a.id)
            .collect();
        HealthSnapshot {
            taken_at: chrono::Utc::now(),
            total_assets: archive_assets.len(),
            missing_files: missing,
            corrupt_assets: corrupt,
            orphaned_derivatives: Vec::new(),
            duplicate_waste_bytes: 0,
        }
    };
    let lock = guard(store, "store").map_err(|e| e.to_string())?;
    lock.insert_health_snapshot(Some(archive.id), &snapshot)
        .map_err(|e| e.to_string())?;
    ctx.report_progress(1.0);
    let note = if errors.is_empty() {
        format!("indexed {} asset(s)", found.len())
    } else {
        format!(
            "indexed {} asset(s), {} warning(s) (see scan errors)",
            found.len(),
            errors.len()
        )
    };
    Ok(note)
}

/// Deterministic, resumable asset id per `(archive, path)` so re-indexing
/// never duplicates rows and interrupted scans reconcile cleanly (§18).
fn asset_id_for(archive: &Uuid, path: &Path) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("tptmai:{archive}:{}", path.display()).as_bytes(),
    )
}

fn archive_modified(path: &Path) -> chrono::DateTime<chrono::Utc> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(Into::into)
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tpt_app_media_asset_intelligence_model::AiSettings;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mai-queue-{tag}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn archive_in(queue: &Mutex<Store>, name: &str, root: PathBuf) -> (Uuid, Archive) {
        let id = Uuid::new_v4();
        let archive = Archive {
            id,
            name: name.into(),
            roots: vec![root],
            watch_enabled: false,
            ai_settings: AiSettings::default(),
        };
        queue
            .lock()
            .expect("lock")
            .upsert_archive(&archive)
            .expect("archive");
        (id, archive)
    }

    fn make_files(root: &Path, names: &[&str]) {
        for (i, n) in names.iter().enumerate() {
            let mut f = std::fs::File::create(root.join(n)).expect("file");
            f.write_all(&[i as u8; 64]).expect("write");
        }
    }

    #[test]
    fn index_job_runs_to_done_and_persists() {
        let dir = temp_dir("index");
        let root = dir.join("archive");
        let db = dir.join("queue.sqlite");
        std::fs::create_dir_all(&root).unwrap();
        make_files(&root, &["a.mkv", "b.webm", "c.opus"]);

        let queue = JobQueue::open(&db).expect("open");
        let (archive_id, _) = archive_in(queue.store(), "main", root);
        let id = queue.enqueue_index(archive_id).expect("enqueue");
        queue.run_until_idle().expect("run");

        let job = queue.get(id).expect("get").expect("present");
        assert_eq!(job.status, JobStatus::Done);
        assert_eq!(job.progress, 1.0);
        let store = queue.store().lock().expect("lock");
        let assets = store.assets_for_archive(archive_id).expect("assets");
        assert_eq!(assets.len(), 3);
        let snaps = store
            .recent_health_snapshots(Some(archive_id), 10)
            .expect("snaps");
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].total_assets, 3);
        drop(store);
        drop(queue);

        // Reopen: the db is durable and re-indexing stays idempotent.
        let reopened = JobQueue::open(&db).expect("reopen");
        let store = reopened.store().lock().expect("lock");
        let assets = store.assets_for_archive(archive_id).expect("assets");
        assert_eq!(assets.len(), 3);
        drop(store);
        drop(reopened);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn cancel_stops_cooperative_work() {
        let queue = JobQueue::in_memory().expect("queue");
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let done_w = done.clone();
        let work: JobWork = Box::new(move |ctx: &mut JobContext<'_>| {
            while done_w.load(Ordering::SeqCst) < 50_000 {
                std::thread::sleep(std::time::Duration::from_micros(50));
                if ctx.is_cancelled() {
                    return Err("cancelled".into());
                }
                done_w.fetch_add(1, Ordering::SeqCst);
                if done_w.load(Ordering::SeqCst).is_multiple_of(1000) {
                    ctx.report_progress(done_w.load(Ordering::SeqCst) as f64 / 50_000.0);
                }
            }
            Ok("finished".into())
        });
        let id = queue
            .enqueue_custom(None, JobKind::Index, work)
            .expect("enqueue");
        let handle = queue.run_in_background();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(done.load(Ordering::SeqCst) > 0);
        queue.cancel(id).expect("cancel");
        handle.join().expect("join").expect("run");
        let job = queue.get(id).expect("get").expect("present");
        assert_eq!(job.status, JobStatus::Cancelled);
        assert!(job.progress < 1.0);
    }

    #[test]
    fn pause_and_resume() {
        let queue = JobQueue::in_memory().expect("queue");
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let done_w = done.clone();
        let work: JobWork = Box::new(move |ctx: &mut JobContext<'_>| {
            while done_w.load(Ordering::SeqCst) < 5_000 {
                std::thread::sleep(std::time::Duration::from_micros(50));
                if ctx.is_cancelled() {
                    return Err("cancelled".into());
                }
                while ctx.is_paused() {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                done_w.fetch_add(1, Ordering::SeqCst);
                if done_w.load(Ordering::SeqCst).is_multiple_of(1000) {
                    ctx.report_progress(done_w.load(Ordering::SeqCst) as f64 / 5_000.0);
                }
            }
            Ok("finished".into())
        });
        let id = queue
            .enqueue_custom(None, JobKind::Tagging, work)
            .expect("enqueue");
        let handle = queue.run_in_background();
        std::thread::sleep(std::time::Duration::from_millis(50));
        queue.pause(id).expect("pause");
        std::thread::sleep(std::time::Duration::from_millis(30));
        let paused = queue.get(id).expect("get").expect("present");
        assert_eq!(paused.status, JobStatus::Paused);
        let before = done.load(Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert_eq!(
            done.load(Ordering::SeqCst),
            before,
            "no progress while paused"
        );
        queue.resume(id).expect("resume");
        handle.join().expect("join").expect("run");
        let job = queue.get(id).expect("get").expect("present");
        assert_eq!(job.status, JobStatus::Done);
    }

    #[test]
    fn interrupt_recovery_requeues_active_jobs() {
        let dir = temp_dir("recover");
        let db = dir.join("queue.sqlite");
        {
            let store = Store::open(&db).expect("open");
            let job = Job {
                id: Uuid::new_v4(),
                archive_id: None,
                kind: JobKind::Index,
                status: JobStatus::Active,
                progress: 0.5,
                error: None,
                note: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            store.put_job(&job).expect("put");
        }
        let queue = JobQueue::open(&db).expect("reopen");
        // Opening requeues the crash-left Active job; a second pass is a no-op.
        assert_eq!(queue.recover_interrupted().expect("recover"), 0);
        let job = queue.list().expect("list")[0].clone();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.progress, 0.0);
        drop(queue);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn queue_respects_parallelism_limit() {
        let queue = JobQueue::in_memory().expect("queue");
        assert!(queue.parallel() >= 1);
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peak = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut ids = Vec::new();
        for _ in 0..4 {
            let active = active.clone();
            let peak = peak.clone();
            let work: JobWork = Box::new(move |_ctx: &mut JobContext<'_>| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(40));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok("done".into())
            });
            ids.push(
                queue
                    .enqueue_custom(None, JobKind::Index, work)
                    .expect("enqueue"),
            );
        }
        queue.run_until_idle().expect("run");
        assert!(peak.load(Ordering::SeqCst) <= queue.parallel());
        for id in ids {
            assert_eq!(
                queue.get(id).expect("get").expect("job").status,
                JobStatus::Done
            );
        }
    }

    #[test]
    fn cancel_finished_job_is_an_error() {
        let queue = JobQueue::in_memory().expect("queue");
        let id = queue.enqueue_tagging(None).expect("enqueue");
        queue.run_until_idle().expect("run");
        assert!(queue.cancel(id).is_err());
    }

    #[test]
    fn worker_panic_is_contained_and_job_stays_requeueable() {
        let queue = JobQueue::in_memory().expect("queue");
        let id = queue
            .enqueue_custom(
                None,
                JobKind::Index,
                Box::new(|_ctx: &mut JobContext<'_>| panic!("worker exploded")),
            )
            .expect("enqueue");
        // A panicking worker must surface as an error, not crash the process.
        let err = queue.run_until_idle().expect_err("run reports panic");
        assert!(format!("{err}").contains("panicked"));
        // The queue stays fully usable afterwards.
        assert!(queue.list().is_ok());
        let second = queue.enqueue_tagging(None).expect("enqueue after panic");
        queue.run_until_idle().expect("run after panic");
        assert_eq!(
            queue.get(second).expect("get").expect("job").status,
            JobStatus::Done
        );
        // The interrupted job is still requeueable on recovery.
        assert_eq!(queue.recover_interrupted().expect("recover"), 1);
        let _ = id;
    }
}
