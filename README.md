# Music Tools

A collection of bash scripts for installing music plugins fast (e.g. when
setting up a new Mac) plus a friendly Ratatui TUI that wraps them.

Setting up DAW environments is a pain and takes forever. This at least
automates the install and patching of your plugins.

## Bash usage

Add `bin/` to your shell `$PATH`:

```sh
# zsh
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# bash
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Run any of the bundled scripts:

```sh
# Install every .pkg in a directory
./bin/install_pkgs /path/to/pkg/directory

# Run every .command patcher in a directory
./bin/run_patchers /path/to/command/directory

# Copy *.vst / *.vst3 / *.component into /Library/Audio/Plug-Ins
cd /path/to/plugins && ./bin/move_kd_plugs

# Strip xattrs/quarantine and re-codesign
./bin/remove_protection "/Applications/My App.app"

# Reset an installed Ableton version (interactive, just `bash bin/reset_ableton`)
./bin/reset_ableton 12.1.5
```

## TUI

```sh
# Build the TUI (requires a stable Rust toolchain via rustup).
make tui

# Run it.
./tui/target/release/mtui

# Or symlink it into bin/ and run like any other command.
make install
./bin/mtui
```

### What you get

A three-pane TUI: a sidebar of every action grouped by category, a
main pane with the input UI (file browser, version picker, or run
button depending on the action), and a streaming log pane at the
bottom. `?` opens a help overlay listing every keybinding; `h` opens
the job history (with re-run); `@` opens the recent-paths palette.

### CLI flags

| Flag | What it does |
|---|---|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |
| `--dry-run` | Print what every action would do, then exit (no TUI launched) |
| `--reset-state` | Delete the persisted state file and exit |
| `--log-level <level>` | Set the tracing log level (default: `info`) |

### State

Per-session state (last action, recent paths, etc.) is persisted to
`$XDG_CONFIG_HOME/mtui/state.json` (default `~/.config/mtui/state.json`).
Per-job log files are written to
`$HOME/Library/Logs/mtui/<job-id>.log` for post-mortem.

### See also

- [`spec.md`](spec.md) — full design and milestone plan
- [`tui/README.md`](tui/README.md) — TUI build details
- `.github/workflows/ci.yml` — CI on macos-latest (`cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release`, smoke tests)

## License

Private. Not for redistribution.
