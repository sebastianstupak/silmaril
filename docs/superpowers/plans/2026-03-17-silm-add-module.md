# silm add module — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `silm add module` (four source modes), `silm module list`, and `silm module remove` — a package-manager-style module system for Silmaril games.

**Architecture:** String-level manipulation of game.toml and Cargo.toml (preserves comments, no subprocess calls). Wiring blocks written as marker-guarded comments in lib.rs/main.rs. Cargo.lock parsed directly for version display. Vendor mode isolated for future license gating.

**Tech Stack:** Rust, `anyhow`, `toml = "0.8"`, `serde`, `clap`, `tracing`, `tempfile` (tests)

---

## Critical Rules (read before touching any code)

- **NEVER use `println!`** — use `tracing::info!("[silm] ...")` for all output
- **Always use `bail!("message")`** for errors, never `return Err(...)`
- **Atomic writes only**: write to `path.with_extension("tmp")`, then `fs::rename(tmp, path)`
- **In-memory rollback**: read originals before any write, restore on failure
- **Tests**: no `println!` in test code either; verify by asserting file contents
- **git commits**: always use `git -c commit.gpgsign=false commit -m "..."`
- **No `cargo add` subprocess** — edit TOML files directly as strings

## Existing utilities to reuse (do NOT duplicate)

From `engine/cli/src/commands/add/wiring.rs`:
- `atomic_write(path: &Path, content: &str) -> Result<()>` — temp→rename
- `find_project_root(start: &Path) -> Result<PathBuf>` — walks up to game.toml
- `Target` enum: `Shared`/`Server`/`Client`, `.crate_subdir()`, `.entry_file()`
- `wiring_target(crate_root: &Path, target: Target) -> PathBuf` — lib.rs or main.rs
- `crate_dir(project_root: &Path, target: Target) -> Result<PathBuf>`

---

## Chunk 1: Foundations

### Task 1: module_wiring.rs — wiring block codegen + Cargo.lock parsing

**Files:**
- Create: `engine/cli/src/codegen/module_wiring.rs`
- Modify: `engine/cli/src/codegen/mod.rs` (add `pub mod module_wiring;`)

- [ ] **Step 1: Write failing tests**

Create `engine/cli/tests/codegen/module_wiring_tests.rs`:

> **Also add** `mod module_wiring_tests;` to `engine/cli/tests/codegen/mod.rs` (after the existing `mod` lines). Without this registration the test file is invisible to `cargo test`.


```rust
use silm::codegen::module_wiring::*;

#[test]
fn test_generate_wiring_block() {
    let block = generate_wiring_block("combat", "silmaril-module-combat", "1.2.3", "CombatModule", "CombatModule::new()");
    assert!(block.contains("// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---"));
    assert!(block.contains("use silmaril_module_combat::CombatModule;"));
    assert!(block.contains("// TODO: register → world.add_module(CombatModule::new());"));
}

#[test]
fn test_has_wiring_block_found() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---\nuse silmaril_module_combat::CombatModule;\n";
    assert!(has_wiring_block(content, "combat"));
}

#[test]
fn test_has_wiring_block_not_found() {
    let content = "// some other code\n";
    assert!(!has_wiring_block(content, "combat"));
}

#[test]
fn test_remove_wiring_block_single() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---\nuse silmaril_module_combat::CombatModule;\n// TODO: register → world.add_module(CombatModule::new());\n\nfn main() {}\n";
    let result = remove_wiring_block(content, "combat");
    assert!(!result.contains("// --- silmaril module: combat"));
    assert!(!result.contains("CombatModule"));
    assert!(result.contains("fn main() {}"));
}

#[test]
fn test_remove_wiring_block_adjacent_blocks() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.0.0) ---\nuse combat;\n// --- silmaril module: health (silmaril-module-health v1.0.0) ---\nuse health;\n";
    let result = remove_wiring_block(content, "combat");
    assert!(!result.contains("use combat;"));
    assert!(result.contains("// --- silmaril module: health ("));
    assert!(result.contains("use health;"));
}

#[test]
fn test_parse_cargo_lock_version_found() {
    let lock = "[[package]]\nname = \"silmaril-module-combat\"\nversion = \"1.2.3\"\nsource = \"registry+...\"\n";
    assert_eq!(parse_cargo_lock_version(lock, "silmaril-module-combat"), Some("1.2.3".to_string()));
}

#[test]
fn test_parse_cargo_lock_version_not_found() {
    let lock = "[[package]]\nname = \"some-other-crate\"\nversion = \"1.0.0\"\n";
    assert_eq!(parse_cargo_lock_version(lock, "silmaril-module-combat"), None);
}

#[test]
fn test_module_type_from_name() {
    assert_eq!(module_type_from_name("combat"), "CombatModule");
    assert_eq!(module_type_from_name("health_regen"), "HealthRegenModule");
    assert_eq!(module_type_from_name("my_module"), "MyModuleModule");
}

#[test]
fn test_crate_name_from_module_name() {
    assert_eq!(crate_name_from_module_name("combat"), "silmaril-module-combat");
    assert_eq!(crate_name_from_module_name("health_regen"), "silmaril-module-health-regen");
}

#[test]
fn test_read_module_metadata_found() {
    let cargo_toml = r#"
[package]
name = "my-combat"
version = "1.0.0"

[package.metadata.silmaril]
module_type = "MyCombatModule"
target = "server"
init = "MyCombatModule::new()"
"#;
    let meta = parse_module_metadata(cargo_toml).unwrap();
    assert_eq!(meta.module_type, "MyCombatModule");
    assert_eq!(meta.target, "server");
    assert_eq!(meta.init, "MyCombatModule::new()");
}

#[test]
fn test_read_module_metadata_absent() {
    let cargo_toml = "[package]\nname = \"my-combat\"\nversion = \"1.0.0\"\n";
    assert!(parse_module_metadata(cargo_toml).is_none());
}
```

- [ ] **Step 2: Run to confirm tests fail**

```bash
cargo test -p silm --test module_wiring_tests 2>&1 | head -20
```

Expected: compilation error (module not found)

