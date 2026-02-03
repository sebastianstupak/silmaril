# Engine Assets

**Pure data structures for game assets - no rendering or GPU dependencies.**

---

## Purpose

This crate provides fundamental asset data structures that can be used across the entire engine stack:

- **Server**: Procedural generation, physics collision meshes (no GPU needed)
- **Client**: Rendering via `engine-renderer` (GPU upload happens there)
- **Tools**: Asset processing, conversion, validation
- **Physics**: Collision detection using mesh geometry

---

## Quick Start

```rust
use engine_assets::{AssetManager, MeshData, TextureData};
use std::path::Path;

// Create asset manager
let manager = AssetManager::new();

// Load assets synchronously
let mesh_handle = manager.load_sync::<MeshData>(Path::new("assets/cube.obj"))?;
let texture_handle = manager.load_sync::<TextureData>(Path::new("assets/brick.png"))?;

// Access assets
if let Some(mesh) = manager.get_mesh(mesh_handle.id()) {
    println!("Loaded mesh with {} vertices", mesh.vertices.len());
}

if let Some(texture) = manager.get_texture(texture_handle.id()) {
    println!("Loaded {}x{} texture", texture.width, texture.height);
}
```

---

## Features

### Core Asset Types

- **Meshes** (`MeshData`): 3D geometry (vertices + indices)
  - OBJ loader
  - glTF loader
  - Procedural primitives (cube, triangle)
  - Bounding box and centroid

- **Textures** (`TextureData`): Image data
  - PNG/JPG loader
  - DDS loader (BC compressed)
  - Mipmap generation
  - Multiple formats (RGBA8, BC7, ASTC, etc.)

- **Shaders** (`ShaderData`): GPU shaders
  - GLSL source loading
  - SPIR-V binary loading
  - Stage detection (vertex, fragment, compute)

- **Materials** (`MaterialData`): PBR materials
  - Base color, metallic-roughness, normal, emissive
  - Texture references (AssetId)
  - Factor overrides

- **Audio** (`AudioData`): Sound effects and music
  - WAV loader
  - Ogg Vorbis loader
  - PCM and compressed formats

- **Fonts** (`FontData`): TTF/OTF fonts
  - Font metrics
  - Style and weight detection

### Asset Management

- **AssetManager**: Central coordinator for all asset operations
- **AssetHandle<T>**: Type-safe, reference-counted handles
- **AssetId**: Content-addressable identifiers (Blake3 hash)
- **AssetRegistry**: Per-type storage with thread-safe access

### Loading Strategies

- **Synchronous**: Blocking loads for small assets (< 1MB)
- **Asynchronous**: Non-blocking loads with Tokio (requires `async` feature)
- **Streaming**: Progressive LOD loading (requires `async` feature)

### Memory Management

- **LRU Cache**: Automatic eviction of least-recently-used assets
- **Memory Budgets**: Per-type and global memory limits
- **Hard/Soft References**: Control eviction behavior

### Hot-Reload (requires `hot-reload` feature)

- **File Watching**: Cross-platform with `notify` crate
- **Debouncing**: Configurable delay to avoid rapid reloads
- **Batching**: Efficient bulk reloading
- **Validation**: Invalid assets don't crash the engine
- **Error Recovery**: Keeps old asset if new version fails

### Network Transfer

- **Client-Server Protocol**: Efficient asset distribution
- **Chunked Transfer**: Large assets sent in chunks (1MB default)
- **Resumable Downloads**: Range requests for interrupted transfers
- **Compression**: LZ4 compression (requires `lz4` feature)
- **Integrity**: Blake3 checksums for validation

### Manifests & Bundles

- **Manifests**: Declarative asset metadata with dependency tracking
- **Bundles**: Packed asset archives with compression
- **Validation**: Multi-layer validation (format, data, checksums)
- **Topological Sorting**: Load assets in dependency order

---

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | LZ4 compression | `lz4_flex` |
| `async` | Async loading with Tokio | `tokio` |
| `hot-reload` | File watching and auto-reload | `notify` |
| `lz4` | LZ4 compression | `lz4_flex` |
| `zstd` | Zstd compression | `zstd` |
| `fbx-support` | FBX file loading | `fbxcel-dom` |
| `backtrace` | Error backtraces | `engine-core/backtrace` |

---

## Architecture

### Design Principles

1. **Pure Data**: Assets are CPU-side data structures, no GPU dependencies
2. **Content-Addressable**: Assets identified by Blake3 hash for deduplication
3. **Thread-Safe**: Parallel loading and concurrent access with minimal locking
4. **Server-Compatible**: No rendering dependencies, works on headless servers
5. **Failure Recovery**: Invalid assets don't crash the engine

### Separation of Concerns

```
engine-assets (pure data)
    ↓
engine-renderer (GPU upload)
    ↓
Vulkan (GPU execution)
```

This architecture allows:
- **Server** builds to exclude rendering dependencies
- **Tools** to process assets without GPU
- **Client** to control GPU resource lifetime

---

## Usage Examples

### Example 1: Load Multiple Assets

