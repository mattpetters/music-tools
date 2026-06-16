//! Top-level application state and event loop.

use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::actions::{self, ActionId, InputKind};
use crate::config::Cli;
use crate::error::Result;
use crate::input::{map_key, InputAction};
use crate::jobs::{JobManager, JobStatus};
use crate::log::{LogLine, LogStream};
use crate::theme::Theme;
use crate::ui::browser::Browser;

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Log,
}

/// Top-level app state.
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
    /// Cached metadata for the sidebar.
    pub action_metas: Vec<actions::ActionInfo>,
    /// Last status of each action's most recent job.
    pub last_status: Vec<Option<JobStatus>>,
    /// Per-action browser state, keyed by action id. Lazily populated
    /// when an action is first focused in the main pane.
    pub browsers: HashMap<ActionId, Browser>,
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
            browsers: HashMap::new(),
        };
        app.push_log(
            LogStream::Info,
            "mtui ready. Press ? for help, Tab to cycle focus, q to quit.".into(),
        );
        app.push_log(
            LogStream::Info,
            "M3: real actions wired. Pick one in the sidebar and use the browser.".into(),
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

    /// Get or create the browser for an action.
    fn browser_for(&mut self, action_id: ActionId, input_kind: InputKind) -> &mut Browser {
        self.browsers
            .entry(action_id)
            .or_insert_with(|| Browser::new(default_browser_path(), input_kind))
    }

    /// Blocking render loop.
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

            for line in self.job_manager.take_new_logs() {
                self.push_log(line.stream, line.message);
            }

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

        if let Some(pos) = self.action_metas.iter().position(|m| m.id == action_id) {
            if pos < self.last_status.len() {
                self.last_status[pos] = Some(status);
            }
        }
    }

    async fn handle_key(&mut self, action: InputAction) {
        // Help overlay is modal.
        if self.show_help {
            match action {
                InputAction::Quit | InputAction::Help | InputAction::Esc => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // Dispatch to the browser first if the main pane is focused.
        // The browser consumes navigation keys and returns false for
        // App-level keys (Enter to run, etc.).
        if self.focus == Focus::Main {
            if let Some(meta) = self.action_metas.get(self.sidebar_index).cloned() {
                if meta.input_kind != InputKind::None {
                    let browser = self.browser_for(meta.id, meta.input_kind);
                    if browser.handle_key(action.clone()) {
                        return;
                    }
                }
            }
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
                // When the sidebar is focused, move the sidebar cursor.
                // When main or log is focused, fall through (the browser
                // already handled it via the dispatch above; for log, the
                // key is unhandled).
                if self.focus == Focus::Sidebar {
                    let max = self.action_metas.len();
                    if self.sidebar_index + 1 < max {
                        self.sidebar_index += 1;
                    }
                }
            }
            InputAction::SidebarUp => {
                if self.focus == Focus::Sidebar && self.sidebar_index > 0 {
                    self.sidebar_index -= 1;
                }
            }
            InputAction::SidebarTop => {
                if self.focus == Focus::Sidebar {
                    self.sidebar_index = 0;
                }
            }
            InputAction::SidebarBottom => {
                if self.focus == Focus::Sidebar && !self.action_metas.is_empty() {
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
                self.push_log(
                    LogStream::Info,
                    "Yank to clipboard will be wired up in M3 (pbcopy).".into(),
                );
            }
            // The browser consumed these, but the App also receives them
            // (we cloned the action). They're no-ops here.
            InputAction::OpenEntry
            | InputAction::GoUp
            | InputAction::ToggleCheck
            | InputAction::PathInput
            | InputAction::GoHome
            | InputAction::Refresh
            | InputAction::Backspace
            | InputAction::Char(_) => {}
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

        // Resolve the action's inputs from the browser (if any).
        let inputs = match meta.input_kind {
            InputKind::None => crate::actions::ResolvedInputs::default(),
            InputKind::Dir => {
                let browser = self.browser_for(meta.id, meta.input_kind);
                match browser.selected_dir() {
                    Some(d) => crate::actions::ResolvedInputs {
                        dir: Some(d),
                        ..Default::default()
                    },
                    None => {
                        self.push_log(
                            LogStream::Failure,
                            format!("Select a directory for \"{}\" first.", meta.label),
                        );
                        return;
                    }
                }
            }
            InputKind::File => {
                let browser = self.browser_for(meta.id, meta.input_kind);
                let file = browser.resolved_file().or_else(|| {
                    // For multi-file actions, run with all checked files.
                    let files = browser.checked_files();
                    files.first().cloned()
                });
                match file {
                    Some(f) => crate::actions::ResolvedInputs {
                        file: Some(f),
                        ..Default::default()
                    },
                    None => {
                        self.push_log(
                            LogStream::Failure,
                            format!("Select a file for \"{}\" first.", meta.label),
                        );
                        return;
                    }
                }
            }
            InputKind::Choice => {
                let browser = self.browser_for(meta.id, meta.input_kind);
                let choice = browser.path.to_string_lossy().to_string();
                if choice.is_empty() {
                    self.push_log(
                        LogStream::Failure,
                        format!("Make a choice for \"{}\" first.", meta.label),
                    );
                    return;
                }
                crate::actions::ResolvedInputs {
                    choice: Some(choice),
                    ..Default::default()
                }
            }
        };

        // Find the action instance and start it.
        for action in actions::all() {
            if action.info().id == meta.id {
                match self.job_manager.start(action.as_ref(), inputs).await {
                    Ok(()) => {
                        self.push_log(LogStream::Info, format!("Started \"{}\".", meta.label));
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

fn default_browser_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}