- [ ] **Step 3: Implement `engine/cli/src/codegen/module_wiring.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub module_type: String,
    pub target: String,
    pub init: String,
}

#[derive(Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: Option<String>,
    metadata: Option<CargoMetadata>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    silmaril: Option<SilmarilMeta>,
}

#[derive(Deserialize)]
struct SilmarilMeta {
    module_type: String,
    target: String,
    init: String,
}

/// Convert snake_case module name to the conventional Rust type name.
/// "combat" → "CombatModule", "health_regen" → "HealthRegenModule"
pub fn module_type_from_name(name: &str) -> String {
    let pascal: String = name
        .split('_')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect();
    format!("{}Module", pascal)
}

/// "combat" → "silmaril-module-combat", "health_regen" → "silmaril-module-health-regen"
pub fn crate_name_from_module_name(name: &str) -> String {
    format!("silmaril-module-{}", name.replace('_', "-"))
}

/// Parse `[package.metadata.silmaril]` from a Cargo.toml string.
pub fn parse_module_metadata(cargo_toml_content: &str) -> Option<ModuleMetadata> {
    let parsed: CargoToml = toml::from_str(cargo_toml_content).ok()?;
    let meta = parsed.package?.metadata?.silmaril?;
    Some(ModuleMetadata {
        module_type: meta.module_type,
        target: meta.target,
        init: meta.init,
    })
}

/// Generate the wiring block to insert into lib.rs / main.rs.
pub fn generate_wiring_block(
    module_name: &str,
    crate_name: &str,
    version: &str,
    module_type: &str,
    init: &str,
) -> String {
    let use_path = crate_name.replace('-', "_");
    format!(
        "// --- silmaril module: {module_name} ({crate_name} v{version}) ---\nuse {use_path}::{module_type};\n// TODO: register → world.add_module({init});\n"
    )
}

/// Check whether an entry-file string contains a wiring block for `module_name`.
/// Uses `" ("` suffix (e.g. `"// --- silmaril module: combat ("`) to prevent
/// prefix collisions (e.g. "combat" must not match "combat-extended").
pub fn has_wiring_block(content: &str, module_name: &str) -> bool {
    let marker = format!("// --- silmaril module: {} (", module_name);
    content.contains(&marker)
}

/// Remove the wiring block for `module_name` from `content`.
/// Block ends at the next `// --- silmaril module:` or EOF.
/// Uses the same `" ("` suffix for start detection to avoid prefix collisions.
pub fn remove_wiring_block(content: &str, module_name: &str) -> String {
    let marker = format!("// --- silmaril module: {} (", module_name);
    let next_marker = "// --- silmaril module:";

    let start = match content.find(&marker) {
        Some(i) => i,
        None => return content.to_string(),
    };

    // Find where the block ends: next marker or EOF
    let after_start = &content[start + marker.len()..];
    let end = match after_start.find(next_marker) {
        Some(i) => start + marker.len() + i,
        None => content.len(),
    };

    let before = content[..start].trim_end_matches('\n').to_string();
    let after = content[end..].to_string();
    if before.is_empty() {
        after.trim_start_matches('\n').to_string()
    } else {
        format!("{}\n{}", before, after.trim_start_matches('\n'))
    }
}

/// Scan Cargo.lock content for the resolved version of `crate_name`.
/// Returns the version string if found.
pub fn parse_cargo_lock_version(lock_content: &str, crate_name: &str) -> Option<String> {
    let name_line = format!("name = \"{}\"", crate_name);
    let mut lines = lock_content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == name_line {
            if let Some(ver_line) = lines.next() {
                let ver_line = ver_line.trim();
                if let Some(rest) = ver_line.strip_prefix("version = \"") {
                    if let Some(ver) = rest.strip_suffix('"') {
                        return Some(ver.to_string());
                    }
                }
            }
        }
    }
    None
}
```

- [ ] **Step 4: Add `pub mod module_wiring;` to `engine/cli/src/codegen/mod.rs` (after line 32)**

```rust
pub mod module_wiring;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p silm --test module_wiring_tests 2>&1
```

Expected: all 10 tests pass

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/codegen/module_wiring.rs engine/cli/src/codegen/mod.rs engine/cli/tests/codegen/module_wiring_tests.rs engine/cli/tests/codegen/mod.rs
git -c commit.gpgsign=false commit -m "feat(cli): add module_wiring codegen — wiring block gen/detect/remove, Cargo.lock parsing"
```

---

### Task 2: game.toml helper functions

**Files:**
- Create: `engine/cli/src/commands/add/module.rs` (start with just game.toml helpers; orchestrator added in Chunk 2)

- [ ] **Step 1: Write failing tests** (add to `engine/cli/tests/codegen/module_wiring_tests.rs`)

```rust
// game.toml helpers
use silm::commands::add::module::{
    game_toml_has_module, append_module_to_game_toml, remove_module_from_game_toml,
};

#[test]
fn test_game_toml_has_module_found() {
    let content = "[modules]\ncombat = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\n";
    assert!(game_toml_has_module(content, "combat"));
}

#[test]
fn test_game_toml_has_module_not_found() {
    let content = "[modules]\n# empty\n";
    assert!(!game_toml_has_module(content, "combat"));
}

#[test]
fn test_append_module_to_game_toml_registry() {
    let content = "[project]\nname = \"test\"\n\n[modules]\n# modules here\n\n[dev]\n";
    let result = append_module_to_game_toml(content, "combat",
        "source = \"registry\", version = \"^1.2.0\", target = \"shared\"");
    assert!(result.contains("combat = { source = \"registry\", version = \"^1.2.0\", target = \"shared\" }"));
    assert!(result.contains("[dev]"));
}

#[test]
fn test_remove_module_from_game_toml() {
    let content = "[modules]\ncombat = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\nhealth = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\n";
    let result = remove_module_from_game_toml(content, "combat");
    assert!(!result.contains("combat ="));
    assert!(result.contains("health ="));
}
```

- [ ] **Step 2: Run to confirm fail**

```bash
cargo test -p silm --test module_wiring_tests 2>&1 | head -10
```

Expected: error (functions not found)

- [ ] **Step 3: Implement the game.toml helper section in `engine/cli/src/commands/add/module.rs`**

