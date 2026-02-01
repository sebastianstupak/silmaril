# Development Workflow

> **Complete development workflow guide**
>
> For both AI agents and human developers

---

## 🚀 **Quick Start**

### **Clone and Setup**

```bash
git clone https://github.com/your-org/agent-game-engine.git
cd agent-game-engine

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Vulkan SDK
# Windows: Download from vulkan.lunarg.com
# Linux: sudo apt install vulkan-tools libvulkan-dev
# macOS: brew install molten-vk

# Build engine
cd engine
cargo build

# Run tests
cargo test --all-features

# Run example
cd ../examples/singleplayer
cargo run --release
```

---

## 🛠️ **Setting Up Development Environment**

### **Initial Setup**

After cloning the repository, run the setup script to install git hooks and configure your development environment:

```bash
# From repository root
./scripts/setup-hooks.sh
```

This script will:
- Install pre-commit hooks for code quality checks
- Verify optional development tools are installed
- Configure your local git repository

### **Pre-commit Hooks**

Pre-commit hooks automatically run before each commit to ensure code quality. They check:

1. **Code Formatting** - Ensures code follows Rust style guidelines
   ```bash
   cargo fmt --check
   ```

2. **Linting** - Catches common mistakes and enforces best practices
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

3. **Unit Tests** - Verifies basic functionality still works
   ```bash
   cargo test --lib
   ```

4. **Dependency Checks** - Ensures no banned dependencies (requires `cargo-deny`)
   ```bash
   cargo deny check bans
   ```

5. **Common Issues** - Detects patterns that violate project standards:
   - `println!`/`eprintln!`/`dbg!` usage (use `tracing` instead)
   - `anyhow::Result` usage (use custom error types)
   - `Box<dyn Error>` usage (use custom error types)

### **Manual Hook Execution**

To run pre-commit checks manually without committing:

```bash
.git/hooks/pre-commit
```

### **Bypassing Hooks** (Not Recommended)

In rare cases where you need to commit without running hooks:

```bash
git commit --no-verify
```

**Warning:** Only use this for work-in-progress commits on feature branches. Never bypass hooks on main branch commits.

### **Optional Development Tools**

Install these tools for enhanced development experience:

```bash
# Dependency auditing and policy enforcement
cargo install cargo-deny

# Auto-rebuild on file changes (hot reload)
cargo install cargo-watch

# CPU profiling with flamegraphs
cargo install flamegraph

# Code coverage
cargo install cargo-tarpaulin
```

### **IDE Setup**

#### **VS Code**

Recommended extensions:
- `rust-analyzer` - Rust language support
- `CodeLLDB` - Debugging
- `crates` - Dependency management
- `Better TOML` - TOML syntax highlighting

Settings (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true
}
```

#### **IntelliJ IDEA / CLion**

- Install Rust plugin
- Enable "Run clippy on save"
- Enable "Format on save"

### **Environment Variables**

For development, set these environment variables:

```bash
# Verbose logging
export RUST_LOG=debug

# Backtrace on panic
export RUST_BACKTRACE=1

# Enable Vulkan validation layers
export VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d  # Linux
# Windows: set VK_LAYER_PATH=C:\VulkanSDK\Bin

# Tracy profiling
export TRACY_ENABLE=1
```

Add to your shell profile (`.bashrc`, `.zshrc`, etc.) for persistence.

---

## 🔄 **Daily Development Loop**

### **1. Start Dev Environment**

```bash
# Local (processes)
./scripts/dev.sh local

# Docker (containers, recommended for multiplayer)
./scripts/dev.sh docker
```

**Hot-reload enabled:** Edit code → Auto rebuild → Auto restart

---

### **2. Write Code**

**Before writing:**
- [ ] Read [CLAUDE.md](../CLAUDE.md) if first time
- [ ] Check [ROADMAP.md](../ROADMAP.md) for current phase
- [ ] Read relevant task file in `docs/tasks/`
- [ ] Read relevant architecture doc

**While writing:**
- [ ] Follow [coding standards](rules/coding-standards.md)
- [ ] Use `tracing` for logging (never `println!`)
- [ ] Use custom error types (never `anyhow`)
- [ ] Abstract platform code (traits, not `#[cfg]`)
- [ ] Write tests FIRST (TDD)

