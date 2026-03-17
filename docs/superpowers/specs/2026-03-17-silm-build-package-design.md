# silm build + silm package — Design Spec

**Date:** 2026-03-17
**Status:** Approved
**ROADMAP:** CLI.5

---

## Goal

Add `silm build` and `silm package` commands to the `silm` CLI. These commands operate inside a Silmaril game project (detected via `game.toml`), build the game for one or more target platforms, and produce distributable artifacts in a `dist/` folder.

These commands are for building **game projects created by `silm new`** — not for building the Silmaril engine itself.

---

## Approach

**Thin wrapper (Approach A).** `silm build` invokes the right underlying tool per platform:

- Native builds: `cargo build`
- Cross-platform native builds: `cross build` (Docker-based, pre-built toolchains)
- WASM client: `trunk build` (Trunk handles wasm-bindgen, wasm-opt, JS glue)

`silm package` runs a release build then assembles `dist/<platform>/` directories and creates a zip per platform.

**Approach B (installer formats — AppImage, DMG, NSIS via `cargo-packager`) is explicitly deferred to CLI.7 Polish.** Note this in implementation comments.

---

## Commands

### `silm build`

```
silm build                          # build all platforms in game.toml [build.platforms]
silm build --platform wasm          # override: build one platform only
silm build --release                # release profile (LTO etc.)
silm build --env-file .env.prod     # load env vars from file (default: .env)
```

### `silm package`

```
silm package                        # package all platforms in game.toml [build.platforms]
silm package --platform native      # native server + client only
silm package --platform wasm        # WASM bundle only
silm package --platform server      # server binary + Dockerfile only
silm package --out-dir ./releases   # override zip output dir (default: project root)
```

`silm package` always builds with `--release` implicitly. The resulting zips are placed in the project root (or `--out-dir`). The `dist/` subfolders remain for inspection and CI use.

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
- `[build.env]`: default env vars baked into builds via `option_env!()`. Overridable at build time.

If `[build]` is absent from `game.toml`, `silm build` without `--platform` errors with a clear message prompting the user to add it.

---

## Platform Targets

| Platform key | Rust target triple | Builds | Tool |
|---|---|---|---|
| `native` | host triple | server + client | `cargo` |
| `windows-x86_64` | `x86_64-pc-windows-gnu` | server + client | `cross` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | server + client | `cross` |
| `linux-arm64` | `aarch64-unknown-linux-gnu` | server + client | `cross` |
| `macos-x86_64` | `x86_64-apple-darwin` | server + client | `cross`* |
| `macos-arm64` | `aarch64-apple-darwin` | server + client | `cross`* |
| `wasm` | `wasm32-unknown-unknown` | client only | `trunk` |
| `server` | host triple | server only | `cargo` |

\* macOS cross-compilation requires an Apple SDK. `cross` supports it but the user must supply `MACOS_SDK_URL` in their environment. `silm build` documents this clearly and surfaces a helpful error if the SDK URL is absent.

**WASM notes:**
- Only the `client/` crate is built for WASM. The server always targets native.
- Trunk must be installed (`cargo install trunk`). `silm build` checks for it on PATH and errors with the install command if absent.
- The game client must have an `index.html` in `client/` for Trunk. `silm new` will include a basic one in the template (handled separately from this spec).

**Cross-compilation notes:**
- `cross` must be installed (`cargo install cross`) and Docker must be running.
- `silm build` checks for both before attempting a cross build and surfaces actionable errors.

---

## Environment Variable Propagation

`silm build` reads env vars and passes them into subprocess environments so they can be baked into binaries at compile time via `option_env!()`.

### Precedence (highest wins)

1. Shell environment variables already set in the calling process
2. `--env-file <path>` specified file
3. `.env` in project root
4. Defaults in game code via `option_env!(...).unwrap_or(...)`

### How it works

`silm build` parses the `.env` file (or `--env-file`) into key=value pairs and merges them with the current process environment before spawning `cargo`/`cross`/`trunk`. Shell env vars are never overwritten — only added if not already present.

Game client code uses compile-time env reading:
```rust
const SERVER_ADDRESS: &str = option_env!("SERVER_ADDRESS")
    .unwrap_or("ws://localhost:7777");
```

`silm` does **not** modify game source code — it only sets the subprocess environment.

### `.env` file format