```rust
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

// ── game.toml string helpers ─────────────────────────────────────────────────

/// Return true if `[modules]` already has an entry for `name`.
/// Matches `name = { ...}` lines (Rust `{{` is a literal `{` in format strings).
pub fn game_toml_has_module(content: &str, name: &str) -> bool {
    let prefix = format!("{} = {{", name);
    content.lines().any(|l| l.trim_start().starts_with(&prefix))
}

/// Append `name = { <fields> }` inside the `[modules]` section of `game_toml_content`.
/// Inserts just before the next section header after `[modules]`, or at EOF.
pub fn append_module_to_game_toml(content: &str, name: &str, fields: &str) -> String {
    let entry = format!("{} = {{ {} }}", name, fields);
    // Find [modules] section
    if let Some(mod_pos) = content.find("[modules]") {
        // Find end of [modules] section (next '[' that starts a new section)
        let after_modules = &content[mod_pos + "[modules]".len()..];
        let insert_offset = after_modules
            .find("\n[")
            .map(|i| i + 1) // position of the '[' line
            .unwrap_or(after_modules.len());
        let insert_at = mod_pos + "[modules]".len() + insert_offset;
        let (before, after) = content.split_at(insert_at);
        let before = before.trim_end_matches('\n');
        format!("{}\n{}\n{}", before, entry, after.trim_start_matches('\n'))
    } else {
        // No [modules] section — append at end
        format!("{}\n[modules]\n{}\n", content.trim_end(), entry)
    }
}

/// Remove the line `<name> = { ... }` from `[modules]` section.
pub fn remove_module_from_game_toml(content: &str, name: &str) -> String {
    let prefix = format!("{} = {{", name);
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with(&prefix))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// ── Cargo.toml string helpers ─────────────────────────────────────────────────

/// Return true if `[dependencies]` already has an entry for `crate_name`.
pub fn cargo_toml_has_dep(content: &str, crate_name: &str) -> bool {
    let prefix = format!("{} =", crate_name);
    let prefix2 = format!("\"{}\"", crate_name); // table format
    content.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with(&prefix) || t.contains(&prefix2)
    })
}

/// Append a dep line into `[dependencies]` of `cargo_toml_content`.
/// `dep_value` is the RHS: e.g. `"^1.2.0"` or `{ git = "...", tag = "v1.0" }`.
pub fn append_dep_to_cargo_toml(content: &str, crate_name: &str, dep_value: &str) -> String {
    let entry = format!("{} = {}", crate_name, dep_value);
    if let Some(dep_pos) = content.find("[dependencies]") {
        let after = &content[dep_pos + "[dependencies]".len()..];
        let insert_offset = after
            .find("\n[")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let insert_at = dep_pos + "[dependencies]".len() + insert_offset;
        let (before, after_sec) = content.split_at(insert_at);
        let before = before.trim_end_matches('\n');
        format!("{}\n{}\n{}", before, entry, after_sec.trim_start_matches('\n'))
    } else {
        format!("{}\n[dependencies]\n{}\n", content.trim_end(), entry)
    }
}

/// Remove the dep line for `crate_name` from `[dependencies]`.
pub fn remove_dep_from_cargo_toml(content: &str, crate_name: &str) -> String {
    let prefix = format!("{} =", crate_name);
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with(&prefix))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Add `"<member_path>"` to `[workspace] members = [...]` array.
pub fn add_workspace_member(content: &str, member_path: &str) -> String {
    let entry = format!("    \"{}\",", member_path);
    // Find the closing `]` of the members array
    if let Some(members_pos) = content.find("members = [") {
        let after = &content[members_pos..];
        if let Some(close) = after.find(']') {
            let insert_at = members_pos + close;
            let (before, after_bracket) = content.split_at(insert_at);
            let before = before.trim_end_matches('\n');
            return format!("{}\n{}\n{}", before, entry, after_bracket.trim_start_matches('\n'));
        }
    }
    content.to_string()
}

/// Remove `"<member_path>"` from `[workspace] members = [...]` array.
pub fn remove_workspace_member(content: &str, member_path: &str) -> String {
    let pattern = format!("\"{}\"", member_path);
    content
        .lines()
        .filter(|l| !l.contains(&pattern))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
```

- [ ] **Step 4: Make the helpers pub from the commands module**

Add to `engine/cli/src/commands/add/mod.rs` line 6: `pub mod module;`

- [ ] **Step 5: Run tests**

```bash
cargo test -p silm --test module_wiring_tests 2>&1
```

Expected: all tests pass (including new game.toml / Cargo.toml helper tests)

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/commands/add/module.rs engine/cli/src/commands/add/mod.rs engine/cli/tests/codegen/module_wiring_tests.rs
git -c commit.gpgsign=false commit -m "feat(cli): add game.toml + Cargo.toml string-level editing helpers for module management"
```

- [ ] **Step 7: Run clippy**

```bash
cargo clippy -p silm 2>&1 | grep "^error"
```

Expected: no errors

---

## Chunk 2: add module command

### Task 3: add/module.rs — registry, git, path modes

**Files:**
- Modify: `engine/cli/src/commands/add/module.rs` (add `add_module` orchestrator)

The command flow:
1. Parse `name` for optional `@version` suffix: `"combat@1.2.0"` → name=`"combat"`, version=`"1.2.0"`
2. Validate: exactly one of `--shared/--server/--client`; `--branch` not allowed; `--vendor + --path` not allowed
3. Determine source mode from flags
4. Check duplicate in game.toml
5. Determine crate name + module_type + target
6. Check duplicate dep in consuming Cargo.toml
7. Read in-memory originals (game.toml, consuming Cargo.toml, wiring target)
8. Edit consuming Cargo.toml (atomic write)
9. Append wiring block to entry file (atomic write)
10. Append module entry to game.toml (atomic write)
11. Rollback all on failure

- [ ] **Step 1: Write failing integration test** (`engine/cli/tests/add_module_integration.rs`)

```rust
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn make_project(dir: &TempDir) {
    // game.toml
    fs::write(dir.path().join("game.toml"), "[project]\nname = \"test-game\"\n\n[modules]\n# modules\n\n[dev]\nserver_package = \"test-game-server\"\nclient_package = \"test-game-client\"\ndev_server_port = 9999\ndev_client_port = 9998\n").unwrap();
    // shared/
    fs::create_dir_all(dir.path().join("shared/src")).unwrap();
    fs::write(dir.path().join("shared/Cargo.toml"),
        "[package]\nname = \"test-game-shared\"\nversion = \"0.1.0\"\n\n[dependencies]\n").unwrap();
    fs::write(dir.path().join("shared/src/lib.rs"), "// shared lib\n").unwrap();
    // server/
    fs::create_dir_all(dir.path().join("server/src")).unwrap();
    fs::write(dir.path().join("server/Cargo.toml"),
        "[package]\nname = \"test-game-server\"\nversion = \"0.1.0\"\n\n[dependencies]\n").unwrap();
    fs::write(dir.path().join("server/src/main.rs"), "fn main() {}\n").unwrap();
    // root Cargo.toml (for vendor tests)
    fs::write(dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"shared\",\n    \"server\",\n]\n").unwrap();
}

#[test]
fn test_add_module_registry_shared() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("silmaril-module-combat"), "dep not in shared/Cargo.toml");

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("// --- silmaril module: combat"), "wiring block missing");
    assert!(lib.contains("use silmaril_module_combat::CombatModule;"), "use statement missing");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("combat ="), "game.toml entry missing");
    assert!(game.contains("source = \"registry\""), "source not registry");
}

