# Engine Renderer

## Purpose
The renderer crate provides high-performance Vulkan-based rendering:
- **Vulkan Context**: Device initialization, swapchain management, and resource allocation
- **Basic Rendering**: Command buffer recording, synchronization, and frame presentation
- **Mesh Rendering**: Vertex/index buffers, pipelines, and draw call batching
- **Frame Capture**: Screenshot and video capture capabilities
- **PBR Materials**: Physically-based rendering with metallic-roughness workflow
- **Lighting**: Forward+ rendering with support for thousands of dynamic lights

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase1-vulkan-context.md](../../docs/phase1-vulkan-context.md)** - Vulkan initialization and device setup
2. **[phase1-basic-rendering.md](../../docs/phase1-basic-rendering.md)** - Command buffers and frame synchronization
3. **[phase1-mesh-rendering.md](../../docs/phase1-mesh-rendering.md)** - Mesh loading and rendering pipeline
4. **[phase1-frame-capture.md](../../docs/phase1-frame-capture.md)** - Screenshot and video capture
5. **[phase4-pbr-materials.md](../../docs/phase4-pbr-materials.md)** - PBR material system
6. **[phase4-lighting.md](../../docs/phase4-lighting.md)** - Forward+ lighting architecture

## Related Crates
- **engine-core**: Uses ECS to query renderable entities
- **engine-lod**: Integrates LOD system for distant objects
- **engine-observability**: Provides GPU profiling metrics

## Quick Example
```rust
use engine_renderer::{Renderer, Mesh, Material};

fn render_frame(renderer: &mut Renderer, world: &World) {
    renderer.begin_frame();

    // Query all renderable entities
    for (transform, mesh, material) in world.query::<(&Transform, &Mesh, &Material)>() {
        renderer.draw_mesh(mesh, material, transform);
    }

    renderer.end_frame();
}
```

## Key Dependencies
- `ash` - Vulkan bindings
- `vk-mem` - Vulkan memory allocator
- `glam` - Math library (vectors, matrices)
- `engine-core` - ECS integration

## Performance Targets
- 60 FPS at 1080p with 100K+ triangles
- Support for 10K+ dynamic lights (Forward+)
- Draw call batching: 1000+ meshes per frame
- GPU memory: <500MB for typical scene
