# Phase 1.4: Template System

> **Unified entity template system**
>
> Templates are YAML files that define entities without IDs. Single format for levels, characters, props, UI, and game state.

---

## Overview

Implement a complete template system with three layers:
1. **Core Layer** - Data structures, loader, validator, compiler
2. **Operations Layer** - Shared business logic (create, validate, compile)
3. **Interface Layer** - CLI, Editor API, Agent API

**No distinction between "scenes" and "prefabs"** - everything is a template that can be nested.

---

## Architecture

### **Vertical Slice: `engine/templating/`**

```
engine/templating/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── template.rs         # Template, EntityDefinition structs
│   ├── loader.rs           # TemplateLoader (spawn into world)
│   ├── operations.rs       # Shared operations (create, validate, etc.)
│   ├── validator.rs        # Template validation
│   ├── compiler.rs         # YAML → Bincode compilation
│   └── error.rs            # TemplateError
│
├── benches/
│   ├── template_loading.rs     # Load performance
│   ├── template_spawning.rs    # Spawn performance
│   ├── template_validation.rs  # Validation performance
│   └── yaml_vs_bincode.rs      # Format comparison
│
└── tests/
    ├── template_tests.rs           # Unit tests
    ├── loader_tests.rs             # Loader tests
    ├── operations_tests.rs         # Operations tests
    ├── validator_tests.rs          # Validator tests
    ├── circular_deps_test.rs       # Edge cases
    ├── error_handling_test.rs      # Error scenarios
    └── fixtures/
        └── test_templates/
            ├── simple.yaml
            ├── nested.yaml
            ├── circular.yaml
            └── invalid.yaml
```

---

## Tasks

### **Task 1: Core Data Structures** ✅

**File:** `engine/templating/src/template.rs`

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

pub struct EntityDefinition {
    pub source: EntitySource,
    pub overrides: HashMap<String, serde_yaml::Value>,
    pub children: HashMap<String, EntityDefinition>,
}

pub enum EntitySource {
    Inline {
        components: HashMap<String, serde_yaml::Value>,
        tags: Vec<String>,
    },
    Reference {
        template: String,
    },
}
```

**Tests:** (Unit tests in `tests/template_tests.rs`)
- [ ] Template creation
- [ ] Entity addition/removal
- [ ] Serialization/deserialization
- [ ] Metadata handling

**Acceptance:**
- ✅ All structs derive Serialize, Deserialize
- ✅ Docs with examples
- ✅ Unit tests pass

---

### **Task 2: Template Loader** ✅

**File:** `engine/templating/src/loader.rs`

```rust
pub struct TemplateLoader {
    cache: HashMap<String, Template>,
}

impl TemplateLoader {
    pub fn load(
        &mut self,
        world: &mut World,
        path: &str,
    ) -> TemplateResult<TemplateInstance>

    fn spawn_entity(
        &mut self,
        world: &mut World,
        name: &str,
        def: &EntityDefinition,
    ) -> TemplateResult<Entity>
}

pub struct TemplateInstance {
    pub name: String,
    pub entities: Vec<Entity>,
    pub references: Vec<TemplateInstance>,
}
```

**Features:**
- ✅ Load template from YAML file
- ✅ Spawn entities into World
- ✅ Resolve template references recursively
- ✅ Apply component overrides
- ✅ Cache loaded templates
- ✅ Handle children hierarchies

**Tests:** (Integration tests in `tests/loader_tests.rs`)
- [ ] Load simple template
- [ ] Load nested template (with children)
- [ ] Load template with references
- [ ] Apply overrides correctly
- [ ] Cache prevents duplicate loads
- [ ] Despawn template instance

**Benchmarks:** (`benches/template_loading.rs`)
- [ ] Load small template (1 entity)
- [ ] Load medium template (100 entities)
- [ ] Load large template (1000 entities)
- [ ] Load with references (10 nested templates)
- [ ] Cache hit performance

**Performance Targets:**
- Small template: < 1ms
- Medium template: < 10ms
- Large template: < 100ms
- Cache hit: < 0.1ms

**Acceptance:**
- ✅ All tests pass
- ✅ Benchmarks meet targets
- ✅ Docs with examples

---

### **Task 3: Template Validator** ✅

**File:** `engine/templating/src/validator.rs`

```rust
pub struct TemplateValidator;

