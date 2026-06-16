//! Generic file/dir browser widget.
//!
//! Renders a path field at the top, a list of entries below, and a
//! one-line hint at the bottom. Supports single-selection (cursor on
//! one entry) and multi-selection (Space toggles a checked state on
//! the entry under the cursor).
//!
//! The widget is generic over input kind: for [`InputKind::Dir`] the
//! caller treats the selected entry as a directory; for
//! [`InputKind::File`] the caller treats the selection as file(s).
//! The widget itself doesn't enforce this.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::actions::InputKind;
use crate::input::InputAction;
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Size and Mtime are bound to `s` (cycle sort) in M3+.
pub enum SortBy {
    Name,
    Size,
    Mtime,
}

pub struct Browser {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    pub show_hidden: bool,
    pub sort: SortBy,
    pub path_input: String,
    pub path_input_active: bool,
    pub error: Option<String>,
    /// Multi-select set (file paths). Used for File input actions
    /// where the user can select multiple files.
    pub checked: std::collections::HashSet<PathBuf>,
    pub input_kind: InputKind,
}

impl Browser {
    pub fn new(initial_path: PathBuf, input_kind: InputKind) -> Self {
        let mut b = Self {
            path: initial_path,
            entries: Vec::new(),
            cursor: 0,
            show_hidden: false,
            sort: SortBy::Name,
            path_input: String::new(),
            path_input_active: false,
            error: None,
            checked: std::collections::HashSet::new(),
            input_kind,
        };
        b.read_dir();
        b
    }

