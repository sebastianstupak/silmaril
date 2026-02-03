# CLI Code Generation Discussion

> Discussion: Adding `silm add component` and `silm add system` commands
>
> Status: Planning Phase
> Date: 2026-02-03

---

## Current State

### CLI Structure
```
engine/cli/src/
├── main.rs                 # Main CLI entry point with Commands enum
├── commands/
│   ├── mod.rs
│   ├── new.rs             # Project scaffolding (silm new)
│   └── template.rs        # Template management (silm template)
└── templates/
    ├── mod.rs
    └── basic.rs           # Basic project template
```

### Existing Commands
- `silm new <name>` - Create new project (multi-crate structure)
- `silm template add/validate/compile/list/tree/rename/delete` - Template management

---

## Proposed: Code Generation Commands

### CLI.2: Component Generation

```bash
# Create a new component
silm add component Health --fields "current:f32,max:f32" --location shared

# With more options
silm add component Inventory \
  --fields "items:Vec<Item>,capacity:usize" \
  --location server \
  --derive "Default,PartialEq" \
  --doc "Player inventory system"

# Interactive mode (prompts for fields)
silm add component Health --interactive
```

### CLI.2: System Generation

```bash
# Create a new system
silm add system health_regen \
  --query "Health,RegenerationRate" \
  --location shared

# With more options
silm add system combat_damage \
  --query "Health,Armor,DamageQueue" \
  --location server \
  --phase update \
  --doc "Apply damage with armor calculation"

# Interactive mode
silm add system health_regen --interactive
```

---

## Implementation Plan

### 1. Add Commands to CLI

**File:** `engine/cli/src/main.rs`

```rust
#[derive(Subcommand)]
enum Commands {
    /// Create a new game project
    New { /* ... */ },

    /// Manage entity templates
    Template { /* ... */ },

    /// Add new code (components, systems, modules) - NEW!
    Add {
        #[command(subcommand)]
        command: commands::add::AddCommand,
    },
}
```

### 2. Create Add Command Module

**File:** `engine/cli/src/commands/add.rs`

```rust
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AddCommand {
    /// Generate a new component
    Component {
        /// Component name (PascalCase, e.g., "Health")
        name: String,

        /// Component fields (comma-separated: "current:f32,max:f32")
        #[arg(short, long)]
        fields: String,

        /// Where to place component: shared, client, or server
        #[arg(short, long, default_value = "shared")]
        location: Location,

        /// Additional derives (Default, PartialEq, etc.)
        #[arg(long)]
        derive: Option<String>,

        /// Documentation string
        #[arg(long)]
        doc: Option<String>,

        /// Interactive mode (prompt for fields)
        #[arg(short, long)]
        interactive: bool,
    },

    /// Generate a new system
    System {
        /// System name (snake_case, e.g., "health_regen")
        name: String,

        /// Query components (comma-separated: "Health,RegenerationRate")
        #[arg(short, long)]
        query: String,

        /// Where to place system: shared, client, or server
        #[arg(short, long, default_value = "shared")]
        location: Location,

        /// System phase: update, fixed_update, or render
        #[arg(long, default_value = "update")]
        phase: SystemPhase,

        /// Documentation string
        #[arg(long)]
        doc: Option<String>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Location {
    Shared,  // {project}-shared/src/
    Client,  // {project}-client/src/
    Server,  // {project}-server/src/
}

#[derive(Debug, Clone, Copy)]
pub enum SystemPhase {
    Update,       // Runs every frame
    FixedUpdate,  // Runs at fixed rate (physics)
    Render,       // Runs before rendering
}

pub fn handle_add_command(cmd: AddCommand) -> Result<()> {
    match cmd {
        AddCommand::Component { name, fields, location, derive, doc, interactive } => {
            if interactive {
                add_component_interactive(&name)
            } else {
                add_component(&name, &fields, location, derive, doc)
            }
        }
        AddCommand::System { name, query, location, phase, doc, interactive } => {
            if interactive {
                add_system_interactive(&name)
            } else {
                add_system(&name, &query, location, phase, doc)
            }
        }
    }
}
```

### 3. Component Code Generation

**File:** `engine/cli/src/commands/add.rs`

