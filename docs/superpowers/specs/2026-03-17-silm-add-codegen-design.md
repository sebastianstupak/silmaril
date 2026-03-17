# silm add — Code Generation Command

**Date:** 2026-03-17
**Status:** Approved

---

## Overview

`silm add component` and `silm add system` are code generation commands that scaffold ECS building blocks into a Silmaril game project using vertical (domain) slicing. The primary consumer is AI agents building games piece by piece without manual file editing.

**This is a full rewrite of the existing `engine/cli/src/commands/add.rs`.** The current implementation uses horizontal slicing (`src/components/`, `src/systems/`) and a `--location` string flag. This spec replaces both with vertical domain slicing and explicit target flags. The existing `add.rs` file is deleted and replaced with the module layout described below.

**Error handling note:** The CLI crate (`engine/cli`) uses `anyhow::Result` throughout as an established convention. This is a known exception to the `define_error!` rule (which applies to engine crates). The `add` command follows this CLI-crate convention. Fixing the CLI to use custom errors is tracked separately and out of scope here.

---

## Commands

```bash
# Add a component
silm add component Health --shared --domain health --fields "current:f32,max:f32"
silm add component Damage --server --domain combat --fields "amount:f32,source:u64"

# Add a system (mut: prefix marks mutable query params)
silm add system health_regen --shared --domain health --query "mut:Health,RegenerationRate"
silm add system apply_damage --server --domain combat --query "mut:Health,mut:Damage"
```

**Project detection:** walk up from `cwd` looking for `game.toml`. Resolve all crate paths relative to the directory that contains `game.toml`, not relative to `cwd`. Walk-up terminates at the filesystem root; if `game.toml` is not found, error: `no game.toml found — run this command from inside a silmaril project`.

---

## Flags

| Flag | Commands | Required | Description |
|---|---|---|---|
| `--shared` | both | one of three | Target the `shared/` crate |
| `--server` | both | one of three | Target the `server/` crate |
| `--client` | both | one of three | Target the `client/` crate |
| `--domain <name>` | both | yes | snake_case domain name (e.g. `health`, `combat`) |
| `--fields "name:type,..."` | component | yes | Comma-separated field definitions |
| `--query "mut:A,B,..."` | system | yes | Component query; `mut:X` = `&mut X`, bare `X` = `&X` |

**Exactly one** of `--shared`, `--server`, `--client` must be provided. Error if zero or more than one.

**Retired/removed flags:**
- `--location` → replaced by `--shared` / `--server` / `--client`
- `--derive` (component) → removed; derive set is now fixed
- `--phase` (system) → removed

**Crate root resolution:** `<project_root>/shared/`, `<project_root>/server/`, `<project_root>/client/`. If the chosen crate directory does not exist, error: `target crate 'server/' not found — is this project set up correctly?`.

**`--fields` limitation:** field types may not contain commas. Types like `HashMap<String, u64>` are not supported via `--fields`; add them manually after generation.

---

## Architecture: Vertical Domain Slicing

Generated code is organized by **domain** (game feature), not by technical type. All code for a domain lives together:

```
shared/src/
├── health/
│   └── mod.rs        ← Health component + health_regen system + all tests
├── movement/
│   └── mod.rs        ← Transform, Velocity components + movement system + tests
└── lib.rs            ← pub mod health; pub mod movement; (auto-updated)

server/src/
├── combat/
│   └── mod.rs        ← Damage component + apply_damage system + tests
└── main.rs           ← pub mod combat; (auto-updated)
```

**Rationale:** Files that change together live together. Adding or removing a game feature means adding or removing one directory. An AI agent can understand a feature by reading one file.

### BasicTemplate update

`engine/cli/src/templates/basic.rs` currently generates `shared/src/components.rs`, `shared/src/systems.rs`, and corresponding `pub mod components; pub mod systems;` in `lib.rs`. These horizontal stubs conflict with vertical domain slicing. As part of this work, `BasicTemplate` is updated to generate a clean `shared/src/lib.rs` with no pre-wired module declarations. The stub files `components.rs` and `systems.rs` are no longer generated.

---

## Auto-Wiring

`silm add` performs all file wiring automatically — no manual `mod` declarations needed.

**Target file mapping:**
| Flag | Domain module | Wiring target |
|---|---|---|
| `--shared` | `shared/src/<domain>/mod.rs` | `shared/src/lib.rs` |
| `--server` | `server/src/<domain>/mod.rs` | `server/src/main.rs` |
| `--client` | `client/src/<domain>/mod.rs` | `client/src/main.rs` |

