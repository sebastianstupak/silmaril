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
use agent_game_engine_core::define_error;

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

---

## 📊 **Error Code Ranges**

| Range | System |
|-------|--------|
| 1000-1999 | Core (ECS, assets) |
| 2000-2999 | Renderer (Vulkan, shaders) |
| 3000-3999 | Networking |
| 4000-4999 | Physics |
| 5000-5999 | Audio |
| 10000+ | Game-specific |

---

## 🧪 **Testing**

```rust
#[test]
fn test_error_serialization() {
    let err = RendererError::VulkanInit {
        details: "No GPU found".to_string()
    };

    assert_eq!(err.code(), ErrorCode::VulkanInitFailed);
    assert_eq!(err.severity(), ErrorSeverity::Critical);
}
```

---

**See also:** [docs/rules/coding-standards.md](docs/rules/coding-standards.md)