```rust
use engine_assets::{AssetManager, MeshData, TextureData, MaterialData};

let manager = AssetManager::new();

// Load mesh
let cube = manager.load_sync::<MeshData>(Path::new("assets/cube.obj"))?;

// Load textures
let base_color = manager.load_sync::<TextureData>(Path::new("assets/brick_color.png"))?;
let normal = manager.load_sync::<TextureData>(Path::new("assets/brick_normal.png"))?;

// Create material referencing textures
let material = MaterialData {
    base_color: Some(base_color.id()),
    normal: Some(normal.id()),
    metallic_factor: 0.2,
    roughness_factor: 0.8,
    ..Default::default()
};
```

### Example 2: Async Loading (requires `async` feature)

```rust
use engine_assets::{AsyncLoader, LoadPriority};

let loader = AsyncLoader::new(Arc::clone(&manager), 4);

// Start multiple async loads
let mesh_fut = loader.load_async::<MeshData>(
    Path::new("assets/level.glb"),
    LoadPriority::High
);

let texture_fut = loader.load_async::<TextureData>(
    Path::new("assets/environment.png"),
    LoadPriority::Normal
);

// Poll progress
println!("Mesh: {:.1}%", mesh_fut.progress() * 100.0);
println!("Texture: {:.1}%", texture_fut.progress() * 100.0);

// Wait for completion
let mesh = mesh_fut.await_result().await?;
let texture = texture_fut.await_result().await?;
```

### Example 3: Hot-Reload (requires `hot-reload` feature)

```rust
use engine_assets::{HotReloader, HotReloadConfig, HotReloadEvent};

let mut reloader = HotReloader::new(manager.clone(), HotReloadConfig::default())?;
reloader.watch(Path::new("assets"))?;

// In game loop
loop {
    reloader.process_events();

    while let Some(event) = reloader.poll_event() {
        match event {
            HotReloadEvent::Modified { path, new_id, .. } => {
                println!("Reloaded: {:?} (new ID: {})", path, new_id);
                // Update GPU resources if needed
            }
            HotReloadEvent::ReloadFailed { path, error, .. } => {
                eprintln!("Reload failed: {:?} - {}", path, error);
            }
            _ => {}
        }
    }

    // ... render frame ...
}
```

### Example 4: Memory Management

```rust
use engine_assets::{LruCache, MemoryBudget, MemorySized};

// Configure memory budgets
let budget = MemoryBudget {
    total: 1024 * 1024 * 1024,  // 1 GB
    mesh: 100 * 1024 * 1024,    // 100 MB
    texture: 500 * 1024 * 1024, // 500 MB
    ..Default::default()
};

let cache = LruCache::new(budget);

// Track asset access
cache.access(asset_id, AssetType::Mesh);

// Check if eviction needed
if cache.is_over_budget(AssetType::Mesh) {
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 10);
    for id in candidates {
        registry.remove(id);
    }
}

// Check memory stats
let stats = cache.stats();
println!("Total memory: {} MB", stats.total_allocated / (1024 * 1024));
println!("Mesh memory: {} MB", stats.mesh_memory / (1024 * 1024));
```

### Example 5: Network Transfer

```rust
use engine_assets::network::{AssetNetworkServer, AssetNetworkClient, TransferPriority};

// Server
let mut server = AssetNetworkServer::new(1024 * 1024);
server.register_asset(mesh_id, mesh_bytes);

// Client
let mut client = AssetNetworkClient::new(4);
client.request_asset(mesh_id, TransferPriority::Critical);

// Transfer loop
if let Some(request) = client.next_request() {
    let responses = server.handle_request(request);
    for response in responses {
        client.handle_message(response)?;
    }
}

// Get completed asset
if let Some(data) = client.take_completed(mesh_id) {
    let mesh = MeshData::parse(&data)?;
}
```

### Example 6: Asset Bundles

```rust
use engine_assets::{AssetBundle, AssetManifest, CompressionFormat};

// Create manifest
let mut manifest = AssetManifest::new();
manifest.add_asset(create_entry(mesh_id, "cube.obj", AssetType::Mesh));
manifest.add_asset(create_entry(texture_id, "brick.png", AssetType::Texture));
manifest.validate()?;  // Check dependencies, cycles

// Pack into bundle
let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Lz4);
bundle.add_asset(mesh_id, mesh_bytes)?;
bundle.add_asset(texture_id, texture_bytes)?;

let packed = bundle.pack()?;
std::fs::write("assets/level1.bundle.lz4", packed)?;

// Unpack bundle
let packed = std::fs::read("assets/level1.bundle.lz4")?;
let bundle = AssetBundle::unpack(&packed)?;

// Load assets from bundle
for entry in bundle.manifest().assets {
    if let Some(data) = bundle.get_asset(entry.id) {
        // Parse and use asset
    }
}
```

---

## Performance

All mesh operations are CPU-bound and optimized for cache locality:

