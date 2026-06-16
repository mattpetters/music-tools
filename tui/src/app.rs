//! Top-level application state and event loop.
//!
//! The event loop is async so we can `await` the JobManager's
//! `start` and `cancel_current` methods. `event::poll` is blocking but
//! safe to call from the `current_thread` runtime — there's no other
//! task to schedule around it.

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::actions::{self};
use crate::config::Cli;
use crate::error::Result;
use crate::input::{map_key, InputAction};
use crate::jobs::{JobManager, JobStatus};
use crate::log::{LogLine, LogStream};
use crate::theme::Theme;

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Log,
}

/// Top-level app state. Owns the theme, focus, sidebar selection, log
/// buffer, help-overlay flag, and the job manager.
pub struct App {
    #[allow(dead_code)] // Consumed in M6 (--reset-state), M8 (--dry-run, --log-level).
    pub cli: Cli,
    pub theme: Theme,
    pub focus: Focus,
    pub sidebar_index: usize,
    pub show_help: bool,
    pub log_lines: Vec<LogLine>,
    pub should_quit: bool,
    pub job_manager: JobManager,
    /// Cached metadata for the sidebar. Loaded once at startup from
    /// [`actions::metadata`].
    pub action_metas: Vec<actions::ActionInfo>,
    /// Last status of each action's most recent job. Indexed by action
    /// position; used to drive sidebar status badges.
    pub last_status: Vec<Option<JobStatus>>,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        let action_metas = actions::metadata();
        let last_status = vec![None; action_metas.len()];

        let mut app = Self {
            cli,
            theme: Theme::default(),
            focus: Focus::Sidebar,
            sidebar_index: 0,
            show_help: false,
            log_lines: Vec::new(),
            should_quit: false,
            job_manager: JobManager::new(),
            action_metas,
            last_status,
        };
        app.push_log(
            LogStream::Info,
            "mtui ready. Press ? for help, Tab to cycle focus, q to quit.".into(),
        );
        app.push_log(
            LogStream::Info,
            "M2: process + askpass + TestAction wired. Press 1 to run the test.".into(),
        );
        app
    }

    pub fn push_log(&mut self, stream: LogStream, message: String) {
        self.log_lines.push(LogLine {
            timestamp: chrono::Local::now(),
            stream,
            message,
        });
    }

    /// Blocking render loop. Returns when the user quits or a draw/poll fails.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        const MAX_LOG_LINES: usize = 2000;
        const POLL_MS: u64 = 50;

        while !self.should_quit {
            terminal.draw(|frame| crate::ui::render(&self, frame))?;

            if event::poll(std::time::Duration::from_millis(POLL_MS))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(map_key(key)).await;
                    }
                }
            }

            // Drain new log lines from the current job into the on-screen
            // log pane. The JobManager keeps the full buffer for history.
            for line in self.job_manager.take_new_logs() {
                self.push_log(line.stream, line.message);
            }

            // Check whether the current job has finished. If so, push a
            // summary line and update the sidebar status badge.
            if let Ok(Some(final_status)) = self.job_manager.poll() {
                self.on_job_finished(final_status);
            }

            if self.log_lines.len() > MAX_LOG_LINES {
                let drop = self.log_lines.len() - MAX_LOG_LINES;
                self.log_lines.drain(0..drop);
            }
        }
        Ok(())
    }

    fn on_job_finished(&mut self, status: JobStatus) {
        let action_id = self
            .job_manager
            .history
            .front()
            .map(|j| j.action_id.clone())
            .unwrap_or_default();
        let (stream, msg) = match status {
            JobStatus::Done => (LogStream::Success, format!("✓ Done: {action_id}")),
            JobStatus::Failed => (
                LogStream::Failure,
                format!("✗ Failed: {action_id} (see log pane for details)"),
            ),
            JobStatus::Cancelled => (LogStream::Info, format!("↻ Cancelled: {action_id}")),
            JobStatus::Running => (LogStream::Info, format!("… Still running: {action_id}")),
        };
        self.push_log(stream, msg);

        // Update sidebar badge for this action.
        if let Some(pos) = self.action_metas.iter().position(|m| m.id == action_id) {
            if pos < self.last_status.len() {
                self.last_status[pos] = Some(status);
            }
        }
    }

    async fn handle_key(&mut self, action: InputAction) {
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
                let max = self.action_metas.len();
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
                if !self.action_metas.is_empty() {
                    self.sidebar_index = self.action_metas.len() - 1;
                }
            }
            InputAction::JumpTo(n) => {
                if n >= 1 && n <= self.action_metas.len() {
                    self.sidebar_index = n - 1;
                    self.focus = Focus::Main;
                }
            }

            InputAction::Enter => {
                self.run_focused().await;
            }
            InputAction::Esc => {
                if self.job_manager.is_running() {
                    self.push_log(LogStream::Info, "Cancelling…".into());
                    if let Err(e) = self.job_manager.cancel_current().await {
                        self.push_log(LogStream::Failure, format!("Cancel failed: {e}"));
                    }
                }
            }
            InputAction::ClearLog => {
                self.log_lines.clear();
                self.push_log(LogStream::Info, "Log cleared.".into());
            }
            InputAction::YankLog => {
                // M3: implement via pbcopy. M2: log a hint.
                self.push_log(
                    LogStream::Info,
                    "Yank to clipboard will be wired up in M3.".into(),
                );
            }
            InputAction::Noop => {}
        }
    }

    async fn run_focused(&mut self) {
        if self.job_manager.is_running() {
            self.push_log(
                LogStream::Failure,
                "A job is already running. Press Esc to cancel.".into(),
            );
            return;
        }
        let Some(meta) = self.action_metas.get(self.sidebar_index).cloned() else {
            return;
        };

        // For M2's placeholders, log a hint and don't actually run.
        if meta.id != actions::TestAction::ID {
            self.push_log(
                LogStream::Info,
                format!(
                    "\"{}\" is a placeholder. Real implementation lands in M3+.",
                    meta.label
                ),
            );
            return;
        }

        // Find the action instance by id and start it.
        for action in actions::all() {
            if action.info().id == meta.id {
                match self.job_manager.start(action.as_ref()).await {
                    Ok(()) => {
                        self.push_log(
                            LogStream::Info,
                            format!("Started \"{}\" (job #{}).", meta.label, "—"),
                        );
                    }
                    Err(e) => {
                        self.push_log(
                            LogStream::Failure,
                            format!("Failed to start \"{}\": {e}", meta.label),
                        );
                    }
                }
                return;
            }
        }
        self.push_log(
            LogStream::Failure,
            format!("No action instance registered for id \"{}\".", meta.id),
        );
    }
}
