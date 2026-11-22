use anyhow::Result;
use clap::Parser;
use netoolfy::cli::Cli;
use netoolfy::run_diagnosis;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let report = run_diagnosis(&cli).await?;
    let code = report.exit_code();
    if code != 0 {
        std::process::exit(code);
    }

    Ok(())
}
