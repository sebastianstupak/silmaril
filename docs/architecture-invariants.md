# Architecture Invariants

> **Architectural principles and enforcement mechanisms for agent-game-engine**
>
> This document codifies the invariants that MUST hold across the entire codebase and explains how they are enforced at compile-time and runtime.

---

## Table of Contents

1. [Architecture Principles](#architecture-principles)
2. [Invariants & Enforcement](#invariants--enforcement)
3. [How to Add Platform-Specific Code](#how-to-add-platform-specific-code)
4. [Testing Strategy](#testing-strategy)
5. [Common Violations & Fixes](#common-violations--fixes)

---

## Architecture Principles

### 1. **Separation of Concerns**

**Business logic MUST NOT contain platform-specific code.**

Platform abstractions live in the `platform` module and expose traits. Business logic uses these traits without knowledge of the underlying platform.

```rust
// ❌ WRONG - Platform code in business logic
fn game_loop() {
    #[cfg(windows)]
    let time = unsafe { /* Windows-specific time API */ };

    #[cfg(unix)]
    let time = unsafe { /* Unix-specific time API */ };
}

// ✅ CORRECT - Business logic uses abstraction
fn game_loop(time_backend: &dyn TimeBackend) {
    let time = time_backend.monotonic_nanos();
}
```

**Rationale:** This enables cross-platform support without littering `#[cfg]` attributes throughout the codebase. It also makes testing easier since we can inject mock implementations.

---

### 2. **Structured Error Handling**

**All errors MUST be typed and implement `EngineError` trait.**

No `anyhow::Result`, `Box<dyn Error>`, or panic-based error handling in production code.

```rust
// ❌ WRONG - Untyped errors
fn load_asset(path: &str) -> Result<Asset, Box<dyn Error>> { }
fn init() -> anyhow::Result<()> { }

// ✅ CORRECT - Typed errors with error codes
use engine_macros::define_error;

define_error! {
    pub enum AssetError {
        NotFound { path: String } = ErrorCode::AssetNotFound, ErrorSeverity::Error,
        LoadFailed { path: String, reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

fn load_asset(path: &str) -> Result<Asset, AssetError> { }
```

**Rationale:** Structured errors enable:
- Programmatic error handling (match on error codes)
- Automatic metrics collection (count errors by subsystem)
- Structured logging (all errors auto-log via `tracing`)
- Better error messages for debugging

---

### 3. **Zero-Cost Abstractions**

**Abstractions MUST NOT impose runtime overhead where possible.**

Use trait objects (`&dyn Trait`) only at system boundaries (initialization, dependency injection). Use generics or static dispatch for hot paths.

```rust
// ✅ CORRECT - One-time initialization with trait object
pub fn create_time_backend() -> Box<dyn TimeBackend> {
    #[cfg(windows)]
    return Box::new(WindowsTime::new()?);

    #[cfg(unix)]
    return Box::new(UnixTime::new()?);
}

// ✅ CORRECT - Hot path uses concrete type (no vtable indirection)
struct GameEngine {
    time: Box<dyn TimeBackend>, // Stored once
}

impl GameEngine {
    fn update(&mut self) {
        // time.monotonic_nanos() - single virtual call per frame
        let frame_start = self.time.monotonic_nanos();

        // Hot loop uses frame_start (no more virtual calls)
        for entity in &self.world.entities {
            // ...
        }
    }
}
```

**Rationale:** Game engines are performance-critical. We use abstractions for maintainability but ensure they compile away or only impose minimal overhead at boundaries.

---

### 4. **Trait-Based Design**

**Platform code MUST be hidden behind traits.**

Each platform feature (time, filesystem, threading, windowing, input) has:
- A public trait defining the interface
- Platform-specific implementations in submodules
- A factory function that returns `Box<dyn Trait>`

```rust
// Public API (in platform/time/mod.rs)
pub trait TimeBackend: Send + Sync {
    fn monotonic_nanos(&self) -> u64;
    fn sleep(&self, duration: Duration);
}

// Platform-specific implementations
#[cfg(windows)]
mod windows {
    struct WindowsTime { /* ... */ }
    impl TimeBackend for WindowsTime { /* ... */ }
}

#[cfg(unix)]
mod unix {
    struct UnixTime { /* ... */ }
    impl TimeBackend for UnixTime { /* ... */ }
}

// Factory selects implementation at compile time
pub fn create_time_backend() -> Result<Box<dyn TimeBackend>, PlatformError> {
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsTime::new()?));

    #[cfg(unix)]
    return Ok(Box::new(unix::UnixTime::new()?));
}
```

**Rationale:** This pattern:
- Keeps platform-specific code isolated
- Makes testing easy (inject test implementations)
- Allows adding new platforms without touching business logic
- Enables compile-time platform selection with runtime polymorphism

---

### 5. **No Printing - Use Structured Logging**

**All output MUST use `tracing` macros, never `println!`/`eprintln!`/`dbg!`.**

```rust
// ❌ WRONG
println!("Player joined: {}", player_id);
eprintln!("Error loading asset: {}", e);
dbg!(transform);

// ✅ CORRECT
use tracing::{info, error, debug};

info!(
    player_id = %player_id,
    username = %player.name,
    "Player joined"
);

error!(
    error = ?e,
    context = "asset_loading",
    "Failed to load asset"
);

debug!(transform = ?transform, "Current transform");
```

**Rationale:**
- Structured logs can be queried, filtered, and analyzed
- Production deployments can route logs to observability systems
- Log levels can be dynamically adjusted without recompiling
- Compile-time enforcement via `#![deny(clippy::print_stdout)]`

---

## Invariants & Enforcement

### Invariant 1: Error Codes in Correct Subsystem Ranges

**Requirement:** Each error code MUST be in the range for its subsystem.

**Error Code Ranges:**
```rust
1000-1099: Core ECS
1100-1199: Serialization
1200-1299: Platform
1300-1399: Rendering
1400-1499: Networking
1500-1599: Physics
1600-1699: Audio
1700-1799: LOD
1800-1899: Interest Management
1900-1999: Auto-update
```

**Enforcement:** Compile-time test in `error.rs`

```rust
#[test]
fn test_error_code_ranges() {
    // Core ECS errors must be 1000-1099
    assert!((ErrorCode::EntityNotFound as u32) >= 1000);
    assert!((ErrorCode::EntityNotFound as u32) < 1100);

    // Serialization errors must be 1100-1199
    assert!((ErrorCode::SerializationFailed as u32) >= 1100);
    assert!((ErrorCode::SerializationFailed as u32) < 1200);

    // Platform errors must be 1200-1299
    assert!((ErrorCode::WindowCreationFailed as u32) >= 1200);
    assert!((ErrorCode::WindowCreationFailed as u32) < 1300);
}
```

**Violation Example:**
```rust
// ❌ WRONG - Platform error in wrong range
pub enum ErrorCode {
    WindowCreationFailed = 1050, // Should be 1200-1299
}

// ✅ CORRECT
pub enum ErrorCode {
    WindowCreationFailed = 1200,
}
```

---

### Invariant 2: All Errors Implement `EngineError` Trait

**Requirement:** Every error type in the engine MUST implement `EngineError`.

**EngineError Trait:**
```rust
pub trait EngineError: std::error::Error + Send + Sync {
    fn code(&self) -> ErrorCode;
    fn severity(&self) -> ErrorSeverity;
    fn log(&self);
}
```

**Enforcement:** Use `define_error!` macro (compile-time)

```rust
use engine_macros::define_error;

define_error! {
    pub enum MyError {
        NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
        InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
    }
}

// Auto-generates:
// - enum MyError { NotFound { id: u32 }, InvalidData { reason: String } }
// - impl Display for MyError
// - impl Error for MyError
// - impl EngineError for MyError
```

**Tests:** Every error type must be tested

```rust
#[test]
fn test_error_codes_and_severity() {
    let error = MyError::NotFound { id: 42 };
    assert_eq!(error.code(), ErrorCode::EntityNotFound);
    assert_eq!(error.severity(), ErrorSeverity::Error);
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MyError>();
}
```

**Violation Example:**
```rust
// ❌ WRONG - Manual error without EngineError
#[derive(Debug)]
pub enum MyError {
    NotFound,
}

impl std::error::Error for MyError {}
impl Display for MyError { /* ... */ }
// Missing EngineError implementation!

// ✅ CORRECT - Use define_error! macro
define_error! {
    pub enum MyError {
        NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
    }
}
```

---

### Invariant 3: Platform Code Only in `platform` Module

**Requirement:** `#[cfg(target_os = ...)]` attributes MUST only appear in:
- `engine/core/src/platform/*` (platform abstraction layer)
- Factory functions that select platform implementations
- Build scripts (`build.rs`)

**Enforcement:**
1. **Compile-time:** Linting via custom build script (future)
2. **Runtime:** Architecture tests (see below)

**Architecture Test:**
```rust
// engine/core/tests/architecture_tests.rs
#[test]
fn test_no_platform_code_in_business_logic() {
    // This test uses static analysis to verify no #[cfg(target_os)]
    // appears outside the platform module

    let source_files = glob("src/**/*.rs")
        .unwrap()
        .filter(|p| !p.to_string_lossy().contains("platform"));

    for file in source_files {
        let content = std::fs::read_to_string(file).unwrap();
        assert!(
            !content.contains("#[cfg(target_os"),
            "Platform-specific code found outside platform module: {:?}",
            file
        );
    }
}
```

**Violation Example:**
```rust
// ❌ WRONG - Platform code in ECS module
// engine/core/src/ecs/world.rs
pub fn get_time() -> f64 {
    #[cfg(windows)]
    return unsafe { /* Windows API */ };

    #[cfg(unix)]
    return unsafe { /* Unix API */ };
}

// ✅ CORRECT - Use platform abstraction
pub fn get_time(time_backend: &dyn TimeBackend) -> f64 {
    time_backend.monotonic_nanos() as f64 / 1_000_000_000.0
}
```

---

### Invariant 4: No `println!`/`eprintln!`/`dbg!` in Production Code

**Requirement:** All logging MUST use `tracing` macros.

**Enforcement:** Compile-time via Clippy lints

```toml
# .cargo/config.toml
[target.'cfg(all())']
rustflags = [
    "-Dwarnings",
    "-Wclippy::print_stdout",
    "-Wclippy::print_stderr",
    "-Wclippy::dbg_macro",
]
```

**Violation Example:**
```rust
// ❌ WRONG - Direct printing
pub fn spawn_entity(&mut self, name: &str) -> Entity {
    let entity = self.next_entity_id();
    println!("Spawned entity: {} ({})", entity.id(), name);
    entity
}

// ✅ CORRECT - Structured logging
use tracing::info;

pub fn spawn_entity(&mut self, name: &str) -> Entity {
    let entity = self.next_entity_id();
    info!(
        entity_id = entity.id(),
        entity_name = name,
        "Entity spawned"
    );
    entity
}
```

---

### Invariant 5: Traits Must Be `Send + Sync`

**Requirement:** All platform abstraction traits MUST be `Send + Sync`.

**Rationale:** Platform backends are shared across threads in the engine.

**Enforcement:** Compile-time (trait bounds) + tests

```rust
pub trait TimeBackend: Send + Sync {
    fn monotonic_nanos(&self) -> u64;
}

#[test]
fn test_time_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn TimeBackend>>();
}
```

**Violation Example:**
```rust
// ❌ WRONG - Not Send + Sync
pub trait TimeBackend {
    fn monotonic_nanos(&self) -> u64;
}

// ✅ CORRECT
pub trait TimeBackend: Send + Sync {
    fn monotonic_nanos(&self) -> u64;
}
```

---

## How to Add Platform-Specific Code

### Step-by-Step Guide

#### 1. Define the Public Trait

Create a trait in `engine/core/src/platform/<feature>/mod.rs`:

```rust
// engine/core/src/platform/audio/mod.rs
use crate::PlatformError;

pub trait AudioBackend: Send + Sync {
    fn initialize(&mut self) -> Result<(), PlatformError>;
    fn play_sound(&self, sound_id: u32) -> Result<(), PlatformError>;
}
```

#### 2. Create Platform-Specific Implementations

Create separate files for each platform:

```rust
// engine/core/src/platform/audio/windows.rs
use super::AudioBackend;
use crate::PlatformError;

pub struct WindowsAudio {
    // Windows-specific state (XAudio2, etc.)
}

impl WindowsAudio {
    pub fn new() -> Result<Self, PlatformError> {
        // Initialize Windows audio
        Ok(Self {})
    }
}

impl AudioBackend for WindowsAudio {
    fn initialize(&mut self) -> Result<(), PlatformError> {
        // Windows-specific initialization
        Ok(())
    }

    fn play_sound(&self, sound_id: u32) -> Result<(), PlatformError> {
        // Windows-specific playback
        Ok(())
    }
}
```

```rust
// engine/core/src/platform/audio/unix.rs
use super::AudioBackend;
use crate::PlatformError;

pub struct UnixAudio {
    // Unix-specific state (ALSA, PulseAudio, etc.)
}

impl UnixAudio {
    pub fn new() -> Result<Self, PlatformError> {
        // Initialize Unix audio
        Ok(Self {})
    }
}

impl AudioBackend for UnixAudio {
    fn initialize(&mut self) -> Result<(), PlatformError> {
        // Unix-specific initialization
        Ok(())
    }

    fn play_sound(&self, sound_id: u32) -> Result<(), PlatformError> {
        // Unix-specific playback
        Ok(())
    }
}
```

#### 3. Add Platform Module Declarations

In `mod.rs`, conditionally compile platform modules:

```rust
// engine/core/src/platform/audio/mod.rs
pub trait AudioBackend: Send + Sync {
    fn initialize(&mut self) -> Result<(), PlatformError>;
    fn play_sound(&self, sound_id: u32) -> Result<(), PlatformError>;
}

// Platform-specific modules (only compile what's needed)
#[cfg(windows)]
mod windows;

#[cfg(all(unix, not(target_os = "macos")))]
mod unix;

#[cfg(target_os = "macos")]
mod macos;
```

#### 4. Create Factory Function

The factory selects the platform implementation at compile time:

```rust
// engine/core/src/platform/audio/mod.rs
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, PlatformError> {
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsAudio::new()?));

    #[cfg(all(unix, not(target_os = "macos")))]
    return Ok(Box::new(unix::UnixAudio::new()?));

    #[cfg(target_os = "macos")]
    return Ok(Box::new(macos::MacOsAudio::new()?));

    #[cfg(not(any(windows, unix)))]
    return Err(PlatformError::PlatformNotSupported {
        platform: std::env::consts::OS.to_string(),
        feature: "audio".to_string(),
    });
}
```

#### 5. Export from Platform Module

Add to `engine/core/src/platform/mod.rs`:

```rust
pub mod audio;
pub use audio::{create_audio_backend, AudioBackend};
```

#### 6. Use in Business Logic

Business logic uses the trait, not platform-specific code:

```rust
// engine/audio/src/lib.rs
use engine_core::platform::{create_audio_backend, AudioBackend};

pub struct AudioSystem {
    backend: Box<dyn AudioBackend>,
}

impl AudioSystem {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            backend: create_audio_backend()?,
        })
    }

    pub fn play(&self, sound_id: u32) -> Result<(), PlatformError> {
        self.backend.play_sound(sound_id)
    }
}
```

#### 7. Add Tests

Every platform module must have tests:

```rust
// engine/core/src/platform/audio/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_backend() {
        let backend = create_audio_backend();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn AudioBackend>>();
    }
}
```

Platform-specific tests in each submodule:

```rust
// engine/core/src/platform/audio/windows.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_audio_creation() {
        let audio = WindowsAudio::new();
        assert!(audio.is_ok());
    }

    #[test]
    fn test_initialize() {
        let mut audio = WindowsAudio::new().unwrap();
        let result = audio.initialize();
        assert!(result.is_ok());
    }
}
```

---

## Testing Strategy

### Unit Tests

Every module has unit tests in the same file:

```rust
// engine/core/src/ecs/world.rs
pub struct World {
    // ...
}

impl World {
    pub fn spawn(&mut self) -> Entity {
        // ...
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));
    }
}
```

**Coverage Target:** 80% minimum for all modules

---

### Integration Tests

Platform abstractions have integration tests in `tests/` directory:

```rust
// engine/core/tests/platform_integration.rs
use engine_core::platform::{create_time_backend, create_filesystem_backend};

#[test]
fn test_time_and_filesystem_together() {
    let time = create_time_backend().unwrap();
    let fs = create_filesystem_backend();

    let start = time.monotonic_nanos();

    let temp = std::env::temp_dir().join("test.txt");
    fs.write_file(&temp, b"test").unwrap();

    let elapsed = time.monotonic_nanos() - start;
    assert!(elapsed > 0);

    std::fs::remove_file(&temp).ok();
}
```

---

### Architecture Tests

Tests that enforce architectural invariants:

```rust
// engine/core/tests/architecture_tests.rs
use glob::glob;

#[test]
fn test_no_platform_code_in_business_logic() {
    let files = glob("src/**/*.rs")
        .unwrap()
        .filter(|p| !p.to_string_lossy().contains("platform"));

    for file in files {
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(
            !content.contains("#[cfg(target_os"),
            "Platform code found outside platform module: {:?}",
            file
        );
    }
}

#[test]
fn test_no_println_in_source() {
    for file in glob("src/**/*.rs").unwrap() {
        let content = std::fs::read_to_string(&file).unwrap();

        // Allow println! in test code
        if content.contains("#[cfg(test)]") {
            continue;
        }

        assert!(
            !content.contains("println!"),
            "println! found in source: {:?}",
            file
        );
    }
}

#[test]
fn test_all_error_types_implement_engine_error() {
    // This test uses syn to parse all error enums and verify
    // they implement EngineError
    // (Implementation requires syn crate in dev-dependencies)
}
```

---

### Error Tests

Every error type has comprehensive tests:

```rust
// engine/core/src/platform/error.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineError;

    #[test]
    fn test_window_creation_error() {
        let error = PlatformError::WindowCreationFailed {
            details: "test".to_string(),
        };
        assert_eq!(error.code(), ErrorCode::WindowCreationFailed);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_filesystem_error() {
        let error = PlatformError::FileSystemError {
            operation: "read".to_string(),
            path: "/tmp/test.txt".to_string(),
            details: "permission denied".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("FileSystemError"));
        assert!(display.contains("read"));
        assert!(display.contains("/tmp/test.txt"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PlatformError>();
    }

    #[test]
    fn test_result_usage() {
        fn returns_error() -> Result<(), PlatformError> {
            Err(PlatformError::InputInitFailed {
                details: "test".to_string(),
            })
        }

        let result = returns_error();
        assert!(result.is_err());
    }
}
```

---

### Platform-Specific Tests

Each platform implementation has its own tests:

```rust
// engine/core/src/platform/threading/windows.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_threading_creation() {
        let threading = WindowsThreading::new();
        assert!(threading.is_ok());
    }

    #[test]
    fn test_set_priorities() {
        let threading = WindowsThreading::new().unwrap();

        assert!(threading.set_thread_priority(ThreadPriority::Low).is_ok());
        assert!(threading.set_thread_priority(ThreadPriority::Normal).is_ok());
        assert!(threading.set_thread_priority(ThreadPriority::High).is_ok());

        // Realtime may fail without admin privileges
        let _ = threading.set_thread_priority(ThreadPriority::Realtime);
    }

    #[test]
    fn test_empty_affinity_fails() {
        let threading = WindowsThreading::new().unwrap();
        let result = threading.set_thread_affinity(&[]);
        assert!(result.is_err());
    }
}
```

**Current Test Count:** 145 passing tests (Phase 1.4 complete)

---

## Common Violations & Fixes

### Violation 1: Platform Code in Business Logic

**Problem:**
```rust
// engine/renderer/src/swapchain.rs
fn create_swapchain() {
    #[cfg(windows)]
    let surface = create_win32_surface();

    #[cfg(unix)]
    let surface = create_xcb_surface();
}
```

**Fix:**
```rust
// Move to engine/core/src/platform/surface/mod.rs
pub trait SurfaceBackend: Send + Sync {
    fn create(&self) -> Result<VkSurfaceKHR, PlatformError>;
}

// Business logic uses trait
fn create_swapchain(surface: &dyn SurfaceBackend) {
    let vk_surface = surface.create()?;
    // ...
}
```

---

### Violation 2: Untyped Errors

**Problem:**
```rust
fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}
```

**Fix:**
```rust
use engine_macros::define_error;

define_error! {
    pub enum ConfigError {
        FileNotFound { path: String } = ErrorCode::FileSystemError, ErrorSeverity::Error,
        ParseFailed { path: String, reason: String } = ErrorCode::YamlDeserializeFailed, ErrorSeverity::Error,
    }
}

fn load_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::FileNotFound {
            path: path.to_string(),
        })?;

    let config: Config = serde_yaml::from_str(&content)
        .map_err(|e| ConfigError::ParseFailed {
            path: path.to_string(),
            reason: e.to_string(),
        })?;

    Ok(config)
}
```

---

### Violation 3: Using `println!` for Logging

**Problem:**
```rust
pub fn connect_to_server(&mut self, addr: &str) {
    println!("Connecting to {}", addr);
    // ...
    println!("Connected!");
}
```

**Fix:**
```rust
use tracing::info;

pub fn connect_to_server(&mut self, addr: &str) {
    info!(server_address = addr, "Connecting to server");
    // ...
    info!(server_address = addr, "Connected to server");
}
```

---

### Violation 4: Missing Error Tests

**Problem:**
```rust
define_error! {
    pub enum MyError {
        NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
    }
}

// No tests!
```

**Fix:**
```rust
define_error! {
    pub enum MyError {
        NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineError;

    #[test]
    fn test_error_code_and_severity() {
        let error = MyError::NotFound { id: 42 };
        assert_eq!(error.code(), ErrorCode::EntityNotFound);
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_error_display() {
        let error = MyError::NotFound { id: 42 };
        let display = format!("{}", error);
        assert!(display.contains("NotFound"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MyError>();
    }
}
```

---

### Violation 5: Trait Not `Send + Sync`

**Problem:**
```rust
pub trait GameBackend {
    fn update(&mut self);
}

// Error: cannot be shared across threads
```

**Fix:**
```rust
pub trait GameBackend: Send + Sync {
    fn update(&mut self);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn GameBackend>>();
    }
}
```

---

## Summary

These architectural invariants ensure:

1. **Cross-platform support** - Business logic is platform-agnostic
2. **Maintainability** - Clear separation of concerns
3. **Debuggability** - Structured errors with automatic logging
4. **Performance** - Zero-cost abstractions where possible
5. **Reliability** - Compile-time and runtime enforcement

**Enforcement Mechanisms:**
- **Compile-time:** Clippy lints, trait bounds, type system
- **Test-time:** Architecture tests, error tests, integration tests
- **CI:** All tests run on Windows, Linux, macOS (x64 + ARM)

**Current Status (Phase 1.4 Complete):**
- ✅ 145 passing tests
- ✅ Error infrastructure with `define_error!` macro
- ✅ Platform abstractions (time, filesystem, threading)
- ✅ Structured logging with `tracing`
- ⏳ Architecture tests (pending)
- ⏳ CI integration (pending)

---

## See Also

- [CLAUDE.md](../CLAUDE.md) - Main AI agent guide
- [docs/error-handling.md](error-handling.md) - Error handling architecture
- [docs/platform-abstraction.md](platform-abstraction.md) - Platform layer design
- [docs/testing-strategy.md](testing-strategy.md) - Testing approach
- [docs/rules/coding-standards.md](rules/coding-standards.md) - Coding standards