Standard `KEY=VALUE` per line, `#` for comments, no shell expansion:
```bash
SERVER_ADDRESS=ws://localhost:7777
SERVER_PORT=7777
# MACOS_SDK_URL=https://...  # only needed for macOS cross-builds
```

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
│   ├── wasm/
│   │   ├── index.html
│   │   ├── <hash>.js
│   │   ├── <hash>_bg.wasm
│   │   └── assets/
│   └── server/
│       ├── server
│       └── Dockerfile
├── my-game-v0.1.0-native.zip
├── my-game-v0.1.0-windows-x86_64.zip
├── my-game-v0.1.0-linux-x86_64.zip
├── my-game-v0.1.0-wasm.zip
└── my-game-v0.1.0-server.zip
```

- **Folder names** under `dist/` are stable (platform key only) — easy to reference in CI scripts.
- **Zip filenames** include game name + version from `game.toml [project]` — self-describing for release uploads.
- Zips land in project root by default, configurable via `--out-dir`.
- `dist/<platform>/` is wiped and recreated on each `silm package` run to avoid stale files.

### Generated Dockerfile (`dist/server/Dockerfile`)

```dockerfile
FROM debian:bookworm-slim
COPY server /usr/local/bin/server
EXPOSE 7777/udp
EXPOSE 443/tcp

# Default env vars from game.toml [build.env] — override at runtime:
# docker run -e SERVER_ADDRESS=wss://prod.example.com ...
ENV SERVER_PORT=7777

ENTRYPOINT ["/usr/local/bin/server"]
```

---

## Error Handling

All errors must be actionable. Required messages:

```
error: no `[build]` section in game.toml
       Add one: [build]\nplatforms = ["native", "wasm"]

error: unknown platform "darwin" in game.toml [build.platforms]
       Known platforms: native, windows-x86_64, linux-x86_64, linux-arm64,
                        macos-x86_64, macos-arm64, wasm, server

error: `trunk` not found — required for wasm builds
       Install: cargo install trunk

error: `cross` not found — required for cross-platform builds
       Install: cargo install cross

error: Docker is not running — required for cross builds
       Start Docker Desktop, then retry.

error: WASM build requires client/index.html — not found
       Add an index.html to your client/ crate root.

error: macOS cross-build requires MACOS_SDK_URL
       Set it in your .env file or shell environment.
       See: https://github.com/cross-rs/cross/wiki/Recipes#apple-darwin-targets
```

---

## Code Structure

New files:
```
engine/cli/src/commands/build/
├── mod.rs          — BuildCommand + PackageCommand enums, handle_build_command,
│                     handle_package_command, platform parsing, tool detection
├── native.rs       — cargo/cross build for server + client
├── wasm.rs         — trunk build for client
├── env.rs          — .env file parsing, env var merge logic
└── package.rs      — dist/ assembly, zip creation, Dockerfile generation
```

Modified:
- `engine/cli/src/commands/mod.rs` — add `pub mod build;`
- `engine/cli/src/main.rs` — add `Commands::Build` and `Commands::Package`
- `engine/cli/src/templates/basic.rs` — add `[build]` section to generated `game.toml`,
  add `client/index.html` stub for WASM

---

## Testing

### Unit tests (`engine/cli/tests/build_unit_tests.rs`)

Pure logic, no subprocess:

- `.env` parsing: key=value, comments, empty lines, blank values
- Env precedence: shell env beats `--env-file` beats `.env` beats nothing
- Platform string → target triple mapping (all 8 platforms)
- Unknown platform → error with known-platforms list
- `dist/` path construction per platform
- Zip filename construction from game name + version
- Dockerfile template generation from `[build.env]` entries

### Integration tests (`engine/cli/tests/build_integration_tests.rs`)

Real filesystem, subprocess mocked (captured command + args, not executed):

- `silm build --platform native` invokes `cargo build` with correct args
- `silm build --platform wasm` invokes `trunk build` with correct args
- `silm build --release` passes `--release` to underlying tool
- Env vars from `.env` are present in subprocess environment
- Shell env vars take precedence over `.env`
- Missing `[build]` in game.toml → correct error
- Unknown platform → correct error

### E2E tests (CI-gated, skipped if tool absent)

- `silm build --platform native` on a `silm new` project → binaries produced in `target/`
- `silm package --platform native` → `dist/native/` populated, zip created
- WASM build: skipped with warning if `trunk` not on PATH (not a test failure)
- `cross` build: skipped with warning if Docker not running (not a test failure)

### Tool detection tests

- `trunk` absent → error with install command
- `cross` absent → error with install command
- Docker not running → error with start instruction

---

## Future Work (CLI.7)

- `cargo-packager` integration: AppImage (Linux), DMG + .app bundle (macOS), NSIS installer (Windows)
- `--target` flag alias for `--platform` (Cargo convention)
- Progress indicators during long builds
- `silm build --watch` (rebuild on file change)
