# `mtui` — TUI for music-tools

This is a Ratatui-based front-end for the bash scripts in `../bin/`. It is
opt-in: the bash scripts remain the source of truth and work without any of
this code.

## Building

From the repo root:

```sh
make tui         # debug build → tui/target/debug/mtui
make             # release build → tui/target/release/mtui (default target)
make install     # symlinks the release binary into bin/mtui
```

Requires a stable Rust toolchain (pinned via `rust-toolchain.toml`):

```sh
rustup default stable   # or: rustup install stable
```

## Running

```sh
./tui/target/release/mtui
# or, after `make install`:
./bin/mtui
```

## Current status: M1 (skeleton)

- Three-pane layout: sidebar / main / log, with status bar and footer.
- Sidebar lists all six actions, grouped by category.
- Main pane shows the focused action's description and a "Coming soon" hint.
- Log pane streams in-app events (welcome, key presses, errors).
- `?` opens a help overlay listing all keybindings.
- `q` / `Ctrl-C` quits cleanly with terminal restoration.

No actions are wired up yet. See `../spec.md` §9 for the milestone plan.
