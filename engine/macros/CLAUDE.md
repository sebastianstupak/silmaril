# Engine Macros

## Purpose
The macros crate provides procedural macros for code generation:
- **Component Derive**: Automatically implement `Component` trait
- **System Macros**: Generate boilerplate for ECS systems
- **Serialization**: Auto-generate serialization code
- **Network Messages**: Generate network message serialization
- **Reflection**: Generate runtime reflection metadata

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase2-proc-macros.md](../../docs/phase2-proc-macros.md)** - Procedural macro design

## Related Crates
- **engine-core**: Provides traits that macros implement
- **All crates**: All crates use these macros

## Quick Example
```rust
use engine_macros::Component;

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

// Automatically implements Component trait with:
// - Type ID generation
// - Storage optimization hints
// - Serialization support
```

## Key Dependencies
- `syn` - Parsing Rust code
- `quote` - Code generation
- `proc-macro2` - Procedural macro utilities

## Performance Targets
- Compile time: <1 second for typical derive usage
- Zero runtime overhead
- Generated code should be as efficient as hand-written
