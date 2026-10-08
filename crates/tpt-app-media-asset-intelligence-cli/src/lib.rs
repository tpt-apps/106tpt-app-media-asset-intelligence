<<<<<<< HEAD
//! CLI: index / search / dedupe / tag over the shared engine (spec §14).
//! Same engine as the GUI: `index` enqueues a durable job on the SQLite
//! store through the job queue, persists assets + archive-health snapshots,
//! and reconciles exact-duplicate groups. Local-only by default; cloud paths
//! require explicit flags. Exit codes are stable: 0 SUCCESS, 1 PARTIAL,
//! 2 INDEXING_FAILED, 3 SEARCH_FAILED, 4 CONFIGURATION_ERROR, 5 INPUT_ERROR,
//! 6 INTERNAL_ERROR.

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use tpt_app_media_asset_intelligence_dedupe::exact_duplicates;
use tpt_app_media_asset_intelligence_model::{AiSettings, Archive, Asset, SearchIndexEntry};
use tpt_app_media_asset_intelligence_persistence::job::JobStatus;
use tpt_app_media_asset_intelligence_persistence::Store;
use tpt_app_media_asset_intelligence_queue::JobQueue;
use tpt_app_media_asset_intelligence_search::{explain_match, matches, parse_query};

/// Data directory override (also honored: `TMAI_DATA_DIR`, then
/// `%LOCALAPPDATA%`/home).
const DATA_DIR_ENV: &str = "TMAI_DATA_DIR";
const DB_FILE: &str = "tptmai.sqlite";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    PartialSuccess = 1,
    IndexingFailed = 2,
    SearchFailed = 3,
    ConfigurationError = 4,
    InputError = 5,
    InternalError = 6,
}

