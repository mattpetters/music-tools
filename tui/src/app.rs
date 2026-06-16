//! Top-level application state and event loop.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::actions::reset_ableton::{self, AbletonVersion};
use crate::actions::{self, ActionId, InputKind};
use crate::config::Cli;
use crate::error::Result;
use crate::input::{map_key, InputAction};
use crate::jobs::{JobManager, JobStatus};
use crate::log::{LogLine, LogStream};
use crate::state::{PersistedState, RecentPath};
use crate::theme::Theme;
use crate::ui::browser::Browser;

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Log,
}

/// State for a `Choice`-input-kind action (e.g. reset_ableton).
pub struct ChoicePicker {
    pub versions: Vec<AbletonVersion>,
    pub cursor: usize,
    #[allow(dead_code)] // Used in M6 to throttle re-discovery.
    pub last_refresh: Option<SystemTime>,
}

impl ChoicePicker {
    pub fn new(versions: Vec<AbletonVersion>) -> Self {
        Self {
            versions,
            cursor: 0,
            last_refresh: None,
        }
    }

    pub fn selected(&self) -> Option<&AbletonVersion> {
        self.versions.get(self.cursor)
    }

    pub fn cursor_down(&mut self) {
        if self.cursor + 1 < self.versions.len() {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
}

/// In-flight confirmation prompt. When set, the App is in confirm mode
/// and Enter runs the action while Esc cancels.
#[derive(Clone)]
pub struct ConfirmPrompt {
    pub action_id: ActionId,
    pub title: String,
    pub detail: String,
    pub danger: bool,
}

/// Recents palette state. When set, a centered popup shows the MRU
/// list of paths; Enter selects one (and updates the focused
/// browser's path), Esc closes.
pub struct RecentsPalette {
    pub paths: Vec<RecentPath>,
    pub cursor: usize,
}

/// Job history overlay. When set, a centered popup lists past jobs;
/// Enter re-runs the focused one with the same inputs.
pub struct HistoryOverlay {
    pub jobs: Vec<crate::jobs::Job>,
    pub cursor: usize,
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
    pub action_metas: Vec<actions::ActionInfo>,
    pub last_status: Vec<Option<JobStatus>>,
    pub browsers: HashMap<ActionId, Browser>,
    /// Per-action picker state for `InputKind::Choice` actions.
    pub choice_state: HashMap<ActionId, ChoicePicker>,
    /// Active confirmation prompt, if any.
    pub confirming: Option<ConfirmPrompt>,
    /// Recents palette state, when open.
    pub recents: Option<RecentsPalette>,
    /// History overlay state, when open.
    pub history_overlay: Option<HistoryOverlay>,
    /// Persisted state (loaded at startup, saved on quit).
    pub state: PersistedState,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        let action_metas = actions::metadata();
        let last_status = vec![None; action_metas.len()];
        let loaded = PersistedState::load();

        // Restore last sidebar selection, if any.
        let sidebar_index = loaded
            .last_action
            .as_deref()
            .and_then(|id| action_metas.iter().position(|m| m.id == id))
            .unwrap_or(0);

        let mut app = Self {
            cli,
            theme: Theme::default(),
            focus: Focus::Sidebar,
            sidebar_index,
            show_help: false,
            log_lines: Vec::new(),
            should_quit: false,
            job_manager: JobManager::new(),
            action_metas,
            last_status,
            browsers: HashMap::new(),
            choice_state: HashMap::new(),
            confirming: None,
            recents: None,
            history_overlay: None,
            state: loaded,
        };
        app.push_log(
            LogStream::Info,
            "mtui ready. Press ? for help, Tab to cycle focus, q to quit.".into(),
        );
        app.push_log(
            LogStream::Info,
            "M6: state persistence + recents. @ opens recents.".into(),
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

    fn browser_for(&mut self, action_id: ActionId, input_kind: InputKind) -> &mut Browser {
        self.browsers
            .entry(action_id)
            .or_insert_with(|| Browser::new(default_browser_path(), input_kind))
    }

    fn choice_picker_for(&mut self, action_id: ActionId) -> &mut ChoicePicker {
        self.choice_state
            .entry(action_id)
            .or_insert_with(|| ChoicePicker::new(reset_ableton::discover()))
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

        // Persist state on quit. Best-effort; don't block quit on failure.
        self.state.last_action = self
            .action_metas
            .get(self.sidebar_index)
            .map(|m| m.id.to_string());
        if let Err(e) = self.state.save() {
            eprintln!("mtui: warning: could not save state: {e}");
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

        // Confirm prompt is modal: Enter runs, Esc cancels.
        if let Some(prompt) = self.confirming.clone() {
            match action {
                InputAction::Enter | InputAction::Quit => {
                    self.confirming = None;
                    self.start_confirmed_action(prompt).await;
                }
                InputAction::Esc => {
                    self.confirming = None;
                    self.push_log(LogStream::Info, format!("Cancelled \"{}\".", prompt.title));
                }
                _ => {}
            }
            return;
        }

        // Dispatch to the choice picker if the focused action is a Choice.
        if self.focus == Focus::Main {
            if let Some(meta) = self.action_metas.get(self.sidebar_index).cloned() {
                if meta.input_kind == InputKind::Choice {
                    match action {
                        InputAction::SidebarDown => {
                            self.choice_picker_for(meta.id).cursor_down();
                            return;
                        }
                        InputAction::SidebarUp => {
                            self.choice_picker_for(meta.id).cursor_up();
                            return;
                        }
                        InputAction::SidebarTop => {
                            if let Some(p) = self.choice_state.get_mut(&meta.id) {
                                p.cursor = 0;
                            }
                            return;
                        }
                        InputAction::SidebarBottom => {
                            if let Some(p) = self.choice_state.get_mut(&meta.id) {
                                if !p.versions.is_empty() {
                                    p.cursor = p.versions.len() - 1;
                                }
                            }
                            return;
                        }
                        InputAction::Enter => {
                            self.begin_confirm_for_choice(&meta);
                            return;
                        }
                        InputAction::Refresh => {
                            // Re-discover installed versions.
                            let fresh = reset_ableton::discover();
                            if let Some(p) = self.choice_state.get_mut(&meta.id) {
                                p.versions = fresh;
                                p.cursor = p.cursor.min(p.versions.len().saturating_sub(1));
                            }
                            self.push_log(
                                LogStream::Info,
                                format!("Refreshed version list for \"{}\".", meta.label),
                            );
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Dispatch to the browser first if the main pane is focused.
        if self.focus == Focus::Main {
            if let Some(meta) = self.action_metas.get(self.sidebar_index).cloned() {
                if meta.input_kind == InputKind::Dir || meta.input_kind == InputKind::File {
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
            InputAction::Recents => {
                if !self.state.recent_paths.is_empty() {
                    self.recents = Some(RecentsPalette {
                        paths: self.state.recent_paths.clone(),
                        cursor: 0,
                    });
                } else {
                    self.push_log(
                        LogStream::Info,
                        "No recent paths yet. Run an action to populate.".into(),
                    );
                }
            }
            InputAction::GoUp => {
                // The browser would have consumed GoUp already (when main
                // is focused and the action has a browser). If we reach
                // here, the action has no browser — treat `h` as history.
                if !self.job_manager.history.is_empty() {
                    self.history_overlay = Some(HistoryOverlay {
                        jobs: self.job_manager.history.iter().cloned().collect(),
                        cursor: 0,
                    });
                } else {
                    self.push_log(LogStream::Info, "No job history yet.".into());
                }
            }
            InputAction::OpenEntry
            | InputAction::ToggleCheck
            | InputAction::PathInput
            | InputAction::GoHome
            | InputAction::Refresh
            | InputAction::Backspace
            | InputAction::Char(_) => {}
            InputAction::Noop => {}
        }
        // After the dispatch, also handle the recents palette if it's open.
        // (We handle it here so any key works, including Char for filtering.)
        if self.recents.is_some() {
            self.handle_recents_key(action.clone()).await;
        }
        if self.history_overlay.is_some() {
            self.handle_history_key(action).await;
        }
    }

    async fn handle_recents_key(&mut self, action: InputAction) {
        match action {
            InputAction::Esc | InputAction::Quit => {
                self.recents = None;
            }
            InputAction::SidebarDown => {
                if let Some(p) = self.recents.as_mut() {
                    if p.cursor + 1 < p.paths.len() {
                        p.cursor += 1;
                    }
                }
            }
            InputAction::SidebarUp => {
                if let Some(p) = self.recents.as_mut() {
                    if p.cursor > 0 {
                        p.cursor -= 1;
                    }
                }
            }
            InputAction::Enter => {
                let chosen = self
                    .recents
                    .as_ref()
                    .and_then(|p| p.paths.get(p.cursor).cloned());
                self.recents = None;
                if let Some(recent) = chosen {
                    self.apply_recent(recent);
                }
            }
            _ => {}
        }
    }

    /// Handle input when the history overlay is open.
    async fn handle_history_key(&mut self, action: InputAction) {
        match action {
            InputAction::Esc | InputAction::Quit | InputAction::GoUp => {
                self.history_overlay = None;
            }
            InputAction::SidebarDown => {
                if let Some(h) = self.history_overlay.as_mut() {
                    if h.cursor + 1 < h.jobs.len() {
                        h.cursor += 1;
                    }
                }
            }
            InputAction::SidebarUp => {
                if let Some(h) = self.history_overlay.as_mut() {
                    if h.cursor > 0 {
                        h.cursor -= 1;
                    }
                }
            }
            InputAction::Enter => {
                let chosen = self
                    .history_overlay
                    .as_ref()
                    .and_then(|h| h.jobs.get(h.cursor).cloned());
                self.history_overlay = None;
                if let Some(job) = chosen {
                    self.rerun_job(job).await;
                }
            }
            _ => {}
        }
    }

    /// Re-run a job from history with the same inputs.
    async fn rerun_job(&mut self, job: crate::jobs::Job) {
        if self.job_manager.is_running() {
            self.push_log(
                LogStream::Failure,
                "A job is already running. Press Esc to cancel.".into(),
            );
            return;
        }
        let Some(meta) = self
            .action_metas
            .iter()
            .find(|m| m.id == job.action_id)
            .cloned()
        else {
            self.push_log(
                LogStream::Failure,
                format!("No action registered for id \"{}\".", job.action_id),
            );
            return;
        };
        self.push_log(
            LogStream::Info,
            format!(
                "Re-running \"{}\" (job #{}) with previous inputs.",
                meta.label, job.id
            ),
        );
        self.start_action(&meta, job.inputs).await;
    }

    fn apply_recent(&mut self, recent: RecentPath) {
        // Update the current browser (if any) to the chosen path.
        if let Some(meta) = self.action_metas.get(self.sidebar_index).cloned() {
            if meta.input_kind == InputKind::Dir || meta.input_kind == InputKind::File {
                let path = PathBuf::from(&recent.path);
                let browser = self.browser_for(meta.id, meta.input_kind);
                browser.set_path(path.clone());
                self.push_log(
                    LogStream::Info,
                    format!("Switched to recent path: {}", recent.path),
                );
            } else {
                self.push_log(
                    LogStream::Info,
                    format!("Recent: {} (not applicable to this action)", recent.path),
                );
            }
        }
    }

    /// Enter was pressed on a Choice action — start a confirm prompt.
    fn begin_confirm_for_choice(&mut self, meta: &actions::ActionInfo) {
        let Some(picker) = self.choice_state.get(&meta.id) else {
            self.push_log(
                LogStream::Failure,
                format!("No versions discovered for \"{}\".", meta.label),
            );
            return;
        };
        let Some(version) = picker.selected() else {
            self.push_log(
                LogStream::Failure,
                format!("No versions discovered for \"{}\".", meta.label),
            );
            return;
        };
        let detail = format!(
            "Ableton {}\n  Prefs: ~/Library/Preferences/Ableton/{}\n  Templates: ~/Music/Ableton/User Library/Templates\n  Backups will land in ~/Desktop/.",
            version.version, version.version
        );
        self.confirming = Some(ConfirmPrompt {
            action_id: meta.id,
            title: format!("Reset \"{}\"", version.version),
            detail,
            danger: true,
        });
    }

    /// Enter was pressed on a None-input action (patch_ozone) — start
    /// a confirm prompt.
    fn begin_confirm_for_no_input(&mut self, meta: &actions::ActionInfo) {
        let (detail, danger) = match meta.id {
            "patch_ozone" => (
                "Patches the iZotope Ozone 12 core binary in place.\n  /Library/Application Support/iZotope/*/iZOzone12Core\n  The patch is reversible only by reinstalling.".to_string(),
                true,
            ),
            _ => (meta.hint.to_string(), false),
        };
        self.confirming = Some(ConfirmPrompt {
            action_id: meta.id,
            title: format!("Run \"{}\"", meta.label),
            detail,
            danger,
        });
    }

    /// Confirm was accepted; resolve inputs and start the job.
    async fn start_confirmed_action(&mut self, prompt: ConfirmPrompt) {
        let Some(meta) = self
            .action_metas
            .iter()
            .find(|m| m.id == prompt.action_id)
            .cloned()
        else {
            return;
        };
        let inputs = match meta.input_kind {
            InputKind::Choice => {
                let Some(picker) = self.choice_state.get(&meta.id) else {
                    return;
                };
                let Some(version) = picker.selected() else {
                    return;
                };
                crate::actions::ResolvedInputs {
                    choice: Some(version.version.clone()),
                    ..Default::default()
                }
            }
            _ => crate::actions::ResolvedInputs::default(),
        };
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

        match meta.input_kind {
            InputKind::None => {
                self.begin_confirm_for_no_input(&meta);
            }
            InputKind::Dir => {
                let browser = self.browser_for(meta.id, meta.input_kind);
                match browser.selected_dir() {
                    Some(d) => {
                        let inputs = crate::actions::ResolvedInputs {
                            dir: Some(d),
                            ..Default::default()
                        };
                        self.start_action(&meta, inputs).await;
                    }
                    None => {
                        self.push_log(
                            LogStream::Failure,
                            format!("Select a directory for \"{}\" first.", meta.label),
                        );
                    }
                }
            }
            InputKind::File => {
                let browser = self.browser_for(meta.id, meta.input_kind);
                let file = browser.resolved_file().or_else(|| {
                    let files = browser.checked_files();
                    files.first().cloned()
                });
                match file {
                    Some(f) => {
                        let inputs = crate::actions::ResolvedInputs {
                            file: Some(f),
                            ..Default::default()
                        };
                        self.start_action(&meta, inputs).await;
                    }
                    None => {
                        self.push_log(
                            LogStream::Failure,
                            format!("Select a file for \"{}\" first.", meta.label),
                        );
                    }
                }
            }
            InputKind::Choice => {
                self.begin_confirm_for_choice(&meta);
            }
        }
    }

    async fn start_action(
        &mut self,
        meta: &actions::ActionInfo,
        inputs: crate::actions::ResolvedInputs,
    ) {
        // Track recents for the directory / file we just used.
        if let Some(p) = inputs.dir.as_ref() {
            self.state.push_recent(p);
        } else if let Some(p) = inputs.file.as_ref() {
            if let Some(parent) = p.parent() {
                self.state.push_recent(parent);
            }
        }

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
    }
}

fn default_browser_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}
