# Template System

> **Unified entity template system for Silmaril**
>
> Templates are YAML files that define entities without IDs. Used for levels, characters, props, UI, and game state.

---

## Overview

The template system provides a **single unified format** for all entity definitions:
- **Levels** (large environments)
- **Characters** (players, NPCs)
- **Props** (reusable objects)
- **UI** (menus, HUD)
- **Game State** (inventory, quests)

**No distinction between "scenes" and "prefabs"** - everything is a template that can be nested.

---

## Architecture

### **Three Layers**

```
┌─────────────────────────────────────────────────────────────┐
│                     INTERFACE LAYER                          │
│  (Different UIs, same operations)                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CLI              Editor UI         Agent API               │
│  (terminal)       (web/desktop)     (chat commands)         │
│     │                 │                  │                  │
│     └─────────────────┼──────────────────┘                  │
│                       ▼                                     │
├─────────────────────────────────────────────────────────────┤
│              OPERATIONS LAYER (Shared Logic)                │
│                                                             │
│  engine/templating/operations.rs                       │
│  • create_template()                                        │
│  • validate_template()                                      │
│  • compile_template()                                       │
│  • list_templates()                                         │
│  • show_template_tree()                                     │
│  • rename_template()                                        │
│  • delete_template()                                        │
│                       ▼                                     │
├─────────────────────────────────────────────────────────────┤
│               CORE LAYER (Data & Logic)                     │
│                                                             │
│  engine/templating/                                    │
│  • Template (struct)                                        │
│  • TemplateLoader (load, spawn into world)                  │
│  • TemplateValidator (check refs, components)               │
│  • TemplateCompiler (YAML → Bincode)                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Template Format (YAML)

### **Basic Template**

```yaml
# assets/templates/characters/player.yaml

metadata:
  name: "Player Character"
  description: "Main player character"
  author: "GameDev Team"
  version: "1.0"

entities:
  Root:
    components:
      Transform:
        position: [0, 0, 0]
        rotation: [0, 0, 0, 1]
        scale: [1, 1, 1]
      Health:
        current: 100.0
        max: 100.0
      CharacterController:
        speed: 5.0
        jump_height: 2.0
      tags: [player, replicate]

    children:
      Camera:
        components:
          Transform:
            position: [0, 1.6, -3]
          Camera:
            fov: 60.0
            near: 0.1
            far: 1000.0

      Weapon:
        components:
          Transform:
            position: [0.5, 1.2, 0.5]
          MeshRenderer:
            mesh: "models/sword.glb"
```

### **Template with References**

```yaml
# assets/templates/levels/battle_arena.yaml

metadata:
  name: "Battle Arena"
  description: "5v5 competitive map"

entities:
  Ground:
    components:
      Transform:
        position: [0, 0, 0]
        scale: [100, 1, 100]
      MeshRenderer:
        mesh: "models/arena_ground.glb"
      Collider:
        type: box
        size: [100, 1, 100]
      tags: [static]

  GuardTower_1:
    template: "templates/props/guard_tower.yaml"  # Reference another template
    overrides:
      Transform:
        position: [30, 0, 30]

  GuardTower_2:
    template: "templates/props/guard_tower.yaml"  # Same template, different instance
    overrides:
      Transform:
        position: [-30, 0, 30]
        rotation: [0, 0.707, 0, 0.707]

  Player:
    template: "templates/characters/player.yaml"
    overrides:
      Transform:
        position: [0, 1, 0]
```

---

## Data Structures

### **Template**

```rust
pub struct Template {
    pub metadata: TemplateMetadata,
    pub entities: HashMap<String, EntityDefinition>,
}

pub struct TemplateMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
}
```

### **EntityDefinition**

```rust
pub struct EntityDefinition {
    pub source: EntitySource,
    pub overrides: HashMap<String, serde_yaml::Value>,
    pub children: HashMap<String, EntityDefinition>,
}

pub enum EntitySource {
    /// Inline entity definition
    Inline {
        components: HashMap<String, serde_yaml::Value>,
        tags: Vec<String>,
    },

