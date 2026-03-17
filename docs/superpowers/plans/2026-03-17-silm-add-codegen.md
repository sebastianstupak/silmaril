# silm add component/system — Code Generation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `silm add component` and `silm add system` CLI commands that scaffold ECS building blocks into vertical domain slices with full auto-wiring.

**Architecture:** Rewrite `engine/cli/src/commands/add.rs` into a module (`add/`) with separate files for component, system, and wiring logic. Update the query parser to accept `mut:X` syntax. Generate code into `src/<domain>/mod.rs` with atomic writes and idempotent `pub mod <domain>;` wiring.

**Tech Stack:** Rust, clap (CLI), anyhow (errors), tracing (output), tempfile (atomic writes), regex (duplicate detection)

---

## File Structure

### New files
- `engine/cli/src/commands/add/mod.rs` — New `AddCommand` enum + entry point
- `engine/cli/src/commands/add/component.rs` — `add_component()` orchestrator
- `engine/cli/src/commands/add/system.rs` — `add_system()` orchestrator
- `engine/cli/src/commands/add/wiring.rs` — Atomic file writes, duplicate detection, mod wiring

### Modified files
- `engine/cli/src/codegen/parser.rs` — Rewrite `parse_query_components` for `mut:X` syntax
- `engine/cli/src/codegen/component.rs` — Fix derives, test module names, remove `derive`/`doc` params
- `engine/cli/src/codegen/system.rs` — New name suffix, `dt` param, remove phase/doc/crate imports
- `engine/cli/src/codegen/mod.rs` — Remove `SystemPhase` re-export
- `engine/cli/src/templates/basic.rs` — Remove horizontal stubs
- `engine/cli/tests/parser_tests.rs` — Migrate `&mut X` → `mut:X`
- `engine/cli/tests/codegen/system_tests.rs` — Migrate + update for new codegen API
- `engine/cli/tests/system_integration.rs` — Migrate query syntax

### Deleted files
- `engine/cli/src/commands/add.rs` — Replaced by `add/` module

---

## Chunk 1: Parser + Codegen Updates

### Task 1: Rewrite `parse_query_components` in `parser.rs`

**Files:**
- Modify: `engine/cli/src/codegen/parser.rs`
- Modify: `engine/cli/tests/parser_tests.rs`

The function currently accepts `&mut X` / `&X`. Rewrite to accept `mut:X` / bare `X`.

- [ ] **Step 1.1: Write the new failing parser tests (inline)**

Replace the inline `#[cfg(test)]` block in `parser.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bare_immutable() {
        let result = parse_query_components("Health,Velocity").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
        assert_eq!(result[1].name, "Velocity");
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_mut_prefix() {
        let result = parse_query_components("mut:Health,RegenerationRate").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].name, "RegenerationRate");
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_multiple_mutable() {
        let result = parse_query_components("mut:Health,mut:Velocity,Mass").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].access, QueryAccess::Mutable);
        assert_eq!(result[2].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_single_component() {
        let result = parse_query_components("Health").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_whitespace_trimmed() {
        let result = parse_query_components("  mut:Health  ,  RegenerationRate  ").unwrap();
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[1].name, "RegenerationRate");
    }

    #[test]
    fn test_old_ampersand_syntax_rejected() {
        let result = parse_query_components("&mut Health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("use 'mut:ComponentName' syntax"), "got: {msg}");
    }

    #[test]
    fn test_old_ampersand_immutable_rejected() {
        let result = parse_query_components("&Health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("use 'mut:ComponentName' syntax"), "got: {msg}");
    }

    #[test]
    fn test_lowercase_component_rejected() {
        let result = parse_query_components("health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid query token"), "got: {msg}");
    }

    #[test]
    fn test_empty_string_rejected() {
        assert!(parse_query_components("").is_err());
    }

    #[test]
    fn test_empty_component_rejected() {
        assert!(parse_query_components("Health,,Velocity").is_err());
    }

    #[test]
    fn test_numbers_in_name_ok() {
        let result = parse_query_components("Camera2D,Transform3D").unwrap();
        assert_eq!(result[0].name, "Camera2D");
        assert_eq!(result[1].name, "Transform3D");
    }

    #[test]
    fn test_type_syntax_immutable() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.type_syntax(), "&Health");
    }

    #[test]
    fn test_type_syntax_mutable() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Mutable);
        assert_eq!(comp.type_syntax(), "&mut Health");
    }

    #[test]
    fn test_var_name() {
        let comp = QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.var_name(), "regeneration_rate");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Health"), "health");
        assert_eq!(to_snake_case("RegenerationRate"), "regeneration_rate");
        assert_eq!(to_snake_case("Transform"), "transform");
    }
}
```

- [ ] **Step 1.2: Run tests to verify they fail**

```bash
cargo test -p silm parse_query_components 2>&1 | head -30
```

Expected: several FAIL — old tests pass, new tests fail (function not changed yet)

- [ ] **Step 1.3: Rewrite `parse_query_components`**

Replace the function body in `engine/cli/src/codegen/parser.rs`:

