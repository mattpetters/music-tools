//! `install_pkgs` action: install every .pkg in a chosen directory.

use crate::actions::{Action, ActionId, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

pub struct InstallPkgsAction;

impl Action for InstallPkgsAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "install_pkgs",
            category: "Installers",
            label: "Install .pkg files",
            hint: "Install every .pkg in the chosen directory",
            input_kind: InputKind::Dir,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec {
        let dir = inputs.dir.expect("install_pkgs requires a directory");
        let script = bin_dir()
            .expect("could not locate bin/ directory; set $MTUI_BIN_DIR or add bin/ to $PATH")
            .join("install_pkgs");
        CommandSpec::new("bash")
            .path_arg(&script)
            .path_arg(&dir)
            .cwd(&dir)
    }
}

#[allow(dead_code)]
pub const ID: ActionId = "install_pkgs";
#[allow(dead_code)]
pub const SCRIPT_NAME: &str = "install_pkgs";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn info_matches() {
        let a = InstallPkgsAction;
        let info = a.info();
        assert_eq!(info.id, "install_pkgs");
        assert_eq!(info.input_kind, InputKind::Dir);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_targets_script_in_bin() {
        let a = InstallPkgsAction;
        let dir = Path::new("/tmp/fake");
        let spec = a.build_command(ResolvedInputs {
            dir: Some(dir.to_path_buf()),
            ..Default::default()
        });
        assert_eq!(spec.program, "bash");
        assert!(spec.args[0].ends_with("install_pkgs"));
        assert_eq!(spec.args[1], "/tmp/fake");
        assert_eq!(spec.cwd.as_deref(), Some(Path::new("/tmp/fake")));
    }
}
