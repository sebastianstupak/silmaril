# silm build + silm package — Design Spec

**Date:** 2026-03-17
**Status:** Approved
**ROADMAP:** CLI.5

---

## Goal

Add `silm build` and `silm package` commands to the `silm` CLI. These commands operate inside a Silmaril game project (detected via `game.toml`), build the game for one or more target platforms, and produce distributable artifacts in a `dist/` folder.

These commands are for building **game projects created by `silm new`** — not for building the Silmaril engine itself.

**Project root detection:** `silm build` and `silm package` reuse the existing `find_project_root` function (walks up from the current directory looking for `game.toml`). They can be run from the project root or any subdirectory within the project.

**Multi-platform failure behavior:** When building all platforms (no `--platform` flag), platform failures are handled as follows:
- macOS targets (`macos-x86_64`, `macos-arm64`): non-fatal — failure is logged via `tracing::warn!` and the build continues with the next platform.
- All other platforms: fatal — the command exits immediately with the error. macOS is the only experimental carve-out in this iteration.

---

## Approach

**Thin wrapper (Approach A).** `silm build` invokes the right underlying tool per platform:

- Native builds: `cargo build`
- Cross-platform native builds: `cross build` (Docker-based, pre-built toolchains)
- WASM client: `trunk build` (Trunk handles wasm-bindgen, wasm-opt, JS glue)

`silm package` runs a release build then assembles `dist/<platform>/` directories and creates a zip per platform.

**Approach B (installer formats — AppImage, DMG, NSIS via `cargo-packager`) is explicitly deferred to CLI.7 Polish.** A `// TODO(CLI.7): cargo-packager` comment is placed in `package.rs` at the zip-creation step.

---

## Commands

### `silm build`

```
silm build                          # build all platforms in game.toml [build.platforms]
silm build --platform wasm          # override: build one platform only
silm build --release                # release profile (LTO etc.)
silm build --env-file .env.prod     # load env vars from file (default: .env)
```

`--platform` accepts a single value. Building multiple specific platforms requires running the command twice. This is a known limitation; multi-value `--platform` can be added in CLI.7.

If `--platform` is given, `[build]` is not required in `game.toml`. If `--platform` is absent and `[build]` is absent, `silm build` errors with a prompt to add `[build]` or use `--platform`.

### `silm package`

```
silm package                        # package all platforms in game.toml [build.platforms]
silm package --platform native      # native server + client only
silm package --platform server      # server binary + Dockerfile only
silm package --platform wasm        # WASM bundle only
silm package --out-dir ./releases   # override zip output dir (default: project root)
```

`silm package` always builds with `--release` implicitly. Zips land in the project root (or `--out-dir`). `dist/<platform>/` is wiped and recreated each run to avoid stale files (no `--no-clean` flag in this iteration).

---

## game.toml Integration

`game.toml` gains a `[build]` section:

```toml
[build]
platforms = ["windows-x86_64", "linux-x86_64", "linux-arm64", "wasm"]

[build.env]
SERVER_ADDRESS = "ws://localhost:7777"
SERVER_PORT = "7777"
```

- `platforms`: list of targets to build when no `--platform` flag is given.
- `[build.env]`: project-level env var defaults committed to the repo. Represents the shared baseline (e.g. default dev server address). Overridden by `.env` or shell env at build time.

Missing `version` in `[project]` defaults to `"0.0.0"` in zip filenames.

---

## Environment Variable Propagation

### Precedence (highest wins)

1. **Shell env** — variables already set in the calling process
2. **`--env-file <path>`** — file specified on the command line
3. **`.env`** — file in project root (ignored if not present, never an error)
4. **`game.toml [build.env]`** — project-level defaults committed to repo
5. **`option_env!(...).unwrap_or(...)`** — hardcoded fallback in game source code

`silm build` merges these layers and passes the result as the subprocess environment for `cargo`/`cross`/`trunk`. A lower-priority source is skipped for any key that is already supplied by a higher-priority source.

### How it works

Build a `HashMap<String, String>` (the "extra env") from the three non-shell sources in ascending priority order:
1. Insert all `game.toml [build.env]` entries.
2. Insert all `.env` entries, overwriting `[build.env]` values (`.env` > `[build.env]`).
3. Insert all `--env-file` entries, overwriting `.env` values (`--env-file` > `.env`).

