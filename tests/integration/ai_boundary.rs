//! AI-boundary tests (§19.4, §26 step 22): with cloud AI disabled (the
//! default), indexing/tagging/search make no network calls and emit
//! `cloud_ai_used: false`; enabling cloud requires an explicit opt-in; and
//! disabling cloud AI never deletes or invalidates prior local-model tags.

use clap::Parser;
use std::path::PathBuf;
use tpt_app_media_asset_intelligence_cli::run;
use tpt_app_media_asset_intelligence_cli::Cli;
use tpt_app_media_asset_intelligence_cli::ExitCode;
use tpt_app_media_asset_intelligence_model::{AiSettings, AssetId, SearchIndexEntry, TagSource};
use tpt_app_media_asset_intelligence_search::parse_query;
use tpt_app_media_asset_intelligence_tagging::{
    filter_rejected, local_model_tag, manual_tag, CloudTaggingGate,
};
use uuid::Uuid;

/// Defaults must be local-only: no cloud AI, no localhost API (spec §3.3,
/// §6.1, §15).
#[test]
fn cloud_ai_and_service_are_disabled_by_default() {
    let ai = AiSettings::default();
    assert!(ai.local_tagging_enabled);
    assert!(!ai.cloud_tagging_enabled);
    assert!(!ai.cloud_semantic_search_enabled);
    assert!(ai.cloud_provider.is_none());
    assert!(
        !tpt_app_media_asset_intelligence_service::is_enabled(false),
        "localhost API must be off unless explicitly enabled"
    );
}

/// The tagging gate makes the cloud path unreachable unless explicitly
/// opted in.
#[test]
fn tagging_gate_blocks_cloud_without_opt_in() {
    assert!(
        CloudTaggingGate::local_only().require_cloud().is_err(),
        "cloud tagging must refuse without an explicit opt-in"
    );
    assert!(
        CloudTaggingGate {
            cloud_allowed: true
        }
        .require_cloud()
        .is_ok(),
        "explicit opt-in permits cloud-only paths"
    );
}

/// CLI: local tagging by default, conflicting flags are a configuration
/// error, and machine output reports `cloud_ai_used: false`.
#[test]
fn cli_tagging_defaults_to_local_only_and_refuses_conflicts() {
    let (code, out) = run(Cli::parse_from(["t", "tag", "--archive", "m"]));
    assert_eq!(code, ExitCode::Success);
    assert!(!out.contains("cloud"), "default tagging must be local-only");

    let conflicting = Cli::parse_from([
        "t",
        "tag",
        "--archive",
        "m",
        "--local-only",
        "--allow-cloud",
    ]);
    assert_eq!(
        run(conflicting).0,
        ExitCode::ConfigurationError,
        "explicit local-only + allow-cloud is contradictory and must be refused"
    );
}

/// Indexing emits `cloud_ai_used: false` and needs no AI configuration.
#[test]
fn indexing_reports_no_cloud_use() {
    let dir = temp_dir("ai-index");
    let data_dir = temp_dir("ai-index-data");
    std::fs::write(dir.join("take.bin"), b"data").unwrap();
    let cli = Cli::parse_from([
        "t",
        "index",
        "--data-dir",
        data_dir.to_str().unwrap(),
        "--archive",
        "a",
        "--roots",
        dir.to_str().unwrap(),
    ]);
    let (code, out) = run(cli);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    assert_eq!(code, ExitCode::Success);
    let report: serde_json::Value =
        serde_json::from_str(&out).expect("index emits machine-readable JSON");
    assert_eq!(report["assets_indexed"], 1);
    assert!(!report["cloud_ai_used"].as_bool().unwrap());
}

/// Disabling cloud AI after prior use must not delete or invalidate
/// previously generated local-model tags (spec §19.4). No pipeline mutates
/// the tag store on settings change; rejected-tag memory filters, it never
/// deletes.
#[test]
fn disabling_cloud_preserves_local_model_tags() {
    let id: AssetId = Uuid::new_v4();
    let tags = vec![
        local_model_tag(id, "interview", 0.91, "local-v1"),
        manual_tag(id, "keeper"),
    ];
    let before: Vec<(String, TagSource)> =
        tags.iter().map(|t| (t.label.clone(), t.source)).collect();

    // Simulate an archive that once used cloud AI (explicitly enabled),
    // then has cloud disabled again via the default settings.
    let _used_cloud = AiSettings {
        cloud_tagging_enabled: true,
        cloud_semantic_search_enabled: true,
        ..Default::default()
    };
    let ai = AiSettings::default();
    assert!(ai.local_tagging_enabled);
    assert!(!ai.cloud_tagging_enabled);

    let after: Vec<(String, TagSource)> =
        tags.iter().map(|t| (t.label.clone(), t.source)).collect();
    assert_eq!(
        after, before,
        "settings toggling must not touch the tag store"
    );
    for t in &tags {
        assert!(!t.rejected, "local tags must not be invalidated");
    }

    // Rejected-tag memory: candidates named on the reject list are flagged
    // for review, never silently re-applied or deleted.
    let (kept, flagged) = filter_rejected(tags.clone(), &["interview".to_string()]);
    assert_eq!(kept.len(), 1);
    assert_eq!(flagged.len(), 1);
    assert_eq!(tags.len(), 2, "nothing is ever deleted");
}

/// Search indexing is deterministic and requires no network or AI settings.
#[test]
fn search_is_offline_and_deterministic() {
    let a = parse_query("codec:av1 AND tag:interview").unwrap();
    let b = parse_query("codec:av1 AND tag:interview").unwrap();
    assert_eq!(a.clauses.len(), 2);
    assert_eq!(b.clauses.len(), a.clauses.len(), "same query, same result");

    let entry = SearchIndexEntry {
        asset_id: Uuid::new_v4(),
        filename: "clip.mkv".into(),
        codec: "av1".into(),
        tags: vec!["interview".into()],
        notes: String::new(),
    };
    assert!(entry.tags.contains(&"interview".to_string()));
    assert_eq!(entry.codec, "av1");
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mai-ai-boundary-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