    /// Re-read the current directory. Resets cursor to 0.
    pub fn read_dir(&mut self) {
        self.error = None;
        self.entries.clear();
        let read = std::fs::read_dir(&self.path);
        match read {
            Err(e) => {
                self.error = Some(format!("cannot read {}: {e}", self.path.display()));
            }
            Ok(rd) => {
                for entry in rd.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let meta = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let is_dir = meta.is_dir();
                    let size = if is_dir { 0 } else { meta.len() };
                    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    self.entries.push(Entry {
                        name,
                        path: entry.path(),
                        is_dir,
                        size,
                        modified,
                    });
                }
                self.sort_entries();
            }
        }
    }

    fn sort_entries(&mut self) {
        match self.sort {
            SortBy::Name => self.entries.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Size => self.entries.sort_by_key(|e| std::cmp::Reverse(e.size)),
            SortBy::Mtime => self.entries.sort_by_key(|e| std::cmp::Reverse(e.modified)),
        }
    }

    /// The entry under the cursor, if any.
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.cursor)
    }

    /// Currently-selected directory (for Dir input kind).
    pub fn selected_dir(&self) -> Option<PathBuf> {
        match self.input_kind {
            InputKind::Dir => self
                .selected_entry()
                .filter(|e| e.is_dir)
                .map(|e| e.path.clone()),
            _ => None,
        }
    }

    /// Currently-checked file paths (for File input kind).
    pub fn checked_files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self.checked.iter().cloned().collect();
        out.sort();
        out
    }

    /// Currently-checked file or the entry under cursor (for File input
    /// kind). Falls back to the cursor entry if no files are checked.
    pub fn resolved_file(&self) -> Option<PathBuf> {
        let files = self.checked_files();
        if !files.is_empty() {
            return Some(files[0].clone()); // primary
        }
        self.selected_entry().map(|e| e.path.clone())
    }

    pub fn cursor_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn cursor_top(&mut self) {
        self.cursor = 0;
    }

    pub fn cursor_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.cursor = self.entries.len() - 1;
        }
    }

    /// Open the entry under the cursor (cd into dir, or no-op for files).
    pub fn open_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor).cloned() {
            if entry.is_dir {
                self.path = entry.path;
                self.checked.clear();
                self.cursor = 0;
                self.read_dir();
            }
        }
    }

    /// Go up one directory.
    pub fn go_up(&mut self) {
        if let Some(parent) = self.path.parent().map(Path::to_path_buf) {
            self.path = parent;
            self.checked.clear();
            self.cursor = 0;
            self.read_dir();
        }
    }

    /// Toggle the "checked" state of the entry under the cursor.
    pub fn toggle_checked(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor) {
            if !entry.is_dir {
                let p = entry.path.clone();
                if !self.checked.remove(&p) {
                    self.checked.insert(p);
                }
            }
        }
    }

    /// Clear all checked entries.
    #[allow(dead_code)] // Bound to `c` (clear selection) in M3+.
    pub fn clear_checked(&mut self) {
        self.checked.clear();
    }

    /// Set the path from the input field (after the user presses Enter
    /// in path-input mode) and re-read.
    pub fn commit_path_input(&mut self) {
        self.path_input_active = false;
        let new_path = std::path::PathBuf::from(self.path_input.trim());
        if new_path.is_absolute() || new_path.starts_with("~") {
            let expanded =
                if let Some(stripped) = new_path.to_str().and_then(|s| s.strip_prefix("~/")) {
                    if let Some(home) = dirs::home_dir() {
                        home.join(stripped)
                    } else {
                        new_path
                    }
                } else {
                    new_path
                };
            self.path = expanded;
            self.cursor = 0;
            self.checked.clear();
            self.read_dir();
        } else {
            self.error = Some(format!("not an absolute path: {}", new_path.display()));
        }
    }

    /// Type a character into the path field.
    pub fn path_input_push(&mut self, c: char) {
        self.path_input.push(c);
    }

    /// Backspace in the path field.
    pub fn path_input_pop(&mut self) {
        self.path_input.pop();
    }

    /// Set the path to a specific value (used by `~` shortcut).
    pub fn set_path(&mut self, p: PathBuf) {
        self.path = p;
        self.cursor = 0;
        self.checked.clear();
        self.read_dir();
    }

    /// Handle a key event. Returns true if the key was consumed.
    pub fn handle_key(&mut self, action: InputAction) -> bool {
        if self.path_input_active {
            match action {
                InputAction::Esc => {
                    self.path_input_active = false;
                    self.path_input.clear();
                }
                InputAction::Enter => {
                    self.commit_path_input();
                }
                InputAction::Backspace => {
                    self.path_input_pop();
                }
                InputAction::Char(c) => {
                    self.path_input_push(c);
                }
                _ => {
                    // All keys are absorbed while path-input is active.
                }
            }
            return true;
        }

        match action {
            InputAction::SidebarDown => self.cursor_down(),
            InputAction::SidebarUp => self.cursor_up(),
            InputAction::SidebarTop => self.cursor_top(),
            InputAction::SidebarBottom => self.cursor_bottom(),
            InputAction::OpenEntry => self.open_selected(),
            InputAction::GoUp => self.go_up(),
            InputAction::ToggleCheck => self.toggle_checked(),
            InputAction::PathInput => {
                self.path_input_active = true;
                self.path_input = self.path.to_string_lossy().to_string();
            }
            InputAction::GoHome => {
                if let Some(home) = dirs::home_dir() {
                    self.set_path(home);
                }
            }
            InputAction::Refresh => self.read_dir(),
            // We deliberately do NOT consume Enter — the App uses Enter
            // to "run the action". The user opens dirs with 'l'/Right.
            _ => return false,
        }
        true
    }

    pub fn render(&self, theme: &Theme, focused: bool, area: Rect, frame: &mut Frame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style_border(focused))
            .title(Span::styled(
                " Browser ",
                theme.style_border(focused).fg(theme.border_focused),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: path field (1 row) | spacer (0) | list (rest) | hint (1 row)
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(3),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(inner);

        // Path field
        let path_display = if self.path_input_active {
            format!("> {}_", self.path_input)
        } else {
            format!("  {}", self.path.display())
        };
        let path_style = if self.path_input_active {
            theme.style_warn()
        } else {
            theme.style_text()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(path_display, path_style)),
            chunks[0],
        );

        // Error line (if any) replaces the first list row
        if let Some(err) = &self.error {
            let line = Line::from(Span::styled(format!("  {}", err), theme.style_danger()));
            frame.render_widget(Paragraph::new(line), chunks[1]);
        } else if self.entries.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled("  (empty directory)", theme.style_dim())),
                chunks[1],
            );
        } else {
            let items: Vec<ListItem> = self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let is_cursor = i == self.cursor;
                    let is_checked = self.checked.contains(&e.path);
                    let cursor_marker = if is_cursor { "▶" } else { " " };
                    let check_marker = match (self.input_kind, e.is_dir) {
                        (InputKind::File, false) if is_checked => "[x] ",
                        (InputKind::File, false) => "[ ] ",
                        _ => "    ",
                    };
                    let type_marker = if e.is_dir { "/" } else { " " };
                    let name_style = if is_cursor {
                        theme.style_highlight()
                    } else if e.is_dir {
                        theme.style_warn()
                    } else {
                        theme.style_text()
                    };
                    let size_str = if e.is_dir {
                        "—".to_string()
                    } else {
                        human_size(e.size)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {} ", cursor_marker), theme.style_dim()),
                        Span::styled(check_marker.to_string(), theme.style_dim()),
                        Span::styled(
                            format!("{:<32}", format!("{}{}", e.name, type_marker)),
                            name_style,
                        ),
                        Span::styled(format!("{:>10}", size_str), theme.style_dim()),
                    ]))
                })
                .collect();

            let mut state = ListState::default();
            state.select(Some(self.cursor));
            let list = List::new(items)
                .highlight_style(theme.style_highlight())
                .highlight_symbol("");
            frame.render_stateful_widget(list, chunks[1], &mut state);
        }

        // Hint line
        let hint = match (self.input_kind, self.path_input_active) {
            (_, true) => "  [Esc] cancel  [Enter] open".to_string(),
            (InputKind::Dir, false) => {
                "  [j/k] move  [Enter] open  [Esc] up  [/] path  [~] home  [r] refresh".to_string()
            }
            (InputKind::File, false) => {
                "  [j/k] move  [Space] check  [Esc] up  [/] path  [r] refresh".to_string()
            }
            _ => String::new(),
        };
        frame.render_widget(
            Paragraph::new(Span::styled(hint, theme.style_dim())),
            chunks[2],
        );
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1}{}", size, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn reads_dir_and_lists_entries() {
        let tmp = temp_dir();
        std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();

        let b = Browser::new(tmp.path().to_path_buf(), InputKind::Dir);
        assert!(b.error.is_none());
        assert!(b.entries.iter().any(|e| e.name == "a.txt" && !e.is_dir));
        assert!(b.entries.iter().any(|e| e.name == "sub" && e.is_dir));
    }

    #[test]
    fn hidden_files_excluded_by_default() {
        let tmp = temp_dir();
        std::fs::write(tmp.path().join(".hidden"), "").unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "").unwrap();

        let b = Browser::new(tmp.path().to_path_buf(), InputKind::Dir);
        assert!(b.entries.iter().any(|e| e.name == "visible.txt"));
        assert!(!b.entries.iter().any(|e| e.name == ".hidden"));

        let mut b2 = b;
        b2.show_hidden = true;
        b2.read_dir();
        assert!(b2.entries.iter().any(|e| e.name == ".hidden"));
    }

    #[test]
    fn cursor_navigation() {
        let tmp = temp_dir();
        std::fs::write(tmp.path().join("a"), "").unwrap();
        std::fs::write(tmp.path().join("b"), "").unwrap();
        std::fs::write(tmp.path().join("c"), "").unwrap();

        let mut b = Browser::new(tmp.path().to_path_buf(), InputKind::Dir);
        assert_eq!(b.cursor, 0);
        b.cursor_down();
        assert_eq!(b.cursor, 1);
        b.cursor_down();
        b.cursor_down();
        // Should not exceed the last entry.
        assert_eq!(b.cursor, 2);
        b.cursor_up();
        assert_eq!(b.cursor, 1);
        b.cursor_top();
        assert_eq!(b.cursor, 0);
        b.cursor_bottom();
        assert_eq!(b.cursor, 2);
    }

    #[test]
    fn toggle_checked_only_for_files() {
        let tmp = temp_dir();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();

        let mut b = Browser::new(tmp.path().to_path_buf(), InputKind::File);
        // Move cursor to a.txt
        let a_pos = b.entries.iter().position(|e| e.name == "a.txt").unwrap();
        b.cursor = a_pos;
        b.toggle_checked();
        assert_eq!(b.checked.len(), 1);
        b.toggle_checked();
        assert!(b.checked.is_empty());

        // Move cursor to sub (a dir)
        let sub_pos = b.entries.iter().position(|e| e.name == "sub").unwrap();
        b.cursor = sub_pos;
        b.toggle_checked();
        assert!(b.checked.is_empty(), "directories shouldn't be checkable");
    }
}
