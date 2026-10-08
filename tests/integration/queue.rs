//! Job-queue integration test (§13.7, §18): an `Index` job left in the
//! `Active` state by a "crash" is recovered on the next app start and its
//! built-in scanner worker re-runs idempotently, persisting assets + an
//! archive-health snapshot.

use std::path::PathBuf;

use tpt_app_media_asset_intelligence_model::{AiSettings, Archive};
use tpt_app_media_asset_intelligence_persistence::{job::JobStatus, Store};
use tpt_app_media_asset_intelligence_queue::JobQueue;
use uuid::Uuid;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mai-queue-int-{tag}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn interrupted_index_job_resumes_on_reopen() {
    let dir = temp_dir("resume");
    let root = dir.join("archive");
    let db = dir.join("queue.sqlite");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.mkv"), b"aaaaaaaa").unwrap();
    std::fs::write(root.join("b.webm"), b"bbbbbbbb").unwrap();
    std::fs::write(root.join("c.opus"), b"cccccccc").unwrap();

    let archive_id = Uuid::new_v4();
    {
        // Pre-crash: the archive + its accident of an interrupted job exist.
        let store = Store::open(&db).expect("open store");
        store
            .upsert_archive(&Archive {
                id: archive_id,
                name: "main".into(),
                roots: vec![root.clone()],
                watch_enabled: false,
                ai_settings: AiSettings::default(),
            })
            .expect("archive");
        store
            .put_job(&tpt_app_media_asset_intelligence_persistence::job::Job {
                id: Uuid::new_v4(),
                archive_id: Some(archive_id),
                kind: tpt_app_media_asset_intelligence_persistence::job::JobKind::Index,
                status: JobStatus::Active,
                progress: 0.4,
                error: None,
                note: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .expect("job");
    }

    // Restart: opening the queue requeues the Active job and runs it with the
    // built-in scanner over the archive roots.
    let queue = JobQueue::open(&db).expect("reopen");
    assert_eq!(
        queue.recover_interrupted().expect("recover"),
        0,
        "recovered at open"
    );
    let before: Vec<Uuid> = queue
        .list()
        .unwrap()
        .into_iter()
        .filter(|j| j.status == JobStatus::Queued)
        .map(|j| j.id)
        .collect();
    assert_eq!(before.len(), 1);
    queue.run_until_idle().expect("run");

    let jobs = queue.list().expect("list");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, JobStatus::Done);
    assert_eq!(jobs[0].progress, 1.0);

    let store = queue.store().lock().expect("lock");
    let assets = store.assets_for_archive(archive_id).expect("assets");
    assert_eq!(assets.len(), 3, "scanner re-ran and persisted all 3 assets");
    let snaps = store
        .recent_health_snapshots(Some(archive_id), 10)
        .expect("snaps");
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].total_assets, 3);
    drop(store);
    drop(queue);

    std::fs::remove_dir_all(&dir).expect("cleanup");
}