#[test]
fn test_add_module_duplicate_rejected() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let result = silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already installed"));
}

#[test]
fn test_add_module_git_tag() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        Some("https://github.com/org/combat"),
        Some("v1.0.0"),
        None,
        None,
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("git = \"https://github.com/org/combat\""));
    assert!(cargo.contains("tag = \"v1.0.0\""));

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"git\""));
    assert!(game.contains("tag = \"v1.0.0\""));
}

#[test]
fn test_add_module_path() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);

    // Create a local module at a sibling path
    let module_dir = TempDir::new().unwrap();
    fs::write(module_dir.path().join("Cargo.toml"),
        "[package]\nname = \"my-combat\"\nversion = \"1.0.0\"\n\n[dependencies]\n").unwrap();
    fs::create_dir_all(module_dir.path().join("src")).unwrap();
    fs::write(module_dir.path().join("src/lib.rs"), "").unwrap();

    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        None,
        None,
        None,
        Some(module_dir.path().to_str().unwrap()),
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("path = "), "no path dep");
    assert!(cargo.contains("my-combat"), "wrong crate name");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"local\""));
}
```

- [ ] **Step 2: Confirm tests fail**

```bash
cargo test -p silm --test add_module_integration test_add_module_registry_shared 2>&1 | tail -5
```

Expected: error (add_module not defined)

- [ ] **Step 3: Implement `add_module` in `engine/cli/src/commands/add/module.rs`**

Append after the helper functions from Task 2:

```rust
use std::env;
use crate::codegen::module_wiring::{
    generate_wiring_block, has_wiring_block, crate_name_from_module_name,
    module_type_from_name, parse_module_metadata,
};
use super::wiring::{
    atomic_write, crate_dir, find_project_root, wiring_target, Target,
};

