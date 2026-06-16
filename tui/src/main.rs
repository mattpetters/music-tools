//! `mtui` — terminal UI for the music-tools scripts.
//!
//! Entry point: parse CLI, init the terminal, run the app, restore the
//! terminal. Supports `--dry-run` (print what each action would do
//! without spawning) and `--reset-state` (clear the on-disk state
//! file). The actual app logic lives in [`app::App`]. The UI is in
//! [`ui`].

mod actions;
mod app;
mod config;
mod error;
mod input;
mod jobs;
mod log;
mod process;
mod state;
mod sudo;
mod theme;
mod ui;

use std::io::IsTerminal;
use std::process::ExitCode;

use color_eyre::eyre::Result;

use crate::actions::ResolvedInputs;
use crate::app::App;
use crate::config::Cli;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    if let Err(err) = run().await {
        ratatui::restore();
        eprintln!("mtui: {err:?}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if cli.reset_state {
        if let Err(e) = crate::state::PersistedState::reset() {
            eprintln!("mtui: failed to reset state: {e}");
        } else {
            eprintln!("mtui: state reset.");
        }
        return Ok(());
    }

    if cli.dry_run {
        run_dry_run();
        return Ok(());
    }

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
    let app = App::new(cli);
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}

/// Print a description of every action and the command it would run
/// with default inputs. Used by `--dry-run`.
fn run_dry_run() {
    println!("mtui {} — dry run", env!("CARGO_PKG_VERSION"));
    println!();
    for action in crate::actions::all() {
        let info = action.info();
        println!("[{}]  {}", info.category, info.label);
        println!("  id:           {}", info.id);
        println!("  hint:         {}", info.hint);
        println!("  input:        {:?}", info.input_kind);
        println!("  requires_sudo: {}", action.requires_sudo());
        // For actions that need Dir/File/Choice inputs, we can't build a
        // meaningful command without user input. Print a placeholder and
        // continue.
        if ResolvedInputs::input_kind_requires_user_input(&info.input_kind) {
            println!(
                "  command:      <requires {:?} input — provide via the TUI>",
                info.input_kind
            );
        } else {
            let spec = action.build_command(ResolvedInputs::default());
            let cwd = spec
                .cwd
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<inherit>".to_string());
            let args = if spec.args.is_empty() {
                String::new()
            } else {
                format!(" {}", spec.args.join(" "))
            };
            println!("  command:      {}{}", spec.program, args);
            println!("  cwd:          {cwd}");
        }
        println!();
    }
}