```rust
/// Parse query components from a string using `mut:X` / bare `X` syntax.
///
/// # Format
/// `[mut:]ComponentName[,[mut:]ComponentName]*`
///
/// # Examples
/// - `"mut:Health,RegenerationRate"` → mutable Health, immutable RegenerationRate
/// - `"mut:Health,mut:Velocity,Mass"` → mutable Health and Velocity, immutable Mass
pub fn parse_query_components(input: &str) -> Result<Vec<QueryComponent>> {
    if input.trim().is_empty() {
        bail!("Query string cannot be empty");
    }

    input
        .split(',')
        .map(|token| {
            let token = token.trim();

            if token.is_empty() {
                bail!("Empty component in query");
            }

            // Reject old &mut / & syntax with a helpful message
            if token.starts_with('&') {
                bail!(
                    "use 'mut:ComponentName' syntax, not '&mut ComponentName' or '&ComponentName': '{}'",
                    token
                );
            }

            let (access, name) = if let Some(rest) = token.strip_prefix("mut:") {
                (QueryAccess::Mutable, rest.trim())
            } else {
                (QueryAccess::Immutable, token)
            };

            if name.is_empty() {
                bail!("Component name cannot be empty after 'mut:'");
            }

            // Must be PascalCase (starts with uppercase)
            if !name.starts_with(|c: char| c.is_uppercase()) {
                bail!(
                    "invalid query token '{}': expected 'ComponentName' or 'mut:ComponentName'",
                    token
                );
            }

            validate_pascal_case(name)?;

            Ok(QueryComponent { name: name.to_string(), access })
        })
        .collect()
}
```

- [ ] **Step 1.4: Run inline tests to verify they pass**

```bash
cargo test -p silm codegen::parser::tests 2>&1 | tail -20
```

Expected: all new tests pass

- [ ] **Step 1.5: Migrate `engine/cli/tests/parser_tests.rs`**