/// Source mode for `silm add module`.
/// `git_url`: Some for git mode, None for registry.
/// `local_path`: Some for path mode.
/// `tag`/`rev`: git pin options.
pub fn add_module(
    name: &str,
    git_url: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    local_path: Option<&str>,
    vendor: bool,
    target: Target,
) -> Result<()> {
    // Parse optional @version from name
    let (module_name, requested_version) = if let Some(at) = name.rfind('@') {
        (&name[..at], Some(&name[at + 1..]))
    } else {
        (name, None)
    };

    // Validation
    // Vendor mode requires --path (git-clone-then-vendor is a future enhancement)
    if vendor && local_path.is_none() {
        bail!("--vendor requires --path: clone the module to a local directory first, then vendor it with --vendor --path <dir>");
    }
    // Note: vendor + local_path is valid (it's how vendor works)

    // Find project root
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;

    // Resolve consuming crate
    let crate_root = crate_dir(&project_root, target)?;
    let game_toml_path = project_root.join("game.toml");
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    // Read originals (in-memory for rollback)
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;
    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry_file = if entry_file.exists() {
        fs::read_to_string(&entry_file)?
    } else {
        String::new()
    };

    // Duplicate check
    if game_toml_has_module(&orig_game_toml, module_name) {
        bail!("module '{}' is already installed — use 'silm module upgrade' to update", module_name);
    }

    // Determine source, crate name, dep value, game.toml fields, module type
    let (crate_name, dep_value, game_entry_fields, module_type, init) = if vendor {
        // Vendor mode: copy local source into modules/<name>/ (isolated for future license gating)
        // local_path is guaranteed Some by the validation above
        let path = local_path.unwrap();
        return add_module_vendor_from_path(module_name, Path::new(path), target, &project_root);
    } else if let Some(path) = local_path {
        // Path mode: read actual crate name + metadata from the module's Cargo.toml
        let mod_cargo = Path::new(path).join("Cargo.toml");
        let mod_cargo_content = fs::read_to_string(&mod_cargo)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {}", mod_cargo.display(), e))?;

        let crate_name = {
            #[derive(serde::Deserialize)]
            struct Pkg { package: PkgInner }
            #[derive(serde::Deserialize)]
            struct PkgInner { name: String }
            let p: Pkg = toml::from_str(&mod_cargo_content)
                .map_err(|e| anyhow::anyhow!("invalid Cargo.toml at {}: {}", path, e))?;
            p.package.name
        };

        let (module_type, init) = if let Some(meta) = parse_module_metadata(&mod_cargo_content) {
            (meta.module_type, meta.init)
        } else {
            (module_type_from_name(module_name), format!("{}::new()", module_type_from_name(module_name)))
        };

        // Make path relative from consuming crate
        let abs_path = std::path::Path::new(path).canonicalize()
            .map_err(|_| anyhow::anyhow!("path '{}' does not exist", path))?;
        let rel_path = pathdiff::diff_paths(&abs_path, &crate_root)
            .unwrap_or_else(|| abs_path.clone())
            .to_string_lossy()
            .replace('\\', "/");

        let dep_value = format!("{{ path = \"{}\" }}", rel_path);
        let game_entry = format!(
            "source = \"local\", path = \"{}\", target = \"{}\", crate = \"{}\"",
            path, target.crate_subdir(), crate_name
        );
        (crate_name, dep_value, game_entry, module_type, init)
    } else if let Some(url) = git_url {
        // Git mode
        let crate_name = crate_name_from_module_name(module_name);
        let pin = if let Some(r) = rev {
            format!(", rev = \"{}\"", r)
        } else if let Some(t) = tag {
            format!(", tag = \"{}\"", t)
        } else {
            String::new()
        };
        let dep_value = format!("{{ git = \"{}\"{} }}", url, pin);

        let game_entry = if let Some(r) = rev {
            format!("source = \"git\", url = \"{}\", rev = \"{}\", target = \"{}\", crate = \"{}\"", url, r, target.crate_subdir(), crate_name)
        } else if let Some(t) = tag {
            format!("source = \"git\", url = \"{}\", tag = \"{}\", target = \"{}\", crate = \"{}\"", url, t, target.crate_subdir(), crate_name)
        } else {
            format!("source = \"git\", url = \"{}\", target = \"{}\", crate = \"{}\"", url, target.crate_subdir(), crate_name)
        };

        let module_type = module_type_from_name(module_name);
        let init = format!("{}::new()", module_type);
        tracing::warn!("[silm] This module is not from crates.io — review the source before use.");
        (crate_name, dep_value, game_entry, module_type, init)
    } else {
        // Registry mode
        let crate_name = crate_name_from_module_name(module_name);
        // Note: "*" means "any version" — a future enhancement can resolve the latest
        // published version from crates.io and write a pinned requirement like "^1.2.0".
        let version = requested_version.unwrap_or("*");
        let dep_value = format!("\"{}\"", version);
        let game_entry = format!(
            "source = \"registry\", version = \"{}\", target = \"{}\", crate = \"{}\"",
            version, target.crate_subdir(), crate_name
        );
        let module_type = module_type_from_name(module_name);
        let init = format!("{}::new()", module_type);
        (crate_name, dep_value, game_entry, module_type, init)
    };

    // Check dep duplicate in Cargo.toml
    if cargo_toml_has_dep(&orig_cargo_toml, &crate_name) {
        bail!("module '{}' is already installed — use 'silm module upgrade' to update", module_name);
    }

    // Apply writes (with rollback on failure)
    let result = (|| -> Result<()> {
        // 1. Add dep to consuming Cargo.toml
        let new_cargo = append_dep_to_cargo_toml(&orig_cargo_toml, &crate_name, &dep_value);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 2. Append wiring block
        if !has_wiring_block(&orig_entry_file, module_name) {
            let block = generate_wiring_block(module_name, &crate_name, "latest", &module_type, &init);
            let new_entry = format!("{}\n{}", orig_entry_file.trim_end(), block);
            atomic_write(&entry_file, &new_entry)?;
        }

        // 3. Update game.toml
        let new_game = append_module_to_game_toml(&orig_game_toml, module_name, &game_entry_fields);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        // Rollback
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry_file);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!("[silm] added {} ({}) → {}/", crate_name, target.crate_subdir(), target.crate_subdir());
    tracing::info!("[silm] wired: {}", entry_file.display());
    tracing::info!("[silm] tracked: game.toml [modules.{}]", module_name);
    Ok(())
}
```

Note: `pathdiff` crate is needed for path relativization. Add to `engine/cli/Cargo.toml`:
```toml
pathdiff = "0.2"
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p silm --test add_module_integration 2>&1
```

Expected: `test_add_module_registry_shared`, `test_add_module_duplicate_rejected`, `test_add_module_git_tag`, `test_add_module_path` pass

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/add/module.rs engine/cli/Cargo.toml engine/cli/tests/add_module_integration.rs
git -c commit.gpgsign=false commit -m "feat(cli): add_module registry/git/path modes with atomic writes and rollback"
```

---

### Task 4: add/module.rs — vendor mode

**Files:**
- Modify: `engine/cli/src/commands/add/module.rs` (add `VendorSource` struct and `add_module_vendor`)

- [ ] **Step 1: Write failing integration test** (append to `add_module_integration.rs`)

```rust
#[test]
fn test_add_module_vendor() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);

    // Create a fake git repo to vendor from (just a directory with Cargo.toml)
    let upstream = TempDir::new().unwrap();
    fs::write(upstream.path().join("Cargo.toml"),
        "[package]\nname = \"silmaril-module-combat\"\nversion = \"1.0.0\"\n\n[dependencies]\n").unwrap();
    fs::create_dir_all(upstream.path().join("src")).unwrap();
    fs::write(upstream.path().join("src/lib.rs"), "pub struct CombatModule;\n").unwrap();

    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module_vendor_from_path(
        "combat",
        upstream.path(),
        silm::commands::add::wiring::Target::Shared,
        &dir.path().to_path_buf(),
    ).unwrap();

    // modules/combat/ should exist
    assert!(dir.path().join("modules/combat/Cargo.toml").exists());

    // Root Cargo.toml should have workspace member
    let root_cargo = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("modules/combat"));

    // shared/Cargo.toml should have path dep
    let shared_cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(shared_cargo.contains("path ="));
    assert!(shared_cargo.contains("modules/combat"));

    // game.toml should have vendor entry
    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"vendor\""));
}
```

- [ ] **Step 2: Confirm test fails**

```bash
cargo test -p silm --test add_module_integration test_add_module_vendor 2>&1 | tail -5
```

- [ ] **Step 3: Implement vendor mode**

Append to `engine/cli/src/commands/add/module.rs`:

```rust
/// Isolated vendor code path (future: license check inserted here).
/// `source_path`: the directory to copy into modules/<name>/.
pub fn add_module_vendor_from_path(
    module_name: &str,
    source_path: &Path,
    target: Target,
    project_root: &Path,
) -> Result<()> {
    let modules_dir = project_root.join("modules").join(module_name);
    if modules_dir.exists() {
        bail!("modules/{} already exists — remove it first", module_name);
    }

    let crate_root = crate_dir(project_root, target)?;
    let game_toml_path = project_root.join("game.toml");
    let root_cargo_path = project_root.join("Cargo.toml");
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    // Read originals
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;
    let orig_root_cargo = fs::read_to_string(&root_cargo_path)?;
    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry = if entry_file.exists() { fs::read_to_string(&entry_file)? } else { String::new() };

    // Duplicate check
    if game_toml_has_module(&orig_game_toml, module_name) {
        bail!("module '{}' is already installed", module_name);
    }

    // Copy source → modules/<name>/
    copy_dir_all(source_path, &modules_dir)?;

    // Read crate name and metadata from vendored Cargo.toml
    let vendored_cargo_content = fs::read_to_string(modules_dir.join("Cargo.toml"))?;
    let crate_name = {
        #[derive(serde::Deserialize)]
        struct Pkg { package: PkgInner }
        #[derive(serde::Deserialize)]
        struct PkgInner { name: String }
        let p: Pkg = toml::from_str(&vendored_cargo_content)
            .map_err(|e| anyhow::anyhow!("invalid Cargo.toml in vendored module: {}", e))?;
        p.package.name
    };

    let (module_type, init) = if let Some(meta) = parse_module_metadata(&vendored_cargo_content) {
        (meta.module_type, meta.init)
    } else {
        (module_type_from_name(module_name), format!("{}::new()", module_type_from_name(module_name)))
    };

    // Path dep from consuming crate to modules/<name>/
    // e.g. from shared/ → ../../modules/combat
    let rel_path = format!("../../modules/{}", module_name);
    let dep_value = format!("{{ path = \"{}\" }}", rel_path);

    let result = (|| -> Result<()> {
        // 1. Add workspace member to root Cargo.toml
        let new_root = add_workspace_member(&orig_root_cargo, &format!("modules/{}", module_name));
        atomic_write(&root_cargo_path, &new_root)?;

        // 2. Add path dep to consuming Cargo.toml
        let new_cargo = append_dep_to_cargo_toml(&orig_cargo_toml, &crate_name, &dep_value);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 3. Wiring block
        if !has_wiring_block(&orig_entry, module_name) {
            let block = generate_wiring_block(module_name, &crate_name, "vendored", &module_type, &init);
            let new_entry = format!("{}\n{}", orig_entry.trim_end(), block);
            atomic_write(&entry_file, &new_entry)?;
        }

        // 4. game.toml
        let game_entry = format!("source = \"vendor\", ref = \"vendored\", target = \"{}\", crate = \"{}\"", target.crate_subdir(), crate_name);
        let new_game = append_module_to_game_toml(&orig_game_toml, module_name, &game_entry);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        let _ = fs::remove_dir_all(&modules_dir);
        let _ = atomic_write(&root_cargo_path, &orig_root_cargo);
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!("[silm] vendored {} → modules/{}/", crate_name, module_name);
    tracing::info!("[silm] wired: {}", entry_file.display());
    tracing::info!("[silm] tracked: game.toml [modules.{}]", module_name);
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
```

The vendor branch in `add_module` (Task 3) already calls `add_module_vendor_from_path` directly — the two functions are now correctly wired.

- [ ] **Step 4: Run tests**

```bash
cargo test -p silm --test add_module_integration 2>&1
```

Expected: all tests pass including `test_add_module_vendor`

- [ ] **Step 5: Clippy**

```bash
cargo clippy -p silm 2>&1 | grep "^error"
```

Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/commands/add/module.rs
git -c commit.gpgsign=false commit -m "feat(cli): add_module vendor mode — isolated VendorSource path, copy + workspace member + rollback"
```

---

## Chunk 3: Management commands, CLI wiring, template update

### Task 5: module/list.rs

**Files:**
- Create: `engine/cli/src/commands/module/mod.rs`
- Create: `engine/cli/src/commands/module/list.rs`

- [ ] **Step 1: Write failing integration test** (append to `add_module_integration.rs`)

```rust
#[test]
fn test_module_list_empty() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    // Should not error on empty [modules]
    let result = silm::commands::module::list::list_modules(&dir.path().to_path_buf());
    assert!(result.is_ok());
}

