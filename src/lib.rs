//! Network diagnostic engine: layered checks and automated root-cause diagnosis.

pub mod checks;
pub mod cli;
pub mod config;
pub mod diagnosis;
pub mod model;
pub mod output;

use checks::Runner;
use cli::Cli;
use model::Report;

pub async fn run_diagnosis(cli: &Cli) -> anyhow::Result<Report> {
    let config = config::Config::from_cli(cli);
    let ctx = checks::Context::new(config);
    let runner = Runner::new();
    let report = runner.run(&ctx).await;

    if cli.json {
        output::json::print_report(&report)?;
    } else {
        output::pretty::print_report(&report);
    }

    Ok(report)
}
