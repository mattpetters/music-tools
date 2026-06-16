//! The Action trait and the in-process action registry.
//!
//! Each `bin/` script (and the `ozone-12` patcher) corresponds to one
//! `Action` implementation. The trait decouples "what does the action
//! need from the user" (the [`InputKind`]) from "how does it execute"
//! ([`build_command`]). New actions are added by:
//!
//! 1. Defining a struct and `impl Action` for it in a submodule
//!    (e.g. [`install_pkgs::InstallPkgsAction`]). Existing submodule
//!    actions: install_pkgs, move_kd_plugs, run_patchers,
//!    remove_protection. Coming soon: reset_ableton, patch_ozone.
//! 2. Adding the struct to [`all`].

use crate::process::CommandSpec;

pub mod install_pkgs;
pub mod locate;
pub mod move_kd_plugs;
pub mod patch_ozone;
pub mod remove_protection;
pub mod reset_ableton;
pub mod run_patchers;

use install_pkgs::InstallPkgsAction;
use move_kd_plugs::MoveKdPlugsAction;
use patch_ozone::PatchOzoneAction;
use remove_protection::RemoveProtectionAction;
use reset_ableton::ResetAbletonAction;
use run_patchers::RunPatchersAction;

/// Inputs collected from the user. Default is empty (used by
/// no-input actions and the TestAction).
#[derive(Debug, Default, Clone)]
#[allow(dead_code)] // `choice` is read in M4 by the version picker.
pub struct ResolvedInputs {
    pub dir: Option<std::path::PathBuf>,
    pub file: Option<std::path::PathBuf>,
    pub choice: Option<String>,
}

impl ResolvedInputs {
    /// True if the given input kind requires user input. Used by
    /// `mtui --dry-run` to decide whether to build a command.
    #[allow(dead_code)]
    pub fn input_kind_requires_user_input(kind: &InputKind) -> bool {
        !matches!(kind, InputKind::None)
    }
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
    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec;
}

// =====================================================================
// TestAction (M2 only; removed once all real actions land)
// =====================================================================

/// A dev-only action that just runs a shell command. Proves the
/// process + streaming + cancellation pipeline end-to-end. Lives under
/// the "Dev" category. Will be removed once all real actions land.
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
// ComingSoonAction (placeholder for actions not yet implemented)
// =====================================================================

/// Placeholder used for actions that aren't wired up yet (M4/M5).
/// Renders a "Coming soon" main pane and logs a hint when run.
#[allow(dead_code)] // Kept for future actions; current 6 actions are all real.
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
        true
    }

    fn build_command(&self, _inputs: ResolvedInputs) -> CommandSpec {
        // We never actually run this — the App short-circuits ComingSoon
        // actions and just logs a "coming soon" line. If we did run it,
        // `true` would exit 0 immediately.
        CommandSpec::new("true")
    }
}

// =====================================================================
// Metadata table (sidebar reads this)
// =====================================================================

/// Static metadata for all registered actions, in display order.
pub fn metadata() -> Vec<ActionInfo> {
    all().iter().map(|a| a.info()).collect()
}

// =====================================================================
// Registry
// =====================================================================

/// All registered actions, in display order. Real actions (M3+) come
/// first by category, then the M4/M5 placeholders, then the dev
/// TestAction at the very end.
pub fn all() -> Vec<Box<dyn Action>> {
    vec![
        // Installers
        Box::new(InstallPkgsAction),
        Box::new(RunPatchersAction),
        // Plugins
        Box::new(MoveKdPlugsAction),
        Box::new(RemoveProtectionAction),
        // Maintenance
        Box::new(ResetAbletonAction),
        // System patches
        Box::new(PatchOzoneAction),
        // Dev
        Box::new(TestAction::new(
            "echo 'hello from mtui'; echo 'this is stderr' >&2; sleep 0.2; echo 'done'",
        )),
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
    fn registry_has_four_real_plus_two_coming_soon_plus_test() {
        let actions = all();
        // 4 real + 2 placeholders + 1 test = 7
        assert_eq!(actions.len(), 7);
        let meta = metadata();
        assert_eq!(meta.len(), 7);
        // The four M3 actions are present and first.
        let ids: Vec<&str> = meta.iter().map(|m| m.id).collect();
        for required in [
            "install_pkgs",
            "run_patchers",
            "move_kd_plugs",
            "remove_protection",
        ] {
            assert!(ids.contains(&required), "missing real action: {required}");
        }
        // All six spec actions plus the dev TestAction.
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
        // TestAction is last.
        assert_eq!(meta.last().unwrap().id, TestAction::ID);
    }
}
