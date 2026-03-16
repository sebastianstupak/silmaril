# silm dev — Hot-Reload Development Command

**Date:** 2026-03-16
**Status:** Approved (rev 3)

---

## Overview

`silm dev` is a development orchestrator for Silmaril game projects. It starts the server and/or client processes, watches the project for changes, and applies them with the lowest disruption possible:

- Asset and config changes are applied live to the running process (no restart)
- Code changes trigger a state-preserving restart (serialize world → rebuild → restore)

This is a layered approach by change type. Full dylib hot-reload was ruled out because `hot-lib-reloader` conflicts with the `tracing` crate and TypeId-based ECS systems.

---

## Commands

```
silm dev              # start server + client together
silm dev server       # server only
silm dev client       # client only
```

**Project detection:** walk up from `cwd` looking for `game.toml`. Error clearly if not found.

---

## Architecture

```
silm dev (DevOrchestrator)
├── FileWatcher        — watches project dirs, classifies changes
├── ProcessManager     — spawns/monitors/restarts server and client
│   └── ProcessKiller  — platform-abstracted graceful kill (Unix/Windows)
├── ReloadClient       — sends reload signals to running processes via TCP
└── OutputMux          — line-buffered, prefixed merge of all process output

Inside each running game process (dev feature only):
└── DevReloadServer    — TCP listener, handles reload signals
    └── ForceReloader  — bridge between async TCP and HotReloader game-loop thread
```

---

## Change Routing

| Change type | Detected by | Action | Sent to |
|---|---|---|---|
| `assets/**` (images, meshes, audio) | FileWatcher | ReloadClient sends `reload_asset` | Both server and client (assets may be needed by either) |
| `config/*.ron` | FileWatcher | ReloadClient sends `reload_config` | Server gets `server.ron`, client gets `client.ron` (matched by filename prefix) |
| `shared/`, `server/`, `client/` `.rs` or `Cargo.toml` | FileWatcher | State-preserving restart | Restarts only the affected process(es): server change restarts server, client change restarts client, shared change restarts both |

---

## `game.toml` Fields Read by `silm dev`

`silm dev` reads the following from `game.toml` at the project root:

```toml
[project]
name = "my-game"

[dev]
server_package = "my-game-server"    # cargo package name for server binary
client_package = "my-game-client"    # cargo package name for client binary
server_port = 7777                   # game server port (informational)
dev_server_port = 9999               # DevReloadServer port for server process
dev_client_port = 9998               # DevReloadServer port for client process
```

`BasicTemplate` generates these fields. `silm dev server` reads `server_package` and `dev_server_port` only; `silm dev client` reads `client_package` and `dev_client_port` only. `silm dev` reads both.

The `dev_server_port` and `dev_client_port` solve the port conflict: when both processes are launched, each binds a different port. `silm dev` sends signals to the right port per process.

---

## Component Design

### FileWatcher

