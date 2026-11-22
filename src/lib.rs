//! # netoolfy
//!
//! All-in-one network diagnostic library with automatic diagnosis.
//!
//! This library provides the core logic for network diagnostics: running
//! layered checks (interface → gateway → DNS → internet → target) and
//! producing a human-readable verdict that identifies the root cause of
//! connectivity problems.

pub mod checks;
pub mod cli;
pub mod config;
pub mod diagnosis;
pub mod model;
pub mod output;

use checks::Runner;
use cli::Cli;
use model::Report;

/// Run a full network diagnosis using the provided CLI configuration.
///
/// Returns a [`Report`] containing all check results, findings, and a
/// verdict with root-cause analysis.
pub async fn run_diagnosis(cli: &Cli) -> anyhow::Result<Report> {
    let config = config::Config::from_cli(cli);
    let ctx = checks::Context::new(config);
    let runner = Runner::new();
    let report = runner.run(&ctx).await;

    // Output the report.
    if cli.json {
        output::json::print_report(&report)?;
    } else {
        output::pretty::print_report(&report);
    }

    Ok(report)
}