#[test]
fn test_module_list_after_add() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    // list should not error even without Cargo.lock (resolves to "?")
    let result = silm::commands::module::list::list_modules(&dir.path().to_path_buf());
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Implement `engine/cli/src/commands/module/list.rs`**

```rust
use anyhow::Result;
use std::fs;
use std::path::Path;
use crate::commands::add::module::game_toml_has_module;
use crate::codegen::module_wiring::parse_cargo_lock_version;

pub fn list_modules(project_root: &Path) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    if !game_toml_path.exists() {
        anyhow::bail!("no game.toml found");
    }
    let content = fs::read_to_string(&game_toml_path)?;

    // Read Cargo.lock if present
    let lock_content = fs::read_to_string(project_root.join("Cargo.lock")).unwrap_or_default();

    // Parse [modules] section manually
    let modules = parse_modules_section(&content);

    if modules.is_empty() {
        tracing::info!("[silm] no modules installed");
        return Ok(());
    }

    let name_w = modules.iter().map(|(n, _)| n.len()).max().unwrap_or(4).max(4);
    let src_w = 8usize;
    let req_w = modules.iter().map(|(_, v)| {
        extract_field(v, "version").or_else(|| extract_field(v, "tag")).or_else(|| extract_field(v, "ref")).or_else(|| extract_field(v, "path")).unwrap_or_default().len()
    }).max().unwrap_or(11).max(11);

    tracing::info!("{:<nw$}  {:<sw$}  {:<rw$}  {:<8}  TARGET",
        "NAME", "SOURCE", "REQUIREMENT", "RESOLVED",
        nw = name_w, sw = src_w, rw = req_w);
    tracing::info!("{}", "-".repeat(name_w + src_w + req_w + 30));

    for (name, fields) in &modules {
        let source = extract_field(fields, "source").unwrap_or_default();
        let requirement = extract_field(fields, "version")
            .or_else(|| extract_field(fields, "tag").map(|t| format!("tag={}", t)))
            .or_else(|| extract_field(fields, "ref").map(|r| format!("ref={}", r)))
            .or_else(|| extract_field(fields, "path"))
            .unwrap_or_default();
        let target = extract_field(fields, "target").unwrap_or_default();

        // Use the stored `crate = "..."` field if present (path/vendor can have non-conventional names)
        let crate_name = extract_field(fields, "crate")
            .unwrap_or_else(|| format!("silmaril-module-{}", name.replace('_', "-")));
        let resolved = if source == "local" || source == "vendor" {
            "(local)".to_string()
        } else {
            parse_cargo_lock_version(&lock_content, &crate_name)
                .unwrap_or_else(|| "?".to_string())
        };

        tracing::info!("{:<nw$}  {:<sw$}  {:<rw$}  {:<8}  {}",
            name, source, requirement, resolved, target,
            nw = name_w, sw = src_w, rw = req_w);
    }

    Ok(())
}

/// Parse the [modules] section into (name, fields_string) pairs.
fn parse_modules_section(content: &str) -> Vec<(String, String)> {
    let mut in_modules = false;
    let mut result = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[modules]" { in_modules = true; continue; }
        if trimmed.starts_with('[') && trimmed != "[modules]" { in_modules = false; continue; }
        if !in_modules || trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if let Some(eq) = trimmed.find(" = {") {
            let name = trimmed[..eq].trim().to_string();
            let fields = trimmed[eq + 4..].trim_end_matches('}').to_string();
            result.push((name, fields));
        }
    }
    result
}

/// Extract a field value from an inline TOML fields string.
/// e.g. extract_field(`source = "registry", version = "^1.0"`, "source") → Some("registry")
fn extract_field(fields: &str, key: &str) -> Option<String> {
    let pattern = format!("{} = \"", key);
    if let Some(start) = fields.find(&pattern) {
        let rest = &fields[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
```

- [ ] **Step 3: Create `engine/cli/src/commands/module/mod.rs`**

```rust
pub mod list;
pub mod remove;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ModuleCommand {
    /// List installed modules and their resolved versions
    List,
    /// Remove a module and its wiring
    Remove {
        /// Module name (e.g. combat)
        name: String,
    },
}

pub fn handle_module_command(command: ModuleCommand, project_root: std::path::PathBuf) -> Result<()> {
    match command {
        ModuleCommand::List => list::list_modules(&project_root),
        ModuleCommand::Remove { name } => remove::remove_module(&name, &project_root),
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p silm --test add_module_integration test_module_list 2>&1
```

