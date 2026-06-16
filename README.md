# Music Tools

This repo is a collection of scripts I wrote to help install music plugins faster (ie; when getting a new computer). Setting up DAW environments is a pain in the ass and takes forever so this at least automates the install and patching of your plugins.

## Usage

For ease of use probably add the `bin` folder to your shell `$PATH`

```sh
# For bash users
cd music-tools
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh users
cd music-tools
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

```sh
# Install packages
./bin/install_pkgs /path/to/pkg/directory

# Run patchers
./bin/run_patchers /path/to/command/directory
```

## TUI

There's also a Ratatui-based TUI that wraps every `bin/` script behind a
guided UI: directory browser, streaming log pane, job history, the works.
The TUI is opt-in — the bash scripts above still work without it.

```sh
# Build the TUI (requires a stable Rust toolchain).
make tui

# Run it.
./tui/target/release/mtui

# Or symlink it into bin/ and run like any other command in this repo.
make install
./bin/mtui
```

See [`tui/README.md`](tui/README.md) for build details and [`spec.md`](spec.md)
for the design and milestone plan. Current status: **M1 (skeleton)** — the
TUI launches and renders the layout, but no actions are wired up yet.
