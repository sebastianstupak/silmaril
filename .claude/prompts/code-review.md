# Code Review Checklist

> Comprehensive code review protocol for silmaril

---

## Overview

This prompt guides thorough code reviews to ensure all code meets project standards before merge.

**Reference:** `docs/rules/coding-standards.md`

---

## Pre-Review Setup

Before starting the review:

1. **Load Context:**
   ```
   - Read docs/rules/coding-standards.md
   - Read docs/architecture.md
   - Read relevant task file from docs/tasks/
   - Check performance targets from docs/performance-targets.md
   ```

2. **Understand Changes:**
   ```
   git diff main...current-branch
   git log main...current-branch
   ```

3. **Run Automated Checks:**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test --all-features
   cargo doc --no-deps
   ```

---

## Critical Violations (MUST FIX)

These violations **BLOCK MERGE** immediately:

### 🚫 1. Forbidden Functions

**Check for:**
```rust
// ❌ FORBIDDEN - Automatic rejection
println!("...");
eprintln!("...");
dbg!(value);
print!("...");
eprint!("...");
```

**Required fix:**
```rust
// ✅ CORRECT
use tracing::{info, warn, error, debug};

info!(player_id = %id, "Player joined");
error!(error = ?e, "Operation failed");
debug!(value = ?value, "Debug info");
```

**Verification:**
```bash
# Must pass (configured in .cargo/config.toml)
cargo clippy -- -D clippy::print_stdout -D clippy::print_stderr -D clippy::dbg_macro
```

---

### 🚫 2. Unwrap/Expect/Panic

**Check for:**
```rust
// ❌ FORBIDDEN
value.unwrap()
value.expect("message")
panic!("error")
unimplemented!()
todo!()  // Only allowed in WIP, not merged code
```

**Required fix:**
```rust
// ✅ CORRECT
value.map_err(|e| CustomError::from(e))?
value.ok_or(CustomError::NotFound)?

// OR with proper context
match value {
    Some(v) => v,
    None => return Err(CustomError::NotFound),
}
```

**Verification:**
```bash
cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
```

---

### 🚫 3. Generic Error Types

**Check for:**
```rust
// ❌ FORBIDDEN
fn load() -> Result<Data, Box<dyn Error>> { }
fn init() -> anyhow::Result<()> { }
fn parse() -> Result<Config, String> { }
```

**Required fix:**
```rust
// ✅ CORRECT
use silmaril_core::error::{define_error, ErrorCode, ErrorSeverity};

define_error! {
    pub enum LoadError {
        NotFound { path: String } = ErrorCode::NotFound, ErrorSeverity::Error,
        InvalidFormat { reason: String } = ErrorCode::InvalidData, ErrorSeverity::Error,
        IoError { source: std::io::Error } = ErrorCode::Io, ErrorSeverity::Error,
    }
}

fn load(path: &str) -> Result<Data, LoadError> {
    // Implementation
}
```

---

### 🚫 4. Unsafe Code Outside FFI

**Check for:**
```rust
// ❌ FORBIDDEN (in business logic)
unsafe fn calculate() { }
unsafe {
    // Raw pointer manipulation
}
```

**Allowed only in:**
```rust
// ✅ ALLOWED (Vulkan FFI, platform APIs only)
#[cfg(feature = "renderer")]
mod vulkan {
    unsafe fn create_instance(
        create_info: &vk::InstanceCreateInfo
    ) -> Result<vk::Instance, VulkanError> {
        // Vulkan FFI requires unsafe
    }
}
```

**Verification:**
- Check that unsafe is ONLY in:
  - `engine/renderer/src/vulkan/` (Vulkan FFI)
  - `engine/platform/src/` (Win32, X11, etc.)
- ALL unsafe blocks must have safety comments

---

### 🚫 5. Platform-Specific Code in Business Logic

**Check for:**
```rust
// ❌ FORBIDDEN
fn update(world: &mut World) {
    #[cfg(windows)]
    let time = get_windows_time();

    #[cfg(unix)]
    let time = get_unix_time();

    // Business logic
}
```

**Required fix:**
```rust
// ✅ CORRECT - Platform abstraction via trait
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

fn update(world: &mut World, clock: &dyn Clock) {
    let time = clock.now();
    // Business logic
}

// Platform-specific implementations in platform crate
#[cfg(windows)]
impl Clock for WindowsClock { }

#[cfg(unix)]
impl Clock for UnixClock { }
```

**Verification:**
- No `#[cfg(windows)]`, `#[cfg(unix)]`, etc. in:
  - `engine/core/`
  - `engine/networking/`
  - Game logic
- Platform abstraction used via traits

---

## Documentation Review

### 1. Public API Documentation

