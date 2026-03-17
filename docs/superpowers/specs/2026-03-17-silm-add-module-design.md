# silm add module — Module Management Command

**Date:** 2026-03-17
**Status:** Approved

---

## Overview

`silm add module` is a package-manager-style command for adding, tracking, and removing Silmaril game modules. Modules are standard Rust crates that extend a game with reusable ECS components, systems, and resources. The command manages Cargo dependencies, generates registration wiring, and tracks module provenance in `game.toml`.

**Three operations in this spec:**
- `silm add module <name>` — add a module (four source modes)
- `silm module list` — list installed modules with resolved versions
- `silm module remove <name>` — remove a module and its wiring

`silm module upgrade` is explicitly out of scope for this iteration (tracked separately).

---

## Commands

```bash
# Registry (crates.io) — official and published community modules
silm add module combat
silm add module combat@1.2.0

# Git — community or pre-release modules
silm add module combat --git https://github.com/org/combat
silm add module combat --git https://github.com/org/combat --tag v1.0.0
silm add module combat --git https://github.com/org/combat --rev abc123f

# Local path — for developing your own modules
silm add module combat --path ../my-combat

# Vendor — clone source into modules/combat/ (future: paid tier)
# --vendor is a mode modifier, combined with a source (--git or implicitly registry)
silm add module combat --vendor
silm add module combat --vendor --git https://github.com/org/combat --tag v1.0.0

# Management
silm module list
silm module remove combat
```

**Target flags:** `silm add module` accepts `--shared`, `--server`, or `--client` (same tri-flag pattern as `silm add component/system`). Exactly one must be provided if the module's `[package.metadata.silmaril]` has no `target` field. If metadata declares a target, the flag is optional and overrides it when supplied.

**`--vendor` is a mode modifier**, not an exclusive source flag. It can be combined with `--git` (clone from git, vendor locally) or used alone (fetch from registry then vendor). It cannot be combined with `--path` (a local path is already local source).

**Branch mode is rejected.** `--branch` is not accepted. Reason: branch references are not reproducible — `cargo update` silently advances the resolved commit. Use `--tag` or `--rev`.

---

## Module Metadata — `[package.metadata.silmaril]`

Module authors declare their integration interface inside the module crate's own `Cargo.toml` using the standard Cargo metadata extension point. The CLI reads it via `cargo metadata`. No separate metadata file is required.

```toml
# Inside silmaril-module-combat's Cargo.toml
[package.metadata.silmaril]
module_type  = "CombatModule"      # Rust type to reference in wiring
target       = "shared"            # shared | server | client
init         = "CombatModule::new()"
silmaril_min = "0.1.0"            # minimum compatible engine version
```

**Convention for official modules:** crate `silmaril-module-<name>` exports type `<Name>Module` (e.g. `CombatModule`). If `[package.metadata.silmaril]` is absent and the crate follows this convention, the CLI derives `module_type` from the crate name. If neither metadata nor convention applies, the CLI emits a manual wiring hint and adds the dep without wiring.

**Crate name resolution per source mode:**
| Source | How crate name is found |
|---|---|
| Registry | Convention: `silmaril-module-<name>` |
| Git | Read `[package.name]` from the git-fetched crate's `Cargo.toml` via `cargo metadata` after dep resolution |
| Path | Read `[package.name]` from `<path>/Cargo.toml` directly |
| Vendor | Read `[package.name]` from cloned `modules/<name>/Cargo.toml` after clone |

The resolved crate name is used for the `use` statement and `game.toml` tracking.

---

## Source Modes

### Registry (default)

Resolves `<name>` to `silmaril-module-<name>` on crates.io. Edits the target crate's `Cargo.toml` directly (TOML manipulation — not `cargo add` subprocess — for consistent rollback control; see Implementation Notes).

```toml
# shared/Cargo.toml
[dependencies]
silmaril-module-combat = "^1.2.0"
```

- Immutable versions, cargo-audit compatible, checksum verified by Cargo
- **Risk:** name squatting on `silmaril-module-*` — the silmaril org should reserve this prefix; future CLI will warn on unverified publishers

### Git

Adds a git dependency via direct TOML edit. Pinned at add-time.

