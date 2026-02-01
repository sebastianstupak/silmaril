# Coding Standards

> **Enforced code style and quality rules**
>
> ⚠️ **MANDATORY** for all code contributions

---

## 🎯 **Core Rules**

### **1. No Printing - Use `tracing` Only**

```toml
# .cargo/config.toml
[lints.clippy]
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"
```

```rust
// ❌ FORBIDDEN
println!("Player joined");
eprintln!("Error: {}", e);
dbg!(value);

// ✅ CORRECT
use tracing::{info, warn, error, debug};

info!(player_id = %id, "Player joined");
error!(error = ?e, "Operation failed");
debug!(value = ?value, "Debug info");
```

---

### **2. Custom Error Types Always**

```rust
// ❌ FORBIDDEN
fn load() -> Result<Data, Box<dyn Error>> { }
fn init() -> anyhow::Result<()> { }

// ✅ CORRECT
define_error! {
    pub enum LoadError {
        NotFound { path: String } = ErrorCode::NotFound, ErrorSeverity::Error,
    }
}

fn load() -> Result<Data, LoadError> { }
```

---

### **3. No Unsafe (Except FFI)**

```rust
// ❌ FORBIDDEN (in business logic)
unsafe fn do_thing() {
    // ...
}

// ✅ ALLOWED (Vulkan FFI only)
#[cfg(feature = "renderer")]
mod vulkan {
    unsafe fn create_instance() {
        // Vulkan requires unsafe
    }
}
```

**Exceptions:** Vulkan FFI, platform APIs (Win32, etc.)

---

### **4. Platform Abstraction**

```rust
// ❌ FORBIDDEN
fn update(world: &mut World) {
    #[cfg(windows)]
    let time = get_windows_time();

    #[cfg(unix)]
    let time = get_unix_time();
}

// ✅ CORRECT
fn update(world: &mut World, clock: &dyn Clock) {
    let time = clock.now();
}
```

---

### **5. Rustfmt + Clippy**

```bash
# Format code (required before commit)
cargo fmt

# Check format
cargo fmt --check

# Clippy (deny warnings)
cargo clippy -- -D warnings
```

**CI blocks merge if formatting or clippy fails.**

---

## 📐 **Code Style**

### **Formatting**

```toml
# rustfmt.toml
max_width = 100
hard_tabs = false
tab_spaces = 4
edition = "2021"
use_small_heuristics = "Max"
```

### **Naming Conventions**

```rust
// Types: PascalCase
pub struct VulkanRenderer { }
pub enum ComponentData { }
pub trait PhysicsBackend { }

// Functions/methods: snake_case
pub fn spawn_entity() { }
pub fn add_component() { }

// Constants: SCREAMING_SNAKE_CASE
pub const MAX_ENTITIES: usize = 100_000;
pub const DEFAULT_PORT: u16 = 8080;

// Modules: snake_case
mod entity_manager;
mod network_client;
```

---

### **Documentation**

```rust
/// Spawns a new entity in the world.
///
/// # Examples
///
/// ```
/// use agent_game_engine::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// world.add(entity, Transform::default());
/// ```
///
/// # Errors
///
/// Returns [`WorldError::EntityLimitReached`] if max entities exceeded.
pub fn spawn(&mut self) -> Result<Entity, WorldError> {
    // ...
}
```

**Requirements:**
- Public APIs: MUST have rustdoc
- Complex functions: SHOULD have examples
- Errors: MUST document error conditions
- Panics: MUST document panic conditions

---

## 🧪 **Testing Requirements**

### **Every Function Needs Tests**

```rust
pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
    self.components.insert(entity, component);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Transform::default());
        assert!(world.get::<Transform>(entity).is_some());
    }
}
```

---

### **Test Organization**

```rust
// Unit tests: same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foo() { }
}

// Integration tests: tests/ directory
// tests/integration_test.rs
use agent_game_engine::*;

#[test]
fn test_full_pipeline() { }
```

---

## 🔒 **Safety & Security**

### **Input Validation**

```rust
// ❌ BAD
pub fn set_health(health: f32) {
    self.health = health;  // Could be negative or NaN!
}

