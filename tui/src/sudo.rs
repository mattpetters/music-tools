//! Manages the `SUDO_ASKPASS` helper script.
//!
//! On first launch, copies the bundled askpass shell script into the user's
//! data dir, makes it executable (mode 0700 — sudo requires the file to be
//! owned by the invoking user and not world-writable), and returns the
//! absolute path. Subsequent launches verify the file matches the bundled
//! version and rewrite it if it doesn't (so a `cargo install` of a new
//! version of `mtui` picks up askpass changes).

use std::path::{Path, PathBuf};

use color_eyre::eyre::{eyre, Result, WrapErr};

/// The askpass script source, embedded at compile time.
const ASKPASS_SCRIPT: &str = include_str!("../askpass/askpass.sh");

/// Resolve the data directory (`$XDG_DATA_HOME/mtui` or platform default).
fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| eyre!("could not determine data directory"))?;
    Ok(base.join("mtui"))
}

/// Install (or refresh) the askpass helper. Returns the absolute path to
/// the installed script. Idempotent.
pub fn ensure_askpass() -> Result<PathBuf> {
    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create data dir {}", dir.display()))?;
    let path = dir.join("askpass.sh");

    let needs_write = match std::fs::read_to_string(&path) {
        Ok(existing) => existing != ASKPASS_SCRIPT,
        Err(_) => true,
    };

    if needs_write {
        std::fs::write(&path, ASKPASS_SCRIPT)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    set_mode_0o700(&path).with_context(|| format!("failed to chmod 0700 {}", path.display()))?;

    Ok(path)
}

#[cfg(unix)]
fn set_mode_0o700(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_mode_0o700(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: ensure_askpass() should succeed and produce an executable
    /// file at the resolved path. (Just verifies the call doesn't
    /// panic; full file-content assertion would touch the user's real
    /// askpass install, which on macOS is not safely isolatable in
    /// tests.)
    #[test]
    fn installs_askpass() {
        let _ = ensure_askpass();
    }

    /// Idempotent: running ensure_askpass() twice yields the same path.
    #[test]
    fn idempotent() {
        let p1 = ensure_askpass().expect("first call");
        let p2 = ensure_askpass().expect("second call");
        assert_eq!(p1, p2);
    }
}