```rust
fn add_component(
    name: &str,
    fields: &str,
    location: Location,
    derive: Option<String>,
    doc: Option<String>,
) -> Result<()> {
    // 1. Validate component name (PascalCase)
    validate_pascal_case(name)?;

    // 2. Parse fields (e.g., "current:f32,max:f32")
    let parsed_fields = parse_fields(fields)?;

    // 3. Determine target directory (directories: shared/client/server)
    let target_dir = match location {
        Location::Shared => PathBuf::from("shared/src/components"),
        Location::Client => PathBuf::from("client/src/components"),
        Location::Server => PathBuf::from("server/src/components"),
    };

    // 4. Generate component code
    let code = generate_component_code(name, &parsed_fields, derive, doc);

    // 5. Write to file
    let file_path = target_dir.join(format!("{}.rs", to_snake_case(name)));
    std::fs::create_dir_all(&target_dir)?;
    std::fs::write(&file_path, code)?;

    // 6. Update mod.rs exports
    update_module_exports(&target_dir, name)?;

    // 7. Update ComponentData enum in shared crate
    update_component_data_enum(name, location)?;

    println!("✓ Component created: {}", file_path.display());
    println!("  Next steps:");
    println!("    1. Review generated code");
    println!("    2. Add to templates (silm template edit)");
    println!("    3. Write tests");

    Ok(())
}

fn generate_component_code(
    name: &str,
    fields: &[(String, String)],
    derive: Option<String>,
    doc: Option<String>,
) -> String {
    let doc_str = doc.unwrap_or_else(|| format!("{} component", name));
    let derives = derive.unwrap_or_else(|| "Debug, Clone".to_string());

    let mut code = String::new();

    // Imports
    code.push_str("use engine_core::ecs::Component;\n");
    code.push_str("use serde::{Deserialize, Serialize};\n\n");

    // Documentation
    code.push_str(&format!("/// {}\n", doc_str));
    code.push_str(&format!("#[derive({}, Component, Serialize, Deserialize)]\n", derives));
    code.push_str(&format!("pub struct {} {{\n", name));

    // Fields
    for (field_name, field_type) in fields {
        code.push_str(&format!("    /// TODO: Document this field\n"));
        code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
    }

    code.push_str("}\n\n");

    // Default implementation (if requested)
    if derives.contains("Default") {
        code.push_str(&format!("impl Default for {} {{\n", name));
        code.push_str("    fn default() -> Self {\n");
        code.push_str("        Self {\n");
        for (field_name, field_type) in fields {
            code.push_str(&format!("            {}: {},\n", field_name, default_value_for_type(field_type)));
        }
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
    }

    // Tests
    code.push_str("#[cfg(test)]\n");
    code.push_str("mod tests {\n");
    code.push_str("    use super::*;\n");
    code.push_str("    use engine_core::ecs::World;\n\n");
    code.push_str(&format!("    #[test]\n"));
    code.push_str(&format!("    fn test_{}_add_get() {{\n", to_snake_case(name)));
    code.push_str("        let mut world = World::new();\n");
    code.push_str("        let entity = world.spawn();\n\n");
    code.push_str(&format!("        let component = {}::default();\n", name));
    code.push_str("        world.add(entity, component.clone());\n\n");
    code.push_str(&format!("        let retrieved = world.get::<{}>(entity).unwrap();\n", name));
    code.push_str("        assert!(world.has::<{}>(entity));\n", name);
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}
```

### 4. System Code Generation

**File:** `engine/cli/src/commands/add.rs`

