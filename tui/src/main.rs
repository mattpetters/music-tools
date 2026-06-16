//! `mtui` — terminal UI for the music-tools scripts.
//!
//! Entry point: parse CLI, init the terminal, run the app, restore the terminal.
//! The actual app logic lives in [`app::App`]. The UI is in [`ui`].

mod app;
mod config;
mod error;
mod input;
mod theme;
mod ui;

use std::io::IsTerminal;
use std::process::ExitCode;

use color_eyre::eyre::Result;

use crate::app::App;
use crate::config::Cli;

fn main() -> ExitCode {
    if let Err(err) = run() {
        // Terminal may or may not still be in raw mode; restore defensively
        // and then print. color-eyre gives us a nice backtrace either way.
        ratatui::restore();
        eprintln!("mtui: {err:?}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if !std::io::stdout().is_terminal() {
        eprintln!(
            "mtui: stdout is not a terminal. Run this from your shell, not from a pipe or CI."
        );
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mtui=info,warn")),
        )
        .with_target(false)
        .init();

    tracing::info!("mtui {} starting", env!("CARGO_PKG_VERSION"));

    let terminal = ratatui::init();
    let result = App::new(cli).run(terminal);
    ratatui::restore();
    result
}
