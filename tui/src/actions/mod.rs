//! The Action trait and the in-process action registry.
//!
//! Each `bin/` script (and the `ozone-12` patcher) corresponds to one
//! `Action` implementation. The trait decouples "what does the action
//! need from the user" (the [`InputKind`]) from "how does it execute"
//! ([`build_command`]). New actions are added by:
//!
//! 1. Defining a struct and `impl Action` for it (real actions live in
//!    submodules: `install_pkgs`, `move_kd_plugs`, etc. — added in M3+).
//! 2. Adding it to [`all`] and [`METADATA`].
//!
//! For M2 we have one real action ([`TestAction`]) and six
//! [`ComingSoonAction`] placeholders that match the spec's six final
//! actions. The placeholders show a "Coming soon" hint in the main pane
//! and are replaced in M3+.

use crate::process::CommandSpec;

/// Inputs collected from the user. Default is empty (used by
/// `TestAction` and by no-input actions).
#[derive(Debug, Default, Clone)]
#[allow(dead_code)] // Fields are populated in M3 when actions take user inputs.
pub struct ResolvedInputs {
    pub dir: Option<std::path::PathBuf>,
    pub file: Option<std::path::PathBuf>,
    pub choice: Option<String>,
}

/// The shape of inputs an action wants. Drives the main-pane widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// No input — just a "Run" button.
    None,
    /// A directory.
    Dir,
    /// A single file.
    File,
    /// A picker over a fixed set of options.
    Choice,
}

/// Stable, machine-readable ID. Used as the sidebar key and in the job
/// log to identify which action produced which line.
pub type ActionId = &'static str;

/// Display metadata.
#[derive(Debug, Clone)]
#[allow(dead_code)] // input_kind is read in M3 (input widgets) and M5 (sidebar polish).
pub struct ActionInfo {
    pub id: ActionId,
    pub category: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub input_kind: InputKind,
}

/// Trait implemented by every action.
pub trait Action: Send + Sync {
    fn info(&self) -> ActionInfo;

    /// Whether the spawned child should be wrapped in `sudo -A -E`. Most
    /// actions install files to `/Library/...` or modify system paths,
    /// so this is `true` by default. `TestAction` and read-only actions
    /// (browser previews, version listers) set it to `false`.
    fn requires_sudo(&self) -> bool {
        true
    }

    /// Build the command to run, given the user's resolved inputs.
    /// Called once, just before the job is started.
    fn build_command(&self, _inputs: ResolvedInputs) -> CommandSpec;
}

// =====================================================================
// TestAction (M2 only)
// =====================================================================

/// A dev-only action that just runs a shell command. Proves the
/// process + streaming + cancellation pipeline end-to-end. Lives under
/// the "Dev" category. Will be removed once real actions land in M3+.
pub struct TestAction {
    pub shell_command: String,
}

impl TestAction {
    pub const ID: ActionId = "test";

    pub fn new(shell_command: impl Into<String>) -> Self {
        Self {
            shell_command: shell_command.into(),
        }
    }
}

impl Action for TestAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: Self::ID,
            category: "Dev",
            label: "Test action",
            hint: "Runs a shell command via the streaming pipeline. Dev only.",
            input_kind: InputKind::None,
        }
    }

    fn requires_sudo(&self) -> bool {
        false
    }

    fn build_command(&self, _inputs: ResolvedInputs) -> CommandSpec {
        CommandSpec::new("bash").args(["-c", &self.shell_command])
    }
}

// =====================================================================
// ComingSoonAction (placeholder for the six real actions)
// =====================================================================

/// Placeholder used in M2 for the actions that aren't wired up yet.
/// Renders a "Coming soon" main pane and logs a hint when run.
#[derive(Debug, Clone, Copy)]
pub struct ComingSoonAction {
    pub id: ActionId,
    pub category: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub input_kind: InputKind,
}