    /// Reference to another template
    Reference {
        template: String,
    },
}
```

---

## Operations API

All shared logic is in `engine/templating/operations.rs`:

### **create_template()**

```rust
pub fn create_template(
    base_path: impl AsRef<Path>,
    options: CreateTemplateOptions,
) -> TemplateResult<PathBuf>

pub struct CreateTemplateOptions {
    pub name: String,
    pub template_type: TemplateType,
    pub description: Option<String>,
    pub author: Option<String>,
}

pub enum TemplateType {
    Level,
    Character,
    Prop,
    UI,
    GameState,
}
```

**Usage:**
```rust
let options = CreateTemplateOptions {
    name: "battle_arena".to_string(),
    template_type: TemplateType::Level,
    description: Some("5v5 map".to_string()),
    author: None,
};

let path = create_template("assets/templates", options)?;
```

### **validate_template()**

```rust
pub fn validate_template(
    template_path: impl AsRef<Path>
) -> TemplateResult<ValidationReport>

pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub entity_count: usize,
    pub template_references: Vec<String>,
}
```

**Validates:**
- ✅ YAML syntax
- ✅ Component references are valid
- ✅ Template references exist
- ✅ No circular dependencies

### **compile_template()**

```rust
pub fn compile_template(
    template_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> TemplateResult<CompiledTemplate>
```

**Pre-compiles YAML → Bincode for faster production loading**

### **Other Operations**

```rust
// List all templates
pub fn list_templates(base_path: impl AsRef<Path>) -> TemplateResult<Vec<TemplateInfo>>

// Show entity hierarchy
pub fn show_template_tree(template_path: impl AsRef<Path>) -> TemplateResult<TemplateTree>

// Rename template
pub fn rename_template(old_path: impl AsRef<Path>, new_name: &str) -> TemplateResult<PathBuf>

// Delete template
pub fn delete_template(path: impl AsRef<Path>) -> TemplateResult<()>
```

---

## Template Loader

Runtime loading and spawning:

```rust
use engine_template_system::loader::TemplateLoader;

let mut loader = TemplateLoader::new();
let instance = loader.load(&mut world, "templates/levels/battle_arena.yaml")?;

// Instance contains all spawned entities
for entity in &instance.entities {
    println!("Spawned entity: {:?}", entity);
}

// Unload when done
instance.despawn(&mut world);
```

---

## CLI Commands

```bash
# Create new template
silm template add battle_arena --type level --description "5v5 competitive map"

# Validate template
silm template validate assets/templates/levels/battle_arena.yaml

# Compile to binary format
silm template compile assets/templates/levels/battle_arena.yaml

# Compile all templates
silm template compile assets/templates --output assets/compiled

# List all templates
silm template list

# Show template hierarchy
silm template tree assets/templates/levels/battle_arena.yaml

# Rename template
silm template rename assets/templates/levels/old_name.yaml new_name

# Delete template
silm template delete assets/templates/levels/unused.yaml
```

---

## Directory Structure

```
assets/
└── templates/
    ├── levels/
    │   ├── battle_arena.yaml
    │   ├── tutorial.yaml
    │   └── dungeon_01.yaml
    │
    ├── characters/
    │   ├── player.yaml
    │   ├── npc_guard.yaml
    │   └── enemy_goblin.yaml
    │
    ├── props/
    │   ├── guard_tower.yaml
    │   ├── treasure_chest.yaml
    │   └── destructible_crate.yaml
    │
    ├── ui/
    │   ├── main_menu.yaml
    │   ├── hud.yaml
    │   └── inventory_panel.yaml
    │
    └── game_state/
        ├── player_inventory.yaml
        └── quest_log.yaml
```

---

## Template vs WorldState

### **Template (Authored Content)**

```yaml
metadata:
  name: "Test Scene"

entities:
  Ground:
    components:
      Transform:
        position: [0, 0, 0]
      tags: [static]
```

**Properties:**
- ✅ Named entities (no IDs)
- ✅ Human-readable
- ✅ Version control friendly
- ✅ Used for: authoring, editing

### **WorldState (Runtime Snapshot)**

```yaml
metadata:
  version: 1
  timestamp: 1738531200
  entity_count: 1
  component_count: 1

entities:
  - entity: { id: 42, generation: 0 }
    alive: true

components:
  "42":
    - Transform:
        position: [0.0, 0.0, 0.0]
```

**Properties:**
- ✅ Entity IDs and generations
- ✅ Complete runtime state
- ✅ Used for: saves, network snapshots, debugging

**Conversion:**
```
Template (YAML) ──[TemplateLoader]──► WorldState (Runtime)
                                         │
                                         ▼
                                    Serialized Snapshot
                                    (Bincode/FlatBuffers)
```

---

## Best Practices

### **Template Organization**

- ✅ **Use folder structure** - `levels/`, `characters/`, `props/`
- ✅ **Descriptive names** - `battle_arena.yaml`, not `level1.yaml`
- ✅ **Add metadata** - name, description, author
- ✅ **Use tags** - `static`, `replicate`, `destructible`

### **Template References**

- ✅ **Use relative paths** - `templates/props/guard_tower.yaml`
- ✅ **Override only what changes** - position, not all components
- ✅ **Avoid deep nesting** - max 2-3 levels of references
- ❌ **Don't create circular refs** - A → B → A (validator catches this)

### **Component Definitions**

- ✅ **Use clear field names** - `position`, not `pos`
- ✅ **Include units in comments** - `speed: 5.0  # m/s`
- ✅ **Group related entities** - use children for hierarchies
- ❌ **Don't duplicate data** - use template references

---

## Performance

### **Development (YAML)**

- Load time: ~1-5ms for small templates
- Parse time: ~10-50ms for large templates (1000+ entities)
- Memory: ~1KB per entity

### **Production (Compiled Bincode)**

- Load time: ~0.1-0.5ms (10-50x faster)
- Parse time: ~1-5ms (10x faster)
- Memory: ~500 bytes per entity (smaller)

**Recommendation:** Use YAML in development, compile to Bincode for production.

---

## Integration with Other Systems

### **Networking**

Templates with `tags: [replicate]` are sent over network:

```yaml
entities:
  Player:
    components:
      Transform: ...
      Health: ...
      tags: [replicate]  # Server sends to clients
```

### **Interest Management**

Templates with `tags: [static]` are not replicated:

```yaml
entities:
  Ground:
    components:
      Transform: ...
      tags: [static]  # Client loads from local file
```

### **Hot-Reload**

Template changes are detected and reloaded automatically:

```rust
let mut hot_reloader = TemplateHotReloader::new();
hot_reloader.watch("assets/templates");

// On file change, reload template
hot_reloader.process_reloads(&mut world, &mut loader);
```

---

## Future Features (Phase 2+)

- [ ] **Visual editor** - Drag-and-drop template editing
- [ ] **Template diffing** - Show changes between versions
- [ ] **Template merging** - Combine multiple templates
- [ ] **Template validation in CI** - Pre-commit hooks
- [ ] **Template analytics** - Usage tracking, dead templates
- [ ] **Template inheritance** - Base templates with variations
- [ ] **Template macros** - Parameterized templates

---

## Error Handling

All operations return `TemplateResult<T>`:

```rust
pub type TemplateResult<T> = Result<T, TemplateError>;

pub enum TemplateError {
    NotFound(PathBuf),
    AlreadyExists(PathBuf),
    InvalidYaml(String),
    UnknownComponent(String),
    CircularReference(String),
    Io(std::io::Error),
    // ...
}
```

**Example:**
```rust
match create_template("assets/templates", options) {
    Ok(path) => println!("Created: {}", path.display()),
    Err(TemplateError::AlreadyExists(path)) => {
        eprintln!("Template already exists: {}", path.display());
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Testing

See `engine/templating/tests/` for comprehensive tests:

- ✅ Template creation
- ✅ Template loading
- ✅ Template validation
- ✅ Template references
- ✅ Circular dependency detection
- ✅ Component parsing
- ✅ Error handling

---

## See Also

- [ECS Architecture](ecs.md)
- [Networking](networking.md)
- [Serialization](serialization.md)
- [CLI Commands](rules/xtask-commands.md)