| Operation | Performance | Notes |
|-----------|-------------|-------|
| Cube creation | ~800 ns | 24 vertices, 36 indices |
| Triangle creation | ~200 ns | 3 vertices, 3 indices |
| OBJ load (simple) | ~5 µs | 3-vertex triangle |
| OBJ load (complex) | ~15 µs | 8 vertices, 2 quads |
| glTF load | < 5 ms | Typical scene |
| PNG load (256x256) | < 5 ms | Image crate |
| DDS load (256x256) | < 1 ms | Pre-compressed |
| Bounding box calc | ~100 ns | SIMD optimized |
| Asset lookup | O(1) | DashMap |
| Hot-reload | < 10 ms | Small assets |

---

## Testing

```bash
# Run all tests
cargo test --package engine-assets --all-features

# Run specific test
cargo test --package engine-assets mesh_loading

# Run benchmarks
cargo bench --package engine-assets

# Run with hot-reload feature
cargo test --package engine-assets --features hot-reload

# Run with async feature
cargo test --package engine-assets --features async
```

---

## Directory Structure

```
engine/assets/
├── src/
│   ├── lib.rs           # Public API
│   ├── asset_id.rs      # AssetId (Blake3 hash)
│   ├── handle.rs        # AssetHandle (type-safe references)
│   ├── registry.rs      # AssetRegistry (per-type storage)
│   ├── manager.rs       # AssetManager (central coordinator)
│   ├── loader.rs        # Loading strategies
│   ├── async_loader.rs  # Async loading (feature: async)
│   ├── memory.rs        # LRU cache and memory management
│   ├── hot_reload.rs    # Hot-reload system (feature: hot-reload)
│   ├── network.rs       # Network transfer protocol
│   ├── manifest.rs      # Asset manifests
│   ├── bundle.rs        # Asset bundles
│   ├── validation.rs    # Multi-layer validation
│   ├── mesh.rs          # MeshData
│   ├── texture.rs       # TextureData
│   ├── shader.rs        # ShaderData
│   ├── material.rs      # MaterialData
│   ├── audio.rs         # AudioData
│   └── font.rs          # FontData
├── tests/               # Integration tests
├── benches/             # Performance benchmarks
├── Cargo.toml
└── README.md            # This file
```

---

## Dependencies

### Core
- `glam` - Math types (Vec2, Vec3, Vec4)
- `blake3` - Content-addressable IDs
- `parking_lot` - Fast RwLock
- `dashmap` - Concurrent HashMap
- `serde` - Serialization
- `bincode` - Binary serialization
- `serde_yaml` - YAML serialization
- `tracing` - Structured logging

### Asset Loaders
- `gltf` - glTF mesh loader
- `image` - PNG/JPG texture loader
- `ddsfile` - DDS texture loader
- `hound` - WAV audio loader
- `lewton` - Ogg Vorbis audio loader
- `ttf-parser` - Font loader

### Optional
- `notify` - File watching (feature: `hot-reload`)
- `tokio` - Async runtime (feature: `async`)
- `lz4_flex` - LZ4 compression (feature: `lz4`)
- `zstd` - Zstd compression (feature: `zstd`)
- `fbxcel-dom` - FBX loader (feature: `fbx-support`)
- `linked-hash-map` - LRU tracking

---

## Integration with Renderer

### GPU Upload (in `engine-renderer`)

```rust
use engine_assets::MeshData;
use engine_renderer::GpuMesh;

// CPU-side mesh (engine-assets)
let mesh_data = MeshData::cube();

// GPU upload (engine-renderer)
let gpu_mesh = GpuMesh::from_mesh_data(&context, &mesh_data)?;
```

### Asset → GPU Caching

```rust
use engine_assets::AssetId;
use std::collections::HashMap;

pub struct GpuAssetCache {
    meshes: HashMap<AssetId, GpuMesh>,
    textures: HashMap<AssetId, GpuTexture>,
}

impl GpuAssetCache {
    pub fn get_or_upload(&mut self, id: AssetId) -> &GpuMesh {
        if !self.meshes.contains_key(&id) {
            let mesh_data = asset_manager.get_mesh(id)?;
            let gpu_mesh = renderer.upload_mesh(&mesh_data)?;
            self.meshes.insert(id, gpu_mesh);
        }
        &self.meshes[&id]
    }
}
```

---

## Use Cases

### Server (No Rendering)

```toml
[dependencies]
engine-assets = { path = "../assets" }
# NO engine-renderer dependency!
```

Use for:
- Procedural generation (MeshData creation)
- Physics collision meshes
- Server-side validation

### Client (With Rendering)

```toml
[dependencies]
engine-assets = { path = "../assets" }
engine-renderer = { path = "../renderer" }
```

Use for:
- Asset loading and caching
- GPU resource management
- Hot-reload during development

### Tools (Asset Processing)

```toml
[dependencies]
engine-assets = { path = "../assets", features = ["fbx-support"] }
```

Use for:
- Asset conversion (OBJ → glTF)
- Bundle packing
- Validation and integrity checking

---

## See Also

- **Full Documentation**: `docs/assets.md`
- **Tutorial**: `docs/tutorials/asset-system-tutorial.md`
- **Task Spec**: `docs/tasks/phase1-7-asset-system.md`
- **Architecture**: `docs/architecture.md`
