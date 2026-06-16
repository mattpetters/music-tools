//! `run_patchers` action: run every .command in a chosen directory.

use crate::actions::{Action, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

pub struct RunPatchersAction;

impl Action for RunPatchersAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "run_patchers",
            category: "Installers",
            label: "Run patchers (.command)",
            hint: "Run every .command in the chosen directory",
            input_kind: InputKind::Dir,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec {
        let dir = inputs.dir.expect("run_patchers requires a directory");
        let script = bin_dir()
            .expect("could not locate bin/ directory; set $MTUI_BIN_DIR or add bin/ to $PATH")
            .join("run_patchers");
        CommandSpec::new("bash")
            .path_arg(&script)
            .path_arg(&dir)
            .cwd(&dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn info_matches() {
        let a = RunPatchersAction;
        let info = a.info();
        assert_eq!(info.id, "run_patchers");
        assert_eq!(info.input_kind, InputKind::Dir);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_targets_script() {
        let a = RunPatchersAction;
        let spec = a.build_command(ResolvedInputs {
            dir: Some(Path::new("/tmp/cmds").to_path_buf()),
            ..Default::default()
        });
        assert_eq!(spec.cwd.as_deref(), Some(Path::new("/tmp/cmds")));
        assert!(spec.args[0].ends_with("run_patchers"));
    }
}
