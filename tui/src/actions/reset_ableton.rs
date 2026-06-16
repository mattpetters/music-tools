//! `reset_ableton` action: discover installed Ableton versions, let the
//! user pick one, and reset its prefs + templates.
//!
//! Discovery scans `~/Library/Preferences/Ableton/*/Preferences.cfg`. A
//! folder is considered an "installed version" if it has a
//! `Preferences.cfg` inside it. The folder name is the version string
//! (e.g. `12.1.5`).
//!
//! The action itself is `InputKind::Choice`; the App reads the picked
//! version from `ResolvedInputs::choice`.

use std::path::Path;
use std::time::SystemTime;

use crate::actions::{Action, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

#[derive(Debug, Clone)]
#[allow(dead_code)] // templates_size is shown in the version picker detail row.
pub struct AbletonVersion {
    pub version: String,
    /// mtime of the `Preferences.cfg` file, if available.
    pub cfg_mtime: Option<SystemTime>,
    /// Total size of the `Undo/` directory in bytes, if any.
    pub undo_size: u64,
    /// Total size of the global `Templates/` directory in bytes.
    /// (We show this per row so the user knows what gets backed up.)
    pub templates_size: u64,
}

pub struct ResetAbletonAction;

impl Action for ResetAbletonAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "reset_ableton",
            category: "Maintenance",
            label: "Reset Ableton",
            hint: "Back up & clear prefs + templates for an installed Ableton version",
            input_kind: InputKind::Choice,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec {
        let version = inputs
            .choice
            .expect("reset_ableton requires a chosen version");
        let script = bin_dir()
            .expect("could not locate bin/ directory; set $MTUI_BIN_DIR or add bin/ to $PATH")
            .join("reset_ableton");
        CommandSpec::new("bash").path_arg(&script).arg(version)
    }
}

/// Discover installed Ableton versions.
pub fn discover() -> Vec<AbletonVersion> {
    let mut out = Vec::new();
    let Some(home) = dirs::home_dir() else {
        return out;
    };
    let prefs_root = home.join("Library").join("Preferences").join("Ableton");
    let Ok(rd) = std::fs::read_dir(&prefs_root) else {
        return out;
    };

    // Compute the Templates dir size once, not per-version.
    let templates_path = home
        .join("Music")
        .join("Ableton")
        .join("User Library")
        .join("Templates");
    let templates_size = dir_size(&templates_path);

    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let cfg = path.join("Preferences.cfg");
        if !cfg.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        let cfg_mtime = std::fs::metadata(&cfg).and_then(|m| m.modified()).ok();
        let undo_size = dir_size(&path.join("Undo"));
        out.push(AbletonVersion {
            version: name,
            cfg_mtime,
            undo_size,
            templates_size,
        });
    }
    out.sort_by(|a, b| a.version.cmp(&b.version));
    out
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(rd) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in rd.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            total += dir_size(&entry.path());
        } else {
            total += meta.len();
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_matches() {
        let a = ResetAbletonAction;
        let info = a.info();
        assert_eq!(info.id, "reset_ableton");
        assert_eq!(info.input_kind, InputKind::Choice);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_passes_version() {
        let a = ResetAbletonAction;
        let spec = a.build_command(ResolvedInputs {
            choice: Some("12.1.5".into()),
            ..Default::default()
        });
        assert!(spec.args[0].ends_with("reset_ableton"));
        assert_eq!(spec.args[1], "12.1.5");
    }

    #[test]
    fn missing_choice_panics() {
        let a = ResetAbletonAction;
        let result = std::panic::catch_unwind(|| a.build_command(ResolvedInputs::default()));
        assert!(result.is_err(), "should panic when choice is missing");
    }

    #[test]
    fn discover_does_not_panic() {
        // We can't assert what's installed on the test machine, but we
        // can at least verify discovery doesn't panic and returns
        // sorted output.
        let versions = discover();
        for pair in versions.windows(2) {
            assert!(pair[0].version <= pair[1].version);
        }
    }
}
