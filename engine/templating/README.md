# Template System

High-performance template system for the Silmaril game engine with automatic bincode compilation for 10-50x faster loading.

## Features

- **YAML Templates**: Human-readable template format for authoring
- **Bincode Compilation**: Automatic compilation to optimized binary format
- **Auto-Detection**: Transparently uses `.bin` files when available
- **Checksum Validation**: Ensures data integrity with xxHash64
- **Caching**: Arc-based template caching for zero-cost cloning
- **Hot Reload**: Watch and recompile templates automatically (coming soon)

## Quick Start

### Creating Templates

Use the CLI to create a new template:

```bash
silm template add my_level --type level --description "First level"
```

This creates `assets/templates/levels/my_level.yaml`:

```yaml
metadata:
  name: "my_level"
  description: "First level"
  author: "Your Name"
  version: "1.0"

entities:
  Root:
    components:
      Transform:
        position: [0, 0, 0]
        rotation: [0, 0, 0, 1]
        scale: [1, 1, 1]
    tags: []
```

### Compiling Templates

Compile a template to bincode for faster loading:

```bash
# Compile single template
silm template compile assets/templates/levels/my_level.yaml

# Compile all templates in directory
silm template compile assets/templates --all
```

This creates `my_level.bin` (10-50x faster to load than YAML).

### Loading Templates

The loader automatically uses `.bin` files when available:

```rust
use engine_templating::TemplateLoader;
use engine_core::ecs::World;

let mut world = World::new();
let mut loader = TemplateLoader::new();

// Automatically uses my_level.bin if it exists, otherwise my_level.yaml
let instance = loader.load(&mut world, "assets/templates/levels/my_level.yaml")?;

println!("Loaded {} entities", instance.entities.len());
```

### Manual Compilation

You can also compile templates programmatically:

```rust
use engine_templating::TemplateCompiler;
use std::path::Path;

let compiler = TemplateCompiler::new();

// Compile single template
compiler.compile(
    Path::new("assets/templates/player.yaml"),
    Path::new("assets/templates/player.bin")
)?;

// Compile entire directory
let count = compiler.compile_directory(Path::new("assets/templates"))?;
println!("Compiled {} templates", count);
```

## Performance

Bincode templates provide significant performance improvements:

| Metric | YAML | Bincode | Improvement |
|--------|------|---------|-------------|
| Load Time | ~5ms | ~0.1-0.5ms | **10-50x faster** |
| File Size | 1000 bytes | 200-400 bytes | **50-80% smaller** |
| Memory | High allocation | Low allocation | **Zero-copy friendly** |

Run benchmarks to verify:

```bash
cargo bench --package engine-templating yaml_vs_bincode
```

## Template Format

Templates support:

- **Inline entities**: Define components directly
- **References**: Reference other templates
- **Hierarchies**: Parent-child relationships
- **Overrides**: Override component values
- **Tags**: Entity classification

Example with all features:

```yaml
metadata:
  name: "Guard Tower"
  description: "Defensive structure"
  version: "1.0"

entities:
  Root:
    components:
      Transform:
        position: [0, 0, 0]
        rotation: [0, 0, 0, 1]
        scale: [1, 1, 1]
      Health:
        current: 500
        max: 500
      MeshRenderer:
        mesh: "models/guard_tower.obj"
        visible: true
    tags: ["structure", "defense"]

    children:
      Guard:
        # Reference another template
        template: "templates/characters/guard.yaml"
        # Override the referenced template
        overrides:
          Transform:
            position: [0, 5, 0]
          Health:
            current: 150
            max: 150
```

## CLI Commands

### Create Template

```bash
silm template add <name> --type <type> [--description <desc>] [--author <author>]
```

Types: `level`, `character`, `prop`, `ui`, `game_state`

### Validate Template

```bash
silm template validate <path>
```

Checks for:
- YAML syntax errors
- Missing required fields
- Invalid component references
- Circular dependencies

### Compile Template

```bash
silm template compile <path> [--output <output>] [--all] [--watch]
```

Options:
- `--output`: Specify output path (default: same name with `.bin`)
- `--all`: Compile all templates in directory recursively
- `--watch`: Watch for changes and recompile (coming soon)

### List Templates

```bash
silm template list [base_path]
```

Lists all templates in the specified directory (default: `assets/templates`).

### Show Template Tree

```bash
silm template tree <path>
```

Displays the entity hierarchy of a template.

### Rename Template

```bash
silm template rename <path> <new_name>
```

### Delete Template

```bash
silm template delete <path> [--yes]
```

Use `--yes` to skip confirmation prompt.

## Architecture

### Modules

- **compiler.rs**: YAML → Bincode compilation with checksum validation
- **loader.rs**: Auto-detecting template loader (prefers .bin, falls back to .yaml)
- **template.rs**: Core template data structures
- **validator.rs**: Template validation and error checking
- **cache.rs**: Template caching system
- **error.rs**: Structured error types

### Compilation Format

Compiled templates use a custom format:

```
┌────────────────────┐
│ Magic Number (u32) │  0x53494C4D ("SILM")
├────────────────────┤
│ Version (u32)      │  Format version (currently 1)
├────────────────────┤
│ Checksum (u64)     │  xxHash64 of template data
├────────────────────┤
│ Template Data      │  Bincode-serialized Template
└────────────────────┘
```

The loader validates:
1. Magic number matches expected value
2. Format version is compatible
3. Checksum matches computed hash

## Testing

Run all tests:

```bash
cargo test --package engine-templating
```

Run specific test suites:

```bash
# Bincode integration tests
cargo test --package engine-templating bincode_integration_test

# Compiler unit tests
cargo test --package engine-templating --lib compiler

# Loader tests
cargo test --package engine-templating loader_tests
```

## Benchmarks

Run all benchmarks:

```bash
cargo bench --package engine-templating
```

Run specific benchmarks:

```bash
# YAML vs Bincode comparison
cargo bench --package engine-templating yaml_vs_bincode

# Template loading performance
cargo bench --package engine-templating template_loading

# Memory usage comparison
cargo bench --package engine-templating memory_usage
```

## Best Practices

1. **Author in YAML**: Always author templates in YAML for readability
2. **Compile for Production**: Compile to bincode before shipping
3. **Version Control YAML Only**: Don't commit `.bin` files (regenerate on build)
4. **Validate Before Compiling**: Use `silm template validate` to catch errors
5. **Use References**: Reuse templates via references instead of duplication
6. **Leverage Caching**: TemplateLoader caches loaded templates automatically

## Workflow

Development workflow:

```bash
# 1. Create template
silm template add my_entity --type character

# 2. Edit the YAML file
vim assets/templates/characters/my_entity.yaml

# 3. Validate
silm template validate assets/templates/characters/my_entity.yaml

# 4. Compile (optional for dev, required for production)
silm template compile assets/templates/characters/my_entity.yaml

# 5. Load in game
# Automatically uses .bin if available, falls back to .yaml
```

Production workflow:

```bash
# Compile all templates before deployment
silm template compile assets/templates --all

# Ship .bin files for fast loading
# Keep .yaml files in source control
```

## Future Enhancements

- [ ] Watch mode for continuous compilation
- [ ] Compression options (zstd, lz4)
- [ ] Hot reload support
- [ ] Template inheritance
- [ ] Procedural generation support
- [ ] Version migration tools
- [ ] Visual template editor

## See Also

- [docs/templating.md](../../docs/templating.md) - Complete templating guide
- [CLAUDE.md](../../CLAUDE.md) - Project development guide
- [examples/](../../examples/) - Example templates and usage