Replace all `&mut X` with `mut:X` and all `&X` with `X` in query strings. Keep field parsing tests unchanged (they don't use query syntax). Specifically:

- `parse_query_components("&Health,&Velocity")` → `parse_query_components("Health,Velocity")`
- `parse_query_components("&mut Health,&RegenerationRate")` → `parse_query_components("mut:Health,RegenerationRate")`
- `parse_query_components("&mut Transform,&mut Velocity,&Mass")` → `parse_query_components("mut:Transform,mut:Velocity,Mass")`
- Tests asserting `is_err()` for missing `&` should now assert `is_err()` for lowercase name
- Add test for old `&` syntax being rejected:

```rust
#[test]
fn test_old_ampersand_syntax_rejected() {
    let result = parse_query_components("&Health");
    assert!(result.is_err());
}
```

- [ ] **Step 1.6: Run external parser tests**

```bash
cargo test -p silm --test parser_tests 2>&1 | tail -20
```

Expected: all pass

- [ ] **Step 1.7: Commit**

```bash
git add engine/cli/src/codegen/parser.rs engine/cli/tests/parser_tests.rs
git -c commit.gpgsign=false commit -m "refactor(cli): rewrite parse_query_components to use mut:X syntax"
```

---

### Task 2: Update `generate_component_code` in `component.rs`

**Files:**
- Modify: `engine/cli/src/codegen/component.rs`
- Modify: `engine/cli/tests/codegen/component_tests.rs`

Changes: remove `derive`/`doc` params, fix derives, fix test module name.

- [ ] **Step 2.1: Write the failing codegen test**

In `engine/cli/src/codegen/component.rs` inline tests, add:

```rust
#[test]
fn test_generate_fixed_derives() {
    let fields = vec![("current".to_string(), "f32".to_string())];
    let code = generate_component_code("Health", &fields);
    assert!(code.contains("#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]"));
}

#[test]
fn test_generate_test_module_name() {
    let fields = vec![("current".to_string(), "f32".to_string())];
    let code = generate_component_code("Health", &fields);
    assert!(code.contains("mod health_tests {"));
    assert!(!code.contains("mod tests {"));
}

#[test]
fn test_generate_uses_serde_json() {
    let fields = vec![("current".to_string(), "f32".to_string())];
    let code = generate_component_code("Health", &fields);
    assert!(code.contains("serde_json"));
}
```

- [ ] **Step 2.2: Run to verify they fail**

```bash
cargo test -p silm codegen::component::tests 2>&1 | grep -E "FAILED|test_generate_fixed|test_generate_test|test_generate_uses"
```

Expected: new tests FAIL

- [ ] **Step 2.3: Update `generate_component_code` signature and implementation**

Change the public function signature and `generate_test_module` helper:

```rust
/// Generate complete component code with fixed derives.
///
/// Always derives: Component, Debug, Clone, PartialEq, Serialize, Deserialize
pub fn generate_component_code(
    name: &str,
    fields: &[(String, String)],
) -> String {
    let snake_name = to_snake_case(name);

    // Fixed derive set — always the same, no customization
    let derives_str = "Component, Debug, Clone, PartialEq, Serialize, Deserialize";

    // Generate struct fields
    let mut fields_code = String::new();
    for (field_name, field_type) in fields {
        fields_code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
    }

    let test_module = generate_test_module(name, &snake_name, fields);

    format!(
        "use engine_core::ecs::Component;\nuse serde::{{Deserialize, Serialize}};\n\n#[derive({derives})]\npub struct {name} {{\n{fields}}}\n\n{tests}",
        derives = derives_str,
        name = name,
        fields = fields_code,
        tests = test_module
    )
}
```

Update `generate_test_module` to:
- Use `mod <snake_name>_tests` instead of `mod tests`
- Use `serde_json` instead of `serde_yaml`
- Use concrete field values in add_get test (first field set to a simple value)

```rust
fn generate_test_module(
    name: &str,
    snake_name: &str,
    fields: &[(String, String)],
) -> String {
    // Build field initializer for tests using default values
    let mut field_inits = String::new();
    for (field_name, field_type) in fields {
        let default_val = default_value_for_type(field_type);
        field_inits.push_str(&format!("            {}: {},\n", field_name, default_val));
    }

    format!(
        r#"#[cfg(test)]
mod {snake_name}_tests {{
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_{snake_name}_add_get() {{
        let mut world = World::new();
        let entity = world.spawn();
        let component = {name} {{
{field_inits}        }};
        world.add(entity, component);
        assert!(world.has::<{name}>(entity));
        let retrieved = world.get::<{name}>(entity).unwrap();
        let _ = retrieved;
    }}

    #[test]
    fn test_{snake_name}_serialization() {{
        let component = {name} {{
{field_inits}        }};
        let json = serde_json::to_string(&component).unwrap();
        let _deserialized: {name} = serde_json::from_str(&json).unwrap();
    }}

    #[test]
    fn test_{snake_name}_remove() {{
        let mut world = World::new();
        let entity = world.spawn();
        let component = {name} {{
{field_inits}        }};
        world.add(entity, component);
        assert!(world.has::<{name}>(entity));
        world.remove::<{name}>(entity);
        assert!(!world.has::<{name}>(entity));
    }}
}}
"#,
        snake_name = snake_name,
        name = name,
        field_inits = field_inits,
    )
}
```

- [ ] **Step 2.4: Update `codegen/mod.rs` re-export** — remove `derive`/`doc` from `generate_component_code` call sites (the function signature changed). The re-export itself stays the same.

- [ ] **Step 2.5: Fix old caller in `commands/add.rs`** — temporarily update the call to `generate_component_code(name, &fields)` (removing `derive` and `doc` args). The whole `add.rs` will be deleted later but it must compile now.

In `engine/cli/src/commands/add.rs` line 94, change:
```rust
// OLD:
let code = generate_component_code(name, &fields, derive.clone(), doc.clone());
// NEW:
let code = generate_component_code(name, &fields);
```

- [ ] **Step 2.6: Run inline component tests**

```bash
cargo test -p silm codegen::component::tests 2>&1 | tail -20
```

Expected: all pass

- [ ] **Step 2.7: Migrate `engine/cli/tests/codegen/component_tests.rs`**

Update all calls to `generate_component_code` to remove the `derive` and `doc` arguments. Update test assertions to match new output (no `impl Default`, `mod health_tests` instead of `mod tests`, `serde_json` instead of `serde_yaml`).

- [ ] **Step 2.8: Run external component tests**

```bash
cargo test -p silm --test component_tests 2>&1 | tail -20
```

Expected: all pass

- [ ] **Step 2.9: Commit**

```bash
git add engine/cli/src/codegen/component.rs engine/cli/src/commands/add.rs engine/cli/tests/codegen/component_tests.rs
git -c commit.gpgsign=false commit -m "refactor(cli): update generate_component_code to fixed derives and domain-scoped test modules"
```

---

### Task 3: Update `generate_system_code` in `system.rs`

**Files:**
- Modify: `engine/cli/src/codegen/system.rs`
- Modify: `engine/cli/src/codegen/mod.rs`
- Modify: `engine/cli/tests/codegen/system_tests.rs`
- Modify: `engine/cli/tests/system_integration.rs`

Changes: remove `SystemPhase`/`doc`, append `_system` suffix, `dt` param, no crate imports, simpler iteration, domain-scoped test module names.

- [ ] **Step 3.1: Write failing inline tests**

Add to the inline `mod tests` in `system.rs`:

```rust
#[test]
fn test_function_name_has_system_suffix() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(code.contains("pub fn health_regen_system("));
    assert!(!code.contains("pub fn health_regen("));
}

#[test]
fn test_parameter_name_is_dt() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(code.contains("dt: f32"));
    assert!(!code.contains("delta_time"));
}

#[test]
fn test_no_crate_components_import() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(!code.contains("use crate::components"));
}

#[test]
fn test_direct_query_iteration() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(code.contains("for (health) in world.query::<(&mut Health,)>()"));
}

#[test]
fn test_test_module_name_has_system_suffix() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(code.contains("mod health_regen_system_tests {"));
}

#[test]
fn test_registration_comment() {
    let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
    let code = generate_system_code("health_regen", &components);
    assert!(code.contains("// To register: app.add_system(health_regen_system)"));
}
```

- [ ] **Step 3.2: Run to verify they fail**

```bash
cargo test -p silm codegen::system::tests 2>&1 | grep -E "FAILED|test_function_name|test_parameter|test_no_crate|test_direct|test_test_module|test_registration"
```

Expected: new tests FAIL

- [ ] **Step 3.3: Rewrite `generate_system_code`**

Replace the entire function and remove `SystemPhase`. New signature:

```rust
/// Generate complete system code.
///
/// Function name = `{name}_system`. Uses `dt: f32` parameter.
/// No crate-level component imports (same-domain components are in scope).
pub fn generate_system_code(name: &str, components: &[QueryComponent]) -> String {
    let fn_name = format!("{}_system", name);
    let test_mod_name = format!("{}_system_tests", name);

    // Build query type string: (&mut Health, &RegenerationRate,)
    let query_types: Vec<String> = components.iter().map(|c| c.type_syntax()).collect();
    let query_tuple = if query_types.len() == 1 {
        format!("({},)", query_types[0])
    } else {
        format!("({})", query_types.join(", "))
    };

    // Build iteration variable binding
    let var_names: Vec<String> = components.iter().map(|c| c.var_name()).collect();
    let iter_binding = if var_names.len() == 1 {
        format!("({},)", var_names[0])
    } else {
        format!("({})", var_names.join(", "))
    };

    format!(
        r#"use engine_core::ecs::World;

// To register: app.add_system({fn_name});
#[tracing::instrument(skip(world))]
pub fn {fn_name}(world: &mut World, dt: f32) {{
    for {iter_binding} in world.query::<{query_tuple}>() {{
        // TODO: implement {name} logic
        let _ = dt;
    }}
}}

#[cfg(test)]
mod {test_mod_name} {{
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_{fn_name}() {{
        let mut world = World::new();
        // TODO: spawn test entity, run system, assert
        {fn_name}(&mut world, 0.016);
    }}
}}
"#,
        fn_name = fn_name,
        name = name,
        query_tuple = query_tuple,
        iter_binding = iter_binding,
        test_mod_name = test_mod_name,
    )
}
```

Also delete the `SystemPhase` enum and its `impl SystemPhase`.

- [ ] **Step 3.4: Update `codegen/mod.rs`** — remove `SystemPhase` from re-exports:

```rust
// Remove this line:
pub use system::{generate_system_code, SystemPhase};
// Replace with:
pub use system::generate_system_code;
```

- [ ] **Step 3.5: Fix old caller in `commands/add.rs`** — temporarily update `generate_system_code` call:

In `engine/cli/src/commands/add.rs`, the `add_system` function currently uses `SystemPhase` and passes `phase` and `doc`. Update:

```rust
// Remove phase parsing entirely
// Change generate_system_code call from:
//   generate_system_code(name, &components, phase, doc.clone())
// To:
let code = generate_system_code(name, &components);
```

Also remove the `SystemPhase` import from `add.rs`.

- [ ] **Step 3.6: Run inline system tests**

```bash
cargo test -p silm codegen::system::tests 2>&1 | tail -20
```

Expected: all pass

- [ ] **Step 3.7: Migrate `engine/cli/tests/codegen/system_tests.rs`**

- Remove all `SystemPhase` references
- Remove `doc` arguments from `generate_system_code` calls
- Update query strings from `&mut X` to `mut:X` (goes through parser now)
- Update test assertions for new function output format (new fn name suffix, no crate imports, etc.)

- [ ] **Step 3.8: Migrate `engine/cli/tests/system_integration.rs`**

Update all query strings from `&mut X` / `&X` syntax to `mut:X` / bare `X`.

- [ ] **Step 3.9: Verify all codegen tests pass**

```bash
cargo test -p silm 2>&1 | grep -E "test result|FAILED" | head -20
```

Expected: all tests pass

- [ ] **Step 3.10: Commit**

```bash
git add engine/cli/src/codegen/ engine/cli/tests/codegen/ engine/cli/tests/system_integration.rs engine/cli/src/commands/add.rs
git -c commit.gpgsign=false commit -m "refactor(cli): update generate_system_code — _system suffix, dt param, domain-local imports"
```

---

## Chunk 2: Wiring Logic + New Add Command

### Task 4: Create `wiring.rs`

**Files:**
- Create: `engine/cli/src/commands/add/wiring.rs`

This module handles all filesystem operations: finding the project root, atomic writes, duplicate detection, and idempotent module wiring.

- [ ] **Step 4.1: Create the directory**

```bash
mkdir -p engine/cli/src/commands/add
```

- [ ] **Step 4.2: Write failing tests first**

Create `engine/cli/src/commands/add/wiring.rs` with tests first:

```rust
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Which crate to target
#[derive(Debug, Clone, Copy)]
pub enum Target {
    Shared,
    Server,
    Client,
}

impl Target {
    /// Subdirectory name relative to project root
    pub fn crate_subdir(&self) -> &'static str {
        match self {
            Target::Shared => "shared",
            Target::Server => "server",
            Target::Client => "client",
        }
    }

    /// Entry point file within src/ (lib.rs for shared, main.rs for server/client)
    pub fn entry_file(&self) -> &'static str {
        match self {
            Target::Shared => "lib.rs",
            Target::Server | Target::Client => "main.rs",
        }
    }
}

/// Walk up from `start` to find `game.toml`. Returns the directory containing it.
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("game.toml").exists() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("no game.toml found — run this command from inside a silmaril project");
        }
    }
}

/// Resolve the crate directory, error if it doesn't exist.
pub fn crate_dir(project_root: &Path, target: Target) -> Result<PathBuf> {
    let dir = project_root.join(target.crate_subdir());
    if !dir.is_dir() {
        bail!(
            "target crate '{}/' not found — is this project set up correctly?",
            target.crate_subdir()
        );
    }
    Ok(dir)
}

/// Resolve domain module file path: `<crate>/src/<domain>/mod.rs`
pub fn domain_file(crate_root: &Path, domain: &str) -> PathBuf {
    crate_root.join("src").join(domain).join("mod.rs")
}

/// Resolve wiring target: `<crate>/src/lib.rs` or `<crate>/src/main.rs`
pub fn wiring_target(crate_root: &Path, target: Target) -> PathBuf {
    crate_root.join("src").join(target.entry_file())
}

/// Check if `pub struct <Name>` (followed by `{` with optional whitespace) exists in file.
pub fn has_duplicate_component(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub struct {}", name);
    Ok(content.lines().any(|line| {
        if let Some(rest) = line.trim_start().strip_prefix(&pattern) {
            rest.trim_start().starts_with('{')
        } else {
            false
        }
    }))
}

/// Check if `pub fn <name>_system(` exists in file.
pub fn has_duplicate_system(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub fn {}_system(", name);
    Ok(content.contains(&pattern))
}

/// Write `content` to `path` atomically (temp file → rename).
/// Creates parent directories if needed.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Append `content` to domain file atomically.
/// If file doesn't exist, creates it. Reads original into memory for rollback.
/// Returns the original content (None if file was new) for rollback on wiring failure.
pub fn append_to_domain_file(file: &Path, content: &str) -> Result<Option<String>> {
    let original = if file.exists() {
        Some(fs::read_to_string(file)?)
    } else {
        None
    };

    let new_content = match &original {
        Some(existing) => format!("{}\n{}", existing, content),
        None => content.to_string(),
    };

    atomic_write(file, &new_content)?;
    Ok(original)
}

/// Add `pub mod <domain>;` to the wiring target file if not already present.
/// Uses atomic write. Returns the original content for rollback.
pub fn wire_module_declaration(target_file: &Path, domain: &str) -> Result<String> {
    let original = if target_file.exists() {
        fs::read_to_string(target_file)?
    } else {
        String::new()
    };

    let declaration = format!("pub mod {};", domain);
    if original.contains(&declaration) {
        return Ok(original); // already wired, nothing to do
    }

    let new_content = format!("{}\n{}\n", original.trim_end(), declaration);
    atomic_write(target_file, &new_content)?;
    Ok(original)
}

/// Rollback domain file: restore original content, or delete if it was newly created.
pub fn rollback_domain_file(file: &Path, original: Option<String>) -> Result<()> {
    match original {
        Some(content) => atomic_write(file, &content),
        None => {
            if file.exists() {
                fs::remove_file(file)?;
            }
            Ok(())
        }
    }
}

/// Rollback wiring target to original content.
pub fn rollback_wiring_target(file: &Path, original: &str) -> Result<()> {
    atomic_write(file, original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_project(tmp: &TempDir) -> PathBuf {
        let root = tmp.path().to_path_buf();
        fs::write(root.join("game.toml"), "[game]\nname = \"test\"").unwrap();
        fs::create_dir_all(root.join("shared/src")).unwrap();
        fs::write(root.join("shared/src/lib.rs"), "").unwrap();
        root
    }

    #[test]
    fn test_find_project_root_from_same_dir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let found = find_project_root(&root).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_from_subdir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let subdir = root.join("shared/src/health");
        fs::create_dir_all(&subdir).unwrap();
        let found = find_project_root(&subdir).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_not_found() {
        let tmp = TempDir::new().unwrap();
        // No game.toml
        let result = find_project_root(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no game.toml found"));
    }

    #[test]
    fn test_crate_dir_ok() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let dir = crate_dir(&root, Target::Shared).unwrap();
        assert_eq!(dir, root.join("shared"));
    }

    #[test]
    fn test_crate_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let result = crate_dir(&root, Target::Server);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("server/"));
    }

    #[test]
    fn test_has_duplicate_component_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub struct Health {\n    pub current: f32,\n}\n").unwrap();
        assert!(has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_component_not_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub struct Damage {\n    pub amount: f32,\n}\n").unwrap();
        assert!(!has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_component_no_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("nonexistent.rs");
        assert!(!has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_system_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub fn health_regen_system(world: &mut World, dt: f32) {\n}\n").unwrap();
        assert!(has_duplicate_system(&file, "health_regen").unwrap());
    }

    #[test]
    fn test_has_duplicate_system_not_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub fn other_system(world: &mut World, dt: f32) {\n}\n").unwrap();
        assert!(!has_duplicate_system(&file, "health_regen").unwrap());
    }

    #[test]
    fn test_wire_module_declaration_adds() {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path().join("lib.rs");
        fs::write(&lib, "// empty\n").unwrap();
        wire_module_declaration(&lib, "health").unwrap();
        let content = fs::read_to_string(&lib).unwrap();
        assert!(content.contains("pub mod health;"));
    }

    #[test]
    fn test_wire_module_declaration_idempotent() {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path().join("lib.rs");
        fs::write(&lib, "pub mod health;\n").unwrap();
        wire_module_declaration(&lib, "health").unwrap();
        let content = fs::read_to_string(&lib).unwrap();
        // Should appear exactly once
        assert_eq!(content.matches("pub mod health;").count(), 1);
    }

    #[test]
    fn test_append_to_domain_file_new() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("health").join("mod.rs");
        let original = append_to_domain_file(&file, "// new content\n").unwrap();
        assert!(original.is_none()); // was new
        assert_eq!(fs::read_to_string(&file).unwrap(), "// new content\n");
    }

    #[test]
    fn test_append_to_domain_file_existing() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "// existing\n").unwrap();
        let original = append_to_domain_file(&file, "// appended\n").unwrap();
        assert_eq!(original.as_deref(), Some("// existing\n"));
        let content = fs::read_to_string(&file).unwrap();
        assert!(content.contains("// existing\n"));
        assert!(content.contains("// appended\n"));
    }

    #[test]
    fn test_rollback_domain_file_new() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("health").join("mod.rs");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "// something\n").unwrap();
        rollback_domain_file(&file, None).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn test_rollback_domain_file_existing() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "// original\n").unwrap();
        rollback_domain_file(&file, Some("// original\n".to_string())).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "// original\n");
    }
}
```

- [ ] **Step 4.3: Run wiring tests to verify they fail**

These tests won't even compile yet since the file is new:

```bash
cargo test -p silm 2>&1 | head -20
```

Expected: compile error (wiring module not declared yet — we'll fix in Task 7)

- [ ] **Step 4.4: Commit (tests only for now)**

```bash
git add engine/cli/src/commands/add/wiring.rs
git -c commit.gpgsign=false commit -m "feat(cli): add wiring.rs with atomic writes and module declaration logic"
```

---

### Task 5: Create `add/component.rs`

**Files:**
- Create: `engine/cli/src/commands/add/component.rs`

- [ ] **Step 5.1: Create `component.rs`**

```rust
use anyhow::{bail, Result};
use std::env;

use crate::codegen::{generate_component_code, parse_fields, to_snake_case, validate_pascal_case};

use super::wiring::{
    append_to_domain_file, crate_dir, domain_file, find_project_root, has_duplicate_component,
    rollback_domain_file, rollback_wiring_target, wire_module_declaration, wiring_target, Target,
};

pub fn add_component(
    name: &str,
    fields_str: &str,
    target: Target,
    domain: &str,
) -> Result<()> {
    // Validate inputs
    validate_pascal_case(name)?;

    let fields = parse_fields(fields_str)?;
    if fields.is_empty() {
        bail!("Component must have at least one field");
    }

    // Find project root and resolve paths
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;
    let crate_root = crate_dir(&project_root, target)?;
    let domain_mod = domain_file(&crate_root, domain);
    let wiring = wiring_target(&crate_root, target);

    // Check for duplicate before writing
    if has_duplicate_component(&domain_mod, name)? {
        bail!(
            "component '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    // Generate code
    let snake_name = to_snake_case(name);
    let code = generate_component_code(name, &fields);

    // Step 1: Append to domain file (atomic)
    let original_domain = append_to_domain_file(&domain_mod, &code)?;

    // Step 2: Wire module declaration (atomic) — rollback domain if this fails
    let wiring_original = match wire_module_declaration(&wiring, domain) {
        Ok(orig) => orig,
        Err(e) => {
            rollback_domain_file(&domain_mod, original_domain)?;
            return Err(e);
        }
    };
    let _ = wiring_original;

    // Success output
    tracing::info!(
        "[silm] {} {}/src/{}/mod.rs",
        if original_domain.is_none() { "created" } else { "updated" },
        target.crate_subdir(),
        domain
    );
    tracing::info!(
        "[silm] wired: added `pub mod {};` to {}/src/{}",
        domain,
        target.crate_subdir(),
        target.entry_file()
    );

    Ok(())
}
```

- [ ] **Step 5.2: Commit**

```bash
git add engine/cli/src/commands/add/component.rs
git -c commit.gpgsign=false commit -m "feat(cli): add component.rs — add_component orchestrator with atomic wiring"
```

---

### Task 6: Create `add/system.rs`

**Files:**
- Create: `engine/cli/src/commands/add/system.rs`

- [ ] **Step 6.1: Create `system.rs`**

```rust
use anyhow::{bail, Result};
use std::env;

use crate::codegen::{generate_system_code, parse_query_components, validate_snake_case};

use super::wiring::{
    append_to_domain_file, crate_dir, domain_file, find_project_root, has_duplicate_system,
    rollback_domain_file, wire_module_declaration, wiring_target, Target,
};

pub fn add_system(
    name: &str,
    query_str: &str,
    target: Target,
    domain: &str,
) -> Result<()> {
    // Validate inputs
    validate_snake_case(name)?;

    let components = parse_query_components(query_str)?;
    if components.is_empty() {
        bail!("Query must have at least one component");
    }

    // Find project root and resolve paths
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;
    let crate_root = crate_dir(&project_root, target)?;
    let domain_mod = domain_file(&crate_root, domain);
    let wiring = wiring_target(&crate_root, target);

    // Check for duplicate before writing
    if has_duplicate_system(&domain_mod, name)? {
        bail!(
            "system '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    // Generate code
    let code = generate_system_code(name, &components);

    // Step 1: Append to domain file (atomic)
    let original_domain = append_to_domain_file(&domain_mod, &code)?;

    // Step 2: Wire module declaration (atomic) — rollback domain if this fails
    match wire_module_declaration(&wiring, domain) {
        Ok(_) => {}
        Err(e) => {
            rollback_domain_file(&domain_mod, original_domain)?;
            return Err(e);
        }
    }

    // Success output
    tracing::info!(
        "[silm] {} {}/src/{}/mod.rs",
        if original_domain.is_none() { "created" } else { "updated" },
        target.crate_subdir(),
        domain
    );
    tracing::info!(
        "[silm] wired: added `pub mod {};` to {}/src/{}",
        domain,
        target.crate_subdir(),
        target.entry_file()
    );

    Ok(())
}
```

- [ ] **Step 6.2: Commit**

```bash
git add engine/cli/src/commands/add/system.rs
git -c commit.gpgsign=false commit -m "feat(cli): add system.rs — add_system orchestrator with atomic wiring"
```

---

### Task 7: Create `add/mod.rs`, delete old `add.rs`, wire up

**Files:**
- Create: `engine/cli/src/commands/add/mod.rs`
- Delete: `engine/cli/src/commands/add.rs`
- Modify: `engine/cli/src/main.rs`

- [ ] **Step 7.1: Create `add/mod.rs`**

```rust
use anyhow::{bail, Result};
use clap::Subcommand;

mod component;
mod system;
pub mod wiring;

use wiring::Target;

#[derive(Subcommand)]
pub enum AddCommand {
    /// Add a new ECS component to a domain slice
    Component {
        /// Component name in PascalCase (e.g., Health, PlayerState)
        name: String,

        /// Component fields (e.g., "current:f32,max:f32")
        #[arg(short, long)]
        fields: String,

        /// Domain name in snake_case (e.g., health, combat)
        #[arg(short, long)]
        domain: String,

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

    /// Add a new ECS system to a domain slice
    System {
        /// System name in snake_case (e.g., health_regen, movement)
        name: String,

        /// Query components (e.g., "mut:Health,RegenerationRate")
        #[arg(short, long)]
        query: String,

        /// Domain name in snake_case (e.g., health, combat)
        #[arg(short, long)]
        domain: String,

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
}

fn resolve_target(shared: bool, server: bool, client: bool) -> Result<Target> {
    match (shared, server, client) {
        (true, false, false) => Ok(Target::Shared),
        (false, true, false) => Ok(Target::Server),
        (false, false, true) => Ok(Target::Client),
        _ => bail!("must specify exactly one of --shared, --server, or --client"),
    }
}

pub fn handle_add_command(command: AddCommand) -> Result<()> {
    match command {
        AddCommand::Component { name, fields, domain, shared, server, client } => {
            let target = resolve_target(shared, server, client)?;
            component::add_component(&name, &fields, target, &domain)
        }
        AddCommand::System { name, query, domain, shared, server, client } => {
            let target = resolve_target(shared, server, client)?;
            system::add_system(&name, &query, target, &domain)
        }
    }
}
```

- [ ] **Step 7.2: Delete the old `add.rs`**

```bash
rm engine/cli/src/commands/add.rs
```

- [ ] **Step 7.3: Verify `commands/mod.rs` still has `pub mod add;`**

The file already has `pub mod add;` — with `add.rs` deleted and `add/mod.rs` present, Rust will find the module directory. No change needed to `commands/mod.rs`.

- [ ] **Step 7.4: Verify `main.rs` still compiles** — `main.rs` imports `commands::add::AddCommand` and `commands::add::handle_add_command` which still exist in `add/mod.rs`. No changes needed.

- [ ] **Step 7.5: Build to verify everything compiles**

```bash
cargo build -p silm 2>&1 | grep -E "^error" | head -20
```

Expected: clean build

- [ ] **Step 7.6: Run all tests**

```bash
cargo test -p silm 2>&1 | grep -E "test result|FAILED" | head -20
```

Expected: all pass

- [ ] **Step 7.7: Commit**

```bash
git add engine/cli/src/commands/add/ engine/cli/src/commands/
git -c commit.gpgsign=false commit -m "feat(cli): replace add.rs with add/ module — new --shared/--server/--client flags and --domain"
```

---

## Chunk 3: Template Update + Integration Tests

### Task 8: Update `BasicTemplate` to remove horizontal stubs

**Files:**
- Modify: `engine/cli/src/templates/basic.rs`

- [ ] **Step 8.1: Remove stub file generation**

In `basic.rs`, remove the `self.shared_components_rs()` and `self.shared_systems_rs()` calls from the `files()` vec, and delete the corresponding methods (`shared_components_rs` and `shared_systems_rs`).

The updated `files()` vec starts (shared section):

```rust
// Shared crate
self.shared_cargo_toml(),
self.shared_lib_rs(),
// (no shared_components_rs or shared_systems_rs)
```

- [ ] **Step 8.2: Update `shared_lib_rs()` to generate a clean lib.rs**

Find the `shared_lib_rs()` method and update its content. Replace whatever it currently generates with:

```rust
fn shared_lib_rs(&self) -> TemplateFile {
    let content = String::from(
        r#"//! Shared game logic — components, systems, and types used by both server and client.
//!
//! Add new domains with: silm add component <Name> --shared --domain <domain>
"#,
    );
    TemplateFile::new("shared/src/lib.rs", content)
}
```

- [ ] **Step 8.3: Build to verify**

```bash
cargo build -p silm 2>&1 | grep "^error"
```

Expected: clean build

- [ ] **Step 8.4: Verify `silm new` still creates a valid project**

```bash
cargo run -p silm -- new test-remove-me 2>&1 | tail -5
ls test-remove-me/shared/src/
rm -rf test-remove-me
```

Expected: only `lib.rs` in `shared/src/` (no `components.rs`, `systems.rs`)

- [ ] **Step 8.5: Commit**

```bash
git add engine/cli/src/templates/basic.rs
git -c commit.gpgsign=false commit -m "refactor(cli): remove horizontal component/system stubs from BasicTemplate"
```

---

### Task 9: Integration Tests

**Files:**
- Create: `engine/cli/tests/add_integration.rs`

These tests run the full `add_component` and `add_system` functions against a real temp filesystem.

- [ ] **Step 9.1: Write integration tests**

Create `engine/cli/tests/add_integration.rs`:

```rust
//! Integration tests for `silm add component` and `silm add system`.
//!
//! Each test creates a minimal project in a temp dir, runs the add function,
//! and asserts the generated file content and wiring.

use silm::commands::add::wiring::Target;
use std::fs;
use tempfile::TempDir;

fn make_project(tmp: &TempDir) -> std::path::PathBuf {
    let root = tmp.path().to_path_buf();
    fs::write(root.join("game.toml"), "[game]\nname=\"test\"\n[dev]\nserver_package=\"test-server\"\nclient_package=\"test-client\"\nserver_port=7777\ndev_server_port=9999\ndev_client_port=9998\n").unwrap();
    fs::create_dir_all(root.join("shared/src")).unwrap();
    fs::write(root.join("shared/src/lib.rs"), "// shared lib\n").unwrap();
    fs::create_dir_all(root.join("server/src")).unwrap();
    fs::write(root.join("server/src/main.rs"), "fn main() {}\n").unwrap();
    fs::create_dir_all(root.join("client/src")).unwrap();
    fs::write(root.join("client/src/main.rs"), "fn main() {}\n").unwrap();
    root
}

#[test]
fn test_add_component_creates_domain_file() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32,max:f32",
        Target::Shared,
        "health",
    ).unwrap();

    let domain_file = root.join("shared/src/health/mod.rs");
    assert!(domain_file.exists(), "domain file should be created");
    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub current: f32,"));
    assert!(content.contains("pub max: f32,"));
    assert!(content.contains("Component, Debug, Clone, PartialEq, Serialize, Deserialize"));
    assert!(content.contains("mod health_tests {"));
}

#[test]
fn test_add_component_wires_lib_rs() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32",
        Target::Shared,
        "health",
    ).unwrap();

    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod health;"));
}

#[test]
fn test_add_component_wires_main_rs_for_server() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Damage",
        "amount:f32",
        Target::Server,
        "combat",
    ).unwrap();

    let main = fs::read_to_string(root.join("server/src/main.rs")).unwrap();
    assert!(main.contains("pub mod combat;"));
}

#[test]
fn test_add_component_duplicate_rejected() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component("Health", "current:f32", Target::Shared, "health").unwrap();
    let result = silm::commands::add::component::add_component("Health", "max:f32", Target::Shared, "health");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_add_two_components_same_domain() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component("Health", "current:f32,max:f32", Target::Shared, "health").unwrap();
    silm::commands::add::component::add_component("MaxHealth", "value:f32", Target::Shared, "health").unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub struct MaxHealth {"));

    // lib.rs wired only once
    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod health;").count(), 1);
}

#[test]
fn test_add_system_creates_domain_file() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health,RegenerationRate",
        Target::Shared,
        "health",
    ).unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub fn health_regen_system("));
    assert!(content.contains("dt: f32"));
    assert!(content.contains("mod health_regen_system_tests {"));
    assert!(content.contains("// To register: app.add_system(health_regen_system)"));
}

#[test]
fn test_add_component_then_system_same_domain() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component("Health", "current:f32,max:f32", Target::Shared, "health").unwrap();
    silm::commands::add::system::add_system("health_regen", "mut:Health,RegenerationRate", Target::Shared, "health").unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub fn health_regen_system("));

    // lib.rs wired exactly once
    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod health;").count(), 1);
}

#[test]
fn test_add_system_duplicate_rejected() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::system::add_system("health_regen", "mut:Health", Target::Shared, "health").unwrap();
    let result = silm::commands::add::system::add_system("health_regen", "mut:Health", Target::Shared, "health");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_missing_crate_dir_errors_clearly() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    // No client/ directory in our make_project... actually we do have it. Let's test server with no dir.
    fs::remove_dir_all(root.join("server")).unwrap();
    let result = silm::commands::add::component::add_component("Health", "hp:f32", Target::Server, "health");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("server/"));
}
```

- [ ] **Step 9.2: Expose needed modules in `lib.rs`**

The integration tests import via `silm::commands::add::component::add_component` etc. Ensure `engine/cli/src/lib.rs` exposes:

```rust
pub mod codegen;
pub mod commands;
pub mod templates;
```

Check current `lib.rs` — if it already exists and exports these, no change needed. If not, add them.

- [ ] **Step 9.3: Run integration tests**

```bash
cargo test -p silm --test add_integration 2>&1 | tail -30
```

Expected: all pass

- [ ] **Step 9.4: Run full test suite**

```bash
cargo test -p silm 2>&1 | grep -E "test result|FAILED"
```

Expected: all tests pass, 0 failures

- [ ] **Step 9.5: Quick smoke test of CLI**

```bash
cd /tmp && rm -rf smoke-test-game
cargo run -p silm -- new smoke-test-game
cd smoke-test-game
cargo run -p silm -- add component Health --shared --domain health --fields "current:f32,max:f32"
cat shared/src/health/mod.rs
cat shared/src/lib.rs
cargo run -p silm -- add system health_regen --shared --domain health --query "mut:Health,RegenerationRate"
cat shared/src/health/mod.rs
cd /tmp && rm -rf smoke-test-game
```

Expected: `mod.rs` contains struct + system, `lib.rs` contains `pub mod health;` exactly once

- [ ] **Step 9.6: Commit**

```bash
git add engine/cli/tests/add_integration.rs engine/cli/src/lib.rs
git -c commit.gpgsign=false commit -m "test(cli): add integration tests for silm add component/system"
```

---

## Notes for Implementer

### Key invariants
- `add_component` and `add_system` are public — expose via `pub use` from `add/mod.rs` or keep as `pub fn` in their submodules with `pub(crate)` visibility as needed for tests
- The wiring module (`wiring.rs`) is `pub mod` in `add/mod.rs` so integration tests can import `Target`
- `tempfile` crate is already in `engine/cli/dev-dependencies` — check `Cargo.toml` and add if missing

### Cargo.toml check
Ensure `engine/cli/Cargo.toml` has `tempfile` in `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
```

### Current test files NOT needing migration
- `component_integration.rs` — uses `add_component` function directly, no query syntax
- `module_exports_integration.rs`, `registry_integration.rs`, `validator_tests.rs` — unrelated to this work
- `dev_*_test.rs` — unrelated to this work