Then, for each key in the extra env map, check whether the key is already set in the shell environment via `std::env::var(&key)`. If it is, skip it (shell wins). If not, pass it to the subprocess via `std::process::Command::env(&key, &value)`.

The subprocess inherits the full shell environment by default (`Command` does not clear env). The above step only adds keys that the shell did not already provide, ensuring shell env always wins.

Game client code uses compile-time env reading:
```rust
const SERVER_ADDRESS: &str = option_env!("SERVER_ADDRESS")
    .unwrap_or("ws://localhost:7777");
```

`silm` does **not** modify game source code — it only sets the subprocess environment.

### `.env` file format

Standard `KEY=VALUE` per line, `#` for comments, no shell expansion. Duplicate keys: last definition wins.
```bash
SERVER_ADDRESS=ws://localhost:7777
SERVER_PORT=7777
# MACOS_SDK_URL=https://...  # only needed for macOS cross-builds
```

### Dockerfile env

The generated `Dockerfile` gets commented `ENV` lines for each key in `game.toml [build.env]`, reminding operators what vars exist and that they can be overridden at runtime:
```dockerfile
# Override at runtime: docker run -e SERVER_ADDRESS=wss://prod.example.com ...
ENV SERVER_PORT=7777
```

---

## Platform Targets

| Platform key | Rust target triple | Host behaviour | Builds | Tool |
|---|---|---|---|---|
| `native` | host triple | always | server + client | `cargo` |
| `server` | host triple | always | server only | `cargo` |
| `windows-x86_64` | `x86_64-pc-windows-gnu` (cross) / `x86_64-pc-windows-msvc` (native) | see note | server + client | `cross` or `cargo` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | always cross | server + client | `cross` |
| `linux-arm64` | `aarch64-unknown-linux-gnu` | always cross | server + client | `cross` |
| `macos-x86_64` | `x86_64-apple-darwin` | experimental | server + client | `cross`* |
| `macos-arm64` | `aarch64-apple-darwin` | experimental | server + client | `cross`* |
| `wasm` | `wasm32-unknown-unknown` | always | client only | `trunk` |

**`native` vs `server`:** `server` is a convenience subset of `native` (server binary only). Having both in `game.toml [build.platforms]` is valid — `server` is built separately into `dist/server/` alongside `dist/native/`. No deduplication of the server binary itself (the build runs twice, which is fast since Cargo caches).

**Windows host detection:** When `silm build --platform windows-x86_64` is run on a Windows host, it uses `cargo build --target x86_64-pc-windows-msvc` (no Docker required). When run on Linux/macOS, it uses `cross build --target x86_64-pc-windows-gnu`. `silm build` detects the host OS via `std::env::consts::OS` and selects accordingly.

**macOS cross-compilation (experimental):** `cross` macOS support requires `MACOS_SDK_URL` in the environment and is fragile. `macos-x86_64` and `macos-arm64` are supported but marked experimental — `silm build` prints a warning before attempting them. Failure is non-fatal when building all platforms; the build continues with other targets.

**WASM:** Only the `client/` crate is built for WASM. The server always stays native.

### Binary names

`silm build` reads `[dev] server_package` and `[dev] client_package` from `game.toml` to determine the `--package` argument. It invokes:
```bash
cargo build --package <server_package> --bin server [--release]
cargo build --package <client_package> --bin client [--release]
```
If `[dev]` is absent, it falls back to `<project_name>-server` and `<project_name>-client`.

**Constraint:** `--bin server` and `--bin client` are hardcoded names. They match the `[[bin]] name = "server"` and `[[bin]] name = "client"` entries generated by `silm new`. If the user renames their binaries in `Cargo.toml`, the build will fail with a Cargo error. This constraint is intentional for simplicity in this iteration; configurable binary names are deferred to CLI.7.

---

## Actual Command Invocations

### Native (`cargo`)
```bash
cargo build --package my-game-server --bin server [--release]
cargo build --package my-game-client --bin client [--release]
```

### Cross-platform (`cross`)
```bash
cross build --target x86_64-pc-windows-gnu --package my-game-server --bin server --release
cross build --target x86_64-pc-windows-gnu --package my-game-client --bin client --release
```