Expected: both list tests pass

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/module/
git -c commit.gpgsign=false commit -m "feat(cli): silm module list — reads game.toml + Cargo.lock, tabular output"
```

---

### Task 6: module/remove.rs

**Files:**
- Create: `engine/cli/src/commands/module/remove.rs`

- [ ] **Step 1: Write failing integration test** (append to `add_module_integration.rs`)

```rust
#[test]
fn test_module_remove() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf()).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(!cargo.contains("silmaril-module-combat"), "dep still in Cargo.toml");

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(!lib.contains("// --- silmaril module: combat"), "wiring block still present");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(!game.contains("combat ="), "game.toml entry still present");
}

#[test]
fn test_module_remove_not_installed() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    let result = silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not installed"));
}
```

- [ ] **Step 2: Implement `engine/cli/src/commands/module/remove.rs`**

```rust
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;
use crate::commands::add::module::{
    game_toml_has_module, remove_module_from_game_toml,
    remove_dep_from_cargo_toml, remove_workspace_member,
};
use crate::commands::add::wiring::{atomic_write, crate_dir, find_project_root, wiring_target, Target};
use crate::codegen::module_wiring::remove_wiring_block;

pub fn remove_module(module_name: &str, project_root: &Path) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;

    if !game_toml_has_module(&orig_game_toml, module_name) {
        bail!("module '{}' is not installed", module_name);
    }

    // Determine target and crate name from game.toml entry
    // (crate name stored in game.toml because path/vendor modules may have non-conventional names)
    let target = detect_target_from_game_toml(&orig_game_toml, module_name)?;
    let source = detect_source_from_game_toml(&orig_game_toml, module_name);
    let crate_name = detect_crate_name_from_game_toml(&orig_game_toml, module_name)
        .unwrap_or_else(|| format!("silmaril-module-{}", module_name.replace('_', "-")));

    let crate_root = crate_dir(project_root, target)?;
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry = if entry_file.exists() { fs::read_to_string(&entry_file)? } else { String::new() };
    let orig_root_cargo = fs::read_to_string(project_root.join("Cargo.toml")).unwrap_or_default();

    let result = (|| -> Result<()> {
        // 1. Remove dep from consuming Cargo.toml
        let new_cargo = remove_dep_from_cargo_toml(&orig_cargo_toml, &crate_name);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 2. Remove wiring block from entry file
        let new_entry = remove_wiring_block(&orig_entry, module_name);
        atomic_write(&entry_file, &new_entry)?;

        // 3. Vendor: remove workspace member + delete modules/<name>/
        if source.as_deref() == Some("vendor") {
            let new_root = remove_workspace_member(&orig_root_cargo, &format!("modules/{}", module_name));
            atomic_write(&project_root.join("Cargo.toml"), &new_root)?;
            let vendor_dir = project_root.join("modules").join(module_name);
            if vendor_dir.exists() {
                fs::remove_dir_all(&vendor_dir)?;
            }
        }

        // 4. Remove from game.toml
        let new_game = remove_module_from_game_toml(&orig_game_toml, module_name);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!("[silm] removed module '{}'", module_name);
    Ok(())
}

fn detect_target_from_game_toml(content: &str, module_name: &str) -> Result<Target> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            if line.contains("\"shared\"") { return Ok(Target::Shared); }
            if line.contains("\"server\"") { return Ok(Target::Server); }
            if line.contains("\"client\"") { return Ok(Target::Client); }
        }
    }
    bail!("cannot determine target for module '{}'", module_name);
}

fn detect_source_from_game_toml(content: &str, module_name: &str) -> Option<String> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            if line.contains("source = \"vendor\"") { return Some("vendor".to_string()); }
            if line.contains("source = \"local\"") { return Some("local".to_string()); }
            if line.contains("source = \"git\"") { return Some("git".to_string()); }
            if line.contains("source = \"registry\"") { return Some("registry".to_string()); }
        }
    }
    None
}

