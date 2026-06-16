//! `patch_ozone` action: binary-patch iZotope Ozone 12's core libraries.
//!
//! The bundled bash script in `ozone-12/patch-ozone.sh` finds
//! `/Library/Application Support/iZotope/*/iZOzone12Core` and applies a
//! `xxd | sed | xxd` round-trip followed by a re-codesign. This is
//! destructive in the sense that it modifies system binaries — the
//! App surfaces a red "danger zone" border and a y/n confirmation
//! before running.

use crate::actions::{Action, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

pub struct PatchOzoneAction;

impl Action for PatchOzoneAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "patch_ozone",
            category: "System patches",
            label: "Patch iZotope Ozone 12",
            hint: "Binary-patch iZotope Ozone 12 core libraries (system-level)",
            input_kind: InputKind::None,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, _inputs: ResolvedInputs) -> CommandSpec {
        let script = bin_dir()
            .expect("could not locate repo root; set $MTUI_BIN_DIR")
            .parent()
            .expect("bin_dir has a parent")
            .join("ozone-12")
            .join("patch-ozone.sh");
        CommandSpec::new("bash").path_arg(&script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_matches() {
        let a = PatchOzoneAction;
        let info = a.info();
        assert_eq!(info.id, "patch_ozone");
        assert_eq!(info.input_kind, InputKind::None);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_targets_ozone_script() {
        let a = PatchOzoneAction;
        let spec = a.build_command(ResolvedInputs::default());
        assert!(spec.args[0].ends_with("patch-ozone.sh"));
    }
}