| Flag | Cargo.toml entry | Reproducibility |
|---|---|---|
| `--tag v1.0.0` | `git = "...", tag = "v1.0.0"` | ⚠️ Tag can be force-pushed |
| `--rev abc123f` | `git = "...", rev = "abc123f"` | ✅ Immutable |
| (neither) | Resolves latest tag → writes `rev = <commit-hash>` | ✅ Pinned at add-time |

**Warning emitted:** `"This module is not from crates.io — review the source before use."`

```toml
# shared/Cargo.toml
[dependencies]
silmaril-module-combat = { git = "https://github.com/org/combat", rev = "abc123f" }
```

### Local Path

Adds a path dependency via direct TOML edit. No network, no security surface. Intended for developing a module before publishing.

```toml
[dependencies]
silmaril-module-combat = { path = "../my-combat" }
```

### Vendor

Clones or copies module source into `modules/<name>/` inside the game project. Adds `modules/<name>` as a workspace member in root `Cargo.toml`. Adds a path dep in the consuming crate.

**This code path must be isolated** (a `VendorSource` struct or equivalent) so a license check can be inserted at the entry point in a future iteration without touching other source modes.

```toml
# Root Cargo.toml — workspace member added
[workspace]
members = [..., "modules/combat"]

# shared/Cargo.toml — path dep
[dependencies]
silmaril-module-combat = { path = "../../modules/combat" }
```

`game.toml` records the upstream URL and pinned ref for future `silm module upgrade`.

---

## `game.toml [modules]` — Requirement Tracking

`game.toml` records what was *requested*. `Cargo.lock` records what was *resolved*. No separate lockfile is needed — Cargo.lock already is the lockfile.

The `[modules]` key already exists as a comment placeholder in the `basic` template (`engine/cli/src/templates/basic.rs`). The template must be updated to reflect this richer schema in its comment example.

```toml
[modules]
combat   = { source = "registry", version = "^1.2.0",   target = "shared" }
movement = { source = "git", url = "https://github.com/org/movement", tag = "v1.0.0", target = "shared" }
ai       = { source = "vendor", upstream = "https://github.com/org/ai", ref = "v2.1.0", target = "server" }
utils    = { source = "local", path = "../utils", target = "shared" }
```

**Fields:**
| Field | Present for | Description |
|---|---|---|
| `source` | all | `registry` \| `git` \| `local` \| `vendor` |
| `version` | registry | semver requirement |
| `url` | git | git remote URL |
| `tag` / `rev` | git | pinned reference (one of these, not both) |
| `upstream` | vendor | git remote for future upgrade |
| `ref` | vendor | pinned commit/tag at vendor time |
| `path` | local | relative path to module root |
| `target` | all | `shared` \| `server` \| `client` |

---

## Wiring Generated Code

After adding the Cargo dependency, the CLI reads `[package.metadata.silmaril]` and generates a wiring block in the target entry file (`lib.rs` for `--shared`, `main.rs` for `--server`/`--client`) using the existing `wiring_target()` helper from `engine/cli/src/commands/add/wiring.rs`.

```rust
// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---
use silmaril_module_combat::CombatModule;
// TODO: register → world.add_module(CombatModule::new());
```

**Why a comment, not a live call:** the engine's `App`/`World` registration API is still stabilising. A generated call using the wrong shape would produce compile errors on every `silm add module`. When the API stabilises the comment becomes a call.

**Idempotency:** the marker `// --- silmaril module: <name>` guards the block. If already present, wiring is skipped. Re-running is safe.

**No metadata fallback:** if `[package.metadata.silmaril]` is absent and the naming convention doesn't apply, emit:
```
[silm] added silmaril-module-combat v1.2.3 → shared/Cargo.toml
[silm] no silmaril metadata found — add registration manually (see module README)
```

---

## `silm module remove <name>` — Unwiring

1. Find the wiring block by marker `// --- silmaril module: <name>`. Block ends at the **next** `// --- silmaril module:` marker, or at end-of-file, whichever comes first. This rule is unambiguous even when multiple module blocks are adjacent.
2. Remove the block from the wiring target file (atomic write).
3. Remove the dep entry from the consuming crate's `Cargo.toml` (TOML edit, atomic write).
4. Remove the workspace member from root `Cargo.toml` (vendor only, atomic write).
5. Delete `modules/<name>/` directory (vendor only).
6. Remove the `[modules.<name>]` entry from `game.toml` (TOML edit, atomic write).