**Steps:**
1. Walk up from `cwd` to find `game.toml`; resolve all paths relative to that directory
2. Validate target crate directory exists
3. Read original `src/<domain>/mod.rs` into memory (if it exists); check for duplicate — error before any write
4. Read original wiring target (`lib.rs` or `main.rs`) into memory
5. Build new `mod.rs` content (original + generated block); write to temp file; atomically rename to `src/<domain>/mod.rs`
6. Add `pub mod <domain>;` to wiring target if not already present; write wiring target
7. **Rollback if step 6 fails:**
   - Restore `src/<domain>/mod.rs` from in-memory original (or delete if newly created)
   - Restore wiring target from in-memory original

**Duplicate detection:**
- Component: scan for `pub struct <Name>` followed immediately by `{` or whitespace-then-`{` (regex: `pub struct <Name>\s*\{`)
- System: scan for `pub fn <name>_system(` (generated name with `_system` suffix)
- Scoped to target file only; duplicates in other domain files are allowed

**Idempotent wiring:** check for `pub mod <domain>;` before adding. Never write it twice.

---

## Generated Code Templates

### Component

```bash
silm add component Health --shared --domain health --fields "current:f32,max:f32"
```

Generates (or appends to) `shared/src/health/mod.rs`.

**Fixed derive set:** always `Component, Debug, Clone, PartialEq, Serialize, Deserialize`. `PartialEq` is always included (unlike the previous implementation which required `--derive PartialEq`).

**Test module name:** `mod <snake_name>_tests` (e.g. `mod health_tests`). Using a name-scoped module avoids collisions when multiple components are appended to the same `mod.rs`.

```rust
use engine_core::ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[cfg(test)]
mod health_tests {
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_health_add_get() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add(entity, Health { current: 100.0, max: 100.0 });
        let h = world.get::<Health>(entity).unwrap();
        assert_eq!(h.current, 100.0);
        assert_eq!(h.max, 100.0);
    }

    #[test]
    fn test_health_serialization() {
        let h = Health { current: 50.0, max: 100.0 };
        let json = serde_json::to_string(&h).unwrap();
        let h2: Health = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_health_remove() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add(entity, Health { current: 100.0, max: 100.0 });
        world.remove::<Health>(entity);
        assert!(world.get::<Health>(entity).is_none());
    }
}
```

### System

```bash
silm add system health_regen --shared --domain health --query "mut:Health,RegenerationRate"
```

Appends to `shared/src/health/mod.rs`.

**Function name:** user input + `_system` suffix → `health_regen_system`.

**World import:** `use engine_core::ecs::World;` is placed at the top of the file (once, idempotent). The function body uses the short form `World`.

**`#[instrument]`:** the generated system always includes `#[instrument(skip(world))]` from `tracing` — consistent with the profiling mandate in CLAUDE.md.

**Test module name:** `mod <name>_system_tests` (e.g. `mod health_regen_system_tests`).

```rust
// To register: app.add_system(health_regen_system);
#[tracing::instrument(skip(world))]
pub fn health_regen_system(world: &mut World, dt: f32) {
    for (health, regen) in world.query::<(&mut Health, &RegenerationRate)>() {
        // TODO: implement health_regen logic
    }
}

#[cfg(test)]
mod health_regen_system_tests {
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_health_regen_system() {
        let mut world = World::new();
        // TODO: spawn test entity, run system, assert
        health_regen_system(&mut world, 0.016);
    }
}
```

**Query grammar** (for `parse_query_components` rewrite):
```
query     = component ("," component)*
component = ("mut:" name) | name
name      = [A-Z][A-Za-z0-9]*    ← PascalCase, no spaces
```
- Whitespace around `,` is trimmed; whitespace within a token is an error
- `mut:` prefix is literal; no space between `:` and name
- Old `&mut X` / `&X` syntax rejected: `"use 'mut:ComponentName' syntax, not '&mut ComponentName'"`
- Any other unrecognised form: `"invalid query token '<token>': expected 'ComponentName' or 'mut:ComponentName'"`

**Query translation:** `mut:X` → `&mut X`, bare `X` → `&X`. Each `mut:` prefix applies to one component only.

**Imports:** same-domain components are in scope via `use super::*` in test modules and via the shared file scope in the function. Cross-domain components are not auto-imported — add manually.

