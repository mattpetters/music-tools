//! Command-line argument parsing.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mtui",
    version,
    about = "Terminal UI for the music-tools scripts",
    long_about = None,
)]
pub struct Cli {
    /// Reset persisted state on startup.
    #[arg(long)]
    pub reset_state: bool,

    /// Print what actions would do without executing them.
    #[arg(long)]
    pub dry_run: bool,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