Rollback: if any step fails, restore all previously modified files from in-memory originals.

---

## `silm module list`

Reads `game.toml [modules]` for the requirement spec. Reads resolved versions from `Cargo.lock` directly (plain text parsing of `[[package]]` blocks — no `cargo metadata` invocation, so this is instant and offline-safe).

```
NAME      SOURCE    REQUIREMENT   RESOLVED  TARGET
combat    registry  ^1.2.0        1.2.3     shared
movement  git       tag=v1.0.0    v1.0.0    shared
ai        vendor    ref=v2.1.0    v2.1.0    server
utils     local     ../utils      (local)   shared
```

Column widths: NAME and SOURCE columns are left-padded to the longest value in each column.

---

## Atomic Writes and Rollback

Steps, in order (non-vendor):
1. Read originals into memory: `game.toml`, consuming crate `Cargo.toml`, wiring target (`lib.rs`/`main.rs`)
2. Edit consuming crate `Cargo.toml` (TOML manipulation, direct — not `cargo add` subprocess)
3. Write consuming crate `Cargo.toml` via temp file + atomic rename
4. Append wiring block to wiring target via temp file + atomic rename
5. Write `game.toml` via temp file + atomic rename
6. **Rollback if any step 3–5 fails:** restore all three files from in-memory originals

Vendor mode additionally:
1. (Before step 1) Read original root `Cargo.toml` into memory
2. Clone/copy to `modules/<name>/`
3. Steps 1–5 above, plus: edit root `Cargo.toml` workspace member list + write via temp rename
4. **Rollback if any step fails after clone:** delete `modules/<name>/`, restore root `Cargo.toml`, consuming crate `Cargo.toml`, wiring target, and `game.toml` from in-memory originals

---

## Implementation Notes

**Direct TOML manipulation, not `cargo add` subprocess.** The existing code (`component.rs`, `system.rs`, `wiring.rs`) edits files directly using the `toml` crate. `silm add module` follows the same pattern for consistent rollback control and to avoid subprocess output routing (which would require `println!`, violating CLAUDE.md). The TOML edit adds a dep entry to `[dependencies]` in the consuming crate's `Cargo.toml`.

**`cargo metadata` usage:** used only to read resolved module metadata (`[package.metadata.silmaril]`) and the resolved crate name for git sources — a one-time call at add-time. The `--no-deps` flag is used where possible to reduce network calls. For `silm module list`, Cargo.lock is parsed directly — no `cargo metadata` invocation.

**`module_wiring.rs`** is a new file in `engine/cli/src/codegen/` responsible for: reading `[package.metadata.silmaril]` from `cargo metadata` output, generating the wiring block string, and detecting the idempotency marker. It calls `wiring_target()` from the existing `engine/cli/src/commands/add/wiring.rs` — it does not duplicate that function.

**Tracing in integration tests:** `tracing::info!` output is silently dropped in tests without a subscriber. Integration tests verify file contents and exit codes — not log output. This matches the behaviour of existing `add_integration.rs` tests.

---

## Security Model

```
crates.io registry (verified org)   ✅✅ Immutable, auditable, cargo-audit
crates.io registry (community)      ✅  Cargo protections, review recommended
git + rev                           ✅  Pinned, not cargo-audit scanned
git + tag                           ⚠️  Tag mutable, warn on use
git + branch                        ❌  Rejected — not supported
local path                          ✅  Developer's own code
vendor                              ✅✅ Source in repo, fully auditable
```

---

## Duplicate Detection

Before any write:
- Check `game.toml [modules]` for existing `<name>` — error: `module 'combat' is already installed — use 'silm module upgrade' to update`
- Check consuming crate `Cargo.toml [dependencies]` for an existing dep matching the resolved crate name — same error with same hint

---

## Error Handling

All errors: `anyhow::Result` (CLI crate convention). Success output: `tracing::info!`.