#[derive(Parser, Debug)]
#[command(
    name = "tpt-media-asset-intel",
    version,
    about = "Local-first media archive intelligence"
)]
pub struct Cli {
    /// Where the archive database lives (default: platform app-data dir).
    #[arg(long, global = true, value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index an archive from one or more roots (durable, resumable).
    Index {
        #[arg(long)]
        archive: String,
        #[arg(long)]
        roots: Vec<PathBuf>,
    },
    /// Search the index (`codec:av1 AND tag:interview`).
    Search {
        #[arg(long)]
        archive: String,
        #[arg(long)]
        query: String,
    },
    /// Recompute and persist exact-duplicate groups, reporting them as JSON.
    Dedupe {
        #[arg(long)]
        archive: String,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Tag assets (local models unless --allow-cloud is passed).
    Tag {
        #[arg(long)]
        archive: String,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        local_only: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        allow_cloud: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct MachineReport {
    pub archive: String,
    pub archive_id: String,
    pub assets_indexed: usize,
    pub duplicates_found: usize,
    pub cloud_ai_used: bool,
}

fn default_data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os(DATA_DIR_ENV) {
        return PathBuf::from(d);
    }
    if let Some(app) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(app).join("tpt-media-asset-intelligence");
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        return PathBuf::from(home).join(".tpt-media-asset-intelligence");
    }
    PathBuf::from(".")
}

fn db_file(data_dir: &Path) -> PathBuf {
    data_dir.join(DB_FILE)
}

/// Open the store + queue for a data directory, creating it if needed.
fn open_queue(data_dir: &Path) -> Result<JobQueue, String> {
    std::fs::create_dir_all(data_dir).map_err(|e| format!("data dir {data_dir:?}: {e}"))?;
    JobQueue::open(&db_file(data_dir)).map_err(|e| e.to_string())
}

/// The archive named `name`, or None. Name collisions resolve to the first.
fn archive_by_name(store: &Store, name: &str) -> Result<Option<Archive>, String> {
    store
        .list_archives()
        .map(|a| a.into_iter().find(|a| a.name == name))
        .map_err(|e| format!("store: {e}"))
}

fn find_or_create_archive(store: &Store, name: &str, roots: &[PathBuf]) -> Result<Archive, String> {
    if let Some(mut a) = archive_by_name(store, name)? {
        if !roots.is_empty() {
            let mut all = a.roots.clone();
            all.extend_from_slice(roots);
            all.sort();
            all.dedup();
            a.roots = all;
            store
                .upsert_archive(&a)
                .map_err(|e| format!("store: {e}"))?;
        }
        return Ok(a);
    }
    if roots.is_empty() {
        return Err(format!("archive '{name}' not found"));
    }
    let a = Archive {
        id: Uuid::new_v4(),
        name: name.to_string(),
        roots: roots.to_vec(),
        watch_enabled: false,
        ai_settings: AiSettings::default(),
    };
    store
        .upsert_archive(&a)
        .map_err(|e| format!("store: {e}"))?;
    Ok(a)
}

/// Persist the current exact-duplicate groups for an archive, replacing the
/// previous run's groups (recomputing is the CLI semantics, §8).
fn reconcile_duplicate_groups(
    queue: &JobQueue,
    archive_id: Uuid,
    assets: &[Asset],
) -> Result<usize, String> {
    let store = queue.store();
    let fingerprints: HashMap<Uuid, String> = assets
        .iter()
        .map(|a| (a.id, a.fingerprint.clone()))
        .collect();
    let groups = exact_duplicates(&fingerprints);
    let store = store
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    for old in store
        .duplicate_groups_for_archive(archive_id)
        .map_err(|e| e.to_string())?
    {
        store
            .delete_duplicate_group(old.id)
            .map_err(|e| e.to_string())?;
    }
    for g in &groups {
        store.upsert_duplicate_group(g).map_err(|e| e.to_string())?;
    }
    Ok(groups.len())
}

fn entry_for_asset(store: &Store, asset: &Asset) -> SearchIndexEntry {
    let filename = asset
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tags = store
        .tags_for_asset(asset.id)
        .map(|tags| {
            tags.into_iter()
                .filter(|(_, t)| !t.rejected)
                .map(|(_, t)| t.label)
                .collect()
        })
        .unwrap_or_default();
    SearchIndexEntry {
        asset_id: asset.id,
        filename,
        codec: asset.technical_metadata.codec.clone(),
        tags,
        notes: String::new(),
    }
}

pub fn run(cli: Cli) -> (ExitCode, String) {
    let data_dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    run_with(&data_dir, cli.command)
}

fn run_with(data_dir: &Path, command: Commands) -> (ExitCode, String) {
    match command {
        Commands::Index { archive, roots } => {
            if roots.is_empty() {
                return (ExitCode::InputError, "no --roots provided".to_string());
            }
            let queue = match open_queue(data_dir) {
                Ok(q) => q,
                Err(e) => return (ExitCode::ConfigurationError, e),
            };
            match run_index(&queue, &archive, &roots) {
                Ok(report) => match serde_json::to_string_pretty(&report) {
                    Ok(out) => (ExitCode::Success, out),
                    Err(_) => (ExitCode::InternalError, "failed to serialize report".into()),
                },
                Err(e) => (ExitCode::IndexingFailed, e),
            }
        }
        Commands::Search { archive, query } => {
            let q = match parse_query(&query) {
                Ok(q) => q,
                Err(e) => return (ExitCode::SearchFailed, format!("query error: {e}")),
            };
            let queue = match open_queue(data_dir) {
                Ok(q) => q,
                Err(e) => return (ExitCode::ConfigurationError, e),
            };
            match run_search(&queue, &archive, &q) {
                Ok(out) => (ExitCode::Success, out),
                Err(e) => (ExitCode::ConfigurationError, e),
            }
        }
        Commands::Dedupe { archive, report: _ } => {
            let queue = match open_queue(data_dir) {
                Ok(q) => q,
                Err(e) => return (ExitCode::ConfigurationError, e),
            };
            let report = match run_dedupe(&queue, &archive) {
                Ok(r) => r,
                Err(e) => return (ExitCode::IndexingFailed, e),
            };
            match serde_json::to_string_pretty(&report) {
                Ok(out) => (ExitCode::Success, out),
                Err(_) => (ExitCode::InternalError, "failed to serialize report".into()),
            }
        }
        Commands::Tag {
            archive: _,
            local_only,
            allow_cloud,
        } => {
            if allow_cloud && local_only {
                return (
                    ExitCode::ConfigurationError,
                    "cannot combine --local-only with --allow-cloud".to_string(),
                );
            }
            if allow_cloud {
                (
                    ExitCode::Success,
                    "cloud tagging explicitly enabled".to_string(),
                )
            } else {
                (ExitCode::Success, "local-only tagging".to_string())
            }
=======
//! Library surface of the `tpt-media-asset-intel` CLI (spec §14).
//!
//! The command definitions, exit-code contract and machine-readable output
//! types live in the library (not `main.rs`) so they are unit-testable and
//! reusable by the service crate. The CLI uses the same indexing/search engine
//! as the GUI (spec §3.6, §14).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cli {
    //! Command-line argument definitions (clap derive).
    use clap::{Parser, Subcommand};

    /// TPT Media Asset Intelligence — local-first media archive indexing, search,
    /// dedupe and archive health (spec §1).
    #[derive(Debug, Parser)]
    #[command(name = "tpt-media-asset-intel", version, about)]
    pub struct Cli {
        /// The command to run.
        #[command(subcommand)]
        pub command: Command,
    }

    /// The CLI commands (spec §14).
    #[derive(Debug, Subcommand)]
    pub enum Command {
        /// Index an archive (batch indexing, spec §14).
        Index {
            /// Archive configuration file (YAML).
            #[arg(long)]
            archive: std::path::PathBuf,
            /// Archive roots to scan; overrides the config file's roots.
            #[arg(long = "root")]
            roots: Vec<std::path::PathBuf>,
        },
        /// Search an indexed archive (spec §14).
        Search {
            /// The archive name.
            #[arg(long)]
            archive: String,
            /// Query in the shared query language (e.g. `codec:prores AND tag:interview`).
            #[arg(long)]
            query: String,
        },
        /// Produce a duplicate report (spec §14).
        Dedupe {
            /// The archive name.
            #[arg(long)]
            archive: String,
            /// Write the JSON report to this path.
            #[arg(long)]
            report: std::path::PathBuf,
        },
        /// Run local-model tagging (spec §14).
        Tag {
            /// The archive name.
            #[arg(long)]
            archive: String,
            /// Restrict to local models (the default; present to make the
            /// offline guarantee explicit at call sites).
            #[arg(long, default_value_t = true)]
            local_only: bool,
            /// Explicitly permit a cloud model. Without this flag the CLI makes
            /// it *impossible* to invoke cloud AI (spec §14).
            #[arg(long, requires = "cloud_confirmed")]
            cloud: bool,
            /// Second explicit confirmation required for `--cloud` (spec §14).
            #[arg(long)]
            cloud_confirmed: bool,
        },
    }
}

pub mod exit_code {
    //! The stable exit-code contract (spec §14).
    //!
    //! These codes are a public contract: scripts and CI integrate against
    //! them, so their numeric values must never change.

    /// The stable exit codes (spec §14).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ExitCode {
        /// 0 — everything requested succeeded.
        Success,
        /// 1 — some items succeeded and some failed (e.g. indexing finished
        /// with per-file errors).
        PartialSuccess,
        /// 2 — the indexing run failed.
        IndexingFailed,
        /// 3 — the search run failed.
        SearchFailed,
        /// 4 — configuration invalid (bad archive config, missing settings).
        ConfigurationError,
        /// 5 — invalid user input (bad query syntax, missing file).
        InputError,
        /// 6 — an internal fault; report as a bug.
        InternalError,
    }

    impl From<ExitCode> for i32 {
        fn from(code: ExitCode) -> i32 {
            match code {
                ExitCode::Success => 0,
                ExitCode::PartialSuccess => 1,
                ExitCode::IndexingFailed => 2,
                ExitCode::SearchFailed => 3,
                ExitCode::ConfigurationError => 4,
                ExitCode::InputError => 5,
                ExitCode::InternalError => 6,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn numeric_values_match_the_spec_contract() {
            assert_eq!(i32::from(ExitCode::Success), 0);
            assert_eq!(i32::from(ExitCode::PartialSuccess), 1);
            assert_eq!(i32::from(ExitCode::IndexingFailed), 2);
            assert_eq!(i32::from(ExitCode::SearchFailed), 3);
            assert_eq!(i32::from(ExitCode::ConfigurationError), 4);
            assert_eq!(i32::from(ExitCode::InputError), 5);
            assert_eq!(i32::from(ExitCode::InternalError), 6);
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
        }
    }
}

<<<<<<< HEAD
fn run_index(queue: &JobQueue, archive: &str, roots: &[PathBuf]) -> Result<MachineReport, String> {
    let store = queue
        .store()
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    let a = find_or_create_archive(&store, archive, roots)?;
    drop(store);

    let job_id = queue.enqueue_index(a.id).map_err(|e| e.to_string())?;
    queue
        .run_until_idle()
        .map_err(|e| format!("indexing failed: {e}"))?;

    let store = queue
        .store()
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    let job = store
        .get_job(job_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "index job disappeared".to_string())?;
    if job.status == JobStatus::Failed {
        return Err(job.error.unwrap_or_else(|| "index job failed".into()));
    }
    let assets = store.assets_for_archive(a.id).map_err(|e| e.to_string())?;
    let assets_indexed = assets.len();
    drop(store);

    let duplicates_found = reconcile_duplicate_groups(queue, a.id, &assets)?;

    Ok(MachineReport {
        archive: archive.to_string(),
        archive_id: a.id.to_string(),
        assets_indexed,
        duplicates_found,
        cloud_ai_used: false,
    })
}

fn run_search(
    queue: &JobQueue,
    archive: &str,
    query: &tpt_app_media_asset_intelligence_search::Query,
) -> Result<String, String> {
    let store = queue
        .store()
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    let a = archive_by_name(&store, archive)?
        .ok_or_else(|| format!("archive '{archive}' not found"))?;
    let assets = store.assets_for_archive(a.id).map_err(|e| e.to_string())?;
    let mut out = String::new();
    let mut hits = 0usize;
    for asset in &assets {
        let entry = entry_for_asset(&store, asset);
        if matches(&entry, query) {
            let reasons = explain_match(&entry, query).join("; ");
            out.push_str(&format!(
                "{}{}\n",
                asset.path.display(),
                if reasons.is_empty() {
                    String::new()
                } else {
                    format!("  [{reasons}]")
                }
            ));
            hits += 1;
        }
    }
    out.push_str(&format!("{hits} match(es)"));
    Ok(out)
}

fn run_dedupe(queue: &JobQueue, archive: &str) -> Result<MachineReport, String> {
    let store = queue
        .store()
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    let a = archive_by_name(&store, archive)?
        .ok_or_else(|| format!("archive '{archive}' not found"))?;
    let assets = store.assets_for_archive(a.id).map_err(|e| e.to_string())?;
    let assets_count = assets.len();
    drop(store);

    let duplicates_found = reconcile_duplicate_groups(queue, a.id, &assets)?;

    Ok(MachineReport {
        archive: archive.to_string(),
        archive_id: a.id.to_string(),
        assets_indexed: assets_count,
        duplicates_found,
        cloud_ai_used: false,
    })
=======
pub mod output {
    //! Machine-readable result envelopes (spec §14).

    use serde::{Deserialize, Serialize};

    /// Result envelope for `index` (spec §14 example).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct IndexResult {
        /// The archive that was indexed.
        pub archive: String,
        /// Assets indexed by this run.
        pub assets_indexed: u64,
        /// Duplicate groups found so far.
        pub duplicates_found: u64,
        /// `true` iff any cloud AI feature contributed. Always `false` unless
        /// the user explicitly enabled a cloud feature (spec §14, §17).
        pub cloud_ai_used: bool,
    }

    /// Result envelope for `search`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct SearchHit {
        /// Matching asset id.
        pub asset: String,
        /// Why this result matched: matched field/tag or similarity score
        /// (spec §10, §13.2).
        pub explanation: String,
    }

    /// Result envelope for `search`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct SearchResult {
        /// The archive that was searched.
        pub archive: String,
        /// The parsed query text.
        pub query: String,
        /// Matches in relevance order.
        pub hits: Vec<SearchHit>,
        /// `true` iff any cloud AI feature contributed (spec §14).
        pub cloud_ai_used: bool,
    }
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
<<<<<<< HEAD
    use super::*;
    use std::fs;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mai-cli-{tag}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn index_requires_roots() {
        let cli = Cli::parse_from(["t", "index", "--archive", "main"]);
        assert!(matches!(run(cli).0, ExitCode::InputError));
    }

    #[test]
    fn bad_query_is_search_failed() {
        let cli = Cli::parse_from(["t", "search", "--archive", "m", "--query", "codec:"]);
        assert!(matches!(run(cli).0, ExitCode::SearchFailed));
    }

    #[test]
    fn conflicting_tag_flags_are_configuration_error() {
        let cli = Cli::parse_from([
            "t",
            "tag",
            "--archive",
            "m",
            "--local-only",
            "--allow-cloud",
        ]);
        assert!(matches!(run(cli).0, ExitCode::ConfigurationError));
    }

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::PartialSuccess as i32, 1);
        assert_eq!(ExitCode::IndexingFailed as i32, 2);
        assert_eq!(ExitCode::SearchFailed as i32, 3);
        assert_eq!(ExitCode::ConfigurationError as i32, 4);
        assert_eq!(ExitCode::InputError as i32, 5);
        assert_eq!(ExitCode::InternalError as i32, 6);
    }