**Check:**
```rust
// ❌ MISSING DOCS
pub fn spawn(&mut self) -> Result<Entity, WorldError> { }

// ✅ COMPLETE
/// Spawns a new entity in the world.
///
/// # Examples
///
/// ```
/// use silmaril::*;
///
/// let mut world = World::new();
/// let entity = world.spawn()?;
/// ```
///
/// # Errors
///
/// Returns [`WorldError::EntityLimitReached`] if max entities exceeded.
pub fn spawn(&mut self) -> Result<Entity, WorldError> {
    // Implementation
}
```

**Requirements:**
- [ ] All `pub fn` have rustdoc comments
- [ ] Examples provided for non-trivial functions
- [ ] Errors section documents all error cases
- [ ] Panics section if function can panic
- [ ] Safety section for unsafe functions

**Verification:**
```bash
cargo doc --no-deps
# Check for warnings about missing documentation
```

---

### 2. Module Documentation

**Check:**
```rust
// ❌ MISSING
mod entity;

// ✅ COMPLETE
/// Entity management for the ECS.
///
/// This module provides the core entity allocation and lifetime management.
/// Entities are allocated using generational indices for safety.
mod entity;
```

---

## Test Coverage Review

### 1. Unit Tests Required

**Check for:**
- [ ] Every public function has at least one test
- [ ] Error cases are tested
- [ ] Edge cases are covered
- [ ] Tests are organized in `#[cfg(test)] mod tests`

**Example:**
```rust
pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
    self.components.insert(entity, component);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_component_success() {
        let mut world = World::new();
        let entity = world.spawn().unwrap();

        world.add_component(entity, Transform::default());
        assert!(world.get::<Transform>(entity).is_some());
    }

    #[test]
    fn test_add_component_invalid_entity() {
        let mut world = World::new();
        let invalid = Entity::from_raw(999);

        let result = world.add_component(invalid, Transform::default());
        assert!(result.is_err());
    }
}
```

---

### 2. Integration Tests

**Required for:**
- Cross-platform functionality
- End-to-end workflows
- Subsystem integration

**Location:** `tests/` directory

---

### 3. Property-Based Tests

**Required for:**
- Serialization/deserialization (roundtrip)
- State synchronization
- Complex algorithms

**Example:**
```rust
#[cfg(test)]
mod prop_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_serialization_roundtrip(entities in prop::collection::vec(any::<Entity>(), 0..100)) {
            let world = create_world_with_entities(&entities);
            let serialized = world.serialize()?;
            let deserialized = World::deserialize(&serialized)?;

            prop_assert_eq!(world, deserialized);
        }
    }
}
```

---

## Performance Review

### 1. Check Against Targets

**Reference:** `docs/performance-targets.md`

**Verify:**
```rust
// Add benchmarks for critical paths
#[cfg(test)]
mod benches {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_spawn_entities(c: &mut Criterion) {
        c.bench_function("spawn 10k entities", |b| {
            b.iter(|| {
                let mut world = World::new();
                for _ in 0..10_000 {
                    world.spawn().unwrap();
                }
            });
        });
    }

    criterion_group!(benches, bench_spawn_entities);
    criterion_main!(benches);
}
```

**Targets to verify:**
- Spawn 10k entities < 1ms
- Query 10k entities < 0.5ms
- Serialize 1000 entities < 5ms (bincode)
- Network latency overhead < 5ms

---

### 2. Common Performance Issues

**Check for:**

❌ **Unnecessary allocations:**
```rust
// BAD
fn get_entities(&self) -> Vec<Entity> {
    self.entities.clone()
}

// GOOD
fn get_entities(&self) -> &[Entity] {
    &self.entities
}
```

❌ **Missing iterator chains:**
```rust
// BAD
let mut result = Vec::new();
for entity in &entities {
    if entity.is_alive() {
        result.push(entity.id());
    }
}

// GOOD
let result: Vec<_> = entities
    .iter()
    .filter(|e| e.is_alive())
    .map(|e| e.id())
    .collect();
```

❌ **Individual operations instead of batch:**
```rust
// BAD
for entity in entities {
    world.spawn_with_components(entity);
}

// GOOD
world.spawn_batch(entities);
```

---

## Security Review

### 1. Input Validation

**Check server-side validation:**
```rust
#[server_only]
fn process_input(input: &PlayerInput) -> Result<(), ValidationError> {
    // ✅ REQUIRED: Validate on server
    if !input.is_physically_possible() {
        return Err(ValidationError::ImpossibleAction);
    }

    if input.timestamp < last_input_time {
        return Err(ValidationError::OutOfOrder);
    }

    // Process
}
```

---

### 2. Sanitize User Data

**Check for:**
- Path traversal prevention
- SQL injection (if using DB)
- Command injection
- Buffer overflows (in unsafe)

---

### 3. Network Security