---

### **3. Test Your Changes**

```bash
# Format code
cargo fmt

# Check format
cargo fmt --check

# Clippy (lints)
cargo clippy --workspace -- -D warnings

# Unit tests
cargo test --lib

# Integration tests
cargo test --tests

# Doc tests
cargo test --doc

# Specific test
cargo test test_name

# Benchmarks (if performance-sensitive code)
cargo bench

# All platforms (local)
./scripts/test-all-platforms.sh
```

---

### **4. Profile (if needed)**

```bash
# Tracy profiling
cargo build --features profiling
./target/debug/client
# Open Tracy profiler (separate app)

# Flamegraph
cargo install flamegraph
cargo flamegraph --bin client

# Benchmarks with criterion
cargo bench --bench ecs_benchmark
# Results in target/criterion/
```

---

### **5. Commit**

```bash
# Pre-commit checklist
cargo fmt --check                # Format
cargo clippy -- -D warnings      # Lints
cargo test --all-features        # Tests
cargo doc --no-deps              # Docs build

# Commit
git add .
git commit -m "feat(ecs): add query filters

Implements .with() and .without() filters for queries.
Allows filtering entities by component presence/absence.

Closes #123"

# Push
git push origin feat/query-filters
```

---

## 🌳 **Branch Strategy**

### **Branch Naming**

```
<type>/<description>

Examples:
feat/pbr-rendering
fix/vulkan-crash
docs/architecture
refactor/ecs-storage
perf/query-optimization
```

### **Commit Message Format**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructure
- `perf`: Performance
- `test`: Tests
- `chore`: Build/tooling

**Example:**
```
feat(renderer): add shadow mapping

Implements cascaded shadow maps for directional lights.
Uses 4 cascades with PCF filtering.

Performance: < 2ms for 3 lights at 1080p.

Closes #456
```

---

## 🧪 **Testing Workflow**

### **TDD (Test-Driven Development)**

1. **Write test first** (RED)
```rust
#[test]
fn test_feature() {
    let result = new_feature();
    assert_eq!(result, expected);
}
```

2. **Run test** (fails)
```bash
cargo test test_feature
# Fails as expected
```

3. **Implement feature** (GREEN)
```rust
fn new_feature() -> Type {
    // Implementation
}
```

4. **Run test** (passes)
```bash
cargo test test_feature
# Passes!
```

5. **Refactor** (REFACTOR)
```rust
fn new_feature() -> Type {
    // Cleaner implementation
}
```

6. **Verify** (still passes)
```bash
cargo test test_feature
```

---

### **E2E Testing**

```bash
# Start test environment
cd tests/e2e
docker-compose up --abort-on-container-exit

# Or manually
cargo run --bin server &
SERVER_PID=$!
cargo run --bin client &
CLIENT_PID=$!

# Wait, then cleanup
kill $SERVER_PID $CLIENT_PID
```

---

## 🐛 **Debugging**

### **Verbose Logging**

```bash
RUST_LOG=trace cargo run
RUST_LOG=agent_game_engine=debug cargo run
RUST_LOG=agent_game_engine_networking=trace cargo run
```

### **Vulkan Validation**

```bash
# Enable validation layers (dev builds)
VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d cargo run

# Windows
set VK_LAYER_PATH=C:\VulkanSDK\Bin
cargo run
```

### **GDB/LLDB**

```bash
# Build with debug info
cargo build

# Run in debugger
rust-gdb target/debug/client
(gdb) run
(gdb) bt  # Backtrace on crash
```

### **Tracy Zones**

```rust
use tracing::instrument;

#[instrument]
fn expensive_function() {
    // Shows in Tracy with timing
}
```

---

## 📊 **Performance Monitoring**

### **Continuous Benchmarking**

