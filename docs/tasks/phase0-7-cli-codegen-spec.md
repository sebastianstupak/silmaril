# Phase 0.7 CLI Code Generation - Detailed Specification

> **Complete implementation specification for `silm add component` and `silm add system`**
>
> Status: Ready for Implementation
> Priority: 🔴 CRITICAL
> Estimated Time: 12-17 days

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Component Generation](#component-generation)
4. [System Generation](#system-generation)
5. [Component Registry](#component-registry)
6. [Module Export Management](#module-export-management)
7. [Interactive Mode](#interactive-mode)
8. [Testing Strategy](#testing-strategy)
9. [Benchmarking](#benchmarking)
10. [Implementation Milestones](#implementation-milestones)

---

## Overview

### Goals

Enable developers to generate components and systems via CLI:

```bash
# Component generation
silm add component Health --fields "current:f32,max:f32" --location shared

# System generation
silm add system health_regen --query "&mut Health,&RegenerationRate" --location shared

# Interactive mode
silm add component Health --interactive
```

### Success Criteria

- ✅ Generated code compiles without errors
- ✅ Generated tests pass
- ✅ Module exports auto-update
- ✅ Component registry updates
- ✅ Template validation uses registry
- ✅ Performance: < 100ms per generation
- ✅ Test coverage: > 85%

---

## Architecture

### Project Structure

```
my-game/
├── .silmaril/
│   └── components.json       # Component/system registry
├── shared/                   # Shared code (client + server)
│   └── src/
│       ├── components/
│       │   ├── mod.rs        # Auto-updated exports
│       │   └── health.rs     # Generated component
│       └── systems/
│           ├── mod.rs        # Auto-updated exports
│           └── health_regen.rs  # Generated system
├── server/                   # Server-only code
│   └── src/
│       ├── components/
│       └── systems/
└── client/                   # Client-only code
    └── src/
        ├── components/
        └── systems/
```

### CLI Command Structure

```rust
// engine/cli/src/main.rs
#[derive(Subcommand)]
enum Commands {
    New { /* ... */ },
    Template { /* ... */ },
    Add {                          // NEW
        #[command(subcommand)]
        command: commands::add::AddCommand,
    },
}
```

### Module Organization

```
engine/cli/src/
├── main.rs                    # CLI entry point
├── commands/
│   ├── mod.rs
│   ├── new.rs
│   ├── template.rs
│   └── add.rs                 # NEW: Code generation
├── codegen/                   # NEW: Code generation logic
│   ├── mod.rs
│   ├── component.rs           # Component code generator
│   ├── system.rs              # System code generator
│   ├── parser.rs              # Field/query parser
│   ├── validator.rs           # Name validation
│   └── registry.rs            # Component registry
└── templates/
    └── basic.rs
```

---

## Component Generation

### Command Specification

```bash
silm add component <NAME> [OPTIONS]

ARGUMENTS:
  <NAME>  Component name in PascalCase (e.g., "Health", "Inventory")

OPTIONS:
  -f, --fields <FIELDS>       Component fields (e.g., "current:f32,max:f32")
  -l, --location <LOCATION>   Location: shared, client, server [default: shared]
  -d, --derive <DERIVES>      Additional derives (e.g., "Default,PartialEq")
  --doc <DOC>                 Documentation string
  -i, --interactive           Interactive mode (prompts for fields)
  -h, --help                  Print help
```

### Examples

```bash
# Basic component
silm add component Health --fields "current:f32,max:f32"

# With location
silm add component CameraState --fields "fov:f32,zoom:f32" --location client

# With additional derives
silm add component Transform \
  --fields "position:[f32;3],rotation:[f32;4],scale:[f32;3]" \
  --derive "Default,PartialEq,Copy"

# With documentation
silm add component Health \
  --fields "current:f32,max:f32" \
  --doc "Player health with current and maximum values"

# Interactive mode
silm add component Health --interactive
```

### Generated Code Template

```rust
use engine_core::ecs::Component;
use serde::{Deserialize, Serialize};

/// {doc_string}
#[derive({derives}, Component, Serialize, Deserialize)]
pub struct {name} {
    /// TODO: Document this field
    pub {field_name}: {field_type},
    // ... more fields
}

impl Default for {name} {
    fn default() -> Self {
        Self {
            {field_name}: {default_value},
            // ...
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_{snake_name}_add_get() {
        let mut world = World::new();
        let entity = world.spawn();

        let component = {name}::default();
        world.add(entity, component.clone());

        let retrieved = world.get::<{name}>(entity).unwrap();
        assert!(world.has::<{name}>(entity));
    }

    #[test]
    fn test_{snake_name}_serialization() {
        let component = {name}::default();

        let yaml = serde_yaml::to_string(&component).unwrap();
        let deserialized: {name} = serde_yaml::from_str(&yaml).unwrap();

        // Field-specific assertions
    }
}
```

### Field Parsing

**Format:** `name:type[,name:type]*`

**Examples:**
- `"current:f32,max:f32"` → `[(current, f32), (max, f32)]`
- `"position:[f32;3]"` → `[(position, [f32; 3])]`
- `"items:Vec<Item>,capacity:usize"` → `[(items, Vec<Item>), (capacity, usize)]`

**Parser Implementation:**

```rust
pub fn parse_fields(input: &str) -> Result<Vec<(String, String)>> {
    input
        .split(',')
        .map(|field| {
            let parts: Vec<&str> = field.trim().split(':').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid field format: '{}'. Expected 'name:type'", field);
            }
            let name = parts[0].trim().to_string();
            let type_str = parts[1].trim().to_string();

            validate_field_name(&name)?;
            validate_type_syntax(&type_str)?;

            Ok((name, type_str))
        })
        .collect()
}
```

### Name Validation

**Component Names (PascalCase):**
- Must start with uppercase letter
- Only alphanumeric characters
- Examples: `Health`, `PlayerState`, `MeshRenderer`

**Field Names (snake_case):**
- Must start with lowercase letter or underscore
- Only alphanumeric and underscores
- Examples: `current`, `max_health`, `_internal_id`

```rust
pub fn validate_pascal_case(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Component name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_uppercase() {
        anyhow::bail!("Component name must start with uppercase: '{}'", name);
    }

    if !name.chars().all(|c| c.is_alphanumeric()) {
        anyhow::bail!("Component name must be alphanumeric: '{}'", name);
    }

    Ok(())
}

pub fn validate_snake_case(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Field name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_lowercase() && first_char != '_' {
        anyhow::bail!("Field name must start with lowercase or underscore: '{}'", name);
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Field name must be alphanumeric or underscore: '{}'", name);
    }

    Ok(())
}
```

### Default Value Generation

```rust
pub fn default_value_for_type(type_str: &str) -> String {
    match type_str {
        "f32" | "f64" => "0.0".to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" |
        "u8" | "u16" | "u32" | "u64" | "u128" |
        "isize" | "usize" => "0".to_string(),
        "bool" => "false".to_string(),
        "String" => "String::new()".to_string(),
        s if s.starts_with("Vec<") => "Vec::new()".to_string(),
        s if s.starts_with("Option<") => "None".to_string(),
        s if s.starts_with("[") && s.ends_with("]") => {
            // Array type: [f32; 3] -> [0.0; 3]
            if let Some(inner) = extract_array_type(s) {
                format!("[{}; {}]", default_value_for_type(&inner.0), inner.1)
            } else {
                "Default::default()".to_string()
            }
        }
        _ => "Default::default()".to_string(),
    }
}
```

---

## System Generation

### Command Specification

```bash
silm add system <NAME> [OPTIONS]

ARGUMENTS:
  <NAME>  System name in snake_case (e.g., "health_regen", "movement")

OPTIONS:
  -q, --query <QUERY>         Query components (e.g., "&mut Health,&RegenerationRate")
  -l, --location <LOCATION>   Location: shared, client, server [default: shared]
  -p, --phase <PHASE>         System phase: update, fixed_update, render [default: update]
  --doc <DOC>                 Documentation string
  -i, --interactive           Interactive mode
  -h, --help                  Print help
```

### Examples

```bash
# Basic system
silm add system health_regen --query "&mut Health,&RegenerationRate"

# With phase
silm add system physics_step --query "&mut Transform,&Velocity" --phase fixed_update

# Client-only rendering system
silm add system camera_update --query "&mut Camera,&Transform" --location client --phase render

# Interactive mode
silm add system health_regen --interactive
```

### Generated Code Template

```rust
use engine_core::ecs::{Query, World};
use tracing::{debug, instrument};

use crate::components::{ComponentA, ComponentB};

/// {doc_string}
///
/// # Phase
/// {phase}
///
/// # Query
/// - {component_list}
#[instrument(skip(world))]
pub fn {name}(world: &mut World, delta_time: f32) {
    let query = world.query::<({query_types})>();

    for (entity, ({query_vars})) in query.iter() {
        // TODO: Implement system logic

        debug!(?entity, "Processing entity");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_{name}_basic() {
        let mut world = World::new();

        // TODO: Setup test entities
        let entity = world.spawn();
        // world.add(entity, ComponentA::default());

        {name}(&mut world, 0.016);

        // TODO: Assert expected behavior
    }

    #[test]
    fn test_{name}_multiple_entities() {
        let mut world = World::new();

        for i in 0..10 {
            let entity = world.spawn();
            // Setup components
        }

        {name}(&mut world, 0.016);

        // Verify all entities updated
    }

    #[test]
    fn test_{name}_no_matching_entities() {
        let mut world = World::new();

        // Should not crash with no entities
        {name}(&mut world, 0.016);
    }
}
```

### Query Parsing

**Format:** `[&mut |&]ComponentName[,[&mut |&]ComponentName]*`

**Examples:**
- `"&mut Health,&RegenerationRate"` → `[("Health", Mutable), ("RegenerationRate", Immutable)]`
- `"&Transform,&Velocity"` → `[("Transform", Immutable), ("Velocity", Immutable)]`
- `"&mut Transform,&mut Velocity,&Mass"` → Multiple mutable

**Parser Implementation:**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryAccess {
    Immutable,
    Mutable,
}

pub struct QueryComponent {
    pub name: String,
    pub access: QueryAccess,
}

pub fn parse_query_components(input: &str) -> Result<Vec<QueryComponent>> {
    input
        .split(',')
        .map(|comp| {
            let comp = comp.trim();

            let (access, name) = if let Some(stripped) = comp.strip_prefix("&mut ") {
                (QueryAccess::Mutable, stripped)
            } else if let Some(stripped) = comp.strip_prefix("&") {
                (QueryAccess::Immutable, stripped)
            } else {
                anyhow::bail!("Query component must start with '&' or '&mut': '{}'", comp);
            };

            let name = name.trim().to_string();
            validate_pascal_case(&name)?;

            Ok(QueryComponent { name, access })
        })
        .collect()
}
```

---

## Component Registry

### File Format: `.silmaril/components.json`

```json
{
  "version": "1.0",
  "last_updated": "2026-02-03T15:30:00Z",
  "components": [
    {
      "name": "Health",
      "location": "shared",
      "file": "shared/src/components/health.rs",
      "fields": [
        {
          "name": "current",
          "type": "f32",
          "doc": null
        },
        {
          "name": "max",
          "type": "f32",
          "doc": null
        }
      ],
      "derives": ["Debug", "Clone", "Default", "Component", "Serialize", "Deserialize"],
      "documentation": "Player health with current and maximum values",
      "created_at": "2026-02-03T10:30:00Z"
    }
  ],
  "systems": [
    {
      "name": "health_regen",
      "location": "shared",
      "file": "shared/src/systems/health_regen.rs",
      "query": [
        { "component": "Health", "access": "mutable" },
        { "component": "RegenerationRate", "access": "immutable" }
      ],
      "phase": "update",
      "documentation": "Regenerate health over time",
      "created_at": "2026-02-03T10:45:00Z"
    }
  ]
}
```

### Registry Data Structures

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRegistry {
    pub version: String,
    pub last_updated: String,
    pub components: Vec<ComponentEntry>,
    pub systems: Vec<SystemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub name: String,
    pub location: String,  // "shared", "client", "server"
    pub file: PathBuf,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub documentation: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntry {
    pub name: String,
    pub location: String,
    pub file: PathBuf,
    pub query: Vec<QueryComponentInfo>,
    pub phase: String,  // "update", "fixed_update", "render"
    pub documentation: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryComponentInfo {
    pub component: String,
    pub access: String,  // "mutable" or "immutable"
}
```

### Registry Operations

```rust
impl ComponentRegistry {
    /// Load registry from .silmaril/components.json
    pub fn load() -> Result<Self> {
        let path = PathBuf::from(".silmaril/components.json");

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let registry: Self = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Save registry to .silmaril/components.json
    pub fn save(&self) -> Result<()> {
        let path = PathBuf::from(".silmaril/components.json");

        // Create .silmaril directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a component to the registry
    pub fn add_component(&mut self, entry: ComponentEntry) -> Result<()> {
        // Check for duplicates
        if self.components.iter().any(|c| c.name == entry.name && c.location == entry.location) {
            anyhow::bail!("Component '{}' already exists in location '{}'", entry.name, entry.location);
        }

        self.components.push(entry);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Add a system to the registry
    pub fn add_system(&mut self, entry: SystemEntry) -> Result<()> {
        // Check for duplicates
        if self.systems.iter().any(|s| s.name == entry.name && s.location == entry.location) {
            anyhow::bail!("System '{}' already exists in location '{}'", entry.name, entry.location);
        }

        self.systems.push(entry);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Find a component by name
    pub fn find_component(&self, name: &str) -> Option<&ComponentEntry> {
        self.components.iter().find(|c| c.name == name)
    }

    /// Validate that all query components exist
    pub fn validate_query(&self, components: &[QueryComponent]) -> Result<()> {
        for comp in components {
            if self.find_component(&comp.name).is_none() {
                anyhow::bail!("Component '{}' not found in registry", comp.name);
            }
        }
        Ok(())
    }
}
```

---

## Module Export Management

### Auto-Update mod.rs

When a component/system is generated, automatically update the corresponding `mod.rs` file.

**File:** `shared/src/components/mod.rs`

```rust
// Auto-generated by silm CLI - DO NOT EDIT MANUALLY
// Run `silm add component` to add new components

pub mod health;        // Added: 2026-02-03T10:30:00Z
pub mod inventory;     // Added: 2026-02-03T11:15:00Z
pub mod velocity;      // Added: 2026-02-03T12:00:00Z

pub use health::Health;
pub use inventory::Inventory;
pub use velocity::Velocity;
```

### Implementation

```rust
pub fn update_module_exports(
    target_dir: &Path,
    item_name: &str,
    item_type: &str,  // "component" or "system"
) -> Result<()> {
    let mod_file = target_dir.join("mod.rs");
    let snake_name = to_snake_case(item_name);

    // Read existing mod.rs or create new
    let mut content = if mod_file.exists() {
        std::fs::read_to_string(&mod_file)?
    } else {
        format!(
            "// Auto-generated by silm CLI - DO NOT EDIT MANUALLY\n\
             // Run `silm add {}` to add new {}s\n\n",
            item_type, item_type
        )
    };

    // Check if already exists
    let mod_line = format!("pub mod {};", snake_name);
    if content.contains(&mod_line) {
        return Ok(()); // Already exported
    }

    // Find insertion point (after header, before first blank line or EOF)
    let lines: Vec<&str> = content.lines().collect();
    let mut insert_idx = 0;
    let mut found_header = false;

    for (idx, line) in lines.iter().enumerate() {
        if line.starts_with("//") {
            found_header = true;
        } else if found_header && line.is_empty() {
            insert_idx = idx;
            break;
        }
    }

    // Add module declaration
    let timestamp = chrono::Utc::now().to_rfc3339();
    let new_mod = format!("pub mod {};  // Added: {}", snake_name, timestamp);

    // Add re-export
    let new_export = format!("pub use {}::{};", snake_name, item_name);

    // Insert into content
    // ... (implementation details)

    std::fs::write(&mod_file, content)?;
    Ok(())
}

pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}
```

---

## Interactive Mode

### Component Interactive Flow

```
$ silm add component Health --interactive

🎮 Creating new component...

Component name: Health
Location (shared/client/server) [shared]: shared

Add field (name:type, or Enter to finish): current:f32
Add field (name:type, or Enter to finish): max:f32
Add field (name:type, or Enter to finish):

Additional derives (comma-separated) [Debug,Clone]: Debug,Clone,Default
Documentation: Player health with current and maximum values

📋 Summary:
  Name: Health
  Location: shared
  Fields:
    • current: f32
    • max: f32
  Derives: Debug, Clone, Default, Component, Serialize, Deserialize
  Documentation: Player health with current and maximum values

Create component? [Y/n]: y

✅ Component created successfully!

📁 Files:
  ✓ shared/src/components/health.rs
  ✓ shared/src/components/mod.rs (updated)
  ✓ .silmaril/components.json (updated)

📝 Next steps:
  1. Review generated code
  2. Add to templates: silm template edit player.yaml
  3. Implement Default trait (if needed)
  4. Write additional tests
```

### System Interactive Flow

```
$ silm add system health_regen --interactive

🎮 Creating new system...

System name: health_regen
Location (shared/client/server) [shared]: shared
Phase (update/fixed_update/render) [update]: update

Add query component (&ComponentName or &mut ComponentName, Enter to finish): &mut Health
Add query component (&ComponentName or &mut ComponentName, Enter to finish): &RegenerationRate
Add query component (&ComponentName or &mut ComponentName, Enter to finish):

Documentation: Regenerate health over time based on regeneration rate

📋 Summary:
  Name: health_regen
  Location: shared
  Phase: update
  Query:
    • &mut Health
    • &RegenerationRate
  Documentation: Regenerate health over time based on regeneration rate

Create system? [Y/n]: y

✅ System created successfully!

📁 Files:
  ✓ shared/src/systems/health_regen.rs
  ✓ shared/src/systems/mod.rs (updated)
  ✓ .silmaril/components.json (updated)

📝 Next steps:
  1. Review generated code
  2. Implement system logic
  3. Register in main.rs: app.add_system(health_regen)
  4. Run tests: cargo test health_regen
```

---

## Testing Strategy

### Test Pyramid

Following CLAUDE.md guidelines:

```
       ╱ ╲
      ╱   ╲         E2E (TestContainers)
     ╱─────╲        - Full project scaffold + compile
    ╱       ╲       - 2-3 comprehensive tests
   ╱─────────╲
  ╱           ╲     Integration
 ╱─────────────╲    - Component + system generation
╱               ╲   - File operations
────────────────────
    Unit Tests       - Parsers, validators, code gen
    (Foundation)     - 50+ tests
```

### Unit Tests

**File:** `engine/cli/tests/codegen_unit_tests.rs`

```rust
mod field_parser {
    use engine_cli::codegen::parser::parse_fields;

    #[test]
    fn test_parse_simple_fields() {
        let result = parse_fields("current:f32,max:f32").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("current".to_string(), "f32".to_string()));
        assert_eq!(result[1], ("max".to_string(), "f32".to_string()));
    }

    #[test]
    fn test_parse_complex_types() {
        let result = parse_fields("items:Vec<Item>,capacity:usize").unwrap();
        assert_eq!(result[0].1, "Vec<Item>");
    }

    #[test]
    fn test_parse_array_types() {
        let result = parse_fields("position:[f32;3]").unwrap();
        assert_eq!(result[0].1, "[f32;3]");
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = parse_fields("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_field_name() {
        let result = parse_fields(":f32");
        assert!(result.is_err());
    }
}

mod name_validator {
    use engine_cli::codegen::validator::{validate_pascal_case, validate_snake_case};

    #[test]
    fn test_valid_pascal_case() {
        assert!(validate_pascal_case("Health").is_ok());
        assert!(validate_pascal_case("PlayerState").is_ok());
        assert!(validate_pascal_case("MeshRenderer2D").is_ok());
    }

    #[test]
    fn test_invalid_pascal_case() {
        assert!(validate_pascal_case("health").is_err());  // lowercase
        assert!(validate_pascal_case("player-state").is_err());  // hyphen
        assert!(validate_pascal_case("123Health").is_err());  // starts with number
        assert!(validate_pascal_case("").is_err());  // empty
    }

    #[test]
    fn test_valid_snake_case() {
        assert!(validate_snake_case("health_regen").is_ok());
        assert!(validate_snake_case("movement").is_ok());
        assert!(validate_snake_case("_internal").is_ok());
    }

    #[test]
    fn test_invalid_snake_case() {
        assert!(validate_snake_case("HealthRegen").is_err());  // PascalCase
        assert!(validate_snake_case("health-regen").is_err());  // hyphen
        assert!(validate_snake_case("").is_err());  // empty
    }
}

mod query_parser {
    use engine_cli::codegen::parser::{parse_query_components, QueryAccess};

    #[test]
    fn test_parse_immutable_query() {
        let result = parse_query_components("&Health,&Velocity").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_mutable_query() {
        let result = parse_query_components("&mut Health,&RegenerationRate").unwrap();
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_missing_ampersand() {
        let result = parse_query_components("Health");
        assert!(result.is_err());
    }
}

mod code_generator {
    use engine_cli::codegen::component::generate_component_code;

    #[test]
    fn test_generate_simple_component() {
        let fields = vec![
            ("current".to_string(), "f32".to_string()),
            ("max".to_string(), "f32".to_string()),
        ];

        let code = generate_component_code(
            "Health",
            &fields,
            Some("Debug,Clone,Default".to_string()),
            Some("Player health".to_string())
        );

        assert!(code.contains("pub struct Health"));
        assert!(code.contains("pub current: f32"));
        assert!(code.contains("pub max: f32"));
        assert!(code.contains("impl Default for Health"));
        assert!(code.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_generate_without_default() {
        let fields = vec![("value".to_string(), "String".to_string())];

        let code = generate_component_code(
            "Name",
            &fields,
            Some("Debug,Clone".to_string()),
            None
        );

        assert!(!code.contains("impl Default for Name"));
    }
}

mod registry {
    use engine_cli::codegen::registry::{ComponentRegistry, ComponentEntry};

    #[test]
    fn test_add_component() {
        let mut registry = ComponentRegistry::default();

        let entry = ComponentEntry {
            name: "Health".to_string(),
            location: "shared".to_string(),
            // ... other fields
        };

        assert!(registry.add_component(entry).is_ok());
        assert_eq!(registry.components.len(), 1);
    }

    #[test]
    fn test_duplicate_component() {
        let mut registry = ComponentRegistry::default();

        let entry = ComponentEntry {
            name: "Health".to_string(),
            location: "shared".to_string(),
            // ...
        };

        registry.add_component(entry.clone()).unwrap();
        let result = registry.add_component(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_component() {
        let mut registry = ComponentRegistry::default();
        // Add component...

        let found = registry.find_component("Health");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Health");
    }
}
```

**Target:** 50+ unit tests, > 90% code coverage

### Integration Tests

**File:** `engine/cli/tests/codegen_integration_tests.rs`

```rust
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_component_generation_e2e() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create project structure
    create_project_structure(&temp_dir.path());

    // Generate component
    let result = engine_cli::commands::add::add_component(
        "Health",
        "current:f32,max:f32",
        Location::Shared,
        Some("Debug,Clone,Default".to_string()),
        None
    );

    assert!(result.is_ok());

    // Verify file created
    let component_file = temp_dir.path().join("shared/src/components/health.rs");
    assert!(component_file.exists());

    // Verify content
    let content = std::fs::read_to_string(&component_file).unwrap();
    assert!(content.contains("pub struct Health"));
    assert!(content.contains("pub current: f32"));
    assert!(content.contains("impl Default for Health"));

    // Verify mod.rs updated
    let mod_file = temp_dir.path().join("shared/src/components/mod.rs");
    let mod_content = std::fs::read_to_string(&mod_file).unwrap();
    assert!(mod_content.contains("pub mod health"));
    assert!(mod_content.contains("pub use health::Health"));

    // Verify registry updated
    let registry_file = temp_dir.path().join(".silmaril/components.json");
    assert!(registry_file.exists());

    let registry: ComponentRegistry = serde_json::from_str(
        &std::fs::read_to_string(&registry_file).unwrap()
    ).unwrap();

    assert_eq!(registry.components.len(), 1);
    assert_eq!(registry.components[0].name, "Health");
}

#[test]
fn test_system_generation_e2e() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create project structure
    create_project_structure(&temp_dir.path());

    // Generate system
    let result = engine_cli::commands::add::add_system(
        "health_regen",
        "&mut Health,&RegenerationRate",
        Location::Shared,
        SystemPhase::Update,
        None
    );

    assert!(result.is_ok());

    // Verify file created
    let system_file = temp_dir.path().join("shared/src/systems/health_regen.rs");
    assert!(system_file.exists());

    // Verify content
    let content = std::fs::read_to_string(&system_file).unwrap();
    assert!(content.contains("pub fn health_regen"));
    assert!(content.contains("world.query::<(&mut Health, &RegenerationRate)>()"));
    assert!(content.contains("#[cfg(test)]"));
}

#[test]
fn test_generated_code_compiles() {
    // This test uses actual cargo to compile generated code
    let temp_dir = TempDir::new().unwrap();

    // Create full project
    create_full_project(&temp_dir.path());

    // Generate component + system
    // ...

    // Try to compile
    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "Generated code should compile");
}
```

**Target:** 15+ integration tests

### E2E Tests with TestContainers

**File:** `engine/cli/tests/e2e_tests.rs`

```rust
use testcontainers::{clients, images};

#[test]
#[ignore]  // Run with: cargo test --ignored
fn test_full_project_workflow() {
    // 1. Create new project
    // 2. Add components
    // 3. Add systems
    // 4. Compile project
    // 5. Run tests
    // 6. Verify everything works
}

#[test]
#[ignore]
fn test_template_integration() {
    // 1. Create project
    // 2. Add component
    // 3. Create template using component
    // 4. Validate template
    // 5. Compile and run
}
```

**Target:** 2-3 E2E tests (run in CI)

---

## Benchmarking

### Performance Targets

| Operation | Target | Critical |
|-----------|--------|----------|
| Component generation | < 100ms | < 200ms |
| System generation | < 100ms | < 200ms |
| Field parsing | < 1ms | < 5ms |
| Registry update | < 20ms | < 50ms |
| Module export update | < 50ms | < 100ms |

### Benchmark Suite

**File:** `engine/cli/benches/codegen_benches.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_cli::codegen::*;

fn bench_field_parsing(c: &mut Criterion) {
    c.bench_function("parse simple fields", |b| {
        b.iter(|| {
            parser::parse_fields(black_box("current:f32,max:f32"))
        });
    });

    c.bench_function("parse complex fields", |b| {
        b.iter(|| {
            parser::parse_fields(black_box(
                "items:Vec<Item>,capacity:usize,metadata:HashMap<String,Value>"
            ))
        });
    });
}

fn bench_component_generation(c: &mut Criterion) {
    let fields = vec![
        ("current".to_string(), "f32".to_string()),
        ("max".to_string(), "f32".to_string()),
    ];

    c.bench_function("generate component code", |b| {
        b.iter(|| {
            component::generate_component_code(
                black_box("Health"),
                black_box(&fields),
                Some("Debug,Clone,Default".to_string()),
                None
            )
        });
    });
}

fn bench_system_generation(c: &mut Criterion) {
    let components = vec![
        QueryComponent { name: "Health".to_string(), access: QueryAccess::Mutable },
        QueryComponent { name: "RegenerationRate".to_string(), access: QueryAccess::Immutable },
    ];

    c.bench_function("generate system code", |b| {
        b.iter(|| {
            system::generate_system_code(
                black_box("health_regen"),
                black_box(&components),
                SystemPhase::Update,
                None
            )
        });
    });
}

fn bench_registry_operations(c: &mut Criterion) {
    c.bench_function("registry load", |b| {
        b.iter(|| {
            ComponentRegistry::load()
        });
    });

    c.bench_function("registry add component", |b| {
        let mut registry = ComponentRegistry::default();
        b.iter(|| {
            let entry = ComponentEntry {
                name: format!("Component{}", rand::random::<u32>()),
                // ...
            };
            let _ = registry.add_component(entry);
        });
    });
}

criterion_group!(
    benches,
    bench_field_parsing,
    bench_component_generation,
    bench_system_generation,
    bench_registry_operations
);
criterion_main!(benches);
```

**Run benchmarks:**
```bash
cargo bench --package engine-cli codegen
```

---

## Implementation Milestones

### Milestone 1: Basic Component Generation (3-4 days)

**Tasks:**
- [ ] Create `engine/cli/src/codegen/` module structure
- [ ] Implement field parser (`parser.rs`)
- [ ] Implement name validator (`validator.rs`)
- [ ] Implement component code generator (`component.rs`)
- [ ] Add `AddCommand::Component` to CLI
- [ ] Wire up command handler
- [ ] Write 25+ unit tests
- [ ] Write 5+ integration tests

**Deliverables:**
- `silm add component Health --fields "current:f32,max:f32"` works
- Generated code compiles
- Tests pass

### Milestone 2: Basic System Generation (3-4 days)

**Tasks:**
- [ ] Implement query parser (`parser.rs`)
- [ ] Implement system code generator (`system.rs`)
- [ ] Add `AddCommand::System` to CLI
- [ ] Wire up command handler
- [ ] Write 20+ unit tests
- [ ] Write 5+ integration tests

**Deliverables:**
- `silm add system health_regen --query "..."` works
- Generated code compiles
- Tests pass

### Milestone 3: Component Registry (2-3 days)

**Tasks:**
- [ ] Implement `ComponentRegistry` struct (`registry.rs`)
- [ ] Implement load/save operations
- [ ] Integrate with component generation
- [ ] Integrate with system generation
- [ ] Validate query components against registry
- [ ] Write 15+ unit tests
- [ ] Write 3+ integration tests

**Deliverables:**
- `.silmaril/components.json` created and updated
- Template validation can use registry
- Tests pass

### Milestone 4: Module Export Management (2-3 days)

**Tasks:**
- [ ] Implement `update_module_exports()` function
- [ ] Integrate with component generation
- [ ] Integrate with system generation
- [ ] Handle edge cases (existing files, conflicts)
- [ ] Write 10+ unit tests
- [ ] Write 3+ integration tests

**Deliverables:**
- `mod.rs` files auto-update
- No duplicate exports
- Tests pass

### Milestone 5: Interactive Mode (2-3 days)

**Tasks:**
- [ ] Implement interactive component prompts
- [ ] Implement interactive system prompts
- [ ] Add `--interactive` flag
- [ ] Validate user input
- [ ] Pretty-print summary
- [ ] Write 5+ integration tests

**Deliverables:**
- `silm add component Health --interactive` works
- User-friendly prompts
- Tests pass

### Milestone 6: Benchmarking & Optimization (1-2 days)

**Tasks:**
- [ ] Write benchmark suite
- [ ] Run baseline benchmarks
- [ ] Identify bottlenecks
- [ ] Optimize hot paths
- [ ] Document performance

**Deliverables:**
- Benchmarks pass targets
- Performance documented

### Milestone 7: E2E Testing (1-2 days)

**Tasks:**
- [ ] Write E2E tests with TestContainers
- [ ] Test full project workflow
- [ ] Test template integration
- [ ] CI integration

**Deliverables:**
- E2E tests pass
- CI runs E2E tests

---

## Success Metrics

### Code Quality
- ✅ 100% of public APIs documented
- ✅ Zero clippy warnings
- ✅ Zero unsafe code
- ✅ > 85% test coverage

### Performance
- ✅ Component generation: < 100ms
- ✅ System generation: < 100ms
- ✅ Registry operations: < 20ms

### User Experience
- ✅ Clear error messages
- ✅ Helpful next steps
- ✅ Interactive mode intuitive
- ✅ Generated code compiles

### Integration
- ✅ Works with templates
- ✅ Registry validated by template validation
- ✅ Module exports correct
- ✅ No manual intervention needed

---

**Last Updated:** 2026-02-03
**Status:** Ready for Implementation