**Verify:**
- [ ] Server validates ALL client inputs
- [ ] Rate limiting in place
- [ ] Anti-cheat measures for movement/actions
- [ ] Fog of war prevents information leaks

---

## Architecture Review

### 1. ECS Patterns

**Check:**
- [ ] Components are data-only (no methods except derive)
- [ ] Systems operate on components via queries
- [ ] No direct entity references (use Entity ID)
- [ ] Proper separation of concerns

**Example:**
```rust
// ✅ CORRECT
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

// System operates on components
fn movement_system(
    query: Query<(&Transform, &mut Velocity)>,
    delta_time: f32,
) {
    for (transform, velocity) in query.iter() {
        // Update logic
    }
}
```

---

### 2. Error Handling Patterns

**Check:**
- [ ] Errors propagate up with `?`
- [ ] Error context is preserved
- [ ] Recovery strategies are clear
- [ ] User-facing errors are helpful

**Example:**
```rust
// ✅ CORRECT
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::ReadFailed {
            path: path.to_string(),
            source: e,
        })?;

    let config = serde_yaml::from_str(&contents)
        .map_err(|e| ConfigError::ParseFailed {
            path: path.to_string(),
            source: e,
        })?;

    Ok(config)
}
```

---

### 3. Client/Server Separation

**Check:**
```rust
// ✅ CORRECT - Attributes used
#[server_only]
fn calculate_damage() { }

#[client_only]
fn render_effect() { }

#[shared_system]
fn physics_step() { }
```

**Verify:**
- [ ] Server-only code is not in client binary
- [ ] Client-only code is not in server binary
- [ ] Shared code is properly marked

---

## Code Quality Review

### 1. Naming Conventions

**Check:**
```rust
// ✅ CORRECT
pub struct VulkanRenderer { }      // Types: PascalCase
pub enum ComponentData { }
pub trait PhysicsBackend { }

pub fn spawn_entity() { }          // Functions: snake_case
pub fn add_component() { }

pub const MAX_ENTITIES: usize = 100_000;  // Constants: SCREAMING_SNAKE_CASE
pub const DEFAULT_PORT: u16 = 8080;

mod entity_manager;                // Modules: snake_case
mod network_client;
```

---

### 2. Code Complexity

**Check for:**
- Functions > 50 lines (consider splitting)
- Nested if/match > 3 levels (refactor)
- Duplicated code (extract to function)
- Magic numbers (use named constants)

---

### 3. Anti-Patterns

**Watch for:**

❌ **Stringly-typed:**
```rust
// BAD
fn get_component(&self, name: &str) -> Option<&dyn Component>;

// GOOD
fn get_component<T: Component>(&self) -> Option<&T>;
```

❌ **Boolean parameters:**
```rust
// BAD
fn create(true, false, true);

// GOOD
fn create(config: CreateConfig {
    resizable: true,
    fullscreen: false,
    vsync: true,
});
```

❌ **Premature optimization:**
```rust
// BAD (before profiling)
fn update(&mut self) {
    unsafe {
        // Complex SIMD assembly
    }
}

// GOOD (simple first)
fn update(&mut self) {
    for entity in &self.entities {
        entity.update();
    }
}
```

---

## Final Checklist

Before approving merge:

### Automated Checks
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test --all-features` passes (100%)
- [ ] `cargo doc --no-deps` builds without warnings

### Manual Checks
- [ ] No forbidden functions (println!, unwrap!, etc.)
- [ ] Custom error types used
- [ ] No unsafe except in FFI
- [ ] Platform abstraction used
- [ ] All public APIs documented
- [ ] Tests cover new code
- [ ] Performance targets met (if applicable)
- [ ] Security reviewed (if network/server code)
- [ ] Architecture patterns followed

### Documentation
- [ ] ROADMAP.md updated if task completed
- [ ] Commit message follows conventional commits
- [ ] Code comments explain "why", not "what"

---

## Review Feedback Template

When providing review feedback:

```markdown
## Code Review: [PR Title]

### Summary
[Brief overview of changes]

### Critical Issues (MUST FIX) 🚫
1. **[Issue]** (Line X)
   - Problem: [Description]
   - Fix: [Solution]
   - Reference: [coding-standards.md section]

### Suggestions (SHOULD FIX) ⚠️
1. **[Suggestion]** (Line Y)
   - Current: [What's there]
   - Suggested: [Better approach]
   - Reason: [Why]

### Questions ❓
1. [Question about design decision]

### Positive Notes ✅
- [What was done well]
- [Good patterns used]

### Verdict
- [ ] Approved (no blocking issues)
- [ ] Approved with suggestions (non-blocking)
- [ ] Changes requested (blocking issues)

### Next Steps
1. [Action item 1]
2. [Action item 2]
```

---

**Last Updated:** 2026-02-01