/// Read the actual crate name from the game.toml `crate = "..."` field.
/// Falls back to None if absent (caller should derive from module name as fallback).
fn detect_crate_name_from_game_toml(content: &str, module_name: &str) -> Option<String> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            // extract_field-style inline search for `crate = "<name>"`
            let pat = "crate = \"";
            if let Some(start) = line.find(pat) {
                let rest = &line[start + pat.len()..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p silm --test add_module_integration 2>&1
```

Expected: all tests pass including `test_module_remove` and `test_module_remove_not_installed`

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/module/remove.rs
git -c commit.gpgsign=false commit -m "feat(cli): silm module remove — unwire dep + entry file block + game.toml with rollback"
```

---

### Task 7: CLI registration (AddCommand + ModuleCommand + main.rs)

**Files:**
- Modify: `engine/cli/src/commands/add/mod.rs` — add `Module` variant
- Modify: `engine/cli/src/commands/mod.rs` — add `pub mod module;`
- Modify: `engine/cli/src/main.rs` — add `Module` to `Commands`

- [ ] **Step 1: Add `Module` variant to `AddCommand` in `engine/cli/src/commands/add/mod.rs`**

Add after the `System { ... }` variant (around line 62), and update `handle_add_command`:

```rust
/// Add a game module (registry, git, path, or vendor)
Module {
    /// Module name, optionally with version: combat or combat@1.2.0
    name: String,

    /// Git URL (git source mode)
    #[arg(long)]
    git: Option<String>,

    /// Git tag to pin (use with --git)
    #[arg(long)]
    tag: Option<String>,

    /// Git commit hash to pin (use with --git)
    #[arg(long)]
    rev: Option<String>,

    /// Local path to module source
    #[arg(long)]
    path: Option<String>,

    /// Vendor mode: copy source into modules/<name>/
    #[arg(long)]
    vendor: bool,

    /// Target the shared crate
    #[arg(long, conflicts_with_all = ["server", "client"])]
    shared: bool,

    /// Target the server crate
    #[arg(long, conflicts_with_all = ["shared", "client"])]
    server: bool,

    /// Target the client crate
    #[arg(long, conflicts_with_all = ["shared", "server"])]
    client: bool,
},
```

Update `handle_add_command` to include:

```rust
AddCommand::Module { name, git, tag, rev, path, vendor, shared, server, client } => {
    let target = resolve_target(shared, server, client)?;
    module::add_module(
        &name,
        git.as_deref(),
        tag.as_deref(),
        rev.as_deref(),
        path.as_deref(),
        vendor,
        target,
    )
}
```

- [ ] **Step 2: Add `pub mod module;` to `engine/cli/src/commands/mod.rs`**

```rust
pub mod add;
pub mod dev;
pub mod module;
pub mod new;
pub mod template;
```

- [ ] **Step 3: Register `Module` subcommand in `engine/cli/src/main.rs`**

Add to `Commands` enum:
```rust
/// Manage installed game modules
Module {
    #[command(subcommand)]
    command: commands::module::ModuleCommand,
},
```

Add to `match cli.command`:
```rust
Commands::Module { command } => {
    let cwd = std::env::current_dir()?;
    let project_root = commands::add::wiring::find_project_root(&cwd)?;
    commands::module::handle_module_command(command, project_root)?;
}
```

- [ ] **Step 4: Build to verify**

```bash
cargo build -p silm 2>&1 | grep "^error"
```

Expected: no errors

- [ ] **Step 5: Smoke test**

```bash
./target/debug/silm add module --help
./target/debug/silm module --help
./target/debug/silm module list --help
./target/debug/silm module remove --help
```

Expected: each shows help text without error

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/commands/add/mod.rs engine/cli/src/commands/mod.rs engine/cli/src/main.rs
git -c commit.gpgsign=false commit -m "feat(cli): register silm add module + silm module list/remove in CLI"
```

---

### Task 8: Template update (basic.rs)

**Files:**
- Modify: `engine/cli/src/templates/basic.rs` — update `[modules]` comment example

- [ ] **Step 1: Update the game_toml() comment** (around line 59)

Replace:
```
# combat = {{ source = "git", version = "0.1.0" }}
```
With:
```
# combat = {{ source = "registry", version = "^1.0.0", target = "shared" }}
```

- [ ] **Step 2: Build and run template unit tests**

```bash
cargo test -p silm --lib 2>&1 | grep "FAIL\|error"
```

Expected: no failures

- [ ] **Step 3: Commit**

```bash
git add engine/cli/src/templates/basic.rs
git -c commit.gpgsign=false commit -m "docs(cli): update game.toml [modules] comment example with full schema"
```

- [ ] **Step 4: Full check**

```bash
cargo clippy -p silm 2>&1 | grep "^error"
cargo test -p silm 2>&1 | tail -20
```

Expected: no errors, all tests pass

---

## Chunk 4: Remaining tests + final verification

### Task 9: Complete unit test suite

**Files:**
- Modify: `engine/cli/tests/codegen/module_wiring_tests.rs` (verify all test cases from spec)

Verify these test cases are present and passing (add any missing):

- [ ] **game.toml helpers:** has_module, append, remove (✅ Task 2)
- [ ] **Cargo.toml helpers:** has_dep, append_dep, remove_dep, add_workspace_member, remove_workspace_member

Add missing Cargo.toml helper tests:

```rust
#[test]
fn test_cargo_has_dep_found() {
    let content = "[dependencies]\nsome-crate = \"1.0\"\n";
    assert!(silm::commands::add::module::cargo_toml_has_dep(content, "some-crate"));
}

#[test]
fn test_cargo_append_dep() {
    let content = "[package]\nname = \"foo\"\n\n[dependencies]\n";
    let result = silm::commands::add::module::append_dep_to_cargo_toml(content, "combat", "\"^1.0\"");
    assert!(result.contains("combat = \"^1.0\""));
}

#[test]
fn test_cargo_remove_dep() {
    let content = "[dependencies]\ncombat = \"^1.0\"\nhealth = \"^1.0\"\n";
    let result = silm::commands::add::module::remove_dep_from_cargo_toml(content, "combat");
    assert!(!result.contains("combat ="));
    assert!(result.contains("health ="));
}

#[test]
fn test_add_workspace_member() {
    let content = "[workspace]\nmembers = [\n    \"shared\",\n]\n";
    let result = silm::commands::add::module::add_workspace_member(content, "modules/combat");
    assert!(result.contains("\"modules/combat\""));
    assert!(result.contains("\"shared\""));
}

#[test]
fn test_remove_workspace_member() {
    let content = "[workspace]\nmembers = [\n    \"shared\",\n    \"modules/combat\",\n]\n";
    let result = silm::commands::add::module::remove_workspace_member(content, "modules/combat");
    assert!(!result.contains("modules/combat"));
    assert!(result.contains("\"shared\""));
}
```

- [ ] **Run all unit tests**

```bash
cargo test -p silm --test module_wiring_tests 2>&1
```

Expected: all pass

- [ ] **Commit**

```bash
git add engine/cli/tests/codegen/module_wiring_tests.rs
git -c commit.gpgsign=false commit -m "test(cli): complete module_wiring unit test suite"
```

---

### Task 10: Complete integration test suite + final verification

**Files:**
- Modify: `engine/cli/tests/add_module_integration.rs` (add remaining cases from spec)

- [ ] **Add remaining integration tests**

```rust
#[test]
fn test_wiring_block_idempotent() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let before = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();

    // Second add with different module, should not duplicate wiring for first
    silm::commands::add::module::add_module(
        "health", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    let count = lib.matches("// --- silmaril module: combat").count();
    assert_eq!(count, 1, "wiring block should appear exactly once");
}

#[test]
fn test_add_module_server_target() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Server,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("server/Cargo.toml")).unwrap();
    assert!(cargo.contains("silmaril-module-combat"));

    let main_rs = fs::read_to_string(dir.path().join("server/src/main.rs")).unwrap();
    assert!(main_rs.contains("// --- silmaril module: combat"));

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("target = \"server\""));
}

#[test]
fn test_remove_preserves_adjacent_modules() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();
    silm::commands::add::module::add_module(
        "health", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf()).unwrap();

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(!lib.contains("// --- silmaril module: combat"), "combat block still present");
    assert!(lib.contains("// --- silmaril module: health"), "health block removed incorrectly");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(!game.contains("combat ="));
    assert!(game.contains("health ="));
}

#[test]
fn test_git_rev_pinning() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        Some("https://github.com/org/combat"),
        None,
        Some("abc123f"),
        None,
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("rev = \"abc123f\""));
    assert!(!cargo.contains("tag ="));
}
```

- [ ] **Run all integration tests**

```bash
cargo test -p silm --test add_module_integration 2>&1
```

Expected: all tests pass

- [ ] **Run full test suite**

```bash
cargo test -p silm 2>&1 | tail -30
```

Expected: all tests pass, no regressions

- [ ] **Run clippy**

```bash
cargo clippy -p silm 2>&1 | grep "^error\|^warning\[" | head -20
```

Expected: no errors; address any warnings

- [ ] **Final commit**

```bash
git add engine/cli/tests/add_module_integration.rs
git -c commit.gpgsign=false commit -m "test(cli): complete integration test suite for silm add module + list + remove"
```

- [ ] **Verify silm binary end-to-end help text**

```bash
cargo build -p silm --quiet
./target/debug/silm add module --help
./target/debug/silm module list --help
./target/debug/silm module remove --help
```

Expected: all show correct help text with all flags documented
