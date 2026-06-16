.PHONY: tui build release check fmt clippy test install clean all help

# Default target: release build of the TUI binary.
all: release

## release: optimized build (default).
release:
	cd tui && cargo build --release

## tui: alias for `release`.
tui: release

## build: debug build, faster, slower binary.
build:
	cd tui && cargo build

## check: type-check without producing binaries.
check:
	cd tui && cargo check

## fmt: run rustfmt across the workspace.
fmt:
	cd tui && cargo fmt --all

## clippy: lint with deny-warnings.
clippy:
	cd tui && cargo clippy --all-targets -- -D warnings

## test: run all tests.
test:
	cd tui && cargo test

## install: build release and symlink into bin/mtui so the README's PATH
## advice lets `mtui` resolve to the TUI binary.
install: release
	ln -sf ../tui/target/release/mtui bin/mtui

## clean: remove build artifacts.
clean:
	cd tui && cargo clean

## help: print this list.
help:
	@grep -E '^##' Makefile | sed -E 's/^## //'