impl Action for ComingSoonAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: self.id,
            category: self.category,
            label: self.label,
            hint: self.hint,
            input_kind: self.input_kind,
        }
    }

    fn requires_sudo(&self) -> bool {
        // The real actions need sudo; the placeholder is a no-op that
        // doesn't even spawn anything, so sudo doesn't matter — but
        // returning true keeps behavior consistent for when these get
        // swapped out in M3+.
        true
    }

    fn build_command(&self, _inputs: ResolvedInputs) -> CommandSpec {
        // We never actually run this — the App short-circuits ComingSoon
        // actions and just logs a "coming soon" line. If we did run it,
        // the command would do nothing.
        CommandSpec::new("true")
    }
}

// =====================================================================
// Metadata table (sidebar reads this)
// =====================================================================

/// Static metadata for all registered actions, in display order.
/// The sidebar reads from this; the App uses [`all`] to start jobs.
pub fn metadata() -> Vec<ActionInfo> {
    all().iter().map(|a| a.info()).collect()
}

// =====================================================================
// Registry
// =====================================================================

/// All registered actions, in display order. New actions are appended.
pub fn all() -> Vec<Box<dyn Action>> {
    vec![
        Box::new(TestAction::new(
            "echo 'hello from mtui'; echo 'this is stderr' >&2; sleep 0.2; echo 'done'",
        )),
        Box::new(ComingSoonAction {
            id: "install_pkgs",
            category: "Installers",
            label: "Install .pkg files",
            hint: "Install every .pkg in a chosen directory",
            input_kind: InputKind::Dir,
        }),
        Box::new(ComingSoonAction {
            id: "run_patchers",
            category: "Installers",
            label: "Run patchers (.command)",
            hint: "Run every .command in a chosen directory",
            input_kind: InputKind::Dir,
        }),
        Box::new(ComingSoonAction {
            id: "move_kd_plugs",
            category: "Plugins",
            label: "Move k'd plugins",
            hint: "Copy *.vst / *.vst3 / *.component into /Library/Audio/Plug-Ins",
            input_kind: InputKind::Dir,
        }),
        Box::new(ComingSoonAction {
            id: "remove_protection",
            category: "Plugins",
            label: "Remove protection",
            hint: "Strip xattrs/quarantine and re-codesign a file or folder",
            input_kind: InputKind::File,
        }),
        Box::new(ComingSoonAction {
            id: "reset_ableton",
            category: "Maintenance",
            label: "Reset Ableton",
            hint: "Back up & clear prefs + templates for an installed Ableton version",
            input_kind: InputKind::Choice,
        }),
        Box::new(ComingSoonAction {
            id: "patch_ozone",
            category: "System patches",
            label: "Patch iZotope Ozone 12",
            hint: "Binary-patch iZotope Ozone 12 core libraries (system-level)",
            input_kind: InputKind::None,
        }),
    ]
}

/// Number of registered actions. Convenience for the sidebar.
#[allow(dead_code)] // Used in render tests and the bottom keybinding row.
pub fn count() -> usize {
    metadata().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_runs_without_sudo() {
        let a = TestAction::new("echo x");
        assert!(!a.requires_sudo());
        let spec = a.build_command(ResolvedInputs::default());
        assert_eq!(spec.program, "bash");
        assert!(spec.args.contains(&"-c".to_string()));
    }

    #[test]
    fn registry_has_test_plus_six_placeholders() {
        let actions = all();
        assert_eq!(actions.len(), 7, "1 test + 6 placeholders");
        let meta = metadata();
        assert_eq!(meta.len(), 7);
        // First entry is the test action, in the Dev category.
        assert_eq!(meta[0].id, "test");
        assert_eq!(meta[0].category, "Dev");
        // Six placeholders cover the spec's six final actions.
        let ids: Vec<&str> = meta.iter().map(|m| m.id).collect();
        for required in [
            "install_pkgs",
            "run_patchers",
            "move_kd_plugs",
            "remove_protection",
            "reset_ableton",
            "patch_ozone",
        ] {
            assert!(ids.contains(&required), "missing action: {required}");
        }
    }
}