    #[test]
    fn index_persists_assets_and_duplicates() {
        let roots = temp_root("index");
        fs::write(roots.join("a.mkv"), [0u8; 16]).unwrap();
        fs::write(roots.join("b.mkv"), [0u8; 16]).unwrap(); // same bytes → duplicate group
        fs::write(roots.join("c.txt"), b"unique").unwrap();
        let data_dir = temp_root("index-data");

        let cli = Cli::parse_from([
            "t",
            "index",
            "--data-dir",
            data_dir.to_str().unwrap(),
            "--archive",
            "main",
            "--roots",
            roots.to_str().unwrap(),
        ]);
        let (code, out) = run(cli);
        let _ = fs::remove_dir_all(&roots);
        assert_eq!(code, ExitCode::Success);
        let report: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(report["assets_indexed"], 3);
        assert_eq!(report["duplicates_found"], 1);
        assert!(!report["cloud_ai_used"].as_bool().unwrap());
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn search_returns_matches_from_the_persisted_archive() {
        let roots = temp_root("search");
        fs::write(roots.join("interview.mkv"), b"aaa").unwrap();
        fs::write(roots.join("b-roll.mkv"), b"bbb").unwrap();
        let data_dir = temp_root("search-data");

        let index = Cli::parse_from([
            "t",
            "index",
            "--data-dir",
            data_dir.to_str().unwrap(),
            "--archive",
            "news",
            "--roots",
            roots.to_str().unwrap(),
        ]);
        assert_eq!(run(index).0, ExitCode::Success);

        let search = Cli::parse_from([
            "t",
            "search",
            "--data-dir",
            data_dir.to_str().unwrap(),
            "--archive",
            "news",
            "--query",
            "interview",
        ]);
        let (code, out) = run(search);
        let _ = fs::remove_dir_all(&roots);
        assert_eq!(code, ExitCode::Success);
        assert!(out.contains("interview.mkv"));
        assert!(!out.contains("b-roll.mkv"));
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn search_for_missing_archive_is_configuration_error() {
        let data_dir = temp_root("search-missing");
        let search = Cli::parse_from([
            "t",
            "search",
            "--data-dir",
            data_dir.to_str().unwrap(),
            "--archive",
            "ghost",
            "--query",
            "anything",
        ]);
        assert!(matches!(run(search).0, ExitCode::ConfigurationError));
        let _ = fs::remove_dir_all(&data_dir);
=======
    use super::cli::{Cli, Command};
    use clap::Parser;

    #[test]
    fn index_command_parses_archive_and_roots() {
        let cli = Cli::try_parse_from([
            "tpt-media-asset-intel",
            "index",
            "--archive",
            "archive-config.yaml",
            "--root",
            "/mnt/media-nas/projects",
            "--root",
            "D:/archive",
        ])
        .unwrap();
        match cli.command {
            Command::Index { archive, roots } => {
                assert_eq!(archive, std::path::PathBuf::from("archive-config.yaml"));
                assert_eq!(roots.len(), 2);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn search_command_parses_the_query_verbatim() {
        let cli = Cli::try_parse_from([
            "tpt-media-asset-intel",
            "search",
            "--archive",
            "main",
            "--query",
            "codec:prores AND tag:interview",
        ])
        .unwrap();
        match cli.command {
            Command::Search { archive, query } => {
                assert_eq!(archive, "main");
                assert_eq!(query, "codec:prores AND tag:interview");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn cloud_tagging_requires_double_confirmation() {
        // `--cloud` alone is rejected: the CLI makes it impossible to invoke a
        // cloud AI feature without an explicit flag pair (spec §14).
        let err = Cli::try_parse_from([
            "tpt-media-asset-intel",
            "tag",
            "--archive",
            "main",
            "--cloud",
        ])
        .unwrap_err();
        assert!(err.use_stderr());

        let cli = Cli::try_parse_from([
            "tpt-media-asset-intel",
            "tag",
            "--archive",
            "main",
            "--cloud",
            "--cloud-confirmed",
        ])
        .unwrap();
        match cli.command {
            Command::Tag {
                local_only,
                cloud,
                cloud_confirmed,
                ..
            } => {
                assert!(local_only, "local-only defaults true");
                assert!(cloud);
                assert!(cloud_confirmed);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn index_result_envelope_matches_the_spec_shape() {
        let result = super::output::IndexResult {
            archive: "main".to_string(),
            assets_indexed: 48_201,
            duplicates_found: 214,
            cloud_ai_used: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        for key in [
            "\"archive\":\"main\"",
            "\"assets_indexed\":48201",
            "\"duplicates_found\":214",
            "\"cloud_ai_used\":false",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
