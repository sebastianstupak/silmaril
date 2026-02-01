# Engine Build Utils

## Purpose
Shared build-time utilities for enforcing architectural rules and code quality standards across all engine modules. This crate is used in `build.rs` files to ensure consistency and maintainability.

## Features

### 1. Print Statement Checking
Enforces use of structured logging (tracing) instead of `println!`/`eprintln!`/`dbg!` in production code.

- ✅ Allowed in: `tests/`, `benches/`, `examples/`
- ❌ Forbidden in: `src/` (production code)
- 🔄 Replacement: Use `tracing::info!`, `tracing::error!`, `tracing::debug!`

### 2. Module Structure Validation
Checks that required modules exist according to architectural guidelines.

- Configurable list of required modules
- Warning or error mode (fail build or just warn)
- Helps enforce consistent crate structure

### 3. Directory Scanning
Reusable utilities for recursively scanning Rust source files.

- Filters .rs files
- Identifies test/bench/example files
- Efficient callback-based processing

## Usage in build.rs

### Simple (Default Configuration)
```rust
// build.rs
fn main() {
    engine_build_utils::run_standard_checks();
}
```

### Custom Configuration
```rust
// build.rs
use engine_build_utils::{PrintCheckConfig, ModuleCheckConfig};

fn main() {
    engine_build_utils::rerun_if_src_changed();

    // Custom print checking
    let print_config = PrintCheckConfig::new("src")
        .fail_on_violation(true)
        .error_prefix("MY_MODULE");

    engine_build_utils::check_no_print_statements(&print_config);

    // Custom module checking
    let module_config = ModuleCheckConfig::new("src")
        .required_modules(vec![
            "lib.rs".to_string(),
            "components.rs".to_string(),
        ])
        .fail_on_missing(false);

    engine_build_utils::check_module_structure(&module_config);
}
```

## Design Principles

1. **Zero Runtime Dependencies** - Build utils should not add to binary size
2. **Configurable** - Each crate can customize checks for its needs
3. **Clear Error Messages** - Violations should be easy to understand and fix
4. **Non-Invasive** - Checks run at build time, no runtime overhead

## Adding to Your Module

1. Add to `Cargo.toml`:
```toml
[build-dependencies]
engine-build-utils = { path = "../build-utils" }
```

2. Create `build.rs`:
```rust
fn main() {
    engine_build_utils::run_standard_checks();
}
```

3. Customize as needed for your module's requirements

## Extending

To add new checks:

1. Create a new module in `src/` (e.g., `src/my_check.rs`)
2. Export configuration and check function
3. Add to `lib.rs` exports
4. Document usage in this file

## Related Documentation

- [architecture.md](../../docs/architecture.md) - Overall architectural guidelines
- [coding-standards.md](../../docs/rules/coding-standards.md) - Code quality rules

---

**Status:** ✅ Stable (Phase 1.4)
**Used By:** All engine modules with `build.rs`
