//! Top-level application state and event loop.
//!
//! The event loop is intentionally simple in M1: poll for key events, dispatch
//! to [`App::handle_input`], redraw. M2+ will add a `JobManager` and async
//! event sources (process output streams).

use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::config::Cli;
use crate::error::Result;
use crate::input::{map_key, InputAction};
use crate::theme::Theme;

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Log,
}

/// Visual stream classification for a log line. Drives color and glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Out/Err/Success/Failure are wired up in M2 (process output streams).
pub enum LogStream {
    Info,
    Out,
    Err,
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Local>,
    pub stream: LogStream,
    pub message: String,
}

/// Top-level app state. Owns the theme, focus, sidebar selection, log buffer,
/// and the help-overlay flag.
pub struct App {
    #[allow(dead_code)] // Consumed in M2+ for --dry-run, --reset-state, --log-level.
    pub cli: Cli,
    pub theme: Theme,
    pub focus: Focus,
    pub sidebar_index: usize,
    pub show_help: bool,
    pub log_lines: Vec<LogLine>,
    pub should_quit: bool,
}

impl App {
    #[allow(dead_code)] // cli is consumed in M2+ (--dry-run, --reset-state, --log-level).
    pub fn new(cli: Cli) -> Self {
        let mut app = Self {
            cli,
            theme: Theme::default(),
            focus: Focus::Sidebar,
            sidebar_index: 0,
            show_help: false,
            log_lines: Vec::new(),
            should_quit: false,
        };
        app.push_log(
            LogStream::Info,
            "mtui ready. Press ? for help, Tab to cycle focus, q to quit.".into(),
        );
        app.push_log(
            LogStream::Info,
            "Skeleton build — no actions wired up yet. See spec.md §9.".into(),
        );
        app
    }

    pub fn push_log(&mut self, stream: LogStream, message: String) {
        self.log_lines.push(LogLine {
            timestamp: Local::now(),
            stream,
            message,
        });
    }

    /// Blocking render loop. Returns when the user quits or a draw/poll fails.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Cap log buffer to avoid unbounded growth across long sessions.
        const MAX_LOG_LINES: usize = 2000;

        while !self.should_quit {
            terminal.draw(|frame| crate::ui::render(&self, frame))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_input(map_key(key));
                    }
                }
            }

            if self.log_lines.len() > MAX_LOG_LINES {
                let drop = self.log_lines.len() - MAX_LOG_LINES;
                self.log_lines.drain(0..drop);
            }
        }
        Ok(())
    }

    fn handle_input(&mut self, action: InputAction) {
        // Help overlay is modal: it eats everything except close.
        if self.show_help {
            match action {
                InputAction::Quit | InputAction::Help | InputAction::Esc => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        match action {
            InputAction::Quit => self.should_quit = true,
            InputAction::Help => self.show_help = true,

            InputAction::TabFocusNext => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Main,
                    Focus::Main => Focus::Log,
                    Focus::Log => Focus::Sidebar,
                }
            }
            InputAction::TabFocusPrev => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Log,
                    Focus::Main => Focus::Sidebar,
                    Focus::Log => Focus::Main,
                }
            }

            InputAction::SidebarDown => {
                let max = crate::ui::sidebar::ACTION_COUNT;
                if self.sidebar_index + 1 < max {
                    self.sidebar_index += 1;
                }
            }
            InputAction::SidebarUp => {
                if self.sidebar_index > 0 {
                    self.sidebar_index -= 1;
                }
            }
            InputAction::SidebarTop => self.sidebar_index = 0,
            InputAction::SidebarBottom => {
                self.sidebar_index = crate::ui::sidebar::ACTION_COUNT - 1;
            }
            InputAction::JumpTo(n) => {
                let max = crate::ui::sidebar::ACTION_COUNT;
                if n >= 1 && n <= max {
                    self.sidebar_index = n - 1;
                    self.focus = Focus::Main;
                }
            }

            InputAction::Enter => {
                let label = crate::ui::sidebar::action_label(self.sidebar_index)
                    .unwrap_or("(unknown)")
                    .to_string();
                self.push_log(
                    LogStream::Info,
                    format!(
                        "Run pressed for \"{}\" — coming in a later milestone.",
                        label
                    ),
                );
            }
            InputAction::Esc => {
                // M7: cancel a running job. M1: no-op.
            }
            InputAction::ClearLog => {
                self.log_lines.clear();
                self.push_log(LogStream::Info, "Log cleared.".into());
            }
            InputAction::YankLog => {
                // M3: implement via pbcopy. M1: log a hint.
                self.push_log(
                    LogStream::Info,
                    "Yank to clipboard will be wired up in M3.".into(),
                );
            }
            InputAction::Noop => {}
        }
    }
}