| Scenario | Error message |
|---|---|
| No `game.toml` found | `no game.toml found — run this command from inside a silmaril project` |
| Module already installed | `module 'combat' is already installed — use 'silm module upgrade' to update` |
| Target crate dir missing | `target crate 'shared/' not found — is this project set up correctly?` |
| No target (no metadata, no flag) | `must specify one of --shared, --server, or --client when the module has no silmaril metadata` |
| Git URL with `--branch` | `--branch is not supported: use --tag or --rev for reproducible builds` |
| `--vendor` with `--path` | `--vendor cannot be combined with --path: a local path is already local source` |
| Cargo.toml edit fails | surface OS error |
| Vendor clone fails | `failed to clone <url>: <os error>` |
| `cargo metadata` fails | `failed to read module metadata: <cargo error>` |

**Success output:**
```
[silm] added silmaril-module-combat v1.2.3 (registry) → shared/
[silm] wired: shared/src/lib.rs
[silm] tracked: game.toml [modules.combat]
```

---

## Files Changed

```
engine/cli/src/commands/add/mod.rs           — add Module variant to AddCommand
engine/cli/src/commands/add/module.rs        — new: add_module() orchestrator (four source modes)
engine/cli/src/commands/module/mod.rs        — new: ModuleCommand enum (List, Remove)
engine/cli/src/commands/module/list.rs       — new: list_modules() — reads game.toml + Cargo.lock
engine/cli/src/commands/module/remove.rs     — new: remove_module() — unwires + removes dep
engine/cli/src/commands/mod.rs               — export module subcommand
engine/cli/src/main.rs                       — register Module top-level subcommand
engine/cli/src/codegen/module_wiring.rs      — new: read cargo metadata, generate wiring block
engine/cli/src/templates/basic.rs            — update [modules] comment example with richer schema
engine/cli/tests/codegen/module_wiring_tests.rs — new: pure codegen/parsing unit tests
engine/cli/tests/add_module_integration.rs      — new: tempfile project integration tests
```

---

## Testing

Tests follow the existing split: pure logic tests in `tests/codegen/`, filesystem tests in `tests/add_module_integration.rs`. See CLAUDE.md Rule 6 (3-tier test hierarchy).

### Codegen unit tests (`engine/cli/tests/codegen/module_wiring_tests.rs`)

These tests require no tempfile project and exercise pure parsing/generation functions:

- `game.toml [modules]` TOML round-trip for all four source modes
- Wiring block generation from metadata: correct use statement, correct comment
- Wiring block idempotency: wiring block marker detected, second write skipped
- Unwiring block parser: block boundary stops at next `// --- silmaril module:` marker, not beyond
- Duplicate detection logic: installed module entry detected before any write
- Git branch flag rejection: `--branch` returns clear error
- `--vendor` + `--path` flag rejection: correct error message
- Cargo.lock parsing: correct version extracted for registry and git package entries

### Integration tests — tempfile isolated projects (`engine/cli/tests/add_module_integration.rs`)

Tests use `CWD_LOCK: Mutex<()>` (same pattern as existing `add_integration.rs`) for `set_current_dir` safety.

- Registry mode: dep added to `shared/Cargo.toml`, wiring block in `shared/src/lib.rs`, `game.toml` entry written
- Git mode (`--tag`): git dep with tag in `Cargo.toml`, `game.toml` entry with url + tag
- Git mode (no tag): resolves to `rev = <hash>`, pinned in Cargo.toml
- Path mode: path dep in `Cargo.toml`, `game.toml` entry with path
- Vendor mode: `modules/combat/` created, workspace member in root `Cargo.toml`, path dep in consuming crate, `game.toml` entry with upstream + ref
- No metadata fallback: dep added, manual hint emitted, wiring block absent
- Server target (`--server`): dep in `server/Cargo.toml`, wiring in `server/src/main.rs`, `main.rs` wired with `pub mod` if domain
- Duplicate rejected before any write (file content unchanged)
- `silm module list`: correct table output, local module shows `(local)` for resolved version
- `silm module remove`: dep removed from `Cargo.toml`, block removed from entry file, `game.toml` entry gone; adjacent module blocks intact
- Rollback (registry): simulate Cargo.toml write failure → original restored
- Rollback (vendor): simulate wiring failure after clone → `modules/<name>/` deleted, root `Cargo.toml` restored

---

## Out of Scope

- `silm module upgrade` — tracked separately
- Verified publisher allowlist — future security hardening
- Vendor mode license gating — architecture is ready (isolated `VendorSource` code path); enforcement is future
- Module dependency resolution (modules depending on other modules)
- `silm module info <name>`
- Interactive prompts
- `--branch` support
