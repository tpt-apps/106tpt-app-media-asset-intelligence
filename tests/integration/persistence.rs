//! On-disk durability integration test (§16): everything written to a real
//! file survives close/reopen, proving the SQLite store is the durable source
//! of truth rather than memory-only state.

use std::path::PathBuf;

use chrono::Utc;

use tpt_app_media_asset_intelligence_model::{
    AiSettings, Archive, Asset, Derivative, DerivativeKind, MatchKind, MediaType, Scene, Tag,
    TagSource, TechnicalMetadata,
};
use tpt_app_media_asset_intelligence_persistence::Store;
use uuid::Uuid;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt-persistence-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn archive(id: Uuid) -> Archive {
    Archive {
        id,
        name: "main".into(),
        roots: vec![PathBuf::from("C:\\archive")],
        watch_enabled: true,
        ai_settings: AiSettings::default(),
    }
}

fn asset(id: Uuid, archive: Uuid, name: &str) -> Asset {
    Asset {
        id,
        archive,
        path: PathBuf::from(format!("C:\\archive\\{name}")),
        fingerprint: format!("fp-{name}"),
        size_bytes: 1_048_576,
        modified_time: Utc::now(),
        media_type: MediaType::Video,
        technical_metadata: TechnicalMetadata {
            codec: "av1".into(),
            container: "matroska".into(),
            supported: true,
            width: Some(1920),
            height: Some(1080),
            duration_secs: Some(90.0),
        },
    }
}

#[test]
fn archive_and_assets_survive_reopen() {
    let dir = temp_dir();
    let db = dir.join("index.sqlite");
    let (_, asset_id) = {
        let store = Store::open(&db).expect("open new db");
        let archive = archive(Uuid::new_v4());
        store.upsert_archive(&archive).expect("store archive");
        let a = asset(Uuid::new_v4(), archive.id, "clip.mkv");
        store.upsert_asset(&a).expect("store asset");
        store
            .upsert_derivative(&Derivative {
                asset_id: a.id,
                kind: DerivativeKind::Waveform,
                path: PathBuf::from("C:\\cache\\clip.mkv.wave"),
            })
            .expect("store derivative");
        store
            .add_tag(&Tag {
                asset_id: a.id,
                label: "interview".into(),
                source: TagSource::Manual,
                confidence: None,
                model_version: None,
                rejected: false,
            })
            .expect("store tag");
        store
            .upsert_duplicate_group(&tpt_app_media_asset_intelligence_model::DuplicateGroup {
                id: Uuid::new_v4(),
                asset_ids: vec![a.id],
                match_kind: MatchKind::ExactHash,
                reviewed: false,
                keeper: None,
            })
            .expect("store group");
        store
            .replace_scenes(
                a.id,
                &[Scene {
                    asset_id: a.id,
                    index: 0,
                    start_secs: 0.0,
                    end_secs: Some(5.0),
                }],
            )
            .expect("store scenes");
        (archive.id, a.id)
    };

    let reopened = Store::open(&db).expect("reopen db");
    assert_eq!(reopened.list_archives().expect("archives").len(), 1);
    let a = reopened
        .get_asset(asset_id)
        .expect("asset")
        .expect("present");
    assert_eq!(a.fingerprint, "fp-clip.mkv");
    assert_eq!(
        reopened.derivatives_for_asset(asset_id).expect("d").len(),
        1
    );
    assert_eq!(reopened.tags_for_asset(asset_id).expect("t").len(), 1);
    assert_eq!(
        reopened
            .duplicate_groups_for_asset(asset_id)
            .expect("g")
            .len(),
        1
    );
    assert_eq!(reopened.scenes_for_asset(asset_id).expect("s").len(), 1);
    drop(reopened);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn wipes_and_jobs_are_durable() {
    let dir = temp_dir();
    let db = dir.join("index.sqlite");
    let store = Store::open(&db).expect("open new db");
    store.set_preference("ui.theme", "dark").expect("pref");
    store
        .put_job(
            &tpt_app_media_asset_intelligence_persistence::job::Job::new(
                None,
                tpt_app_media_asset_intelligence_persistence::job::JobKind::Index,
            ),
        )
        .expect("job");
    drop(store);

    let reopened = Store::open(&db).expect("reopen db");
    assert_eq!(
        reopened
            .get_preference("ui.theme")
            .expect("pref")
            .as_deref(),
        Some("dark")
    );
    assert_eq!(reopened.list_jobs().expect("jobs").len(), 1);
    drop(reopened);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn archive_delete_cascades() {
    let dir = temp_dir();
    let db = dir.join("index.sqlite");
    let store = Store::open(&db).expect("open new db");
    let a = archive(Uuid::new_v4());
    store.upsert_archive(&a).expect("archive");
    let asset_id = Uuid::new_v4();
    store
        .upsert_asset(&asset(asset_id, a.id, "clip.mkv"))
        .expect("asset");
    store
        .upsert_derivative(&Derivative {
            asset_id,
            kind: DerivativeKind::Thumbnail,
            path: PathBuf::from("C:\\cache\\clip.mkv.thumb"),
        })
        .expect("derivative");
    drop(store);

    let reopened = Store::open(&db).expect("reopen db");
    reopened.delete_archive(a.id).expect("delete archive");
    assert!(reopened.get_asset(asset_id).expect("asset").is_none());
    assert!(reopened
        .derivatives_for_asset(asset_id)
        .expect("d")
        .is_empty());
    drop(reopened);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
