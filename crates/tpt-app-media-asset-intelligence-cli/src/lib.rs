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
        }
    }
}

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
}

#[cfg(test)]
mod tests {
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
    }
}
