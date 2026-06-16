//! `remove_protection` action: strip xattrs/quarantine and re-codesign
//! a file or directory.

use crate::actions::{Action, ActionInfo, InputKind, ResolvedInputs};
use crate::process::CommandSpec;

use super::locate::bin_dir;

pub struct RemoveProtectionAction;

impl Action for RemoveProtectionAction {
    fn info(&self) -> ActionInfo {
        ActionInfo {
            id: "remove_protection",
            category: "Plugins",
            label: "Remove protection",
            hint: "Strip xattrs/quarantine and re-codesign a file or folder",
            input_kind: InputKind::File,
        }
    }

    fn requires_sudo(&self) -> bool {
        true
    }

    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec {
        let file = inputs.file.expect("remove_protection requires a file");
        let script = bin_dir()
            .expect("could not locate bin/ directory; set $MTUI_BIN_DIR or add bin/ to $PATH")
            .join("remove_protection");
        CommandSpec::new("bash").path_arg(&script).path_arg(&file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn info_matches() {
        let a = RemoveProtectionAction;
        let info = a.info();
        assert_eq!(info.id, "remove_protection");
        assert_eq!(info.input_kind, InputKind::File);
        assert!(a.requires_sudo());
    }

    #[test]
    fn command_passes_file() {
        let a = RemoveProtectionAction;
        let spec = a.build_command(ResolvedInputs {
            file: Some(Path::new("/Applications/My App.app").to_path_buf()),
            ..Default::default()
        });
        assert!(spec.args[0].ends_with("remove_protection"));
        assert_eq!(spec.args[1], "/Applications/My App.app");
    }
}