```rust
fn add_system(
    name: &str,
    query: &str,
    location: Location,
    phase: SystemPhase,
    doc: Option<String>,
) -> Result<()> {
    // 1. Validate system name (snake_case)
    validate_snake_case(name)?;

    // 2. Parse query components
    let components = parse_query_components(query)?;

    // 3. Determine target directory
    let target_dir = match location {
        Location::Shared => PathBuf::from("{project}-shared/src/systems"),
        Location::Client => PathBuf::from("{project}-client/src/systems"),
        Location::Server => PathBuf::from("{project}-server/src/systems"),
    };

    // 4. Generate system code
    let code = generate_system_code(name, &components, phase, doc);

    // 5. Write to file
    let file_path = target_dir.join(format!("{}.rs", name));
    std::fs::create_dir_all(&target_dir)?;
    std::fs::write(&file_path, code)?;

    // 6. Update mod.rs exports
    update_module_exports(&target_dir, name)?;

    // 7. Register system in app (TODO: automated registration)
    println!("✓ System created: {}", file_path.display());
    println!("  Next steps:");
    println!("    1. Review generated code");
    println!("    2. Register in main.rs: app.add_system({})", name);
    println!("    3. Write tests");

    Ok(())
}

fn generate_system_code(
    name: &str,
    components: &[QueryComponent],
    phase: SystemPhase,
    doc: Option<String>,
) -> String {
    let doc_str = doc.unwrap_or_else(|| format!("{} system", name.replace('_', " ")));

    let mut code = String::new();

    // Imports
    code.push_str("use engine_core::ecs::{Query, World};\n");
    code.push_str("use tracing::{debug, instrument};\n\n");

    // Component imports (TODO: detect location)
    for component in components {
        code.push_str(&format!("use crate::components::{};\n", component.name));
    }
    code.push_str("\n");

    // Documentation
    code.push_str(&format!("/// {}\n", doc_str));
    code.push_str("///\n");
    code.push_str(&format!("/// # Phase\n"));
    code.push_str(&format!("/// {:?}\n", phase));
    code.push_str("///\n");
    code.push_str("/// # Query\n");
    for component in components {
        code.push_str(&format!("/// - {}{}\n",
            if component.mutable { "&mut " } else { "&" },
            component.name
        ));
    }
    code.push_str("#[instrument(skip(world))]\n");
    code.push_str(&format!("pub fn {}(world: &mut World, delta_time: f32) {{\n", name));

    // Query
    code.push_str("    let query = world.query::<(");
    for (i, component) in components.iter().enumerate() {
        if i > 0 { code.push_str(", "); }
        code.push_str(if component.mutable { "&mut " } else { "&" });
        code.push_str(&component.name);
    }
    code.push_str(")>();\n\n");

    // Loop
    code.push_str("    for (entity, (");
    for (i, component) in components.iter().enumerate() {
        if i > 0 { code.push_str(", "); }
        code.push_str(&to_snake_case(&component.name));
    }
    code.push_str(")) in query.iter() {\n");
    code.push_str("        // TODO: Implement system logic\n");
    code.push_str("        debug!(?entity, \"Processing entity\");\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Tests
    code.push_str("#[cfg(test)]\n");
    code.push_str("mod tests {\n");
    code.push_str("    use super::*;\n\n");
    code.push_str(&format!("    #[test]\n"));
    code.push_str(&format!("    fn test_{}_basic() {{\n", name));
    code.push_str("        let mut world = World::new();\n");
    code.push_str("        // TODO: Setup test entities\n\n");
    code.push_str(&format!("        {}(&mut world, 0.016);\n", name));
    code.push_str("        // TODO: Assert expected behavior\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}
```

---

## Integration with Templates

### Workflow: Component → Template

1. **Create component** with `silm add component Health --fields "current:f32,max:f32"`
   - Generates `{project}-shared/src/components/health.rs`
   - Updates `ComponentData` enum
   - Updates module exports

2. **Use in template** with `silm template edit player.yaml`
   - Template editor opens (or use text editor)
   - Add component to entity:
   ```yaml
   entities:
     Player:
       components:
         Transform:
           position: [0, 0, 0]
         Health:           # Newly generated component
           current: 100.0
           max: 100.0
   ```

3. **Validate template** with `silm template validate player.yaml`
   - Checks that `Health` component exists
   - Validates field types

### Future: Template-Driven Component Creation

**Phase 2 Enhancement:**
```bash
# Edit template, add unknown component
silm template edit player.yaml
# Add: Health: { current: 100, max: 100 }

# Validate detects unknown component
silm template validate player.yaml
# Error: Component 'Health' not found. Create it? [y/N]
# Creates component automatically if confirmed
```

---

## Interactive Mode

### Component Interactive Mode

```bash
$ silm add component Health --interactive

Creating new component...

Component name: Health
Location (shared/client/server) [shared]: shared
Add field (name:type, or Enter to finish): current:f32
Add field (name:type, or Enter to finish): max:f32
Add field (name:type, or Enter to finish):
Additional derives (comma-separated) [Debug,Clone]: Debug,Clone,Default
Documentation: Player health with current/max values

Summary:
  Name: Health
  Location: shared
  Fields:
    - current: f32
    - max: f32
  Derives: Debug, Clone, Default

Create component? [Y/n]: y

✓ Component created: my-game-shared/src/components/health.rs
```

### System Interactive Mode

```bash
$ silm add system health_regen --interactive

Creating new system...

System name: health_regen
Location (shared/client/server) [shared]: shared
Phase (update/fixed_update/render) [update]: update
Add query component (Name or &mut Name, Enter to finish): &mut Health
Add query component (Name or &mut Name, Enter to finish): &RegenerationRate
Add query component (Name or &mut Name, Enter to finish):
Documentation: Regenerate health over time

Summary:
  Name: health_regen
  Location: shared
  Phase: update
  Query: &mut Health, &RegenerationRate

Create system? [Y/n]: y

✓ System created: my-game-shared/src/systems/health_regen.rs
```

---

## Module Export Management

### Auto-Update mod.rs

