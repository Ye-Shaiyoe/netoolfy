//! Network diagnostic engine: layered checks and automated root-cause diagnosis.

pub mod checks;
pub mod cli;
pub mod config;
pub mod diagnosis;
pub mod model;
pub mod output;
pub mod platform;

use checks::Runner;
use cli::Cli;
use model::Report;

pub async fn run_diagnosis(cli: &Cli) -> anyhow::Result<Report> {
    let config = config::Config::from_cli(cli);
    let ctx = checks::Context::with_config(config);
    let mut runner = Runner::new();
    runner.add_check(Box::new(checks::interface::InterfaceCheck::new()));
    runner.add_check(Box::new(checks::gateway::GatewayCheck::new()));
    let report = runner.run(&ctx).await;

    if cli.json {
        output::json::print_report(&report)?;
    } else {
        output::pretty::print_report(&report);
    }

    Ok(report)
}
