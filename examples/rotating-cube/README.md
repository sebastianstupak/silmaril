# rotating-cube

A visual rendering demo built with Silmaril game engine.

## What This Demo Shows

This example demonstrates the core rendering pipeline in action:
- **ECS Integration**: Components (Transform, MeshRenderer, RotationSpeed) drive rendering
- **Mesh Rendering**: Procedurally generated cube mesh uploaded to GPU
- **Camera System**: Perspective camera with view/projection matrices
- **Update Loop**: Game logic updates (rotation) followed by rendering
- **Vulkan Backend**: Full Vulkan rendering with swapchain, command buffers, and synchronization

## Quick Start

### Prerequisites

- Rust 1.75+ ([rustup.rs](https://rustup.rs/))
- Vulkan SDK ([vulkan.lunarg.com](https://vulkan.lunarg.com/))
  - **Windows**: Download from LunarG website
  - **Linux**: `sudo apt install vulkan-tools libvulkan-dev` (Ubuntu/Debian)
  - **macOS**: Install MoltenVK via Homebrew

### Build & Run

```bash
# Run the rotating cube demo
cd examples/rotating-cube
cargo run --bin client

# Or build first, then run
cargo build
./target/debug/client  # Windows: .\target\debug\client.exe
```

**Expected Output:**
- A window opens showing a rotating cube
- Cube rotates around Y axis (up) at 1 radian/second
- Dark blue background (RGB: 0.1, 0.1, 0.15)
- Console logs show FPS every second
- Demo runs for 10 seconds then exits

### Development Commands

```bash
# Format code
cargo xtask fmt

# Run lints
cargo xtask clippy

# Run tests
cargo xtask test all

# Run checks (fmt + clippy + test)
cargo xtask check

# Build for release
cargo xtask build release

# Package for distribution
cargo xtask package
```

## Project Structure

```
rotating-cube/
├── Cargo.toml             # Workspace definition
├── shared/                # Shared game logic (components + systems)
│   ├── src/
│   │   ├── components.rs  # Transform, MeshRenderer, RotationSpeed
│   │   └── systems.rs     # rotation_system
├── client/                # Client-only logic (main loop, rendering)
│   └── src/main.rs        # Initializes renderer, creates ECS world, runs loop
├── templates/             # Entity templates (YAML format)
│   └── cube.yaml          # Rotating cube template
└── xtask/                 # Build automation tasks
```

## Implementation Details

### Components (shared/src/components.rs)

- **Transform**: Position, rotation (quaternion), scale
- **MeshRenderer**: References mesh by ID (u64), visibility flag
- **RotationSpeed**: Rotation rate in radians per second

### Systems (shared/src/systems.rs)

- **rotation_system**: Applies rotation around Y axis using quaternion math

### Main Loop (client/src/main.rs)

1. **Initialization**
   - Create asset manager, load cube mesh
   - Create ECS world
   - Spawn camera entity (positioned at [0, 2, 8])
   - Spawn cube entity (positioned at [0, 0, -5])
   - Initialize Vulkan renderer

2. **Game Loop** (60 FPS)
   - Calculate delta time
   - Run rotation_system to update cube orientation
   - Call `renderer.render_meshes(&world, &assets)`
   - Sleep to maintain ~60 FPS

3. **Rendering Pipeline**
   - Renderer queries ECS for entities with Transform + MeshRenderer
   - Uploads mesh to GPU cache (if not already cached)
   - Calculates MVP matrices (Model * View * Projection)
   - Records Vulkan command buffer with draw calls
   - Submits to GPU and presents frame

## Technical Details

### Engine Crates Used

- **engine-core**: ECS (World, Entity, Component), Transform, Camera, MeshRenderer
- **engine-math**: Vec3, Quat, Transform (with glam backend)
- **engine-renderer**: Renderer, WindowConfig, Vulkan integration
- **engine-assets**: MeshData, Vertex, AssetId

### Performance

- **Target**: 60 FPS at 1080p
- **Cube**: 24 vertices, 36 indices (12 triangles)
- **Draw calls**: 1 per frame
- **GPU memory**: ~1 KB for cube mesh

### Customization

Modify the demo by editing:

1. **Rotation speed**: Change `RotationSpeed::new(1.0)` in main.rs
2. **Camera position**: Change `Vec3::new(0.0, 2.0, 8.0)` in main.rs
3. **Cube position**: Change `[0.0, 0.0, -5.0]` in templates/cube.yaml
4. **Background color**: Change `renderer.set_clear_color([0.1, 0.1, 0.15, 1.0])`
5. **Window size**: Change `WindowConfig { width: 1280, height: 720 }`

## Troubleshooting

**Window doesn't appear:**
- Check Vulkan drivers are installed
- Verify Vulkan SDK is in PATH
- Try running with `RUST_LOG=info` for debug output

**Low FPS:**
- Check GPU supports Vulkan 1.2+
- Verify no other GPU-intensive apps running
- Try building with `--release` flag

## Next Steps

This demo is intentionally simple. For a full game, you'd add:
- Input handling (keyboard, mouse)
- Multiple meshes and materials
- Lighting (directional, point, spot)
- Physics integration
- Audio system
- UI rendering
- Networking (multiplayer)

## License

Licensed under Apache-2.0
