# CLI.6 Integration & Polish — Design Spec

**Date:** 2026-03-18
**Status:** Approved
**ROADMAP:** CLI.6

---

## Items

### 1. Multi-value `--platform`

Change `--platform` from `Option<String>` to `Option<Vec<String>>` in `BuildCommand` and `PackageCommand`. Use clap's `#[arg(long, num_args = 1..)]`. Update all call sites that read `.platform`.

### 2. Shell completions (`silm completions <shell>`)

Add `clap_complete` dependency. New `Commands::Completions` variant with a `shell` arg (bash, zsh, fish, powershell). New `commands/completions.rs` that calls `clap_complete::generate()` to stdout. Register in `main.rs`.

### 3. `silm build --watch`

Add `--watch` flag to `BuildCommand`. When set: run build once, then start a file watcher on `shared/`, `server/`, `client/`, and `assets/`. On file change, re-run the build. Use `notify` (already a dep via `notify-debouncer-full`). Ctrl+C exits. Debounce 500ms to avoid rapid rebuilds.

### 4. Progress indicators

Add `indicatif` dependency. In `build_platform` and `handle_package_command`, wrap each platform build with a spinner: `ProgressBar::new_spinner()` with message `"building <platform>..."`. On success: finish with `"✓ <platform>"`. On failure: finish with `"✗ <platform>"`. Keep tracing logs for detailed output.

### 5. `cargo-packager` integration

Add `--installer` flag to `PackageCommand`. When set, after creating zips, check for `cargo-packager` on PATH. If found, generate a `packager.toml` in the project root from game.toml metadata (name, version, description, binary paths), then run `cargo packager --config packager.toml`. If not found, print info message suggesting install. Supported formats: AppImage (Linux), DMG (macOS), NSIS (Windows).

### 6. Install/publish metadata

Add proper `[package]` metadata to `engine/cli/Cargo.toml`: description, repository, license, homepage, categories, keywords, readme. This enables `cargo install silm`.
