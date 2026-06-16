//! `move_kd_plugs` action: copy *.vst / *.vst3 / *.component from a chosen
//! directory into `/Library/Audio/Plug-Ins/...`.

use crate::actions::{Action, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

pub struct MoveKdPlugsAction;

impl Action for MoveKdPlugsAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "move_kd_plugs",
            category: "Plugins",
            label: "Move k'd plugins",
            hint: "Copy *.vst / *.vst3 / *.component into /Library/Audio/Plug-Ins",
            input_kind: InputKind::Dir,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec {
        let dir = inputs.dir.expect("move_kd_plugs requires a directory");
        // move_kd_plugs is CWD-driven: it `cp`s `*.vst` etc. in the current
        // working directory. We set the child's cwd to the chosen dir.
        let script = bin_dir()
            .expect("could not locate bin/ directory; set $MTUI_BIN_DIR or add bin/ to $PATH")
            .join("move_kd_plugs");
        CommandSpec::new("bash").path_arg(&script).cwd(&dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn info_matches() {
        let a = MoveKdPlugsAction;
        let info = a.info();
        assert_eq!(info.id, "move_kd_plugs");
        assert_eq!(info.input_kind, InputKind::Dir);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_sets_cwd_to_input_dir() {
        let a = MoveKdPlugsAction;
        let spec = a.build_command(ResolvedInputs {
            dir: Some(Path::new("/tmp/kd").to_path_buf()),
            ..Default::default()
        });
        assert_eq!(spec.cwd.as_deref(), Some(Path::new("/tmp/kd")));
        assert!(spec.args[0].ends_with("move_kd_plugs"));
    }
}
