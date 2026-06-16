# `mtui` — Ratatui front-end for `music-tools`

> **Status:** Approved v1.0, ready to implement. Open questions resolved; see §10.
> **Goal:** Wrap the five scripts in `bin/` (and the `ozone-12/patch-ozone.sh` outlier) in a friendly terminal UI so the workflow on a fresh Mac is: launch one binary, pick a directory, hit Enter, watch the logs scroll.

---

## 1. Goals & non-goals

### Goals

- One binary, `mtui`, that exposes every existing `bin/` action behind a guided UI.
- Each action takes its inputs through a purpose-built picker (directory browser, file browser, version list) — no more "paste a path you can't easily inspect."
- Streaming, colorized logs in a bottom pane; status badges in the sidebar; job history view.
- Reuses the existing bash scripts unchanged. They remain the source of truth for what each action *does*; the TUI is a coordinator.
- Plays nicely with `sudo` on macOS without breaking raw-mode TUI (via `SUDO_ASKPASS` + `osascript`).
- State (recent paths, last-used action, window size) persists across runs in a portable, XDG-style location.
- Stays optional: the bash scripts still work on their own; nobody is forced to install a Rust toolchain to use the repo.

### Non-goals (v1)

- Cross-platform. **macOS only.** All target scripts assume `sudo`, `installer`, `xattr`, `codesign`, `/Library/Audio/Plug-Ins/...`, and `~/Library/Preferences/Ableton/...`. We can revisit Linux later, but a Linux port will probably be a separate spec.
- A web UI, a Tauri/Egui companion, or anything that requires a browser/window manager.
- Replacing the bash scripts with native Rust implementations. (We may *mirror* a couple — see §6.4 on `reset_ableton` — but only when the bash version can't be cleanly driven by a non-tty parent.)
- Network features (downloading plugins, syncing between machines, etc.).
- Plugin system / scripting API. The action registry is a Rust trait, but it's compiled in.

---

## 2. User stories

1. **Fresh Mac, installing a pile of `.pkg`s.** "I just downloaded a folder of installers. I want to point the tool at that folder, see what's in it, and install them all in one go while I watch the log."
2. **Moving cracked plugins.** "I have a folder of `.vst`/`.vst3`/`.component` files. I want to drop them in the right system folders and then strip protection from each one."
3. **Running a directory of patchers.** "A release came as a folder of `.command` files. Run them all in order, show me what each one did, and tell me which failed."
4. **Resetting Ableton.** "I'm troubleshooting a corrupt session. Walk me through backing up and clearing the prefs/templates for a specific installed version."
5. **Re-running a previous job.** "I just reset Ableton 12.1.5. I want to do it again for 12.2.0 without re-picking the version."

---

## 3. Existing system analysis

Summarized for the implementer. The current scripts are kept verbatim; the TUI is a wrapper.

### 3.1 `bin/install_pkgs <dir>`

- Validates `<dir>` is a directory.
- `cd "$DIR"`, loops over `*.pkg`, runs `sudo installer -pkg "$PKG" -target /` for each.
- No parallelism. No per-file error handling — one failure doesn't stop the others (the loop just keeps going because `sudo installer` is a separate process).
- Has a `-h` help flag.

**TUI mapping:** directory input. Optionally: multi-select individual `.pkg` files (default: select all). Show the file list with sizes and a "run all" / "run selected" toggle.

### 3.2 `bin/move_kd_plugs`

- **No arguments.** Operates on the current working directory.
- Three `sudo cp -r` calls — one per plugin type — into `/Library/Audio/Plug-Ins/{VST,VST3,Components}/`.
- Glob expansion is shell-side, so it will silently no-op if there are no matches.

**TUI mapping:** directory input — the TUI must `cd` into the chosen dir before invoking, or invoke the script with that dir as CWD. The TUI should also pre-scan the directory and warn if any of the three globs is empty ("no `*.vst` found, skip?").

### 3.3 `bin/remove_protection <file>`

- Single argument: a path to a file (or directory, `xattr -r` is recursive).
- `xattr -cr`, `xattr -d com.apple.quarantine -r`, `codesign --force --sign -`.
- The `hex` function is defined but unused in this script (vestigial, do not preserve).

**TUI mapping:** file input. The natural mode is "drop a directory and apply this to everything inside" — the script's `xattr -cr` already recurses. Add a "recursively apply to folder contents" toggle (on by default for directories, hidden for files).

### 3.4 `bin/reset_ableton`

- Interactive: `read -p "Enter the version number to reset: " VERSION` after `ls`-ing the directory.
- Backs up `Preferences.cfg` to `~/Desktop/Preferences.backup.cfg`, deletes the live `Preferences.cfg` and the `Undo/` folder.
- Backs up `~/Music/Ableton/User Library/Templates` to `~/Desktop/Templates Backups`, deletes the live `Templates/`.

**TUI mapping:** *non-trivial.* The bash version blocks on stdin, which we cannot drive cleanly from a TUI. The action should:
1. Discover available versions by reading `~/Library/Preferences/Ableton/*/Preferences.cfg` directly from Rust.
2. Show a picker (versions are folders whose name is the version string, e.g. `12.1.5`).
3. After confirmation, run the equivalent commands via `tokio::process::Command` with `sudo -A` (see §6.1).
   - Alternative: keep a refactored `bin/reset_ableton <version>` that takes the version as an arg and non-interactively does the work. **Preferred** — preserves the "bash is source of truth" principle.

### 3.5 `bin/run_patchers <dir>`

- Loops `*.command` files in the dir, runs `sudo sh "$FILE"` on each.
- Skips non-matches cleanly.

**TUI mapping:** directory input. Multi-select individual `.command` files. Show order (filename-sorted); allow re-ordering by drag-key? — **out of scope for v1**, sort alphabetically and run in that order.

### 3.6 `ozone-12/patch-ozone.sh` (bonus)

- A standalone binary-patcher for iZotope Ozone 12. Lives outside `bin/` but is the same shape (single-purpose, long-running, needs `sudo`).
- `find /Library/Application Support/iZotope -name iZOzone12Core` then a `xxd | sed | xxd` round-trip, then re-codesign.
- Not destructive in a "rm -rf" sense but modifies system binaries. Show an extra-confirm gate in the UI.

**TUI mapping:** zero inputs beyond "are you sure." Add it as a sixth action under a "System patches" subheading in the sidebar.

---

## 4. Architecture

### 4.1 Tech stack

| Concern | Choice | Why |
|---|---|---|
| TUI | [`ratatui`](https://ratatui.rs) | Mandated by user; mature; great layout primitives. |
| Backend | [`crossterm`](https://github.com/crossterm-rs/crossterm) | Best macOS story for raw mode + truecolor + mouse (mouse optional). |
| Async runtime | [`tokio`](https://tokio.rs) | Process spawning, log reading, timeouts. |
| Errors | [`eyre`](https://github.com/eyre-rs/eyre) + `color-eyre` | Pretty backtraces, easy layering. |
| CLI | [`clap`](https://github.com/clap-rs/clap) (derive) | `--config-dir`, `--log-level`, `--dry-run`, `--reset-state`. |
| Serialization | `serde` + `serde_json` | Persisted state. |
| Path handling | `dirs` (XDG) + `home` | XDG-aware but with `$HOME` fallback. |
| Logging (file) | `tracing` + `tracing-subscriber` | Crash-safe log file alongside the on-screen log pane. |

### 4.2 Crate layout

Single crate, not a workspace. The codebase is small.

```
music-tools/
├── bin/                          # UNCHANGED bash scripts
├── ozone-12/                     # UNCHANGED
├── tui/                          # NEW — Rust crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs               # entry: CLI parse, terminal init, app.run()
│   │   ├── app.rs                # top-level App struct + main event loop
│   │   ├── config.rs             # CLI args + persisted state
│   │   ├── state.rs              # PersistedState (recent paths, last action, window size)
│   │   ├── theme.rs              # ColorScheme, Style helpers
│   │   ├── input.rs              # KeyEvent -> InputAction mapping
│   │   ├── jobs/
│   │   │   ├── mod.rs            # JobManager: queue, status, history
│   │   │   └── job.rs            # Job struct (id, action, status, logs, exit code)
│   │   ├── process/
│   │   │   ├── mod.rs            # spawn_with_sudo(), stream stdout/stderr
│   │   │   └── cancel.rs         # SIGTERM -> SIGKILL escalation
│   │   ├── sudo.rs               # ensure_askpass(): install + chmod the askpass script
│   │   ├── actions/
│   │   │   ├── mod.rs            # Action trait, ActionRegistry
│   │   │   ├── install_pkgs.rs
│   │   │   ├── move_kd_plugs.rs
│   │   │   ├── run_patchers.rs
│   │   │   ├── remove_protection.rs
│   │   │   ├── reset_ableton.rs
│   │   │   └── patch_ozone.rs
│   │   ├── ui/
│   │   │   ├── mod.rs            # render(App) -> Frame
│   │   │   ├── layout.rs         # root split: Sidebar | Main | Log
│   │   │   ├── sidebar.rs
│   │   │   ├── browser.rs        # generic file/dir browser widget
│   │   │   ├── log_pane.rs
│   │   │   ├── status_bar.rs
│   │   │   ├── help.rs           # ? overlay
│   │   │   └── confirm.rs        # y/n prompt modal
│   │   └── error.rs              # MtuiError + Result<T> alias
│   ├── tests/                    # integration tests
│   ├── askpass/
│   │   └── askpass.applescript   # template, materialized at runtime
│   └── README.md                 # how to build/run the TUI in isolation
├── Makefile                      # `make tui` builds; `make install` symlinks into bin/
└── spec.md                       # this file
```

### 4.3 Data flow

```
User keypress
    -> crossterm::event::read()
    -> app.handle_key(InputAction)
    -> app.dispatch()   (mutates App state, may enqueue Job)
    -> app.tick()       (polls JobManager, drains child stdout into Job.logs)
    -> app.render()     (ratatui::Terminal::draw)
    -> loop

Job execution (spawned on a tokio task):
    JobManager::start(job):
        ensure_askpass()
        tokio::process::Command::new("sudo")
            .arg("-A").arg("-E")
            .arg("bash").arg(SCRIPT_PATH)
            .args(extra_args)
            .current_dir(cwd)               // for move_kd_plugs
            .env("SUDO_ASKPASS", askpass_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)               // own process group -> cancellable
            .spawn()
        -> tokio::spawn reader tasks feeding Job.logs via mpsc
        -> await exit, set Job.status
```

### 4.4 The `Action` trait

```rust
pub trait Action: Send + Sync {
    fn id(&self) -> &'static str;             // "install_pkgs", "move_kd_plugs", ...
    fn label(&self) -> &'static str;          // "Install .pkg files"
    fn description(&self) -> &'static str;    // shown in main pane
    fn category(&self) -> ActionCategory;     // Installers | Plugins | Maintenance | System
    fn input_spec(&self) -> InputSpec;        // see below
    fn discover(&self, ctx: &Context) -> Vec<InputOption>;  // for picker-style inputs
    fn build_command(&self, inputs: ResolvedInputs) -> CommandSpec;
    fn confirm_message(&self, inputs: &ResolvedInputs) -> Option<String>;
}

pub struct CommandSpec {
    pub program: String,            // usually "sudo"
    pub args: Vec<String>,
    pub env: Vec<(String, String)>, // SUDO_ASKPASS etc
    pub cwd: Option<PathBuf>,
}

pub enum InputSpec {
    None,                                              // e.g. patch_ozone (after confirm)
    Dir { recursive: bool, multi: bool },              // install_pkgs, run_patchers, move_kd_plugs
    File,                                              // remove_protection
    Choice { source: ChoiceSource },                   // reset_ableton -> scan directory
}

pub enum ChoiceSource {
    AbletonVersions,                                   // ~/Library/Preferences/Ableton/*
}
```

Adding a new action = implement the trait + register it. This makes `ozone-12/patch-ozone.sh` trivial to add later, and makes v1 testing easier (a `TestAction` that echoes back its inputs is a one-liner).

---

## 5. UI specification

### 5.1 Layout

```
+---------------------------------------------------------------+
| mtui  1.0.0                                          ? help   |  <- status bar (1 line)
+--------+------------------------------------------------------+
|        |                                                      |
|        |  MAIN PANE (context-sensitive)                       |
| SIDE   |                                                      |
| BAR    |  For Dir inputs:    browser + path field + file list |
| (24c)  |  For File inputs:   browser in file mode             |
|        |  For Choice inputs: version list + confirm           |
|        |  For None:          big red "Run" button             |
|        |                                                      |
+--------+------------------------------------------------------+
| LOG PANE (streaming stdout/stderr, colorized, timestamped)    |
+---------------------------------------------------------------+
| [Tab] cycle focus  [Enter] run  [Esc] cancel  [q] quit        |  <- footer (1 line)
+---------------------------------------------------------------+
```

- Sidebar width: 24 cols, fixed. Resizable in a later version.
- Log pane height: 30% of terminal, draggable with `Ctrl-Up`/`Ctrl-Down` (Ratatui doesn't have a real splitter; we fake it with keybinds).
- All regions are focusable; the focused region has a brighter border.

### 5.2 Sidebar

Vertical list grouped by category:

```
  Installers
    > Install .pkg files
      Run patchers (.command)
  Plugins
      Move k'd plugins
      Remove protection
  Maintenance
      Reset Ableton
  System patches
      Patch iZotope Ozone 12
  ─────────────
      Quit
```

Each entry shows a status badge on the right:
- `·`   idle
- `…`   running (animated dot)
- `✓`   last run succeeded
- `✗`   last run failed
- `↻`   last run cancelled

Navigation: `j`/`k` or arrows. `Enter` selects (focuses main pane for that action). `g`/`G` jump to top/bottom. `q` quits.

### 5.3 Main pane (per action)

#### Common

- Title bar with action label + description.
- "Run" / "Run selected" button in the bottom-right of the pane. Also bound to `Enter` when the browser has focus and at least one item is selected.
- "Refresh" key (`r`) re-runs `discover()`.
- "Clear selection" key (`c`).
- "Show full path" toggle (`p`).
- "Danger zone" warning (red border + ⚠ icon) for actions that touch `/Library/...` or delete files: `move_kd_plugs`, `reset_ableton`, `patch_ozone`.

#### Browser widget

- Two-column layout: left is the directory tree (1 level deep by default; `→` to expand a folder), right is the file list for the currently focused directory.
- Top: editable path field (press `/` to focus, type to filter, `Tab` to commit).
- "Jump to" shortcuts: `~` (home), `.` (cwd at launch), `@` (each entry in "Recent locations" with a number).
- Selection model: `Space` toggles, `a` selects all, `n` selects none, `i` inverts.
- Sort: name, size, mtime — toggle with `s`.
- Hidden files: shown by default for `.command`/`.pkg` workflows; toggle with `.` (the dot key, conventional in tree views).

#### `remove_protection` (file mode)

- Same browser but file-filtered.
- If the selected path is a directory, the "Recursive" toggle appears (default on).
- A small panel shows the file's current xattr summary and codesign status (queried up front via `xattr -l` and `codesign -dv`; cached for 2s to avoid stutter when the user is keyboard-arrowing through files).

#### `reset_ableton` (choice mode)

- Main pane shows a list of discovered versions: each row has the version string, the `Preferences.cfg` mtime, the `Undo/` size, and the `Templates/` size.
- A single "selected" row (no multi).
- Big confirm button: "Reset `<version>` — backs up prefs + templates to `~/Desktop/`, then deletes them."
- On confirm, a second confirmation modal ("type the version number to confirm") to prevent misclicks.

#### `patch_ozone` (no-input mode)

- Centered card with: what it does, what binaries it touches, the exact `find` it'll run (read-only preview, computed live).
- A "Patch" button + a "Dry-run" toggle (defaults to dry-run ON, since this is the only action with no file list to show the user).

### 5.4 Log pane

- Each line: `[HH:MM:SS] [action-id] [stream] message`. Streams are `OUT` (white) and `ERR` (yellow). Lines matching `/Error:|error:|failed|FAIL/` get a red ⛔ prefix. Lines matching `/All .* installed successfully|done|success/i` get a green ✓ prefix.
- Auto-scrolls to bottom when at-bottom; freezes when the user scrolls up (highlight the "press `G` to resume" hint).
- `y` yanks the visible lines to the system clipboard (`pbcopy` on macOS).
- `Ctrl-L` clears the visible log (the on-disk `tracing` log is untouched).
- Per-action log files in `~/Library/Logs/mtui/<job-id>.log` for post-mortem — rotate at 5 MB, keep last 20.

### 5.5 Help overlay

`?` opens a modal listing every keybinding, grouped: *Navigation*, *Selection*, *Jobs*, *View*, *Global*. Scrollable, `Esc` closes. This is the spec of record; bind keys to match.

### 5.6 Keybinding summary (v1)

| Key | Action |
|---|---|
| `q` / `Ctrl-C` | Quit (confirm if a job is running) |
| `?` | Help overlay |
| `Tab` / `Shift-Tab` | Cycle focus: sidebar -> main -> log |
| `1`..`5` | Jump directly to action N in the sidebar |
| `j` / `k` / `↓` / `↑` | Move selection |
| `g` / `G` | Top / bottom of list |
| `h` / `l` / `←` / `→` | Collapse / expand in tree; history nav in browser |
| `Enter` | Confirm / Run |
| `Esc` | Cancel running job / close modal |
| `Space` | Toggle selection in multi-select inputs |
| `a` / `n` / `i` | Select all / none / invert |
| `r` | Refresh |
| `c` | Clear selection |
| `s` | Cycle sort |
| `.` | Toggle hidden files |
| `/` | Focus path field |
| `~` | Jump to `$HOME` |
| `@` | Recent locations palette |
| `p` | Toggle "show full path" |
| `y` | Yank visible log to clipboard |
| `Ctrl-L` | Clear visible log |
| `Ctrl-Up` / `Ctrl-Down` | Resize log pane |

---

## 6. Cross-cutting concerns

### 6.1 `sudo` and the TUI raw-mode problem

A child process started by a TUI in raw mode **cannot read a password from the same terminal** — the TUI owns stdin, and the password prompt is the wrong protocol. The standard fix on macOS is `SUDO_ASKPASS`:

1. On first launch, `mtui` materializes a small shell script at `~/Library/Application Support/mtui/askpass.sh` (XDG: `$XDG_DATA_HOME/mtui/askpass.sh`, default `~/.local/share/mtui/askpass.sh`):

   ```sh
   #!/bin/sh
   # Prompt for the sudo password via the macOS native dialog.
   /usr/bin/osascript <<'EOF'
   on run argv
     set thePrompt to item 1 of argv
     set theResult to text returned of (display dialog thePrompt ¬
       with title "mtui needs permission" ¬
       default answer "" ¬
       with hidden answer ¬
       with icon caution)
     return theResult
   end run
   EOF
   ```

2. `mtui` runs `chmod 700` on it (it's an unprivileged file the user owns; `sudo` will only invoke it if the file is not world-writable).
3. Every `sudo` invocation sets `SUDO_ASKPASS=<that path>` and passes `-A` (and `-E` to preserve `PATH`/env where the script needs it).
4. macOS will only invoke `SUDO_ASKPASS` if there's no recent auth — same caching rules as a normal `sudo`, so a long session only asks once.

The dialog text is supplied by `sudo` ("[sudo] password for matt:"); we pass it through unchanged.

**Edge case:** if the user has `timestamp_timeout=0` in sudoers, the dialog will pop on every command. That's their config; we don't fight it.

### 6.2 Process spawning & streaming

Use `tokio::process::Command`:

```rust
let mut child = Command::new("sudo")
    .args(["-A", "-E", "bash", SCRIPT])
    .args(script_args)
    .current_dir(cwd)
    .env("SUDO_ASKPASS", askpass_path)
    .env_remove("SUDO_USER")  // belt and suspenders
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .process_group(0)         // own pgid -> easy SIGTERM
    .kill_on_drop(true)
    .spawn()?;

// Two reader tasks forward to a mpsc::Sender<LogLine>:
let (tx, rx) = mpsc::channel(256);
tokio::spawn(stream_lines(child.stdout.take().unwrap(), "OUT", tx.clone()));
tokio::spawn(stream_lines(child.stderr.take().unwrap(), "ERR", tx.clone()));
drop(tx);  // so rx finishes when both readers exit

let status = child.wait().await?;
```

`stream_lines` is a small `BufReader::lines()` loop with a 256-line bounded channel; if the channel is full we drop the line and increment a "lines dropped" counter shown in the status bar.

### 6.3 Cancellation

`JobManager::cancel(id)` sends `nix::sys::signal::killpg(-pid, SIGTERM)`. Wait 3s; if the child is still alive, `SIGKILL`. The TUI disables the cancel key during the grace period (shows a countdown) to prevent double-press confusion.

### 6.4 Refactoring `bin/reset_ableton` (the one bash change)

The current `bin/reset_ableton` is a 21-line script that prompts with `read -p`. To drive it from a non-tty parent, change it to:

```bash
#!/bin/bash
# usage: reset_ableton <version>
set -euo pipefail
VERSION="${1:-}"
[[ -z "$VERSION" ]] && { echo "usage: $0 <version>" >&2; exit 2; }
PREFS="$HOME/Library/Preferences/Ableton/$VERSION"
[[ -d "$PREFS" ]] || { echo "no such version: $VERSION" >&2; exit 1; }
# ... existing backup + delete logic, with the version parameterized
```

The TUI's `discover()` returns the list of `<version>` dirs (the ones that contain a `Preferences.cfg`), and `build_command()` produces `sudo -A -E bash bin/reset_ableton <version>`. The bash script stays the source of truth.

This is the **only** bash change. Every other script already takes its inputs as args or CWD, so they're driven verbatim.

### 6.5 State persistence

File: `$XDG_CONFIG_HOME/mtui/state.json` (default `~/.config/mtui/state.json`).

```json
{
  "version": 1,
  "window": { "cols": 120, "rows": 40, "log_pane_pct": 30 },
  "last_action": "install_pkgs",
  "recent_paths": [
    { "path": "~/Downloads/installers", "last_used": "2026-06-15T10:11:12Z" },
    { "path": "~/Downloads/iZotope",     "last_used": "2026-06-14T22:01:00Z" }
  ],
  "show_hidden": true,
  "sort": "name",
  "danger_zone_acknowledged": {
    "move_kd_plugs":  false,
    "patch_ozone":    false
  }
}
```

`recent_paths` is MRU, capped at 16. On `discover()` for a Dir input, if the current path is missing/inaccessible, we walk back through recents until we find one that exists, and surface that to the user ("Falling back to ~/Downloads — original path was gone.").

### 6.6 Error handling

- **Fatal** (terminal init failure, can't load state, can't write askpass): pop a `color_eyre` report and exit non-zero.
- **Recoverable per-action** (bad path, no `.pkg` files, no `*.command` files, version doesn't exist): inline message in the main pane with a "Try again" focus jump. Job is not enqueued.
- **Job failure** (non-zero exit): log pane gets a red banner; sidebar badge flips to ✗; history view shows the exit code and last 50 log lines.
- **`sudo` auth failure** (askpass exits 1 or returns empty): banner "Authentication failed. Try again?" with a retry button that re-issues the same job. The auth dialog is the system's, so we don't get to see the password.

### 6.7 Accessibility / friendliness

- Status bar always shows the next required action ("Pick a directory", "Press Enter to run", "Authenticating…").
- All interactive widgets have a visible focus ring.
- Color is never the only signal — status badges use glyphs (`·…✓✗↻`) in addition to color so colorblind users aren't locked out.
- The footer always shows the context-sensitive binding hint, not a wall of keys.
- `--dry-run` global flag: runs every action's `discover()` and `build_command()` and prints what it would do, without spawning anything.

---

## 7. Testing strategy

| Layer | Approach |
|---|---|
| **Action trait** | Unit tests for `build_command()` on each action — assert the exact `argv` and `cwd`. No I/O. |
| **Discovery** | `discover()` for `reset_ableton` is unit-tested with a fake `$HOME` (`tempfile::TempDir`). |
| **Process layer** | Integration test that registers a `TestAction` whose command is `bash -c 'echo hello; echo world >&2; exit 0'`, runs it, asserts the log lines and the `Done` status. |
| **Streaming** | Stress test: `bash -c 'for i in $(seq 1 10000); do echo line $i; done'`, assert no drops and backpressure behaves. |
| **Cancel** | Test that a long-running child (`sleep 30`) gets SIGTERM'd within 100ms and reaped. |
| **Askpass** | In tests, override `SUDO_ASKPASS` to a fake script that writes to a file and exits 0, so we can assert `sudo -A` was invoked with the right env. |
| **UI** | Snapshot tests via `insta` for each pane, captured with a 120x40 fake terminal. |
| **Manual** | README has a 5-step smoke test: "launch, run `install_pkgs` on a 1-file test dir, see logs, check the file ran, quit." |

CI: GitHub Actions on macos-latest, `cargo test --all-features`, plus `cargo clippy -- -D warnings` and `cargo fmt --check`.

---

## 8. Distribution

### 8.1 `Makefile`

```make
.PHONY: tui install clean

tui:
	cd tui && cargo build --release

install: tui
	ln -sf ../tui/target/release/mtui bin/mtui

clean:
	cd tui && cargo clean
```

### 8.2 README additions

- A "TUI" section under Usage, showing `make tui && ./bin/mtui`.
- A "Building from source" section that lists the Rust toolchain requirement (`rustup default stable` is enough).
- A note that the bash scripts are unchanged and remain the canonical implementation.

### 8.3 Toolchain footprint

- macOS only, so we don't need cross-compilation.
- A `rust-toolchain.toml` pinning stable.
- No external system deps beyond what the bash scripts already use (`sudo`, `osascript`, `installer`, `xattr`, `codesign`, `pbcopy`). All are stock on macOS.

---

## 9. Implementation milestones

Each milestone ends with a green `cargo test`, a README update, and a `make tui` smoke check. PRs are reviewed one milestone at a time.

| # | Milestone | Deliverable |
|---|---|---|
| **M0** | **Spec sign-off** | This `spec.md` reviewed and merged to `main` on a feature branch. |
| **M1** | **Skeleton** | `tui/` crate compiles; `mtui --version` prints; `mtui` launches, shows "Coming soon" in the main pane, quits on `q`. No actions registered yet. |
| **M2** | **Process + askpass** | `process::spawn_with_sudo()` works; the askpass script is installed on first run; a `TestAction` registered under a "Dev" sidebar entry runs and streams logs. |
| **M3** | **Core actions** | `install_pkgs`, `move_kd_plugs`, `run_patchers`, `remove_protection` wired up end-to-end. Browser widget + log pane functional. **This is the first "real" milestone — usable on a fresh Mac for the install workflow.** |
| **M4** | **`reset_ableton`** | Refactored bash script + Rust action. Version picker + double-confirm modal. |
| **M5** | **`patch_ozone` + sidebar categories + help overlay** | All six actions live. Polished sidebar. `?` overlay. |
| **M6** | **State persistence + recents** | `state.json` round-trips; recents palette; window-size restore. |
| **M7** | **Cancellation + job history** | `Esc` cancels; history view (`h` key); per-job log files. |
| **M8** | **Polish + tests** | Snapshot tests, dry-run, clippy clean, README final pass. |
| **M9** | **CI + release** | GitHub Actions on macos-latest; tagged release; install instructions in README. |

---

## 10. Decision log

These were the open questions during spec review. All resolved; recorded here for traceability.

1. **`reset_ableton` refactor** — *Resolved.* The bash script will take `<version>` as a positional arg. This is the only bash change in the project. See §6.4.
2. **`SUDO_ASKPASS` approach** — *Resolved.* Use `SUDO_ASKPASS` pointing at a small `osascript`-driven helper. See §6.1.
3. **macOS-only for v1** — *Resolved.* No Linux/Windows port in v1; this is a personal-tooling project.
4. **Rust toolchain as a soft dep** — *Resolved.* `make tui` is opt-in; bash scripts keep working without Rust. `rust-toolchain.toml` pins stable.
5. **Binary name** — *Resolved.* `mtui`. Used throughout this spec.
6. **Sidebar width fixed at 24 cols for v1** — *Locked in.* Resizable later.
7. **No drag-to-reorder for `run_patchers` in v1** — *Locked in.* Alphabetical sort only.
8. **Per-action log files at `~/Library/Logs/mtui/<job-id>.log`** — *Locked in.* macOS convention; in-repo logs would litter the working dir and risk being committed.
9. **No network features in v1** — *Locked in.* Already in §1 non-goals.
10. **No plugin/scripting API in v1** — *Locked in.* Actions are compiled in. Already in §1 non-goals.

---

## 11. Future work (intentionally not in v1)

- Linux port (the bash scripts are macOS-bound; a Linux port would mean auditing and probably rewriting several of them, then porting the TUI).
- Mouse support (clickable sidebar, clickable buttons). Ratatui supports it; we just don't need it for v1.
- A `for_each <glob>` mode for `install_pkgs` that walks subdirectories.
- A dry-run diff view: "If I run `move_kd_plugs` on this folder, here's exactly which files will be copied where."
- A "session" mode: pick N actions up front, queue them, run them sequentially in one launch (e.g., install packages → run patchers → remove protection in a single keypress).
- Optional telemetry opt-in ("this run took 14s, install 3 pkgs, hit 0 errors") for personal usage stats. Default off.