**File:** `{project}-shared/src/components/mod.rs`

```rust
// Auto-generated by silm CLI - DO NOT EDIT MANUALLY
// Run `silm add component` to add new components

pub mod health;        // Added by: silm add component Health
pub mod velocity;      // Added by: silm add component Velocity
pub mod inventory;     // Added by: silm add component Inventory

pub use health::Health;
pub use velocity::Velocity;
pub use inventory::Inventory;
```

### ComponentData Enum Auto-Update

**File:** `shared/src/lib.rs` (or `shared/src/serialization.rs`)

```rust
// Auto-generated ComponentData enum
// Run `silm add component` to update

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    Transform(Transform),
    Health(Health),          // Auto-added
    Velocity(Velocity),      // Auto-added
    Inventory(Inventory),    // Auto-added
}

// Auto-generated From implementations
impl From<Health> for ComponentData {
    fn from(c: Health) -> Self {
        ComponentData::Health(c)
    }
}
// ... etc
```

### Component Registry (.silmaril/components.json)

**File:** `.silmaril/components.json`

```json
{
  "version": "1.0",
  "components": [
    {
      "name": "Health",
      "location": "shared",
      "file": "shared/src/components/health.rs",
      "fields": [
        { "name": "current", "type": "f32" },
        { "name": "max", "type": "f32" }
      ],
      "derives": ["Debug", "Clone", "Default", "Serialize", "Deserialize"],
      "created_at": "2026-02-03T10:30:00Z"
    },
    {
      "name": "Inventory",
      "location": "shared",
      "file": "shared/src/components/inventory.rs",
      "fields": [
        { "name": "items", "type": "Vec<Item>" },
        { "name": "capacity", "type": "usize" }
      ],
      "derives": ["Debug", "Clone", "Serialize", "Deserialize"],
      "created_at": "2026-02-03T11:15:00Z"
    }
  ],
  "systems": [
    {
      "name": "health_regen",
      "location": "shared",
      "file": "shared/src/systems/health_regen.rs",
      "query": ["&mut Health", "&RegenerationRate"],
      "phase": "update",
      "created_at": "2026-02-03T10:45:00Z"
    }
  ]
}
```

**Usage:**
- Fast lookups during `silm template validate`
- Component discovery for code completion (future)
- Conflict detection when adding components
- Migration tracking

---

## File Structure After Generation

```
my-game/
├── Cargo.toml                # Workspace (members: shared, client, server, xtask)
├── game.toml                 # Game metadata
├── .silmaril/
│   ├── components.json       # Component registry (for validation/CLI)
│   └── editor-config.toml    # Editor settings (future)
├── shared/                   # Directory name (package: my-game-shared)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── components/
│       │   ├── mod.rs           # Auto-updated exports
│       │   ├── health.rs        # Generated by CLI
│       │   └── inventory.rs     # Generated by CLI
│       └── systems/
│           ├── mod.rs           # Auto-updated exports
│           ├── health_regen.rs  # Generated by CLI
│           └── combat_damage.rs # Generated by CLI
├── server/                   # Directory name (package: my-game-server)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── components/          # Server-only components
│       └── systems/             # Server-only systems
├── client/                   # Directory name (package: my-game-client)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── components/          # Client-only components
│       └── systems/             # Client-only systems
├── assets/
│   └── templates/
│       └── characters/
│           └── player.yaml      # Uses generated components
└── xtask/
    └── src/main.rs           # Build automation
```

---

## Testing Strategy

### Component Tests (Auto-Generated)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_health_add_get() {
        let mut world = World::new();
        let entity = world.spawn();

        let component = Health { current: 100.0, max: 100.0 };
        world.add(entity, component.clone());

        let retrieved = world.get::<Health>(entity).unwrap();
        assert_eq!(retrieved.current, 100.0);
        assert_eq!(retrieved.max, 100.0);
    }

    #[test]
    fn test_health_serialization() {
        let component = Health { current: 50.0, max: 100.0 };

        let yaml = serde_yaml::to_string(&component).unwrap();
        let deserialized: Health = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(component.current, deserialized.current);
    }
}
```

### System Tests (Auto-Generated)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_regen_basic() {
        let mut world = World::new();

        let entity = world.spawn();
        world.add(entity, Health { current: 50.0, max: 100.0 });
        world.add(entity, RegenerationRate(10.0));

        health_regen(&mut world, 1.0);

        let health = world.get::<Health>(entity).unwrap();
        assert_eq!(health.current, 60.0);
    }
}
```

---

## Decisions Made

### ✅ 1. Component Location Detection
**Decision:** Option B - Maintain `.silmaril/components.json` manifest

