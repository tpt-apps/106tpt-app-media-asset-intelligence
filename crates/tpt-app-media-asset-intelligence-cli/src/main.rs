use clap::Parser;
use std::process::ExitCode as ProcExit;
use tpt_app_media_asset_intelligence_cli::{run, Cli, ExitCode};

fn main() -> ProcExit {
    let (code, out) = run(Cli::parse());
    println!("{out}");
    ProcExit::from(match code {
        ExitCode::Success => 0,
        ExitCode::PartialSuccess => 1,
        ExitCode::IndexingFailed => 2,
        ExitCode::SearchFailed => 3,
        ExitCode::ConfigurationError => 4,
        ExitCode::InputError => 5,
        ExitCode::InternalError => 6,
    })
}
