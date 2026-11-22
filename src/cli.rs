//! Command-line argument definitions using `clap`.

use clap::Parser;

/// netoolfy — diagnose your network in seconds.
///
/// Runs layered network checks and tells you *where* the problem is
/// in plain language, not just raw numbers.
#[derive(Parser, Debug, Clone)]
#[command(name = "netoolfy", version, about, long_about = None)]
pub struct Cli {
    /// Increase logging verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Output results as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,

    /// Disable colored output (also respects NO_COLOR env var).
    #[arg(long)]
    pub no_color: bool,
}