**Rationale:** Fast lookups, tracks metadata (location, fields, version)

### ✅ 2. Template Editing
**Decision:** Use system text editor OR GUI editor (later in Phase 0.8)

**Rationale:**
- Don't reinvent text editing in CLI
- GUI editor (Tauri + Svelte) is planned for Phase 0.8

### ✅ 3. System Registration
**Decision:** Manual registration

**Rationale:**
- Safer - doesn't modify main.rs automatically
- Developer has full control over system order
- Auto-discovery can come later via proc macros

### ✅ 4. Interactive vs Non-Interactive Default
**Decision:** Interactive if flags missing (Option C)

**Rationale:**
- `silm add component Health` → interactive prompts
- `silm add component Health --fields "..."` → non-interactive
- Best of both worlds

---

## Implementation Timeline

### Milestone 1: Basic Component Generation (3-4 days)
- [ ] Add `AddCommand` enum to CLI
- [ ] Implement `add_component()` function
- [ ] Parse fields from string
- [ ] Generate component code
- [ ] Write to file
- [ ] Basic tests

### Milestone 2: Basic System Generation (3-4 days)
- [ ] Implement `add_system()` function
- [ ] Parse query components
- [ ] Generate system code
- [ ] Write to file
- [ ] Basic tests

### Milestone 3: Module Export Management (2-3 days)
- [ ] Auto-update `mod.rs` files
- [ ] Update `ComponentData` enum
- [ ] Generate `From` implementations
- [ ] Validation tests

### Milestone 4: Interactive Mode (2-3 days)
- [ ] Component interactive prompts
- [ ] System interactive prompts
- [ ] Field validation
- [ ] User confirmation

### Milestone 5: Template Integration (2-3 days)
- [ ] Component registry (`.silmaril/components.json`)
- [ ] Template validation with component checks
- [ ] Template-driven component creation
- [ ] Documentation

**Total:** 12-17 days (~2.5-3.5 weeks)

---

## Success Criteria

- [ ] `silm add component Name --fields "..."` creates valid component
- [ ] `silm add system name --query "..."` creates valid system
- [ ] Generated components compile without errors
- [ ] Generated systems compile without errors
- [ ] Auto-generated tests pass
- [ ] Module exports update correctly
- [ ] ComponentData enum updates correctly
- [ ] Interactive mode works smoothly
- [ ] Templates can reference generated components
- [ ] Template validation detects missing components

---

## Next Steps

1. **Review this document** - Gather feedback on approach
2. **Prototype component generation** - Validate approach with simple example
3. **Implement Milestone 1** - Basic component generation
4. **Iterate based on usage** - Refine based on actual usage patterns
5. **Implement remaining milestones** - Complete full feature set

---

## Implementation Strategy

### Testing Strategy: Test Pyramid

Following CLAUDE.md testing guidelines:

**Unit Tests (engine/cli/tests/):**
- Component/system code generation
- Field parsing (`"current:f32,max:f32"`)
- Name validation (PascalCase, snake_case)
- Template rendering

**Integration Tests (engine/cli/tests/):**
- End-to-end `silm add component` flow
- File creation + module updates
- Component registry updates
- Template validation with generated components

**E2E Tests (with TestContainers):**
- Create full project: `silm new test-game`
- Add component: `silm add component Health`
- Compile project: `cargo build`
- Verify generated code compiles
- Run generated tests

### Benchmarking

Track code generation performance:
- Component generation: < 100ms target
- System generation: < 100ms target
- Module export updates: < 50ms target
- Registry updates: < 20ms target

Benchmark with criterion:
```rust
#[bench]
fn bench_component_generation(b: &mut Bencher) {
    b.iter(|| {
        generate_component_code("Health", &[("current", "f32")], None, None)
    });
}
```

### Multiple Subagents (Future Enhancement)

**Phase 1:** Single `silm` CLI binary (MVP)

**Phase 2:** Specialized subcommands with dedicated logic:
- `silm-codegen` - Component/system generation (can be separate binary)
- `silm-template` - Template operations
- `silm-dev` - Hot-reload development server
- `silm-package` - Distribution packaging

Each subagent can be:
- Developed independently
- Tested in isolation
- Composed into main `silm` CLI

**Architecture:**
```rust
// engine/cli/src/main.rs
match cli.command {
    Commands::Add { .. } => silm_codegen::run(args)?,
    Commands::Template { .. } => silm_template::run(args)?,
    Commands::Dev { .. } => silm_dev::run(args)?,
}
```

---

**Last Updated:** 2026-02-03
