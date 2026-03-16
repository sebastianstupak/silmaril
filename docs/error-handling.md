# Error Handling

> **Custom error types enforced across all crates**
>
> ⚠️ **MANDATORY** - Never use `Box<dyn Error>` or `anyhow` in library code

---

## 🎯 **Core Principles**

1. **Custom types always** - Each crate defines its own error enum
2. **Structured logging** - Errors automatically logged with context
3. **Error codes** - Programmatic handling via numeric codes
4. **Severity levels** - Critical, Error, Warning
5. **No panics in library code** - Return `Result<T, E>` always

---

## 🏗️ **Error Type Macro**

```rust
// engine/core/src/error.rs
use silmaril_core::define_error;

define_error! {
    pub enum RendererError {
        /// Failed to initialize Vulkan
        VulkanInit { details: String } =
            ErrorCode::VulkanInitFailed,
            ErrorSeverity::Critical,

        /// Shader compilation failed
        ShaderCompile { shader: String, error: String } =
            ErrorCode::ShaderCompilationFailed,
            ErrorSeverity::Error,
    }
}
```

**Generates:**
- `impl Display`
- `impl Error`
- `impl EngineError` (custom trait)
- Automatic structured logging
- Error code assignment
- Constructor methods for each variant
- Optional backtrace support (with `backtrace` feature)

---

## 📊 **Error Code Ranges**

| Range | System |
|-------|--------|
| 1000-1099 | Core ECS |
| 1100-1199 | Serialization |
| 1200-1299 | Platform |
| 1300-1399 | Rendering |
| 1400-1499 | Networking |
| 1500-1599 | Physics |
| 1600-1699 | Audio |
| 1700-1799 | LOD |
| 1800-1899 | Interest Management |
| 1900-1999 | Auto-update |
| 2000-2099 | Template System |
| 2100-2199 | Dev Tools |
| 10000+ | Game-specific |

---

## 🔍 **Backtrace Support**

Enable detailed stack traces for errors by enabling the `backtrace` feature:

```toml
# Cargo.toml
[dependencies]
engine-core = { version = "0.1", features = ["backtrace"] }
```

**Usage:**

```rust
// Create an error (backtrace captured automatically when feature is enabled)
let error = SerializationError::yamlserialize("invalid YAML".to_string());

// Access backtrace
if let Some(backtrace) = error.backtrace() {
    println!("Error occurred at:\n{}", backtrace);
}

// Backtrace is automatically included in structured logs
error.log();
```

**How it works:**
- The `define_error!` macro generates constructor methods (e.g., `yamlserialize()`)
- When `backtrace` feature is enabled, these methods capture the backtrace automatically
- When disabled, no backtrace overhead (zero cost)
- Backtraces are included in structured logs when available

**Performance:**
- Without feature: Zero overhead, no backtrace field in enum
- With feature: Backtrace captured at error creation site
- Recommendation: Enable in debug/dev builds, disable in release

---

## 🧪 **Testing**

```rust
#[test]
fn test_error_creation() {
    // Use constructor methods (not struct syntax)
    let err = RendererError::vulkaninit("No GPU found".to_string());

    assert_eq!(err.code(), ErrorCode::VulkanInitFailed);
    assert_eq!(err.severity(), ErrorSeverity::Critical);
}

#[test]
#[cfg(feature = "backtrace")]
fn test_backtrace_captured() {
    let error = SerializationError::yamlserialize("test".to_string());
    assert!(error.backtrace().is_some());
}

#[test]
#[cfg(not(feature = "backtrace"))]
fn test_backtrace_not_captured() {
    let error = SerializationError::yamlserialize("test".to_string());
    assert!(error.backtrace().is_none());
}
```

**Important:** Always use the generated constructor methods (lowercase variant name) instead of struct syntax:

```rust
// ❌ WRONG - Will fail with backtrace feature
let err = MyError::SomeVariant { field: value };

// ✅ CORRECT - Works with or without backtrace feature
let err = MyError::somevariant(value);
```

---

**See also:** [docs/rules/coding-standards.md](docs/rules/coding-standards.md)
