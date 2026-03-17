# silm module — Module Management

`silm add module` and `silm module` provide package-manager-style management of Silmaril game modules. Modules are reusable crates that extend your game (e.g. `silmaril-module-combat`, `silmaril-module-inventory`).

---

## Commands

### `silm add module <name> [flags]`

Adds a module to your game project. Writes the dependency into the target crate's `Cargo.toml`, inserts a wiring comment into the entry file (`lib.rs` or `main.rs`), and records the module in `game.toml [modules]`.

```
silm add module combat --shared                             # registry (crates.io)
silm add module combat@1.2.0 --shared                      # pinned version
silm add module combat --git https://github.com/... --shared          # git HEAD
silm add module combat --git https://github.com/... --tag v1.0 --shared  # git tag
silm add module combat --git https://github.com/... --rev abc123 --shared # git rev
silm add module combat --path ./local/combat --shared       # local path
silm add module combat --path ./local/combat --vendor --shared  # vendor copy
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--shared` | Target the shared crate (`shared/`) |
| `--server` | Target the server crate (`server/`) |
| `--client` | Target the client crate (`client/`) |
| `--git <url>` | Git URL (enables git source mode) |
| `--tag <tag>` | Pin to a git tag (use with `--git`) |
| `--rev <hash>` | Pin to a git commit hash (use with `--git`) |
| `--path <dir>` | Local directory containing the module's `Cargo.toml` |
| `--vendor` | Copy source into `modules/<name>/` instead of referencing in-place (requires `--path`) |

---

## Source Modes

### Registry (default)

Pulls from crates.io. Optionally pin a version using `@`:

```bash
silm add module combat --shared           # any version
silm add module combat@1.2.0 --shared    # exact version
```

Writes to `shared/Cargo.toml`:
```toml
silmaril-module-combat = "^1.2.0"
```

### Git

References a git repository. Optionally pin a tag or commit hash:

```bash
silm add module combat --git https://github.com/myorg/silmaril-module-combat --tag v1.0 --shared
silm add module combat --git https://github.com/myorg/silmaril-module-combat --rev abc123f --shared
```

Writes to `shared/Cargo.toml`:
```toml
silmaril-module-combat = { git = "https://github.com/myorg/silmaril-module-combat", tag = "v1.0" }
```

> **Security:** Git modules are not reviewed. Inspect the source before use.

### Path

References a local directory by relative path. Useful for in-workspace modules or local development:

```bash
silm add module combat --path ./external/combat --shared
```

Writes a relative `path =` dep from the target crate to your module directory:
```toml
silmaril-module-combat = { path = "../../external/combat" }
```

The module directory must contain a valid `Cargo.toml` with a `[package] name` field.

### Vendor

Copies the module source into `modules/<name>/` inside your project. Adds it as a workspace member so it's compiled locally. Use this when you want full control over the source or need to work offline:

```bash
silm add module combat --path ./external/combat --vendor --shared
```

What this does:
1. Copies `./external/combat/` → `modules/combat/`
2. Adds `modules/combat` to `[workspace] members` in root `Cargo.toml`
3. Writes a `path = "../../modules/combat"` dep in `shared/Cargo.toml`
4. Inserts the wiring block in `shared/src/lib.rs`
5. Records `source = "vendor"` in `game.toml [modules]`

> **Note:** Vendor mode is designed for supply-chain/security requirements. In a future release it will be a licensed feature for studios.

---

## Wiring Blocks

After `silm add module`, the entry file gets a marker-guarded comment block:

```rust
// --- silmaril module: combat (silmaril-module-combat v1.2.0) ---
use silmaril_module_combat::CombatModule;
// TODO: register → world.add_module(CombatModule::new());
```

The block is detected by its `// --- silmaril module: <name> (` prefix. `silm module remove` uses this marker to cleanly remove the wiring.

You should follow the `TODO` comment to register the module with your world/app at startup.

---

## `silm module list`

Lists all installed modules and their resolved versions:

```
NAME    SOURCE    REQUIREMENT    RESOLVED  TARGET
-----------------------------------------------
combat  registry  ^1.2.0         1.2.3     shared
health  git       tag=v2.0       ?         server
ai      vendor    (local)        (local)   shared
```

Version resolution reads `Cargo.lock`. If `Cargo.lock` is absent or the module hasn't been resolved yet, the version shows as `?`. Local/vendor modules show `(local)`.

---

## `silm module remove <name>`

Removes a module and cleans up all wiring:

```bash
silm module remove combat
```

What this does:
1. Removes the dependency from `shared/Cargo.toml` (or server/client)
2. Removes the wiring block from `shared/src/lib.rs` (or main.rs)
3. For vendor modules: removes the workspace member from root `Cargo.toml` and deletes `modules/<name>/`
4. Removes the entry from `game.toml [modules]`

All file writes are atomic (temp→rename) with in-memory rollback on failure — if any step fails, all files are restored to their original state.

---

## game.toml tracking

`game.toml` records each installed module under `[modules]`:

```toml
[modules]
combat = { source = "registry", version = "^1.2.0", target = "shared", crate = "silmaril-module-combat" }
health = { source = "git", git = "https://github.com/org/health", tag = "v2.0", target = "server", crate = "silmaril-module-health" }
ai     = { source = "local", path = "../ai-module", target = "shared", crate = "silmaril-module-ai" }
boss   = { source = "vendor", ref = "vendored", target = "shared", crate = "silmaril-module-boss" }
```

The `crate` field stores the actual Rust crate name. This matters for path and vendor modules where the crate name may not follow the `silmaril-module-<name>` convention.

---

## Module Metadata (optional)

Modules can declare their type and init expression in `Cargo.toml` under `[package.metadata.silmaril]`. `silm add module` reads this to generate accurate wiring comments:

```toml
# In your module's Cargo.toml:
[package.metadata.silmaril]
module_type = "CombatModule"
target = "shared"
init = "CombatModule::default()"
```

Without this section, `silm` derives the type name from the module name (e.g. `combat` → `CombatModule`) and uses `CombatModule::new()` as the init expression.