### WASM (`trunk`)
```bash
# Run from project root; --dist points to top-level dist/wasm/
trunk build client/index.html --dist dist/wasm [--release]
```

Trunk is invoked with `--dist dist/wasm` so its output lands directly in the correct location without a separate copy step. The working directory is the project root.

---

## Tool Detection

Before running any build, `silm build` checks for required tools and emits actionable errors:

| Condition | Error message |
|---|---|
| `trunk` not on PATH | `error: 'trunk' not found — install: cargo install trunk` |
| `cross` not on PATH | `error: 'cross' not found — install: cargo install cross` |
| Docker not running (cross needed) | `error: Docker is not running — start Docker Desktop, then retry` |
| `client/index.html` missing (WASM) | `error: WASM build requires client/index.html — not found` |
| `MACOS_SDK_URL` unset (macOS cross) | `error: macOS cross-build requires MACOS_SDK_URL — see: https://github.com/cross-rs/cross/wiki/Recipes` |

Tool detection uses `which`-style PATH lookup (`std::process::Command::new("trunk").arg("--version").output()`). Docker detection uses `docker info` exit code.

Error messages surface via `anyhow::bail!` and are printed by the `main()` error handler. No `println!` or `eprintln!` — all user-facing output uses `tracing::info!` for progress and `anyhow::bail!` for errors, consistent with the rest of the CLI.

---

## Output Structure

### `silm build` output

Artifacts land in standard Cargo output (`target/`) — `silm build` does not move them.

### `silm package` output

```
<project-root>/
├── dist/
│   ├── native/
│   │   ├── server            (or server.exe on Windows host)
│   │   ├── client
│   │   └── assets/           (copied from project-root/assets/ if present)
│   ├── windows-x86_64/
│   │   ├── server.exe
│   │   ├── client.exe
│   │   └── assets/
│   ├── linux-x86_64/
│   │   ├── server
│   │   ├── client
│   │   └── assets/
│   ├── wasm/                 (Trunk output via --dist dist/wasm)
│   │   ├── index.html
│   │   ├── <hash>.js
│   │   ├── <hash>_bg.wasm
│   │   └── assets/
│   └── server/
│       ├── server
│       └── Dockerfile
├── my-game-v0.1.0-native.zip
├── my-game-v0.1.0-windows-x86_64.zip
├── my-game-v0.1.0-wasm.zip
└── my-game-v0.1.0-server.zip
```

- **Folder names** under `dist/` are stable (platform key only) — easy to reference in CI.
- **Zip filenames** include `<project_name>-v<version>-<platform>`. Version from `game.toml [project] version`, defaulting to `0.0.0` if absent.
- Zips are created using the `zip` crate v2 (added to `engine/cli/Cargo.toml`).
- `dist/<platform>/` is wiped and recreated on each `silm package` run.
- `assets/` is copied from `<project-root>/assets/` if the directory exists; silently skipped if absent.

### Generated Dockerfile (`dist/server/Dockerfile`)

```dockerfile
FROM debian:bookworm-slim
COPY server /usr/local/bin/server
EXPOSE 7777/udp

# Override at runtime: docker run -e SERVER_ADDRESS=wss://prod.example.com ...
ENV SERVER_PORT=7777

ENTRYPOINT ["/usr/local/bin/server"]
```

---

## Template Updates (`silm new`)

`engine/cli/src/templates/basic.rs` is updated to:

1. Add `[build]` section to generated `game.toml`:
```toml
[build]
platforms = ["native", "wasm"]

[build.env]
SERVER_ADDRESS = "ws://localhost:7777"
SERVER_PORT = "7777"
```

2. Add `client/index.html` stub for Trunk. Trunk processes this file via its `data-trunk` directive — it does **not** use a manual `<script>` import. The `rel="rust"` link tells Trunk to build the Rust crate in the same directory:
```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8"/>
    <title>My Game</title>
  </head>
  <body>
    <canvas id="silmaril"></canvas>
    <link data-trunk rel="rust" data-wasm-opt="z"/>
  </body>
</html>
```

3. Add `dist/` and `*.zip` to generated `.gitignore`.

---

## Code Structure