impl TemplateValidator {
    pub fn validate(
        &self,
        template: &Template,
        template_path: &Path,
    ) -> TemplateResult<ValidationReport>
}

pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub entity_count: usize,
    pub template_references: Vec<String>,
}
```

**Validation Checks:**
- ✅ YAML syntax is valid
- ✅ All component types exist
- ✅ Template references exist
- ✅ No circular dependencies
- ✅ Component fields are valid
- ✅ Tags are recognized
- ⚠️ Warnings for unused entities
- ⚠️ Warnings for missing metadata

**Tests:** (`tests/validator_tests.rs`)
- [ ] Valid template passes
- [ ] Invalid YAML fails
- [ ] Unknown component fails
- [ ] Missing template reference fails
- [ ] Circular dependency detected
- [ ] Warnings for unused entities

**Tests:** (`tests/circular_deps_test.rs`)
- [ ] Direct circular ref (A → A)
- [ ] Indirect circular ref (A → B → A)
- [ ] Deep circular ref (A → B → C → A)

**Benchmarks:** (`benches/template_validation.rs`)
- [ ] Validate small template
- [ ] Validate large template
- [ ] Validate with many references

**Performance Targets:**
- Small template: < 1ms
- Large template: < 50ms

**Acceptance:**
- ✅ All validation checks implemented
- ✅ All tests pass
- ✅ Benchmarks meet targets

---

### **Task 4: Template Compiler** ✅

**File:** `engine/templating/src/compiler.rs`

```rust
pub struct TemplateCompiler;

impl TemplateCompiler {
    pub fn compile(
        &self,
        template_path: &Path,
        output_path: &Path,
    ) -> TemplateResult<CompiledTemplate>
}

pub struct CompiledTemplate {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub size_bytes: u64,
    pub checksum: u64,
}
```

**Features:**
- ✅ Load YAML template
- ✅ Serialize to Bincode
- ✅ Compute checksum
- ✅ Write to output file
- ✅ Compression optional

**Tests:** (`tests/compiler_tests.rs`)
- [ ] Compile simple template
- [ ] Compiled file is valid Bincode
- [ ] Checksums match
- [ ] Roundtrip (YAML → Bincode → YAML)

**Benchmarks:** (`benches/yaml_vs_bincode.rs`)
- [ ] Load YAML template (parse time)
- [ ] Load Bincode template (parse time)
- [ ] File size comparison
- [ ] Memory usage comparison

**Performance Targets:**
- Bincode load: 10-50x faster than YAML
- Bincode size: 50-80% smaller than YAML

**Acceptance:**
- ✅ All tests pass
- ✅ Benchmarks show speedup
- ✅ Docs explain when to use

---

### **Task 5: Operations Layer** ✅

**File:** `engine/templating/src/operations.rs`

**Shared business logic for CLI/Editor/Agent:**

```rust
pub fn create_template(
    base_path: impl AsRef<Path>,
    options: CreateTemplateOptions,
) -> TemplateResult<PathBuf>

pub fn validate_template(
    template_path: impl AsRef<Path>,
) -> TemplateResult<ValidationReport>

