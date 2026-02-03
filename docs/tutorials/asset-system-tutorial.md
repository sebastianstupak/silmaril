# Asset System Tutorial

**Learn the Silmaril asset system step-by-step.**

---

## Table of Contents

1. [Hello Asset - Load Your First Mesh](#hello-asset---load-your-first-mesh)
2. [Loading Different Asset Types](#loading-different-asset-types)
3. [Async Loading for Better Performance](#async-loading-for-better-performance)
4. [Using Hot-Reload for Development](#using-hot-reload-for-development)
5. [Creating and Using Manifests](#creating-and-using-manifests)
6. [Packing Assets into Bundles](#packing-assets-into-bundles)
7. [Network Asset Transfer](#network-asset-transfer)
8. [Memory Management](#memory-management)
9. [Validation and Error Handling](#validation-and-error-handling)
10. [Integration with Rendering](#integration-with-rendering)

---

## Hello Asset - Load Your First Mesh

Let's start with the simplest possible example: loading a mesh asset.

### Step 1: Add Dependencies

```toml
# Cargo.toml
[dependencies]
engine-assets = { path = "engine/assets" }
```

### Step 2: Create an AssetManager

```rust
use engine_assets::AssetManager;

fn main() {
    let manager = AssetManager::new();
    println!("Asset manager created!");
}
```

The `AssetManager` is the central coordinator for all asset operations. It manages loading, caching, and lifetime of all assets.

### Step 3: Load a Mesh

```rust
use engine_assets::{AssetManager, MeshData};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AssetManager::new();

    // Load a mesh synchronously
    let cube_handle = manager.load_sync::<MeshData>(
        Path::new("assets/cube.obj")
    )?;

    println!("Loaded mesh with ID: {}", cube_handle.id());

    Ok(())
}
```

**What's happening:**
1. `load_sync::<MeshData>` loads the asset synchronously (blocks until complete)
2. Returns an `AssetHandle<MeshData>` which is a type-safe reference to the asset
3. The handle contains an `AssetId` (Blake3 hash of the content)

### Step 4: Access the Mesh Data

```rust
use engine_assets::{AssetManager, MeshData};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AssetManager::new();

    // Load mesh
    let cube_handle = manager.load_sync::<MeshData>(
        Path::new("assets/cube.obj")
    )?;

    // Access the mesh data
    if let Some(mesh) = manager.get_mesh(cube_handle.id()) {
        println!("Mesh info:");
        println!("  Vertices: {}", mesh.vertices.len());
        println!("  Triangles: {}", mesh.triangle_count());
        println!("  Bounding box: {:?}", mesh.bounding_box());
    }

    Ok(())
}
```

**Output:**
```
Mesh info:
  Vertices: 24
  Triangles: 12
  Bounding box: (Vec3(-1.0, -1.0, -1.0), Vec3(1.0, 1.0, 1.0))
```

---

## Loading Different Asset Types

The asset system supports multiple asset types. Let's load one of each.

### Meshes

```rust
use engine_assets::{AssetManager, MeshData};
use std::path::Path;

let manager = AssetManager::new();

// OBJ file
let obj_mesh = manager.load_sync::<MeshData>(
    Path::new("assets/models/character.obj")
)?;

// glTF file
let gltf_mesh = manager.load_sync::<MeshData>(
    Path::new("assets/models/level.glb")
)?;

// Use the mesh
if let Some(mesh) = manager.get_mesh(obj_mesh.id()) {
    println!("Loaded {} vertices", mesh.vertices.len());
}
```

### Textures

```rust
use engine_assets::{AssetManager, TextureData};
use std::path::Path;

let manager = AssetManager::new();

// PNG texture
let png_texture = manager.load_sync::<TextureData>(
    Path::new("assets/textures/brick.png")
)?;

// DDS texture (compressed)
let dds_texture = manager.load_sync::<TextureData>(
    Path::new("assets/textures/terrain.dds")
)?;

// Use the texture
if let Some(texture) = manager.get_texture(png_texture.id()) {
    println!("Loaded {}x{} texture ({:?})",
        texture.width,
        texture.height,
        texture.format
    );
}
```

### Shaders

```rust
use engine_assets::{AssetManager, ShaderData};
use std::path::Path;

let manager = AssetManager::new();

// GLSL source
let vert_shader = manager.load_sync::<ShaderData>(
    Path::new("assets/shaders/basic.vert")
)?;

// SPIR-V binary
let frag_shader = manager.load_sync::<ShaderData>(
    Path::new("assets/shaders/basic.frag.spv")
)?;

// Use the shader
if let Some(shader) = manager.get_shader(vert_shader.id()) {
    println!("Loaded {} shader: {}",
        shader.stage.as_str(),
        shader.entry_point
    );
}
```

### Audio

```rust
use engine_assets::{AssetManager, AudioData};
use std::path::Path;

let manager = AssetManager::new();

// WAV file
let wav_audio = manager.load_sync::<AudioData>(
    Path::new("assets/sounds/explosion.wav")
)?;

// Ogg Vorbis file
let ogg_audio = manager.load_sync::<AudioData>(
    Path::new("assets/music/background.ogg")
)?;

// Use the audio
if let Some(audio) = manager.get_audio(wav_audio.id()) {
    println!("Loaded audio: {} Hz, {} channels",
        audio.sample_rate,
        audio.channels
    );
}
```

### Fonts

```rust
use engine_assets::{AssetManager, FontData};
use std::path::Path;

let manager = AssetManager::new();

// TTF font
let font = manager.load_sync::<FontData>(
    Path::new("assets/fonts/Roboto-Regular.ttf")
)?;

// Use the font
if let Some(font_data) = manager.get_font(font.id()) {
    println!("Loaded font: {} {} {:?}",
        font_data.family,
        font_data.weight.as_str(),
        font_data.style
    );
}
```

---

## Async Loading for Better Performance

Synchronous loading blocks your game loop. Use async loading for large assets or background loading.

### Step 1: Enable Async Feature

```toml
# Cargo.toml
[dependencies]
engine-assets = { path = "engine/assets", features = ["async"] }
tokio = { version = "1.35", features = ["rt", "rt-multi-thread", "macros"] }
```

### Step 2: Create AsyncLoader

```rust
use engine_assets::{AsyncLoader, AssetManager, LoadPriority};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());
    let loader = AsyncLoader::new(manager.clone(), 4); // 4 worker threads

    Ok(())
}
```

### Step 3: Load Async with Progress

```rust
use engine_assets::{AsyncLoader, AssetManager, MeshData, LoadPriority};
use std::sync::Arc;
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());
    let loader = AsyncLoader::new(manager.clone(), 4);

    // Start async load
    let handle = loader.load_async::<MeshData>(
        Path::new("assets/large_level.glb"),
        LoadPriority::High
    );

    // Poll progress while loading
    while !handle.is_complete() {
        println!("Loading: {:.1}%", handle.progress() * 100.0);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Get result
    let mesh_handle = handle.await_result().await?;
    println!("Load complete! Mesh ID: {}", mesh_handle.id());

    Ok(())
}
```

**Output:**
```
Loading: 10.0%
Loading: 50.0%
Loading: 90.0%
Loading: 100.0%
Load complete! Mesh ID: blake3:abc123...
```

### Step 4: Load Multiple Assets in Parallel

```rust
use tokio::join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());
    let loader = AsyncLoader::new(manager.clone(), 4);

    // Start multiple loads in parallel
    let mesh_handle = loader.load_async::<MeshData>(
        Path::new("assets/level.glb"),
        LoadPriority::High
    );

    let texture_handle = loader.load_async::<TextureData>(
        Path::new("assets/environment.png"),
        LoadPriority::Normal
    );

    let audio_handle = loader.load_async::<AudioData>(
        Path::new("assets/music.ogg"),
        LoadPriority::Low
    );

    // Wait for all to complete
    let (mesh, texture, audio) = join!(
        mesh_handle.await_result(),
        texture_handle.await_result(),
        audio_handle.await_result()
    );

    println!("All assets loaded!");
    println!("  Mesh: {}", mesh?.id());
    println!("  Texture: {}", texture?.id());
    println!("  Audio: {}", audio?.id());

    Ok(())
}
```

---

## Using Hot-Reload for Development

Hot-reload automatically reloads assets when files change. Perfect for iterating on art assets.

### Step 1: Enable Hot-Reload Feature

```toml
# Cargo.toml
[dependencies]
engine-assets = { path = "engine/assets", features = ["hot-reload"] }
```

### Step 2: Create HotReloader

```rust
use engine_assets::{AssetManager, HotReloader, HotReloadConfig};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());

    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(300), // Wait 300ms after last write
        enable_batching: true,                          // Batch multiple changes
        max_batch_size: 10,                             // Max 10 assets per batch
        batch_timeout: Duration::from_millis(500),      // Max wait for batch
    };

    let mut hot_reloader = HotReloader::new(manager.clone(), config)?;

    Ok(())
}
```

### Step 3: Watch Asset Directory

```rust
use std::path::Path;

// Watch the assets directory
hot_reloader.watch(Path::new("assets"))?;
println!("Watching assets/ for changes...");
```

### Step 4: Process Events in Game Loop

```rust
use engine_assets::HotReloadEvent;
use std::time::Duration;

// Game loop
loop {
    // Process hot-reload events
    hot_reloader.process_events();

    // Handle events
    while let Some(event) = hot_reloader.poll_event() {
        match event {
            HotReloadEvent::Modified { path, old_id, new_id, asset_type } => {
                println!("✅ Reloaded: {:?} ({})", path, asset_type.as_str());
                println!("   Old ID: {}", old_id);
                println!("   New ID: {}", new_id);

                // Update GPU resources if needed
                // gpu_cache.invalidate(old_id);
                // gpu_cache.upload(new_id);
            }

            HotReloadEvent::ReloadFailed { path, error, .. } => {
                eprintln!("❌ Failed to reload {:?}: {}", path, error);
                // Keep using old version
            }

            HotReloadEvent::BatchReloaded { count, duration_ms } => {
                println!("📦 Batch reload: {} assets in {}ms", count, duration_ms);
            }

            _ => {}
        }
    }

    // ... render frame ...
    std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
}
```

**Output when you edit a file:**
```
✅ Reloaded: "assets/models/cube.obj" (Mesh)
   Old ID: blake3:abc123...
   New ID: blake3:def456...
```

### Step 5: Handle Reload Errors Gracefully

```rust
// Example: Edit cube.obj to be invalid
match event {
    HotReloadEvent::ReloadFailed { path, error, .. } => {
        eprintln!("❌ Failed to reload {:?}", path);
        eprintln!("   Error: {}", error);
        eprintln!("   Keeping old version");

        // Show notification to user
        // ui.show_notification("Asset reload failed - check console");
    }
    _ => {}
}
```

---

## Creating and Using Manifests

Manifests declare asset metadata and dependencies. They enable:
- Dependency tracking (materials depend on textures)
- Integrity validation (checksums)
- Topological sorting (load dependencies first)

### Step 1: Create a Manifest

```rust
use engine_assets::{AssetManifest, AssetEntry, AssetId, AssetType};
use std::path::PathBuf;

let mut manifest = AssetManifest::new();

// Add mesh asset
let mesh_id = AssetId::from_content(b"cube");
let mesh_data = std::fs::read("assets/cube.obj")?;
let mesh_checksum = *blake3::hash(&mesh_data).as_bytes();

let mesh_entry = AssetEntry::new(
    mesh_id,
    PathBuf::from("assets/cube.obj"),
    AssetType::Mesh,
    mesh_data.len() as u64,
    mesh_checksum,
);
manifest.add_asset(mesh_entry);

// Add texture asset
let texture_id = AssetId::from_content(b"brick");
let texture_data = std::fs::read("assets/brick.png")?;
let texture_checksum = *blake3::hash(&texture_data).as_bytes();

let texture_entry = AssetEntry::new(
    texture_id,
    PathBuf::from("assets/brick.png"),
    AssetType::Texture,
    texture_data.len() as u64,
    texture_checksum,
);
manifest.add_asset(texture_entry);
```

### Step 2: Add Dependencies

```rust
// Material depends on textures
let material_id = AssetId::from_content(b"brick_material");
let material_data = create_material_bytes();
let material_checksum = *blake3::hash(&material_data).as_bytes();

let mut material_entry = AssetEntry::new(
    material_id,
    PathBuf::from("assets/brick.mat"),
    AssetType::Material,
    material_data.len() as u64,
    material_checksum,
);

// Add dependencies
material_entry.add_dependency(texture_id);  // Base color texture
material_entry.add_dependency(normal_texture_id);  // Normal map

manifest.add_asset(material_entry);
```

### Step 3: Validate Manifest

```rust
// Validate manifest (checks for missing deps, cycles)
match manifest.validate() {
    Ok(()) => println!("✅ Manifest is valid"),
    Err(e) => {
        eprintln!("❌ Manifest validation failed: {}", e);
        return Err(e.into());
    }
}
```

### Step 4: Get Load Order (Topological Sort)

```rust
// Get assets in dependency order
let load_order = manifest.topological_sort()?;

println!("Load order:");
for (i, asset_id) in load_order.iter().enumerate() {
    if let Some(entry) = manifest.get_asset(*asset_id) {
        println!("  {}. {:?} ({})", i + 1, entry.path, entry.asset_type.as_str());
    }
}
```

**Output:**
```
Load order:
  1. "assets/brick.png" (Texture)
  2. "assets/brick_normal.png" (Texture)
  3. "assets/brick.mat" (Material)
  4. "assets/cube.obj" (Mesh)
```

### Step 5: Save and Load Manifests

```rust
// Save to YAML (human-readable)
let yaml = manifest.to_yaml()?;
std::fs::write("assets/manifest.yaml", yaml)?;
println!("Saved manifest to assets/manifest.yaml");

// Load from YAML
let yaml = std::fs::read_to_string("assets/manifest.yaml")?;
let loaded_manifest = AssetManifest::from_yaml(&yaml)?;

// Or use Bincode (faster, smaller)
let bincode = manifest.to_bincode()?;
std::fs::write("assets/manifest.bin", bincode)?;

let loaded_manifest = AssetManifest::from_bincode(&bincode)?;
```

---

## Packing Assets into Bundles

Bundles pack multiple assets into a single file for distribution.

### Step 1: Create a Bundle from Manifest

```rust
use engine_assets::{AssetBundle, AssetManifest, CompressionFormat};

// Create or load manifest
let manifest = create_manifest()?;

// Create bundle with LZ4 compression
let mut bundle = AssetBundle::from_manifest(
    manifest,
    CompressionFormat::Lz4
);
```

### Step 2: Add Assets to Bundle

```rust
// Add each asset's data
let mesh_id = AssetId::from_content(b"cube");
let mesh_bytes = std::fs::read("assets/cube.obj")?;
bundle.add_asset(mesh_id, mesh_bytes)?;

let texture_id = AssetId::from_content(b"brick");
let texture_bytes = std::fs::read("assets/brick.png")?;
bundle.add_asset(texture_id, texture_bytes)?;

println!("Added {} assets to bundle", bundle.manifest().assets.len());
```

### Step 3: Pack and Save Bundle

```rust
// Pack into binary format
let packed = bundle.pack()?;
println!("Packed bundle: {} bytes", packed.len());

// Save to file
std::fs::write("assets/level1.bundle.lz4", packed)?;
println!("Saved to assets/level1.bundle.lz4");
```

### Step 4: Load and Unpack Bundle

```rust
// Load bundle file
let packed = std::fs::read("assets/level1.bundle.lz4")?;

// Unpack
let bundle = AssetBundle::unpack(&packed)?;
println!("Unpacked bundle with {} assets", bundle.manifest().assets.len());

// Get bundle stats
let stats = bundle.stats();
println!("Bundle stats:");
println!("  Assets: {}", stats.asset_count);
println!("  Total size: {} MB", stats.total_size / (1024 * 1024));
println!("  Compression: {:?}", stats.compression);
```

### Step 5: Load Assets from Bundle

```rust
// Iterate over all assets in bundle
for entry in bundle.manifest().assets {
    println!("Loading: {:?} ({})", entry.path, entry.asset_type.as_str());

    // Get asset data
    if let Some(data) = bundle.get_asset(entry.id) {
        // Parse based on type
        match entry.asset_type {
            AssetType::Mesh => {
                let mesh = MeshData::parse(data)?;
                println!("  Loaded mesh with {} vertices", mesh.vertices.len());
            }
            AssetType::Texture => {
                let texture = TextureData::from_image_bytes(data)?;
                println!("  Loaded {}x{} texture", texture.width, texture.height);
            }
            _ => {}
        }
    }
}
```

---

## Network Asset Transfer

Transfer assets from server to client over the network.

### Server Side

```rust
use engine_assets::network::{AssetNetworkServer, AssetNetworkMessage};

fn setup_server() -> Result<AssetNetworkServer, Box<dyn std::error::Error>> {
    let mut server = AssetNetworkServer::new(1024 * 1024); // 1MB chunks

    // Load and register assets
    let mesh_id = AssetId::from_content(b"cube");
    let mesh_bytes = std::fs::read("assets/cube.obj")?;
    server.register_asset(mesh_id, mesh_bytes);

    let texture_id = AssetId::from_content(b"brick");
    let texture_bytes = std::fs::read("assets/brick.png")?;
    server.register_asset(texture_id, texture_bytes);

    println!("Server registered {} assets", server.asset_count());

    Ok(server)
}

fn handle_client_request(
    server: &AssetNetworkServer,
    request: AssetNetworkMessage
) -> Vec<AssetNetworkMessage> {
    // Process client request
    let responses = server.handle_request(request);

    println!("Sending {} responses to client", responses.len());

    responses
}
```

### Client Side

```rust
use engine_assets::network::{AssetNetworkClient, TransferPriority};

fn setup_client() -> AssetNetworkClient {
    AssetNetworkClient::new(4) // Max 4 concurrent transfers
}

fn request_assets(client: &mut AssetNetworkClient) {
    let mesh_id = AssetId::from_content(b"cube");
    let texture_id = AssetId::from_content(b"brick");

    // Request with priorities
    client.request_asset(mesh_id, TransferPriority::Critical);
    client.request_asset(texture_id, TransferPriority::High);

    println!("Queued {} asset requests", client.pending_count());
}

fn client_loop(client: &mut AssetNetworkClient) -> Result<(), Box<dyn std::error::Error>> {
    // Get next request to send
    if let Some(request) = client.next_request() {
        // Send to server (via network)
        send_to_server(request)?;
    }

    // Receive response from server
    let response = receive_from_server()?;

    // Handle response
    client.handle_message(response)?;

    // Check for completed transfers
    let mesh_id = AssetId::from_content(b"cube");
    if let Some(data) = client.take_completed(mesh_id) {
        println!("✅ Received mesh: {} bytes", data.len());

        // Parse and use asset
        let mesh = MeshData::parse(&data)?;
        println!("   Loaded mesh with {} vertices", mesh.vertices.len());
    }

    Ok(())
}
```

### Progress Tracking

```rust
// Check transfer status
let mesh_id = AssetId::from_content(b"cube");
if let Some(status) = client.status(&mesh_id) {
    match status {
        TransferStatus::Queued => {
            println!("Waiting to start...");
        }
        TransferStatus::InProgress { bytes_received, total_bytes } => {
            let progress = bytes_received as f32 / total_bytes as f32 * 100.0;
            println!("Downloading: {:.1}% ({}/{})",
                progress,
                bytes_received,
                total_bytes
            );
        }
        TransferStatus::Completed => {
            println!("Download complete!");
        }
        TransferStatus::Failed { error } => {
            eprintln!("Download failed: {}", error);
        }
    }
}
```

---

## Memory Management

Control memory usage with budgets and LRU eviction.

### Step 1: Configure Memory Budgets

```rust
use engine_assets::{LruCache, MemoryBudget};

let budget = MemoryBudget {
    total: 1024 * 1024 * 1024,  // 1 GB total
    mesh: 100 * 1024 * 1024,    // 100 MB for meshes
    texture: 500 * 1024 * 1024, // 500 MB for textures
    shader: 10 * 1024 * 1024,   // 10 MB for shaders
    material: 50 * 1024 * 1024, // 50 MB for materials
    audio: 200 * 1024 * 1024,   // 200 MB for audio
    font: 50 * 1024 * 1024,     // 50 MB for fonts
};

let cache = LruCache::new(budget);
```

### Step 2: Track Asset Access

```rust
// Track when assets are accessed
cache.access(mesh_id, AssetType::Mesh);
cache.access(texture_id, AssetType::Texture);

// Most recently accessed assets are kept in memory
```

### Step 3: Check Memory Usage

```rust
let stats = cache.stats();

println!("Memory usage:");
println!("  Total: {} MB", stats.total_allocated / (1024 * 1024));
println!("  Meshes: {} MB", stats.mesh_memory / (1024 * 1024));
println!("  Textures: {} MB", stats.texture_memory / (1024 * 1024));
println!("  Audio: {} MB", stats.audio_memory / (1024 * 1024));
```

### Step 4: Evict Assets When Over Budget

```rust
// Check if over budget
if cache.is_over_budget(AssetType::Mesh) {
    let to_free = cache.memory_to_free(AssetType::Mesh);
    println!("Need to free {} MB of mesh memory",
        to_free / (1024 * 1024)
    );

    // Get eviction candidates (LRU order)
    let candidates = cache.eviction_candidates(
        AssetType::Mesh,
        &mesh_registry,
        10  // Get up to 10 candidates
    );

    // Evict least recently used soft-referenced assets
    for id in candidates {
        println!("Evicting mesh: {}", id);
        mesh_registry.remove(id);
        cache.remove(id, AssetType::Mesh);
    }
}
```

### Step 5: Hard vs Soft References

```rust
use engine_assets::RefType;

// Hard reference (prevents eviction)
let player_mesh = registry.insert_with_reftype(
    player_mesh_id,
    player_mesh_data,
    RefType::Hard
);

// Soft reference (can be evicted)
let background_mesh = registry.insert_with_reftype(
    background_mesh_id,
    background_mesh_data,
    RefType::Soft
);

// Hard-referenced assets will NEVER be evicted
// Soft-referenced assets can be evicted when over budget
```

---

## Validation and Error Handling

Multi-layer validation ensures asset integrity.

### Layer 1: Format Validation

```rust
use engine_assets::{AssetValidator, ValidationError};

// Validate before parsing
let file_data = std::fs::read("assets/cube.obj")?;

match AssetValidator::validate_format(&file_data) {
    Ok(()) => println!("✅ Format is valid"),
    Err(ValidationError::InvalidMagic) => {
        eprintln!("❌ Invalid file format (wrong magic number)");
    }
    Err(ValidationError::UnsupportedVersion) => {
        eprintln!("❌ Unsupported file version");
    }
    Err(e) => {
        eprintln!("❌ Validation failed: {}", e);
    }
}
```

### Layer 2: Data Integrity

```rust
// Parse asset
let mesh = MeshData::from_obj(&obj_data)?;

// Validate parsed data
match mesh.validate_data() {
    Ok(()) => println!("✅ Data is valid"),
    Err(ValidationError::InvalidVertexData) => {
        eprintln!("❌ Mesh contains NaN or Inf values");
    }
    Err(ValidationError::IndexOutOfBounds) => {
        eprintln!("❌ Mesh has invalid indices");
    }
    Err(e) => {
        eprintln!("❌ Validation failed: {}", e);
    }
}
```

### Layer 3: Checksum Validation

```rust
use engine_assets::AssetEntry;

// Load asset entry from manifest
let entry: AssetEntry = load_from_manifest(asset_id)?;

// Load file data
let file_data = std::fs::read(&entry.path)?;

// Verify checksum
if entry.verify_checksum(&file_data) {
    println!("✅ Checksum valid");
} else {
    eprintln!("❌ Checksum mismatch - file may be corrupted");
    return Err("Checksum validation failed".into());
}
```

### Error Recovery

```rust
// Try to load asset, fall back to default on error
let mesh_handle = match manager.load_sync::<MeshData>(path) {
    Ok(handle) => {
        println!("✅ Loaded asset from: {:?}", path);
        handle
    }
    Err(e) => {
        eprintln!("❌ Failed to load {:?}: {}", path, e);
        eprintln!("   Falling back to default mesh");

        // Load default fallback mesh
        manager.load_sync::<MeshData>(
            Path::new("assets/defaults/error_mesh.obj")
        )?
    }
};
```

---

## Integration with Rendering

Assets are CPU-side data. GPU upload happens in `engine-renderer`.

### CPU-Side Asset Loading

```rust
// In your game code
use engine_assets::{AssetManager, MeshData};

let manager = AssetManager::new();
let mesh_handle = manager.load_sync::<MeshData>(
    Path::new("assets/cube.obj")
)?;

// Get CPU-side mesh data
if let Some(mesh_data) = manager.get_mesh(mesh_handle.id()) {
    // mesh_data is MeshData (CPU-side)
    println!("Vertices: {}", mesh_data.vertices.len());
}
```

### GPU Upload (in renderer)

```rust
// In engine-renderer crate
use engine_assets::MeshData;
use engine_renderer::GpuMesh;

// Upload to GPU
let gpu_mesh = GpuMesh::from_mesh_data(&renderer.context, &mesh_data)?;

// Now gpu_mesh can be used for rendering
```

### Asset → GPU Caching

```rust
use std::collections::HashMap;
use engine_assets::AssetId;

// Cache GPU resources by AssetId
struct GpuAssetCache {
    meshes: HashMap<AssetId, GpuMesh>,
    textures: HashMap<AssetId, GpuTexture>,
}

impl GpuAssetCache {
    fn get_or_upload_mesh(
        &mut self,
        id: AssetId,
        manager: &AssetManager,
        renderer: &mut Renderer,
    ) -> Result<&GpuMesh, Error> {
        // Check if already uploaded
        if !self.meshes.contains_key(&id) {
            // Get CPU-side data
            let mesh_data = manager.get_mesh(id)
                .ok_or(Error::AssetNotFound)?;

            // Upload to GPU
            let gpu_mesh = renderer.upload_mesh(&mesh_data)?;

            // Cache GPU resource
            self.meshes.insert(id, gpu_mesh);
        }

        Ok(&self.meshes[&id])
    }
}
```

### Render Loop Example

```rust
// Game render loop
loop {
    // Get mesh to render
    let mesh_handle = get_player_mesh();

    // Get or upload GPU mesh
    let gpu_mesh = gpu_cache.get_or_upload_mesh(
        mesh_handle.id(),
        &asset_manager,
        &mut renderer,
    )?;

    // Render
    renderer.draw_mesh(gpu_mesh, transform);
}
```

---

## Next Steps

You've learned the basics of the asset system! Here are some next steps:

1. **Read the Full Documentation**: `docs/assets.md`
2. **Explore Examples**: `examples/` directory
3. **Check the Task Spec**: `docs/tasks/phase1-7-asset-system.md`
4. **Review Tests**: `engine/assets/tests/` for more usage patterns
5. **Run Benchmarks**: `cargo bench --package engine-assets`

---

## Common Patterns

### Pattern 1: Load Level Assets

```rust
fn load_level(manager: &AssetManager, level_name: &str) -> Result<LevelAssets, Error> {
    let base_path = format!("assets/levels/{}/", level_name);

    Ok(LevelAssets {
        terrain_mesh: manager.load_sync::<MeshData>(
            &Path::new(&format!("{}terrain.glb", base_path))
        )?,
        environment_texture: manager.load_sync::<TextureData>(
            &Path::new(&format!("{}environment.png", base_path))
        )?,
        background_music: manager.load_sync::<AudioData>(
            &Path::new(&format!("{}music.ogg", base_path))
        )?,
    })
}
```

### Pattern 2: Preload Critical Assets

```rust
fn preload_critical_assets(manager: &AssetManager) -> Result<(), Error> {
    println!("Preloading critical assets...");

    // Player character (hard reference, never evict)
    let player_mesh = manager.load_sync::<MeshData>(
        Path::new("assets/characters/player.glb")
    )?;
    registry.set_ref_type(player_mesh.id(), RefType::Hard);

    // UI assets (hard reference)
    let ui_font = manager.load_sync::<FontData>(
        Path::new("assets/ui/main_font.ttf")
    )?;
    registry.set_ref_type(ui_font.id(), RefType::Hard);

    println!("✅ Critical assets loaded");
    Ok(())
}
```

### Pattern 3: Progressive Asset Loading

```rust
async fn load_level_progressive(
    manager: Arc<AssetManager>,
    loader: &AsyncLoader,
    level_id: &str,
) -> Result<(), Error> {
    // Phase 1: Load critical assets synchronously (loading screen)
    let loading_screen = manager.load_sync::<TextureData>(
        Path::new("assets/ui/loading.png")
    )?;
    show_loading_screen(loading_screen.id());

    // Phase 2: Load essential assets async (high priority)
    let terrain = loader.load_async::<MeshData>(
        Path::new(&format!("assets/levels/{}/terrain.glb", level_id)),
        LoadPriority::Critical
    );

    // Phase 3: Load secondary assets in background (low priority)
    let background_music = loader.load_async::<AudioData>(
        Path::new(&format!("assets/levels/{}/music.ogg", level_id)),
        LoadPriority::Low
    );

    // Wait for essential assets
    let terrain_handle = terrain.await_result().await?;

    // Start game (background assets still loading)
    start_gameplay();

    // Background music will be ready eventually
    tokio::spawn(async move {
        if let Ok(music_handle) = background_music.await_result().await {
            play_background_music(music_handle.id());
        }
    });

    Ok(())
}
```

---

**Happy asset management!** 🎮