New files:
```
engine/cli/src/commands/build/
├── mod.rs       — BuildCommand + PackageCommand enums, handle_build_command,
│                  handle_package_command, platform resolution, tool detection
├── native.rs    — cargo/cross build for server + client; Windows host detection
├── wasm.rs      — trunk build invocation
├── env.rs       — .env file parsing, game.toml [build.env] parsing, env merge
└── package.rs   — dist/ assembly, asset copy, zip creation, Dockerfile generation
                   (TODO(CLI.7): cargo-packager integration for AppImage/DMG/NSIS)
```

Modified:
- `engine/cli/src/commands/mod.rs` — add `pub mod build;`
- `engine/cli/src/main.rs` — add `Commands::Build` and `Commands::Package`
- `engine/cli/src/templates/basic.rs` — add `[build]` to game.toml, add `client/index.html`, update `.gitignore`
- `engine/cli/Cargo.toml` — add `zip = "2"` dependency

---

## Testing

### Unit tests (`engine/cli/tests/build_tests.rs`)

Pure logic, no subprocess, single-crate. These are Tier 1 tests per CLAUDE.md:

- `.env` parsing: `KEY=VALUE`, comments, empty lines, blank values, duplicate keys (last wins)
- Env merge: shell > `--env-file` > `.env` > `[build.env]`; higher priority never overwritten
- Platform string → Rust target triple mapping (all 8 platforms)
- Platform string → tool selection (cargo vs cross vs trunk)
- Windows host detection: `windows-x86_64` uses `msvc` on Windows host, `gnu` via `cross` elsewhere
- Unknown platform string → error with known-platforms list
- `dist/` path construction per platform
- Zip filename construction: name + version; missing version → `0.0.0`
- Dockerfile template generation from `[build.env]` entries
- `game.toml [build]` parsing: platforms list, env section, missing section

### Integration tests (`engine/cli/tests/build_integration_tests.rs`)

Real filesystem + real project structure, subprocess command captured (not executed) via a mock runner trait. Single-crate (Tier 1 per CLAUDE.md since only the CLI crate is imported):

- `silm build --platform native` → captured command is `cargo build --package ... --bin server`
- `silm build --platform wasm` → captured command is `trunk build client/index.html --dist dist/wasm`
- `silm build --release` → `--release` flag present in captured command
- Env vars from `.env` present in captured subprocess environment
- Shell env vars take precedence over `.env` vars
- `--env-file` takes precedence over `.env`
- `[build.env]` vars present when no `.env` file exists
- Missing `[build]` + no `--platform` → error with helpful message
- Missing `[build]` + `--platform native` → succeeds (no error)
- Unknown platform `"darwin"` → error listing known platforms
- `silm package` on a fake project: `dist/native/` created, zip created, Dockerfile present in `dist/server/`
- Assets dir present → copied to `dist/<platform>/assets/`
- Assets dir absent → no error

### E2E tests (CI-gated, in `scripts/e2e-tests/test-silm-build.sh`)

Real tool invocations on a real `silm new` project. Skips gracefully if tool is absent:

- `silm build --platform native` → binaries appear in `target/debug/`
- `silm build --platform native --release` → binaries appear in `target/release/`
- `silm package --platform native` → `dist/native/` populated, zip created at project root
- `silm package --platform server` → `dist/server/` has binary + Dockerfile
- WASM: skipped with `INFO` log if `trunk` not on PATH (not a test failure)
- `cross`: skipped with `INFO` log if Docker not running (not a test failure)

### Tool detection tests (within `build_tests.rs`)

- `trunk` absent → `anyhow::Error` with "cargo install trunk" in message
- `cross` absent → `anyhow::Error` with "cargo install cross" in message
- Docker not running → `anyhow::Error` with "start Docker Desktop" in message

---

## Known Limitations (this iteration)

- `--platform` accepts a single value; multi-value support deferred to CLI.7
- macOS cross-compilation is experimental and may fail; non-fatal in multi-platform builds
- No installer formats (AppImage, DMG, NSIS) — deferred to CLI.7 via `cargo-packager`
- No `--watch` mode for `silm build`
- No `--no-clean` flag for `silm package`

---

## Future Work (CLI.7)

- `cargo-packager` integration: AppImage (Linux), DMG + .app bundle (macOS), NSIS installer (Windows)
- Multi-value `--platform` flag
- `silm build --watch` (rebuild on file change)
- Progress bars during long builds
- `--target` alias for `--platform` (Cargo convention alignment)
