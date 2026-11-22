//! netoolfy — all-in-one network diagnostic tool with automatic diagnosis.
//!
//! This binary is intentionally thin: it parses CLI arguments, calls library
//! functions, and sets the process exit code.

use anyhow::Result;
use clap::Parser;

use netoolfy::cli::Cli;
use netoolfy::run_diagnosis;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity.
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

    // Exit code: 0 = healthy, 1 = warnings, 2 = failures.
    let code = report.exit_code();
    if code != 0 {
        std::process::exit(code);
    }

    Ok(())
}
