# Engine LOD

## Purpose
The LOD (Level of Detail) crate provides automatic detail reduction:
- **LOD Rendering**: Automatic mesh simplification based on distance
- **LOD Networking**: Reduce network bandwidth by sending lower detail to distant clients
- **Automatic Generation**: Generate LOD levels from high-poly meshes
- **Smooth Transitions**: Fade between LOD levels to avoid popping

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase3-lod-rendering.md](../../docs/phase3-lod-rendering.md)** - LOD rendering system
2. **[phase3-lod-networking.md](../../docs/phase3-lod-networking.md)** - Network LOD integration

## Related Crates
- **engine-renderer**: Renders appropriate LOD level based on distance
- **engine-networking**: Sends lower LOD data to distant clients
- **engine-core**: Stores LOD data in ECS components

## Quick Example
```rust
use engine_lod::{LodGenerator, LodLevel};

fn generate_lods(mesh: &Mesh) -> Vec<LodLevel> {
    let generator = LodGenerator::new();

    // Generate 4 LOD levels with 50%, 25%, 12.5%, 6.25% triangles
    generator.generate(mesh, &[0.5, 0.25, 0.125, 0.0625])
}

fn select_lod(distance: f32) -> usize {
    match distance {
        d if d < 10.0 => 0,  // Full detail
        d if d < 50.0 => 1,  // 50% detail
        d if d < 100.0 => 2, // 25% detail
        _ => 3,              // 6.25% detail
    }
}
```

## Key Dependencies
- `meshopt` - Mesh simplification
- `engine-core` - ECS integration
- `engine-renderer` - Rendering integration

## Performance Targets
- 4+ LOD levels per mesh
- Automatic generation in <100ms per mesh
- Smooth transitions without visible popping