```bash
# Run benchmarks
cargo bench

# Compare with baseline
cargo bench -- --save-baseline main
git checkout feat/optimization
cargo bench -- --baseline main
```

### **Profiling Tools**

| Tool | Use Case | Command |
|------|----------|---------|
| **Tracy** | Real-time profiling | `cargo build --features profiling` |
| **Flamegraph** | CPU profiling | `cargo flamegraph` |
| **Criterion** | Benchmarking | `cargo bench` |
| **Valgrind** | Memory leaks | `valgrind ./target/debug/client` |
| **heaptrack** | Heap profiling | `heaptrack ./target/debug/client` |

---

## 🔧 **Common Tasks**

### **Add New Component**

1. Define component type
```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MyComponent {
    pub value: f32,
}
```

2. Add to ComponentData enum
```rust
pub enum ComponentData {
    MyComponent(MyComponent),
    // ...
}
```

3. Update serialization
```rust
impl WorldState {
    fn add_component(&mut self, entity: Entity, data: ComponentData) {
        match data {
            ComponentData::MyComponent(c) => self.add(entity, c),
            // ...
        }
    }
}
```

4. Write tests
```rust
#[test]
fn test_my_component() { }
```

---

### **Add New System**

1. Define system function
```rust
pub fn my_system(query: Query<(&MyComponent, &mut Transform)>, dt: f32) {
    for (my_comp, transform) in query.iter_mut() {
        transform.position += my_comp.value * dt;
    }
}
```

2. Register in app
```rust
app.add_system(my_system);
```

3. Test
```rust
#[test]
fn test_my_system() {
    let mut world = World::new();
    // Setup
    my_system(&mut world, 1.0);
    // Assert
}
```

---

### **Add Platform-Specific Code**

1. Define trait
```rust
pub trait MyPlatformFeature {
    fn do_thing(&self) -> Result<Output, Error>;
}
```

2. Implement per platform
```rust
#[cfg(windows)]
mod windows {
    impl MyPlatformFeature for WindowsImpl {
        fn do_thing(&self) -> Result<Output, Error> {
            // Windows-specific code
        }
    }
}

// Similar for linux, macos
```

3. Factory function
```rust
pub fn create_platform_feature() -> Box<dyn MyPlatformFeature> {
    #[cfg(windows)]
    return Box::new(windows::WindowsImpl::new());

    #[cfg(unix)]
    return Box::new(unix::UnixImpl::new());
}
```

4. Test on all platforms (CI does this)

---

## 📚 **Documentation**

### **Writing Rustdoc**

```rust
/// Brief description (one line).
///
/// Longer description with details.
/// Can span multiple paragraphs.
///
/// # Examples
///
/// ```
/// use agent_game_engine::*;
///
/// let world = World::new();
/// let entity = world.spawn();
/// ```
///
/// # Errors
///
/// Returns [`WorldError::EntityLimitReached`] if limit exceeded.
///
/// # Panics
///
/// Panics if entity is not alive.
pub fn my_function() -> Result<Entity, WorldError> {
    // ...
}
```

### **Building Docs**

```bash
# Build docs
cargo doc --no-deps --open

# Check for broken links
cargo doc --no-deps 2>&1 | grep warning

# Test doc examples
cargo test --doc
```

---

## 🎯 **CI/CD**

### **What CI Checks**

On every push/PR:
- [ ] Format (`cargo fmt --check`)
- [ ] Lints (`cargo clippy -- -D warnings`)
- [ ] Tests (all platforms)
- [ ] Benchmarks (no regressions > 10%)
- [ ] Docs build
- [ ] Coverage (> 80%)
- [ ] Security audit

### **Fixing CI Failures**

```bash
# Reproduce locally
./scripts/ci-local.sh

# Format
cargo fmt

# Fix clippy
cargo clippy --fix --workspace

# Run tests
cargo test --all-features

# Check benchmarks
cargo bench
```

---

## 🚀 **Release Process**

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Tag release
4. Build binaries
5. Create GitHub release
6. Publish crates (if public)

```bash
cargo release patch  # or minor, major
```

---

**Last Updated:** 2026-01-31
