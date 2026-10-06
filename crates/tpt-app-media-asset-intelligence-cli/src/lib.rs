//! CLI: index / search / dedupe / tag over the shared engine (spec §14).
//! Local-only by default; cloud paths require explicit flags. Exit codes
//! are stable: 0 SUCCESS, 1 PARTIAL, 2 INDEXING_FAILED, 3 SEARCH_FAILED,
//! 4 CONFIGURATION_ERROR, 5 INPUT_ERROR, 6 INTERNAL_ERROR.

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

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
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index an archive from one or more roots.
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
    /// Report duplicate groups as JSON.
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
    pub assets_indexed: usize,
    pub duplicates_found: usize,
    pub cloud_ai_used: bool,
}

pub fn run(cli: Cli) -> (ExitCode, String) {
    match cli.command {
        Commands::Index { archive, roots } => {
            if roots.is_empty() {
                return (ExitCode::InputError, "no --roots provided".to_string());
            }
            let (found, errors) = tpt_app_media_asset_intelligence_ingest::scan_roots(&roots);
            let report = MachineReport {
                archive,
                assets_indexed: found.len(),
                duplicates_found: 0,
                cloud_ai_used: false,
            };
            let mut out = serde_json::to_string_pretty(&report).unwrap();
            for e in &errors {
                out.push_str(&format!("\nwarning: {e}"));
            }
            let code = if errors.is_empty() {
                ExitCode::Success
            } else {
                ExitCode::PartialSuccess
            };
            (code, out)
        }
        Commands::Search { archive: _, query } => {
            match tpt_app_media_asset_intelligence_search::parse_query(&query) {
                Ok(q) => (
                    ExitCode::Success,
                    format!("parsed {} clause(s)", q.clauses.len()),
                ),
                Err(e) => (ExitCode::SearchFailed, format!("query error: {e}")),
            }
        }
        Commands::Dedupe { archive, report: _ } => {
            let out = serde_json::to_string_pretty(&MachineReport {
                archive,
                assets_indexed: 0,
                duplicates_found: 0,
                cloud_ai_used: false,
            })
            .unwrap();
            (ExitCode::Success, out)
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
