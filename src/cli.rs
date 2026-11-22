use clap::Parser;

/// Automated network diagnosis tool.
#[derive(Parser, Debug, Clone)]
#[command(name = "netoolfy", version, about, long_about = None)]
pub struct Cli {
    /// Logging verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Output results as JSON.
    #[arg(long)]
    pub json: bool,

    /// Disable colored output.
    #[arg(long)]
    pub no_color: bool,
}