pub fn compile_template(
    template_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> TemplateResult<CompiledTemplate>

pub fn list_templates(
    base_path: impl AsRef<Path>,
) -> TemplateResult<Vec<TemplateInfo>>

pub fn show_template_tree(
    template_path: impl AsRef<Path>,
) -> TemplateResult<TemplateTree>

pub fn rename_template(
    old_path: impl AsRef<Path>,
    new_name: &str,
) -> TemplateResult<PathBuf>

pub fn delete_template(
    path: impl AsRef<Path>,
) -> TemplateResult<()>
```

**Tests:** (`tests/operations_tests.rs`)
- [ ] create_template creates file
- [ ] create_template fails if exists
- [ ] validate_template returns report
- [ ] compile_template creates .bin file
- [ ] list_templates finds all templates
- [ ] show_template_tree shows hierarchy
- [ ] rename_template renames file
- [ ] delete_template removes file

**Tests:** (`tests/error_handling_test.rs`)
- [ ] Create template in non-existent directory
- [ ] Validate non-existent file
- [ ] Compile invalid template
- [ ] Rename to existing name
- [ ] Delete non-existent file

**Acceptance:**
- ✅ All operations implemented
- ✅ All tests pass
- ✅ Error messages are clear
- ✅ Docs with examples

---

### **Task 6: CLI Commands** ✅

**File:** `engine/cli/src/commands/template.rs`

**Thin wrapper around operations:**

```bash
silm template add <name> --type <level|character|prop|ui|game_state>
silm template validate <path>
silm template compile <path> [--output <dir>]
silm template list [<base_path>]
silm template tree <path>
silm template rename <path> <new_name>
silm template delete <path> [--yes]
```

**Tests:** (Integration tests in `engine/cli/tests/template_cli_test.rs`)
- [ ] `silm template add` creates file
- [ ] `silm template validate` shows errors
- [ ] `silm template compile` creates .bin
- [ ] `silm template list` shows all templates
- [ ] `silm template tree` shows hierarchy
- [ ] `silm template rename` renames file
- [ ] `silm template delete` removes file

**Acceptance:**
- ✅ All commands work
- ✅ Help text is clear
- ✅ Error messages are helpful
- ✅ Integration tests pass

---

### **Task 7: Error Handling** ✅

**File:** `engine/templating/src/error.rs`

```rust
use silmaril_core::define_error;

define_error! {
    pub enum TemplateError {
        NotFound { path: String } = ErrorCode::TemplateNotFound, ErrorSeverity::Error,
        AlreadyExists { path: String } = ErrorCode::TemplateAlreadyExists, ErrorSeverity::Error,
        InvalidYaml { reason: String } = ErrorCode::TemplateInvalidYaml, ErrorSeverity::Error,
        UnknownComponent { component: String } = ErrorCode::TemplateUnknownComponent, ErrorSeverity::Error,
        CircularReference { path: String } = ErrorCode::TemplateCircularReference, ErrorSeverity::Error,
        Io { source: std::io::Error } = ErrorCode::TemplateIo, ErrorSeverity::Error,
        Serialization { reason: String } = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
    }
}

pub type TemplateResult<T> = Result<T, TemplateError>;
```

**Tests:** (`tests/error_handling_test.rs`)
- [ ] NotFound error with correct path
- [ ] AlreadyExists error with path
- [ ] InvalidYaml with reason
- [ ] CircularReference detection
- [ ] Io errors wrapped correctly

**Acceptance:**
- ✅ Follows error handling guidelines
- ✅ All error types covered
- ✅ Error messages are clear

---

## Testing Strategy (Test Pyramid)

### **Unit Tests** (Bottom of Pyramid) 🟢

**Location:** `engine/templating/tests/`

**Coverage:**
- [ ] Template struct creation
- [ ] Entity definition parsing
- [ ] Component parsing
- [ ] Metadata handling
- [ ] Error type creation

**Count:** ~20 tests

---

### **Integration Tests** (Middle of Pyramid) 🟡

**Location:** `engine/templating/tests/` + `engine/shared/tests/`

**Coverage:**
- [ ] Load template from file
- [ ] Spawn entities into World
- [ ] Resolve template references
- [ ] Validate templates
- [ ] Compile templates
- [ ] Operations (create, list, etc.)
- [ ] Error scenarios

**Count:** ~30 tests

---

### **E2E Tests** (Top of Pyramid) 🔴

**Location:** `engine/cli/tests/`

**Coverage:**
- [ ] Full CLI workflow (create → validate → compile)
- [ ] Multi-template references
- [ ] Hot-reload scenario
- [ ] Editor API workflow

**Count:** ~10 tests

---

### **Benchmarks** ⚡

**Location:** `engine/templating/benches/`

**Coverage:**
- [ ] Template loading (YAML)
- [ ] Template loading (Bincode)
- [ ] Template spawning (small/medium/large)
- [ ] Template validation
- [ ] Template compilation
- [ ] Cache performance

**Count:** ~15 benchmarks

**Performance Targets:**
```
Template Loading (YAML):
  Small (1 entity):       < 1ms
  Medium (100 entities):  < 10ms
  Large (1000 entities):  < 100ms

Template Loading (Bincode):
  Small:                  < 0.1ms  (10x faster)
  Medium:                 < 1ms    (10x faster)
  Large:                  < 10ms   (10x faster)

Template Spawning:
  100 entities:           < 5ms
  1000 entities:          < 50ms

Template Validation:
  Small:                  < 1ms
  Large:                  < 50ms

Cache Hit:                < 0.1ms
```

---

## Dependencies

### **Crate Dependencies**

```toml
[dependencies]
engine-core = { path = "../core" }
engine-math = { path = "../math" }

serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
bincode = "1.3"

walkdir = "2.5"
tracing = "0.1"
thiserror = "1.0"

[dev-dependencies]
tempfile = "3.10"
criterion = "0.5"
```

### **Other Systems**

- ✅ `engine-core` (World, Entity, Component)
- ✅ `engine-serialization` (ComponentData)
- ⏸️ `engine-assets` (Optional: asset loading)

---

## Acceptance Criteria

### **Phase 1.4 Complete When:**

- [x] All 7 tasks complete
- [x] Template system crate compiles
- [x] All unit tests pass (~20 tests)
- [x] All integration tests pass (~30 tests)
- [x] All E2E tests pass (~10 tests)
- [x] All benchmarks run (~15 benchmarks)
- [x] Performance targets met
- [x] CLI commands work
- [x] Documentation complete
- [x] Code review checklist passes

### **Code Review Checklist:**

- [ ] No println!/eprintln!/dbg! (use tracing)
- [ ] Custom error types (TemplateError)
- [ ] All public APIs documented
- [ ] Benchmarks included
- [ ] Tests follow pyramid (20/30/10 split)
- [ ] No unsafe code without justification
- [ ] Error messages are helpful

---

## Timeline

**Estimated:** 2-3 days (with parallel agents)

**Breakdown:**
- Task 1 (Data structures): 2 hours
- Task 2 (Loader): 4 hours
- Task 3 (Validator): 3 hours
- Task 4 (Compiler): 2 hours
- Task 5 (Operations): 3 hours
- Task 6 (CLI): 2 hours
- Task 7 (Error handling): 1 hour
- Tests: 4 hours
- Benchmarks: 2 hours
- Documentation: 2 hours

**Total:** ~25 hours (can be parallelized)

---

## Out of Scope (Phase 2+)

- [ ] Visual template editor (Phase 0.8)
- [ ] Template hot-reload (Phase 0.7)
- [ ] Template diffing/merging
- [ ] Template validation in CI
- [ ] Template analytics
- [ ] Template inheritance
- [ ] Template macros/parameterization

---

## Success Metrics

- ✅ Template creation in < 100ms
- ✅ Template loading 10x faster with Bincode
- ✅ Zero runtime allocations for cached templates
- ✅ 100% test coverage on core logic
- ✅ CLI commands feel responsive
- ✅ Error messages are clear and actionable

---

## See Also

- [Template System Documentation](../templating.md)
- [ECS Architecture](../ecs.md)
- [Error Handling](../error-handling.md)
- [Testing Strategy](../testing-strategy.md)