---

## Error Handling

All errors: `anyhow::Result`. Success output: `tracing::info!` (never `println!`).

| Scenario | Error message |
|---|---|
| No target flag | `must specify exactly one of --shared, --server, or --client` |
| Multiple target flags | `must specify exactly one of --shared, --server, or --client` |
| No `--domain` | `--domain is required` |
| No `--fields` (component) | `--fields is required for component` |
| No `--query` (system) | `--query is required for system` |
| No `game.toml` found | `no game.toml found — run this command from inside a silmaril project` |
| Target crate dir missing | `target crate 'server/' not found — is this project set up correctly?` |
| Duplicate component | `component 'Health' already exists in shared/src/health/mod.rs` |
| Duplicate system | `system 'health_regen' already exists in shared/src/health/mod.rs` |
| Write failure | rolls back, reports OS error |

**Success output** (via `tracing::info!`):
```
[silm] created shared/src/health/mod.rs
[silm] wired: added `pub mod health;` to shared/src/lib.rs
```

---

## Files Changed

### New module (replaces `add.rs`)

`engine/cli/src/commands/add.rs` → **deleted**. Replaced with:

```
engine/cli/src/commands/add/
├── mod.rs        — AddCommand enum (Component | System) + entry point
├── component.rs  — component codegen
├── system.rs     — system codegen
└── wiring.rs     — mod.rs append + lib.rs/main.rs idempotent update
```

### Parser rewrite

`engine/cli/src/codegen/parser.rs::parse_query_components` is **rewritten** to accept `mut:X` syntax. The old `&mut X` / `&X` syntax is **dropped**. All existing usages of the old syntax must be migrated:

- `engine/cli/tests/codegen/system_tests.rs` — migrate all `&mut X` / `&X` to `mut:X` / `X`
- `engine/cli/tests/parser_tests.rs` — same migration
- `engine/cli/tests/system_integration.rs` — same migration
- `engine/cli/tests/component_integration.rs` — audit for any query usage

### BasicTemplate update

`engine/cli/src/templates/basic.rs` — remove generation of `components.rs`, `systems.rs`, and their `pub mod` declarations from `lib.rs`. Generate a clean `lib.rs` with no pre-wired modules.

### `engine/cli/src/main.rs`

Update to register the new `add` subcommand module path.

---

## Testing

### Unit tests (`engine/cli/tests/`)

- `--fields "current:f32,max:f32"` → `[("current", "f32"), ("max", "f32")]`
- `--query "mut:Health,RegenerationRate"` → `[(Health, Mut), (RegenerationRate, Ref)]`
- `--query "mut:Health,mut:Velocity,Mass"` → `[(Health, Mut), (Velocity, Mut), (Mass, Ref)]`
- Component codegen: correct struct + fixed derives + `mod <name>_tests` block
- System codegen: `mut:Health` → `&mut Health`, function named `<name>_system`, has `#[instrument]`
- Duplicate detection: `pub struct Health ` found → error before any write
- Duplicate system: `pub fn health_regen_system(` found → error before any write
- Wiring: `pub mod health;` added once to lib.rs, idempotent on second call
- Wiring: `pub mod combat;` added to main.rs for `--server`
- Missing flag → clear error message

### Integration tests (`engine/cli/tests/`)

Using `tempfile` for isolated project dirs with minimal `game.toml` + crate structure:
- Full component run: file created, lib.rs wired, generated code is valid Rust
- Full system run: appended to existing domain file, wiring idempotent
- Two components same domain: both in `mod.rs`, wired once, no duplicate `pub mod`
- Component + system same domain: correct combined `mod.rs`
- Walk-up: run from two directory levels inside project, `game.toml` found, paths correct
- Walk-up termination: run from temp dir with no `game.toml` anywhere → clear error
- `--server` with no `server/` dir: clear error
- Rollback (new file): mock step 5 failure → `mod.rs` deleted
- Rollback (append): mock step 5 failure → original `mod.rs` content restored

---

## Out of Scope

- `silm add module` (module management / vendoring) — tracked separately
- Interactive field prompts
- Type validation (field types passed through as-is)
- Field types containing commas (e.g. `HashMap<String, u64>`) — add manually
- `ComponentData` enum entries for serialization — manual step
- System registration — comment hint generated instead
- Fixing CLI anyhow usage — tracked separately
