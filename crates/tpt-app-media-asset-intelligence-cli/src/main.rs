//! `tpt-media-asset-intel` — the batch interface (spec §14).
//!
//! The engine boundary (spec §3.6): this binary drives the same engine the GUI
//! uses. Phase 1 wires the real engine behind each command; the argument
//! parsing, exit-code contract and output envelopes are already final.

#![forbid(unsafe_code)]

use std::process::ExitCode as ProcessExitCode;

use clap::Parser as _;

use tpt_app_media_asset_intelligence_cli::cli::{Cli, Command};
use tpt_app_media_asset_intelligence_cli::exit_code::ExitCode;

fn main() -> ProcessExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let exit = match cli.command {
        // Phase 1 (spec §26 steps 16–18) wires each command to the engine. The
        // exit-code contract is already stable, so scripts can integrate now.
        Command::Index { .. } => unimplemented("index"),
        Command::Search { .. } => unimplemented("search"),
        Command::Dedupe { .. } => unimplemented("dedupe"),
        Command::Tag { .. } => unimplemented("tag"),
    };
    ProcessExitCode::from(i32::from(exit) as u8)
}

/// Reports a command whose engine is not wired yet, honestly and stably.
fn unimplemented(command: &str) -> ExitCode {
    eprintln!(
        "error: the `{command}` command is not wired to the engine yet \
         (planned for Phase 1, spec.txt §26); no work was performed"
    );
    ExitCode::InternalError
}