Uses `notify-debouncer-full` (separate crate from raw `notify`; does not affect `engine-assets`' existing raw `notify` v6 usage — both coexist in the workspace at different layers).

Watches these paths and classifies events:

| Watched path | Extensions | Event emitted |
|---|---|---|
| `shared/src/`, `server/src/`, `client/src/` | `.rs`, `Cargo.toml` | `ChangeKind::Code { crate_name }` |
| `assets/` | `.png`, `.jpg`, `.jpeg`, `.obj`, `.gltf`, `.glb`, `.ogg`, `.wav`, `.mp3` | `ChangeKind::Asset { path }` |
| `config/` | `.ron` | `ChangeKind::Config { path }` |

Debounce windows: 500ms for code (batch multiple saves from an editor), 200ms for assets and config.

`FileWatcher` must handle `notify::Error::Io` gracefully (e.g., buffer overflow on busy Windows directories via `ReadDirectoryChangesW`) — log a warning via `tracing::warn!` and continue rather than crash.

Sends `FileChange { kind, path, timestamp }` events via `tokio::sync::mpsc` channel to `DevOrchestrator`.

### ProcessManager

Uses `tokio::process::Command` with `kill_on_drop(true)`.

Stdout and stderr are captured with `Stdio::piped()` (both for `cargo build` and `cargo run`) and forwarded to `OutputMux` in separate tokio tasks using `tokio::io::AsyncBufReadExt::lines()`. Reading lines in separate tasks avoids deadlock from I/O buffering.

Process state machine:
```
Stopped → Starting → Running → Restarting → Stopped
```

On code change:
1. Send `SerializeState` to `DevReloadServer` via `ReloadClient`, wait for ack
2. Call `ProcessKiller::kill_graceful(child)`, wait up to 2s, then force-kill
3. Run `cargo build --features dev --package <server_package|client_package>` with `Stdio::piped()`, forward output to `OutputMux` under `[build]` prefix
4. Relaunch with `cargo run --features dev --package <server_package|client_package>`
5. `DevReloadServer` inside the new process restores state from `.silmaril/dev-state.yaml` on startup

`silm dev` creates `.silmaril/` at the project root on startup, before launching any processes.

#### ProcessKiller (platform abstraction)

```rust
// async-trait is required for object-safe async fn in traits.
// Add async-trait = "0.1" to engine/dev-tools/hot-reload/Cargo.toml.
#[async_trait::async_trait]
pub trait ProcessKiller: Send + Sync {
    async fn kill_graceful(&self, child: &mut Child, timeout: Duration) -> Result<(), DevError>;
}

#[cfg(unix)]
struct UnixKiller;   // SIGTERM → wait timeout → SIGKILL

#[cfg(windows)]
struct WindowsKiller; // GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT) → wait timeout → TerminateProcess
```

`ProcessManager` takes `Box<dyn ProcessKiller>`. A factory function in `process.rs` returns the correct implementation based on compile target. No `#[cfg]` in business logic.

### ReloadClient

Thin async TCP client that connects to the process's `dev_*_port` and sends newline-delimited JSON:

```json
{"type": "reload_asset", "path": "assets/textures/grass.png"}
{"type": "reload_config", "path": "config/server.ron"}
{"type": "serialize_state"}
```

Retry applies to the **connection phase only** (TCP connect), not to the message send. Up to 3 connection attempts with 100ms backoff; once connected, the write is attempted once.

For `reload_asset` and `reload_config`: if connection fails after retries, drop with `tracing::debug!`. Not an error — the process will see the updated file on its next poll or the next explicit load.

For `serialize_state`: if connection fails after retries, log `tracing::warn!("[dev] could not reach process for state serialization — restarting with clean state")` and proceed with the restart. The new process starts without state (clean start). This is not an error, just a notice to the developer.

### DevReloadServer

Lives in new crate `engine/dev-tools/hot-reload/`. The `dev` feature enables the server; when `dev` is off, `DevReloadServer::start()` is a no-op `async fn` that returns `()` immediately. The argument type is `Option<Arc<Mutex<HotReloader>>>`: `Some(bridge)` when `dev` feature is on, `None` otherwise — the no-op path ignores it. This means **no `#[cfg]` at the call site in `main.rs`**:

```rust
// Always compiles. No-op in release builds (dev feature off).
// hot_reloader is Option<Arc<Mutex<HotReloader>>>, Some only when dev feature on.
DevReloadServer::start(hot_reloader).await;
```

The `dev` feature controls whether `start()` actually binds a port; the call site is always the same.

Starts a tokio TCP listener on the port passed via env var `SILMARIL_DEV_PORT` (set by `silm dev` per process, defaulting to `game.toml` values). If `TcpListener::bind()` fails (port already in use), logs `tracing::warn!("DevReloadServer: port {port} in use, hot-reload signals disabled")` and returns without crashing.

Handles three message types:

- `reload_asset` → sends path to `ForceReloader` via channel
- `reload_config` → re-reads the `.ron` file and updates live config
- `serialize_state` → dispatches `StateHandoff::save()` via `tokio::task::spawn_blocking` (blocking fs I/O must not run on the async executor), sends ack only after `sync_all()` completes inside the blocking task

#### ForceReloader (HotReloader bridge)

`HotReloader` in `engine-assets` is poll-based and game-loop-thread-owned. To drive it from an async TCP task, a `ForceReloader` bridge is added:

- `HotReloader` gains a new method: `pub fn force_reload(&self, path: &Path) -> Result<(), AssetError>`
- `ForceReloader` holds an `Arc<Mutex<HotReloader>>`
- When `DevReloadServer` receives `reload_asset`, it sends the path via `tokio::sync::mpsc` to a dedicated task that holds `Arc<Mutex<HotReloader>>` and calls `force_reload()`
- This keeps the async/sync boundary explicit and avoids blocking the TCP listener

### State Handoff

**Save** (`StateHandoff::save`) — called inside `tokio::task::spawn_blocking`:
1. Lock world, snapshot to `WorldState`
2. Serialize to YAML bytes
3. Open `.silmaril/dev-state.yaml` with `std::fs::File` (synchronous, not `tokio::fs`)
4. Write bytes, call `file.flush()`, then `file.sync_all()` — guarantees durability before returning
5. Return `Ok(())` to the blocking task; the async caller sends ack to `ReloadClient` only after the `spawn_blocking` future resolves

**Restore** (`StateHandoff::restore`):
On startup in dev mode, the binary checks for `.silmaril/dev-state.yaml`:
- Found and valid → call `world.clear()`, restore from `WorldState::restore()`, delete file, `tracing::info!("[dev] state restored")`
- Found but corrupt → `tracing::warn!("[dev] state file corrupt, clean start")`, delete file, proceed normally
- Not found → clean start (normal first launch)

`StateHandoff` API:
```rust
pub struct StateHandoff {
    path: PathBuf, // .silmaril/dev-state.yaml
}

impl StateHandoff {
    pub fn new(project_root: &Path) -> Self
    pub fn save(&self, world: &World) -> Result<(), DevError>
    pub fn restore(&self, world: &mut World) -> Result<RestoreResult, DevError>
    pub fn exists(&self) -> bool
}

pub enum RestoreResult { Restored, CleanStart }
```

### OutputMux

Uses `tokio::sync::mpsc`. All sources send `OutputLine { source: Source, text: String }`. A single writer task drains the channel and writes to stdout with a `Mutex<std::io::Stdout>` to prevent interleaving.

Lines are the atomic unit — `tokio::io::AsyncBufReadExt::lines()` is used on all piped streams so partial writes never reach the display.

| Source | Prefix | Color |
|---|---|---|
| `cargo build` stdout/stderr | `[build]` | Yellow |
| Server stdout/stderr | `[server]` | Blue |
| Client stdout/stderr | `[client]` | Green |
| Dev system messages | `[dev]` | Cyan |
| Errors | — | Red |

---

## Error Handling

`engine/dev-tools/hot-reload/` defines its own error type using the project's `define_error!` macro. Error codes in range 2100–2199.

**Required changes to `engine/core/src/error.rs`:**
- Add five new variants to `ErrorCode`: `DevPortBindFailed = 2100`, `DevSerializeFailed = 2101`, `DevRestoreFailed = 2102`, `DevReloadFailed = 2103`, `DevTcpSendFailed = 2104`
- Add `2100..=2199 => "Dev Tools"` arm to `ErrorCode::subsystem()`
- Update the range table in `docs/error-handling.md`

These changes must be made before `DevError` can compile, since `define_error!` references `ErrorCode` variants by name.

```rust
define_error! {
    pub enum DevError {
        PortBindFailed { port: u16 } = ErrorCode::DevPortBindFailed, ErrorSeverity::Warning,
        SerializeFailed { reason: String } = ErrorCode::DevSerializeFailed, ErrorSeverity::Error,
        RestoreFailed { reason: String } = ErrorCode::DevRestoreFailed, ErrorSeverity::Warning,
        ReloadFailed { path: String, reason: String } = ErrorCode::DevReloadFailed, ErrorSeverity::Warning,
        TcpSendFailed { reason: String } = ErrorCode::DevTcpSendFailed, ErrorSeverity::Warning,
    }
}
```

`engine/cli/src/commands/dev/` follows the existing `engine/cli` convention of using `anyhow::Result` (the entire CLI crate already uses `anyhow`; correcting this is out of scope for this feature and tracked separately).

---

## New Crate: `engine/dev-tools/hot-reload/`

```
engine/dev-tools/hot-reload/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── server.rs      — DevReloadServer (no-op when dev feature off)
    ├── client.rs      — DevReloadClient
    ├── messages.rs    — ReloadMessage enum (shared by both sides)
    ├── handoff.rs     — StateHandoff
    └── force_reload.rs — ForceReloader (HotReloader bridge)
```

`Cargo.toml` features:
```toml
[features]
dev = []

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = "1"
async-trait = "0.1"
engine-core = { path = "../../core" }
engine-assets = { path = "../../assets", features = ["hot-reload"] }
```

`async-trait` is required for object-safe `async fn` in the `ProcessKiller` trait with `Box<dyn ProcessKiller>`.

---

## New CLI Files

```
engine/cli/src/commands/dev/
├── mod.rs           — DevCommand enum + entry point
├── orchestrator.rs  — ties watcher, process manager, reload client together
├── watcher.rs       — FileWatcher using notify-debouncer-full
├── process.rs       — ProcessManager + ProcessKiller trait + platform impls
├── reload_client.rs — ReloadClient TCP sender
└── output.rs        — OutputMux
```

New dependency for `engine/cli/Cargo.toml`:
```toml
notify-debouncer-full = "0.3"
```

---

## Template Changes

`BasicTemplate` (`engine/cli/src/templates/basic.rs`) is updated:

**Generated `game.toml`** — adds `[dev]` section with package names and ports.

**Generated `server/Cargo.toml` and `client/Cargo.toml`:**
```toml
[features]
dev = ["engine-dev-tools/dev"]

[dependencies]
engine-dev-tools = { path = "../../engine/dev-tools/hot-reload", optional = true }
```

**Generated `server/src/main.rs` and `client/src/main.rs`** — call `DevReloadServer::start()` unconditionally (no `#[cfg]`); it is a no-op when the `dev` feature is off.

`silm dev` always passes `--features dev` when launching processes. `silm build --release` never passes it.

---

## Testing

### Unit tests (`engine/dev-tools/hot-reload/tests/` and `engine/cli/tests/`)

- `ReloadMessage` serializes and deserializes correctly (round-trip)
- `FileWatcher` emits `Code` change for `.rs` edits in `shared/src/`
- `FileWatcher` emits `Asset` change for `.png` edits in `assets/`
- `FileWatcher` emits `Config` change for `.ron` edits in `config/`
- `DevReloadServer::start()` returns immediately (no-op) when `dev` feature is off

### Cross-crate integration tests (`engine/shared/tests/`)

- `DevReloadServer` starts, accepts connection, processes `reload_asset` message, calls `ForceReloader`
- `StateHandoff::save()` + `StateHandoff::restore()` round-trip: spawn world with entities, save, create new world, restore, assert entity count and component values match
- `StateHandoff::restore()` on a corrupt file returns `CleanStart` without panic

### Benchmarks (`engine/shared/benches/`)

- `bench_state_save` — time to serialize a world with 1K, 10K entities to `.silmaril/dev-state.yaml`
- `bench_state_restore` — time to restore the same worlds
- `bench_reload_message_rtt` — TCP round-trip: send `reload_asset` to `DevReloadServer`, receive ack

### E2E test script

`scripts/e2e-tests/test-silm-dev.sh`:
1. `silm new test-game --template basic`
2. `silm dev &` — start dev mode
3. Touch `assets/textures/test.png` — assert `[client] asset reloaded` in output within 1s
4. Touch `config/server.ron` — assert `[server]` config reload log within 1s
5. Touch `shared/src/lib.rs` — assert `[build]` output + `[dev] state restored` within 30s
6. Kill `silm dev`, assert clean exit

---

## Startup `.silmaril/` Directory

`silm dev` creates `<project_root>/.silmaril/` on startup before launching any processes. Already in `.gitignore` (generated by `BasicTemplate`).

---

## Graceful Shutdown

When the user presses Ctrl+C, `silm dev` catches the signal via `tokio::signal::ctrl_c()`, sends SIGTERM (or `CTRL_BREAK_EVENT` on Windows) to all managed processes, waits up to 3s for clean exit, then force-kills. Logs `[dev] shutting down` before exiting.

---

## Out of Scope

- Shell completions for `silm dev`
- `silm dev --multi N` (multiple clients)
- Metrics dashboard in dev mode
- dylib hot-reload of game logic (tracing + ECS TypeId conflicts)
- Fixing `engine/cli` anyhow usage (tracked separately)
