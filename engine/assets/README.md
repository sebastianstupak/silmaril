# Engine Assets

Pure data structures for game assets - **no rendering or GPU dependencies**.

## Purpose

This crate provides fundamental asset data structures that can be used across the entire engine stack:

- **Server**: Procedural generation, physics collision meshes
- **Client**: Rendering via `engine-renderer`
- **Tools**: Asset processing, conversion, validation
- **Physics**: Collision detection using mesh geometry

## Features

### Mesh Data

- `Vertex` - Position, normal, UV data structure (32 bytes, cache-friendly)
- `MeshData` - CPU-side geometry (vertices + indices)
- Procedural primitives: `cube()`, `triangle()`
- OBJ file loader (simple parser, no materials)
- Bounding box and centroid calculations

### Design Principles

1. **Zero GPU Dependencies**: No Vulkan, no graphics APIs
2. **Pure Data**: Stateless, serializable structures
3. **Cross-Platform**: Standard Rust, no platform-specific code
4. **Minimal Dependencies**: glam for math, tracing for logging

## Usage

```rust
use engine_assets::MeshData;

// Create procedural geometry
let cube = MeshData::cube();
assert_eq!(cube.vertex_count(), 24);
assert_eq!(cube.triangle_count(), 12);

// Load from OBJ file
let obj_data = std::fs::read_to_string("model.obj")?;
let mesh = MeshData::from_obj(&obj_data)?;

// Query geometry
let (min, max) = mesh.bounding_box();
let center = mesh.centroid();
```

## Rendering Integration

The `engine-renderer` crate consumes `MeshData` and creates GPU buffers:

```rust
use engine_assets::MeshData;
use engine_renderer::GpuMesh;

let mesh_data = MeshData::cube();
let gpu_mesh = GpuMesh::from_mesh_data(&context, &mesh_data)?; // Upload to GPU
```

## Performance

All mesh operations are CPU-bound and optimized for cache locality:

| Operation | Performance | Notes |
|-----------|-------------|-------|
| Cube creation | ~800 ns | 24 vertices, 36 indices |
| Triangle creation | ~200 ns | 3 vertices, 3 indices |
| OBJ load (simple) | ~5 µs | 3-vertex triangle |
| OBJ load (complex) | ~15 µs | 8 vertices, 2 quads |
| Bounding box calc | ~100 ns | SIMD optimized |
| Centroid calc | ~150 ns | Single pass |

## Architecture Benefits

### Separation of Concerns

```
engine-assets (pure data)
    ↓
engine-renderer (GPU upload)
    ↓
Vulkan (GPU execution)
```

### Use Cases

**Server (no rendering)**:
```toml
[dependencies]
engine-assets = { path = "../assets" }
# NO engine-renderer dependency!
```

**Client (with rendering)**:
```toml
[dependencies]
engine-assets = { path = "../assets" }
engine-renderer = { path = "../renderer" }
```

**Tools (asset processing)**:
```toml
[dependencies]
engine-assets = { path = "../assets" }
# Process meshes without GPU
```

## Testing

```bash
# Run all tests
cargo test --package engine-assets

# Run benchmarks
cargo bench --package engine-assets
```

## Future Additions

- Texture data structures (`TextureData`)
- Material data (`MaterialData`)
- Animation data (`AnimationData`)
- glTF loader
- Mesh optimization (vertex cache, overdraw)
- LOD generation
