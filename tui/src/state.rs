//! Persisted app state. Loaded at startup, saved on quit.
//!
//! Spec §6.5: `$XDG_CONFIG_HOME/mtui/state.json` (default
//! `~/.config/mtui/state.json`). Atomic write via tmp+rename.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

const MAX_RECENTS: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedState {
    pub version: u32,
    #[serde(default = "default_window")]
    pub window: WindowState,
    #[serde(default)]
    pub last_action: Option<String>,
    #[serde(default)]
    pub recent_paths: Vec<RecentPath>,
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_log_pane_pct")]
    pub log_pane_pct: u8,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub danger_zone_acknowledged: std::collections::HashMap<String, bool>,
}

fn default_window() -> WindowState {
    WindowState {
        cols: 120,
        rows: 40,
    }
}

fn default_log_pane_pct() -> u8 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentPath {
    pub path: String,
    pub last_used: SystemTime,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: 1,
            window: default_window(),
            last_action: None,
            recent_paths: Vec::new(),
            show_hidden: false,
            log_pane_pct: 30,
            sort: None,
            danger_zone_acknowledged: std::collections::HashMap::new(),
        }
    }
}

impl PersistedState {
    /// Path to the state file, or `None` if the user's config dir can't
    /// be located.
    pub fn config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
        Some(base.join("mtui").join("state.json"))
    }

    /// Load from disk. Falls back to defaults on missing file or parse
    /// error (we never want a corrupt state file to brick the app).
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to disk atomically (tmp + rename). Best-effort: errors are
    /// surfaced to the caller but the app continues running.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let s = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, s)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Push a path to the recents list (MRU; capped at 16).
    pub fn push_recent(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        self.recent_paths.retain(|r| r.path != path_str);
        self.recent_paths.insert(
            0,
            RecentPath {
                path: path_str,
                last_used: SystemTime::now(),
            },
        );
        if self.recent_paths.len() > MAX_RECENTS {
            self.recent_paths.truncate(MAX_RECENTS);
        }
    }

    /// Delete the on-disk state file. Used by `--reset-state`.
    pub fn reset() -> std::io::Result<()> {
        if let Some(path) = Self::config_path() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trip() {
        let s = PersistedState::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: PersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn push_recent_is_mru_and_capped() {
        let mut s = PersistedState::default();
        for i in 0..20 {
            s.push_recent(Path::new(&format!("/tmp/path-{i}")));
        }
        assert_eq!(s.recent_paths.len(), MAX_RECENTS);
        // Most recent is first.
        assert_eq!(s.recent_paths[0].path, "/tmp/path-19");
        // Pushing an existing path moves it to the front.
        s.push_recent(Path::new("/tmp/path-5"));
        assert_eq!(s.recent_paths[0].path, "/tmp/path-5");
        assert_eq!(
            s.recent_paths
                .iter()
                .filter(|r| r.path == "/tmp/path-5")
                .count(),
            1,
            "duplicates are removed"
        );
    }

    #[test]
    fn load_returns_default_when_missing() {
        // Override XDG_CONFIG_HOME to point at a nonexistent dir.
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/mtui-nonexistent-xyz");
        let s = PersistedState::load();
        assert_eq!(s, PersistedState::default());
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let mut s = PersistedState {
            last_action: Some("install_pkgs".into()),
            ..Default::default()
        };
        s.push_recent(Path::new("/tmp/foo"));
        s.save().expect("save");

        let loaded = PersistedState::load();
        assert_eq!(loaded.last_action, Some("install_pkgs".into()));
        assert_eq!(loaded.recent_paths[0].path, "/tmp/foo");

        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