// ✅ GOOD
pub fn set_health(&mut self, health: f32) -> Result<(), HealthError> {
    if !health.is_finite() {
        return Err(HealthError::InvalidValue);
    }
    if health < 0.0 {
        return Err(HealthError::Negative);
    }

    self.health = health;
    Ok(())
}
```

---

### **Server Validation**

```rust
#[server_only]
fn process_input(input: &PlayerInput) -> Result<(), ValidationError> {
    // ALWAYS validate on server
    if !input.is_physically_possible() {
        return Err(ValidationError::ImpossibleAction);
    }

    if input.timestamp < last_input_time {
        return Err(ValidationError::OutOfOrder);
    }

    // ... process
}
```

---

## 📦 **Dependency Management**

### **Cargo.toml Organization**

```toml
[package]
name = "agent-game-engine-renderer"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
# Group by category, alphabetical within group

# Core
agent-game-engine-core = { path = "../core" }

# Graphics
ash = "0.38"
gpu-allocator = "0.27"

# Utilities
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
proptest = "1.0"
criterion = "0.5"

[features]
default = []
profiling = ["tracing-tracy"]
```

---

### **Version Pinning**

```toml
# ✅ Exact versions for critical dependencies
ash = "=0.38.0"

# ✅ Caret for stable libraries
serde = "^1.0"

# ❌ AVOID wildcard
reqwest = "*"  # DON'T DO THIS
```

---

## 🚀 **Performance**

### **Avoid Allocations**

```rust
// ❌ BAD
fn get_entities(&self) -> Vec<Entity> {
    self.entities.clone()  // Allocates!
}

// ✅ GOOD
fn get_entities(&self) -> &[Entity] {
    &self.entities  // Borrow
}

// ✅ ALSO GOOD
fn for_each_entity(&self, f: impl FnMut(&Entity)) {
    self.entities.iter().for_each(f);  // Iterator
}
```

---

### **Use Iterators**

```rust
// ❌ BAD
let mut result = Vec::new();
for entity in &entities {
    if entity.is_alive() {
        result.push(entity.id());
    }
}

// ✅ GOOD
let result: Vec<_> = entities
    .iter()
    .filter(|e| e.is_alive())
    .map(|e| e.id())
    .collect();
```

---

### **Batch Operations**

```rust
// ❌ BAD
for entity in entities {
    world.spawn_with_components(entity);  // N allocations
}

// ✅ GOOD
world.spawn_batch(entities);  // 1 allocation
```

---

## 🔄 **Git Workflow**

### **Commit Messages**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Example:**
```
feat(renderer): add PBR material support

Implements physically-based rendering with metallic/roughness workflow.
Supports albedo, normal, metallic, and roughness texture maps.

Closes #123
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting (no code change)
- `refactor`: Code restructure (no behavior change)
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Build/tooling

---

### **Branch Naming**

```
<type>/<short-description>

Examples:
feat/pbr-rendering
fix/vulkan-crash
docs/architecture-guide
```

---

## ✅ **Pre-Commit Checklist**

Before `git commit`:

- [ ] `cargo fmt` (code formatted)
- [ ] `cargo clippy -- -D warnings` (no warnings)
- [ ] `cargo test` (all tests pass)
- [ ] `cargo doc` (docs build)
- [ ] No `println!` / `dbg!` / `eprintln!`
- [ ] No `unsafe` (unless FFI)
- [ ] All public APIs documented
- [ ] Tests added for new code

---

## 🚫 **Anti-Patterns**

### **❌ Stringly-Typed**

```rust
// BAD
fn get_component(&self, name: &str) -> Option<&dyn Component>;

// GOOD
fn get_component<T: Component>(&self) -> Option<&T>;
```

---

### **❌ Boolean Parameters**

```rust
// BAD
fn create(true, false, true);  // What do these mean?

// GOOD
fn create(config: CreateConfig {
    resizable: true,
    fullscreen: false,
    vsync: true,
});
```

---

### **❌ Premature Optimization**

```rust
// BAD: Complex optimization before profiling
fn update(&mut self) {
    unsafe {
        // SIMD assembly
    }
}

// GOOD: Simple, correct, profile later
fn update(&mut self) {
    for entity in &self.entities {
        entity.update();
    }
}
```

---

## 📚 **Resources**

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Error Handling in Rust](https://blog.burntsushi.net/rust-error-handling/)

---

**Last Updated:** 2026-01-31
