//! Locate the `bin/` directory containing the bash scripts.
//!
//! Search order:
//!   1. `$MTUI_BIN_DIR` if set
//!   2. `<cwd>/../bin` (running from `tui/`)
//!   3. `<cwd>/bin` (running from the repo root)
//!   4. `bin/` adjacent to the binary's location (via `current_exe`)
//!   5. `$PATH` (last resort — scripts must be on PATH)
//!
//! Returns the first directory that contains a known sentinel script
//! (`install_pkgs`).

use std::path::{Path, PathBuf};

/// Return the resolved `bin/` directory, or `None` if it can't be found.
pub fn bin_dir() -> Option<PathBuf> {
    resolve()
}

fn resolve() -> Option<PathBuf> {
    // 1. MTUI_BIN_DIR
    if let Ok(p) = std::env::var("MTUI_BIN_DIR") {
        let path = PathBuf::from(p);
        if is_bin_dir(&path) {
            return Some(canonicalize(path));
        }
    }

    // 2-3. cwd-relative
    if let Ok(cwd) = std::env::current_dir() {
        for candidate in [cwd.join("..").join("bin"), cwd.join("bin")] {
            if is_bin_dir(&candidate) {
                return Some(canonicalize(candidate));
            }
        }
    }

    // 4. Adjacent to current_exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // ../bin (binary in tui/target/release/mtui)
            let candidate = parent.join("..").join("..").join("..").join("bin");
            if is_bin_dir(&candidate) {
                return Some(canonicalize(candidate));
            }
            // bin/ (binary in bin/mtui, e.g. after `make install`)
            let candidate = parent.join("bin");
            if is_bin_dir(&candidate) {
                return Some(canonicalize(candidate));
            }
        }
    }

    // 5. PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if is_bin_dir(&dir) {
                return Some(dir);
            }
        }
    }

    None
}

fn is_bin_dir(path: &Path) -> bool {
    path.join("install_pkgs").is_file()
}

fn canonicalize(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_known_directory() {
        // When the tests run, cwd is tui/, so cwd/../bin is the repo's bin/.
        let dir = bin_dir();
        assert!(dir.is_some(), "could not locate bin/");
        let dir = dir.unwrap();
        assert!(dir.join("install_pkgs").is_file());
    }
}
