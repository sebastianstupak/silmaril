# Asset Management System

**Status**: ✅ Implemented (Phase 1.7)

> **Purpose**: Production-grade asset loading, caching, hot-reload, and network transfer for all game asset types.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Asset Types](#asset-types)
4. [Loading Strategies](#loading-strategies)
5. [Memory Management](#memory-management)
6. [Hot-Reload System](#hot-reload-system)
7. [Network Transfer](#network-transfer)
8. [Manifests and Bundles](#manifests-and-bundles)
9. [Validation](#validation)
10. [Integration with Renderer](#integration-with-renderer)
11. [Performance Characteristics](#performance-characteristics)
12. [Examples](#examples)

---

## Overview

The asset management system provides a complete solution for loading, caching, and distributing game assets. It follows a clean architecture pattern where asset data is pure data structures (CPU-side) and GPU upload happens in the renderer crate.

### Key Features

- **Multiple Asset Types**: Meshes, textures, materials, shaders, audio, fonts
- **Loading Strategies**: Synchronous, asynchronous, and streaming
- **Memory Management**: LRU eviction, memory budgets, hard/soft references
- **Hot-Reload**: Automatic file watching with validation and error recovery
- **Network Transfer**: Client-server asset distribution with compression and checksums
- **Manifests & Bundles**: Declarative asset bundles with dependency tracking
- **Validation**: Multi-layer validation (format, data integrity, checksums)

### Design Principles

1. **Pure Data**: Assets are CPU-side data structures, no GPU dependencies
2. **Content-Addressable**: Assets identified by Blake3 hash for deduplication
3. **Thread-Safe**: Parallel loading and concurrent access with minimal locking
4. **Server-Compatible**: No rendering dependencies, works on headless servers
5. **Failure Recovery**: Invalid assets don't crash the engine

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      AssetManager                           │
│  (Coordinates loading, caching, hot-reload)                 │
└──────────────┬──────────────────────────────────────────────┘
               │
       ┌───────┼───────┬──────────┬──────────┬───────────┐
       │       │       │          │          │           │
   ┌───▼───┐ ┌▼─────┐ ┌▼────────┐ ┌▼────────┐ ┌▼─────────┐
   │ Mesh  │ │Texture│ │Material│ │ Shader  │ │  Audio   │
   │Registry│ │Registry│ │Registry│ │Registry│ │ Registry │
   └───┬───┘ └──┬────┘ └──┬──────┘ └──┬──────┘ └──┬───────┘
       │        │         │           │           │
   ┌───▼────────▼─────────▼───────────▼───────────▼─────────┐
   │              AssetHandle<T>                             │
   │  (Type-safe, ref-counted handles)                       │
   └─────────────────────────────────────────────────────────┘
```

### Key Components

#### AssetId

Content-addressable identifier using Blake3 hash (32 bytes).

```rust
pub struct AssetId([u8; 32]);

impl AssetId {
    pub fn from_content(data: &[u8]) -> Self;
    pub fn from_bytes(bytes: [u8; 32]) -> Self;
}
```

**Properties:**
- Deterministic (same content = same ID)
- Cryptographically strong (collision-resistant)
- Enables deduplication (identical assets share same ID)

#### AssetHandle<T>

Type-safe, reference-counted handle to assets.

```rust
pub struct AssetHandle<T> {
    id: AssetId,
    ref_type: RefType, // Hard or Soft
}

pub enum RefType {
    Hard, // Prevents eviction
    Soft, // Can be evicted if memory budget exceeded
}
```

**Properties:**
- Zero-cost abstraction (compiles to simple ID)
- Type-safe (can't mix Handle<Mesh> with Handle<Texture>)
- Automatic cleanup (reference counted)

#### AssetRegistry<T>

Per-type storage with thread-safe access.

```rust
pub struct AssetRegistry<T> {
    assets: DashMap<AssetId, (T, RefCount, Metadata)>,
}
```

**Properties:**
- Thread-safe (concurrent reads, lock-free)
- O(1) lookup by ID
- Tracks reference counts for eviction

#### AssetManager

Central coordinator for all asset operations.

```rust
pub struct AssetManager {
    meshes: Arc<AssetRegistry<MeshData>>,
    textures: Arc<AssetRegistry<TextureData>>,
    shaders: Arc<AssetRegistry<ShaderData>>,
    materials: Arc<AssetRegistry<MaterialData>>,
    audio: Arc<AssetRegistry<AudioData>>,
    fonts: Arc<AssetRegistry<FontData>>,
}
```

**Responsibilities:**
- Load assets (sync/async)
- Manage lifetime (hard/soft references)
- Coordinate hot-reload
- Track path → ID mapping

---

## Asset Types

### 1. Mesh Assets (`MeshData`)

**Purpose**: 3D geometry (vertices + indices)

```rust
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct Vertex {
    pub position: Vec3,    // 12 bytes
    pub normal: Vec3,      // 12 bytes
    pub uv: Vec2,          // 8 bytes
    // Total: 32 bytes (cache-friendly)
}
```

**Features:**
- Procedural primitives (`cube()`, `triangle()`)
- OBJ loader (simple geometry)
- glTF loader (via `gltf` crate)
- Bounding box and centroid calculations

**Usage:**

```rust
// Load from file
let mesh_handle = manager.load_sync::<MeshData>(Path::new("assets/cube.obj"))?;

// Access mesh data
if let Some(mesh) = manager.get_mesh(mesh_handle.id()) {
    println!("Vertices: {}", mesh.vertices.len());
}
```

### 2. Texture Assets (`TextureData`)

**Purpose**: Image data for rendering

```rust
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_levels: Vec<MipLevel>,
    pub data: Vec<u8>,
}

pub enum TextureFormat {
    RGBA8Unorm,
    RGBA8Srgb,
    BC7Unorm,      // Desktop compression
    ASTC4x4Unorm,  // Mobile compression
    // ... more formats
}
```

**Features:**
- PNG/JPG loader (via `image` crate)
- DDS loader (BC compressed textures)
- Mipmap generation
- Format conversion

**Loaders:**
- **PNG/JPG**: `image` crate (cross-platform)
- **DDS**: `ddsfile` crate (BC compressed textures)

**Usage:**

```rust
let texture_handle = manager.load_sync::<TextureData>(Path::new("assets/brick.png"))?;

if let Some(texture) = manager.get_texture(texture_handle.id()) {
    println!("{}x{} {:?}", texture.width, texture.height, texture.format);
}
```

### 3. Shader Assets (`ShaderData`)

**Purpose**: GPU shader programs

```rust
pub struct ShaderData {
    pub stage: ShaderStage,       // Vertex, Fragment, Compute
    pub source: ShaderSource,     // GLSL or SPIR-V
    pub entry_point: String,      // Entry function (default: "main")
}

pub enum ShaderSource {
    Glsl(String),
    Spirv(Vec<u32>),
}

pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}
```

**Features:**
- GLSL source loading (text)
- SPIR-V binary loading (pre-compiled)
- Automatic stage detection

**Usage:**

```rust
// GLSL shader
let vert_shader = manager.load_sync::<ShaderData>(Path::new("shaders/basic.vert"))?;

// SPIR-V binary
let frag_shader = manager.load_sync::<ShaderData>(Path::new("shaders/basic.frag.spv"))?;
```

### 4. Material Assets (`MaterialData`)

**Purpose**: PBR material properties

```rust
pub struct MaterialData {
    pub base_color: Option<AssetId>,           // Texture ID
    pub metallic_roughness: Option<AssetId>,   // Texture ID
    pub normal: Option<AssetId>,               // Texture ID
    pub emissive: Option<AssetId>,             // Texture ID
    pub base_color_factor: Vec4,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}
```

**Features:**
- PBR workflow (metallic-roughness)
- Texture references (AssetId)
- Factor overrides (multiply textures)

**Usage:**

```rust
let material = MaterialData {
    base_color: Some(texture_id),
    metallic_factor: 0.8,
    roughness_factor: 0.2,
    ..Default::default()
};
```

### 5. Audio Assets (`AudioData`)

**Purpose**: Sound effects and music

```rust
pub struct AudioData {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: AudioFormat,
    pub data: Vec<u8>,
}

pub enum AudioFormat {
    Pcm16,     // Uncompressed 16-bit PCM
    Vorbis,    // Ogg Vorbis (compressed)
}
```

**Features:**
- WAV loader (via `hound` crate)
- Ogg Vorbis loader (via `lewton` crate)

**Usage:**

```rust
let audio_handle = manager.load_sync::<AudioData>(Path::new("assets/explosion.wav"))?;
```

### 6. Font Assets (`FontData`)

**Purpose**: TTF/OTF font data

```rust
pub struct FontData {
    pub family: String,
    pub style: FontStyle,
    pub weight: FontWeight,
    pub data: Vec<u8>,  // TTF/OTF bytes
}

pub enum FontStyle {
    Normal,
    Italic,
}

pub enum FontWeight {
    Normal,
    Bold,
}
```

**Features:**
- TTF/OTF loader (via `ttf-parser` crate)
- Font metrics extraction

**Usage:**

```rust
let font_handle = manager.load_sync::<FontData>(Path::new("assets/Roboto-Regular.ttf"))?;
```

---

## Loading Strategies

### 1. Synchronous Loading

**Use Case**: Small assets, critical startup assets

```rust
let mesh_handle = manager.load_sync::<MeshData>(Path::new("assets/cube.obj"))?;
```

**Properties:**
- **Blocking**: Blocks caller thread until complete
- **Fast**: No async overhead
- **Simple**: Immediate result
- **Target**: < 16ms for typical assets

**When to use:**
- Startup assets (< 1MB)
- Critical assets (player model, UI)
- Loading screens

### 2. Asynchronous Loading

**Use Case**: Large assets, background loading

```rust
let mesh_handle = manager.load_async::<MeshData>(Path::new("assets/large_level.glb")).await?;
```

**Properties:**
- **Non-blocking**: Doesn't block main thread
- **Parallel**: Uses Tokio thread pool
- **Progress**: Can track progress
- **Target**: 0ms main thread impact

**When to use:**
- Large assets (> 1MB)
- Background loading (while playing)
- Batch loading (many assets)

**Implementation:**

```rust
// Read file asynchronously
let data = tokio::fs::read(path).await?;

// Parse in background thread (CPU-bound)
let asset = tokio::task::spawn_blocking(move || {
    T::parse(&data)
}).await??;
```

### 3. Streaming Loading

**Use Case**: Progressive LOD, large textures

```rust
let streaming_handle = loader.load_streaming::<TextureData>(
    Path::new("assets/terrain_4k.dds")
).await?;

// Access current LOD (starts with low-res)
let current_lod = streaming_handle.current_lod();

// Wait for higher LOD
streaming_handle.upgrade_to_lod(2).await?;
```

**Properties:**
- **Progressive**: Shows low-res immediately
- **Bandwidth-aware**: Upgrades as bandwidth allows
- **Cancellable**: Can cancel mid-stream

**Implementation:**

```rust
pub struct StreamingHandle<T> {
    lod_levels: Vec<AssetHandle<T>>, // LOD 0 (lowest) to LOD N (highest)
    current_lod: AtomicUsize,
}
```

**When to use:**
- Large textures (> 10MB)
- Progressive meshes (LOD streaming)
- Network-constrained environments

---

## Memory Management

### LRU Cache

Tracks asset access and evicts least-recently-used soft-referenced assets.

```rust
pub struct LruCache {
    budget: MemoryBudget,
    stats: Arc<RwLock<MemoryStats>>,
    mesh_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    texture_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    // ... per-type LRU lists
}
```

**Features:**
- **Per-type budgets**: Separate budgets for meshes, textures, etc.
- **Global budget**: Total memory limit
- **LRU eviction**: Least recently used evicted first
- **Hard references**: Prevent eviction

**Usage:**

```rust
use engine_assets::{LruCache, MemoryBudget};

let budget = MemoryBudget {
    total: 1024 * 1024 * 1024,  // 1 GB
    mesh: 100 * 1024 * 1024,    // 100 MB
    texture: 500 * 1024 * 1024, // 500 MB
    shader: 10 * 1024 * 1024,   // 10 MB
    material: 50 * 1024 * 1024, // 50 MB
    audio: 200 * 1024 * 1024,   // 200 MB
    font: 50 * 1024 * 1024,     // 50 MB
};

let cache = LruCache::new(budget);

// Track asset access
cache.access(asset_id, AssetType::Mesh);

// Check if eviction needed
if cache.is_over_budget(AssetType::Mesh) {
    let to_free = cache.memory_to_free(AssetType::Mesh);
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 10);

    for id in candidates {
        registry.remove(id);
        cache.remove(id, AssetType::Mesh);
    }
}
```

### Memory Budgets

**Default Budgets** (configurable):

| Asset Type | Default Budget | Critical Budget |
|------------|---------------|-----------------|
| **Meshes** | 100 MB | 200 MB |
| **Textures** | 500 MB | 1 GB |
| **Shaders** | 10 MB | 20 MB |
| **Materials** | 50 MB | 100 MB |
| **Audio** | 200 MB | 400 MB |
| **Fonts** | 50 MB | 100 MB |
| **Total** | 1 GB | 2 GB |

### Hard vs Soft References

**Hard References**: Prevent eviction

```rust
let handle = registry.insert_with_reftype(id, asset, RefType::Hard);
```

**Use cases:**
- Currently visible objects
- Player character
- Critical UI assets

**Soft References**: Can be evicted

```rust
let handle = registry.insert_with_reftype(id, asset, RefType::Soft);
```

**Use cases:**
- Background objects
- Distant terrain
- Recently used but not visible

---

## Hot-Reload System

Watches filesystem for changes and automatically reloads modified assets.

### Features

- **File Watching**: Cross-platform with `notify` crate
- **Debouncing**: Configurable delay (default: 300ms)
- **Batching**: Groups multiple changes for efficiency
- **Validation**: Validates before reload (invalid assets don't crash)
- **Error Recovery**: Keeps old asset if new version fails
- **Events**: Notifications for reload success/failure

### Usage

```rust
use engine_assets::{AssetManager, HotReloader, HotReloadConfig};
use std::sync::Arc;

let manager = Arc::new(AssetManager::new());
let config = HotReloadConfig {
    debounce_duration: Duration::from_millis(300),
    enable_batching: true,
    max_batch_size: 10,
    batch_timeout: Duration::from_millis(500),
};

let mut hot_reloader = HotReloader::new(manager.clone(), config)?;

// Start watching assets directory
hot_reloader.watch(Path::new("assets"))?;

// In game loop
loop {
    hot_reloader.process_events();

    // Poll for reload events
    while let Some(event) = hot_reloader.poll_event() {
        match event {
            HotReloadEvent::Modified { path, old_id, new_id, .. } => {
                println!("Reloaded: {:?}", path);
                // Update GPU resources, etc.
            }
            HotReloadEvent::ReloadFailed { path, error, .. } => {
                eprintln!("Failed to reload {:?}: {}", path, error);
            }
            _ => {}
        }
    }

    // ... render frame ...
}
```

### Events

```rust
pub enum HotReloadEvent {
    Created { path, asset_type, asset_id },
    Modified { path, asset_type, old_id, new_id },
    Deleted { path, asset_type, asset_id },
    ReloadFailed { path, asset_type, error },
    BatchReloaded { count, duration_ms },
}
```

### Safety

**Validation Before Reload:**

```rust
// 1. Load new version
let new_data = load_asset(path)?;

// 2. Validate (don't crash if invalid)
validate_asset(&new_data)?;

// 3. Double-buffer GPU resources (if applicable)
let new_gpu = upload_to_gpu(&new_data)?;

// 4. Atomic swap (use new, drop old)
registry.insert(id, new_data);
gpu_cache.insert(id, new_gpu);
```

**Error Recovery:**

- Invalid assets → keep old version
- Parsing errors → log error, continue
- GPU upload fails → keep old GPU resource

---

## Network Transfer

Client-server asset distribution with compression and integrity validation.

### Protocol

```rust
pub enum AssetNetworkMessage {
    Request { asset_id, resume_offset },
    Response { asset_id, data, checksum, compressed },
    Chunk { asset_id, offset, total_size, data, compressed },
    Complete { asset_id, checksum },
    Error { asset_id, error },
}
```

### Features

- **Chunked Transfer**: Large assets sent in 1MB chunks
- **Resumable Downloads**: Range requests for interrupted transfers
- **Blake3 Checksums**: Integrity validation
- **LZ4 Compression**: Fast compression for compressible assets
- **Priority Queuing**: Critical assets transferred first
- **Deduplication**: AssetId prevents duplicate transfers

### Usage

**Server:**

```rust
use engine_assets::network::{AssetNetworkServer, AssetNetworkMessage};

let mut server = AssetNetworkServer::new(1024 * 1024); // 1MB chunks

// Register assets
server.register_asset(mesh_id, mesh_bytes);
server.register_asset(texture_id, texture_bytes);

// Handle client request
let request = AssetNetworkMessage::Request { asset_id: mesh_id, resume_offset: None };
let responses = server.handle_request(request);

// Send responses to client
for response in responses {
    send_to_client(response);
}
```

**Client:**

```rust
use engine_assets::network::{AssetNetworkClient, TransferPriority};

let mut client = AssetNetworkClient::new(4); // Max 4 concurrent transfers

// Request asset with priority
client.request_asset(mesh_id, TransferPriority::Critical);

// Get next request to send
if let Some(request) = client.next_request() {
    send_to_server(request);
}

// Handle server response
client.handle_message(response)?;

// Get completed asset
if let Some(data) = client.take_completed(mesh_id) {
    // Parse and use asset
    let mesh = MeshData::parse(&data)?;
}
```

### Transfer Priorities

```rust
pub enum TransferPriority {
    Critical = 3,  // Player model, UI
    High = 2,      // Nearby NPCs, weapons
    Normal = 1,    // Background objects
    Low = 0,       // Distant terrain, decorations
}
```

### Compression

**Automatic Selection:** Compresses if >10% size reduction

```rust
fn compress_if_beneficial(data: &[u8]) -> (Vec<u8>, bool) {
    let compressed = lz4_flex::compress_prepend_size(data);
    let ratio = compressed.len() as f32 / data.len() as f32;

    if ratio < 0.9 {
        (compressed, true)  // At least 10% reduction
    } else {
        (data.to_vec(), false)  // Not worth compressing
    }
}
```

**Typical Compression Ratios:**

| Asset Type | Ratio | Notes |
|------------|-------|-------|
| Shaders (GLSL) | 3:1 | Text compresses well |
| Fonts (TTF) | 2:1 | Good compression |
| Textures (PNG) | 1:1 | Already compressed |
| Meshes (binary) | 1.2:1 | Some redundancy |
| Audio (WAV) | 1:1 | Raw PCM doesn't compress |

---

## Manifests and Bundles

### Manifests

Declarative asset metadata with dependency tracking.

```rust
pub struct AssetManifest {
    pub version: u32,
    pub assets: Vec<AssetEntry>,
}

pub struct AssetEntry {
    pub id: AssetId,
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub size_bytes: u64,
    pub checksum: [u8; 32],  // Blake3 hash
    pub dependencies: Vec<AssetId>,
}
```

**Features:**
- **Dependency tracking**: Materials depend on textures
- **Integrity validation**: Blake3 checksums
- **Topological sorting**: Load dependencies first
- **Cycle detection**: Prevents circular dependencies

**Example Manifest (YAML):**

```yaml
version: 1
assets:
  - id: "mesh/cube.obj"
    path: "meshes/cube.obj"
    asset_type: Mesh
    size_bytes: 1024
    checksum: "abc123..."
    dependencies: []

  - id: "texture/brick.png"
    path: "textures/brick.png"
    asset_type: Texture
    size_bytes: 524288
    checksum: "def456..."
    dependencies: []

  - id: "material/brick.mat"
    path: "materials/brick.mat"
    asset_type: Material
    size_bytes: 256
    checksum: "ghi789..."
    dependencies:
      - "texture/brick.png"
      - "texture/brick_normal.png"
```

**Usage:**

```rust
use engine_assets::{AssetManifest, AssetEntry};

// Create manifest
let mut manifest = AssetManifest::new();

// Add assets
let mesh_entry = AssetEntry::new(
    mesh_id,
    PathBuf::from("meshes/cube.obj"),
    AssetType::Mesh,
    1024,
    checksum,
);
manifest.add_asset(mesh_entry);

// Validate
manifest.validate()?;  // Checks for cycles, missing deps

// Topological sort (load order)
let load_order = manifest.topological_sort()?;
for asset_id in load_order {
    load_asset(asset_id);
}

// Serialize to YAML
let yaml = manifest.to_yaml()?;
std::fs::write("assets/manifest.yaml", yaml)?;

// Deserialize from YAML
let manifest = AssetManifest::from_yaml(&yaml)?;
```

### Bundles

Packed asset archives with compression.

```rust
pub struct AssetBundle {
    manifest: AssetManifest,
    assets: HashMap<AssetId, Vec<u8>>,
    compression: CompressionFormat,
}

pub enum CompressionFormat {
    None,   // No compression
    Lz4,    // Fast compression
    Zstd,   // Better compression ratio
}
```

**Features:**
- **Multiple assets**: Pack many assets into one file
- **Compression**: LZ4 or Zstd compression
- **Integrity**: Blake3 checksums per asset
- **Efficient**: Single file for distribution

**Bundle Format:**

```
[Header: magic, version, compression, count, sizes]
[Manifest: serialized AssetManifest]
[Asset 1: id, size, data]
[Asset 2: id, size, data]
...
```

**Usage:**

```rust
use engine_assets::{AssetBundle, CompressionFormat};

// Create bundle
let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Lz4);

// Add assets
bundle.add_asset(mesh_id, mesh_bytes)?;
bundle.add_asset(texture_id, texture_bytes)?;

// Pack to file
let packed = bundle.pack()?;
std::fs::write("assets/level1.bundle.lz4", packed)?;

// Unpack from file
let packed = std::fs::read("assets/level1.bundle.lz4")?;
let bundle = AssetBundle::unpack(&packed)?;

// Extract assets
if let Some(mesh_data) = bundle.get_asset(mesh_id) {
    let mesh = MeshData::parse(mesh_data)?;
}
```

---

## Validation

Multi-layer validation ensures asset integrity.

### Layer 1: Format Validation

Validates file format (magic numbers, version, headers).

```rust
pub trait AssetValidator {
    fn validate_format(data: &[u8]) -> Result<(), ValidationError>;
}

impl AssetValidator for MeshData {
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        // Check magic number
        if &data[0..4] != b"MESH" {
            return Err(ValidationError::InvalidMagic);
        }

        // Check version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version > CURRENT_VERSION {
            return Err(ValidationError::UnsupportedVersion);
        }

        Ok(())
    }
}
```

### Layer 2: Data Integrity

Validates data sanity (bounds checks, NaN/Inf detection).

```rust
impl AssetValidator for MeshData {
    fn validate_data(&self) -> Result<(), ValidationError> {
        // Check for NaN/Inf in vertices
        for vertex in &self.vertices {
            if !vertex.position.is_finite() {
                return Err(ValidationError::InvalidVertexData);
            }
        }

        // Check index bounds
        for &index in &self.indices {
            if index >= self.vertices.len() as u32 {
                return Err(ValidationError::IndexOutOfBounds);
            }
        }

        Ok(())
    }
}
```

### Layer 3: Checksum Validation

Validates content integrity (Blake3 checksums).

```rust
impl AssetEntry {
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        let computed = blake3::hash(data);
        computed.as_bytes() == &self.checksum
    }
}
```

**Usage:**

```rust
// Validate before load
AssetValidator::validate_format(&file_data)?;

// Validate after parse
let asset = MeshData::parse(&file_data)?;
asset.validate_data()?;

// Validate checksum (in manifest)
if !entry.verify_checksum(&file_data) {
    return Err(ValidationError::ChecksumMismatch);
}
```

---

## Integration with Renderer

Assets are CPU-side data; GPU upload happens in `engine-renderer`.

### Separation of Concerns

```
engine-assets (pure data)
    ↓
engine-renderer (GPU upload)
    ↓
Vulkan (GPU execution)
```

### GPU Upload Example

```rust
// In engine-renderer crate
impl Renderer {
    pub fn upload_mesh(&mut self, mesh_data: &MeshData) -> Result<GpuMesh, RendererError> {
        GpuMesh::from_mesh_data(&self.context, mesh_data)
    }

    pub fn upload_texture(&mut self, texture_data: &TextureData) -> Result<GpuTexture, RendererError> {
        // Create Vulkan image
        let image = create_vulkan_image(&self.context, texture_data)?;

        // Upload data
        upload_texture_data(&self.context, &image, texture_data)?;

        // Transition layout
        transition_image_layout(&self.context, &image, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)?;

        Ok(GpuTexture { image, view, sampler })
    }
}
```

### Asset → GPU Handle Mapping

```rust
pub struct GpuAssetCache {
    meshes: HashMap<AssetId, GpuMesh>,
    textures: HashMap<AssetId, GpuTexture>,
}

impl GpuAssetCache {
    pub fn get_or_upload_mesh(
        &mut self,
        id: AssetId,
        asset_manager: &AssetManager,
        renderer: &mut Renderer,
    ) -> Result<&GpuMesh, RendererError> {
        if !self.meshes.contains_key(&id) {
            let mesh_data = asset_manager.get_mesh(id)?;
            let gpu_mesh = renderer.upload_mesh(&mesh_data)?;
            self.meshes.insert(id, gpu_mesh);
        }
        Ok(&self.meshes[&id])
    }
}
```

**Properties:**
- **Lazy Upload**: Only upload when needed
- **Caching**: GPU handles cached by AssetId
- **Cleanup**: GPU resources freed when asset evicted

---

## Performance Characteristics

### Load Times

| Asset Type | Sync Load (Target) | Sync Load (Critical) |
|------------|-------------------|---------------------|
| Mesh (OBJ) | < 1ms | < 10ms |
| Mesh (glTF) | < 5ms | < 50ms |
| Texture (PNG) | < 5ms | < 50ms |
| Texture (DDS) | < 1ms | < 10ms |
| Shader (GLSL) | < 1ms | < 10ms |
| Shader (SPIR-V) | < 0.5ms | < 5ms |
| Audio (WAV) | < 5ms | < 50ms |
| Font (TTF) | < 1ms | < 10ms |

### Memory Overhead

| Component | Overhead | Notes |
|-----------|----------|-------|
| AssetHandle | 32 bytes | Just the ID |
| AssetRegistry | ~1% | DashMap overhead |
| LRU Cache | ~0.5% | LinkedHashMap overhead |
| Total | < 2% | Very low overhead |

### Hot-Reload Performance

| Operation | Time | Notes |
|-----------|------|-------|
| File change detection | < 1ms | notify crate |
| Reload (small asset) | < 10ms | Validation + parse |
| Reload (large asset) | < 100ms | Async reload |
| Batch reload (10 assets) | < 50ms | Parallel loading |

### Network Transfer

| Metric | Target | Critical |
|--------|--------|----------|
| Transfer (1MB asset) | < 10ms | < 100ms |
| Chunked transfer (10MB) | < 50ms | < 500ms |
| Compression ratio | > 1.5:1 | > 1.2:1 |
| Checksum validation | < 1ms | < 10ms |

---

## Examples

### Example 1: Basic Asset Loading

```rust
use engine_assets::{AssetManager, MeshData};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AssetManager::new();

    // Load a mesh synchronously
    let cube_handle = manager.load_sync::<MeshData>(Path::new("assets/cube.obj"))?;

    // Access the mesh
    if let Some(mesh) = manager.get_mesh(cube_handle.id()) {
        println!("Loaded cube with {} vertices", mesh.vertices.len());
    }

    Ok(())
}
```

### Example 2: Async Loading with Progress

```rust
use engine_assets::{AsyncLoader, AssetManager, MeshData, LoadPriority};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());
    let loader = AsyncLoader::new(manager.clone(), 4);

    // Start async load
    let handle = loader.load_async::<MeshData>(
        Path::new("assets/large_level.glb"),
        LoadPriority::Normal
    );

    // Poll progress
    while !handle.is_complete() {
        println!("Loading: {:.1}%", handle.progress() * 100.0);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Get result
    let mesh_handle = handle.await_result().await?;
    println!("Load complete!");

    Ok(())
}
```

### Example 3: Hot-Reload System

```rust
use engine_assets::{AssetManager, HotReloader, HotReloadConfig, HotReloadEvent};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(AssetManager::new());
    let mut hot_reloader = HotReloader::new(manager.clone(), HotReloadConfig::default())?;

    // Watch assets directory
    hot_reloader.watch(Path::new("assets"))?;

    // Game loop
    loop {
        // Process hot-reload events
        hot_reloader.process_events();

        while let Some(event) = hot_reloader.poll_event() {
            match event {
                HotReloadEvent::Modified { path, .. } => {
                    println!("Reloaded: {:?}", path);
                }
                HotReloadEvent::ReloadFailed { path, error, .. } => {
                    eprintln!("Failed: {:?} - {}", path, error);
                }
                _ => {}
            }
        }

        // ... render frame ...
    }

    Ok(())
}
```

### Example 4: Network Asset Transfer

```rust
use engine_assets::network::{AssetNetworkServer, AssetNetworkClient, TransferPriority};

// Server side
let mut server = AssetNetworkServer::new(1024 * 1024);
server.register_asset(mesh_id, mesh_bytes);

// Client side
let mut client = AssetNetworkClient::new(4);
client.request_asset(mesh_id, TransferPriority::Critical);

// Client sends request
let request = client.next_request().unwrap();
send_to_server(request);

// Server handles request and sends responses
let responses = server.handle_request(request);
for response in responses {
    send_to_client(response);
}

// Client receives and validates
client.handle_message(response)?;
let data = client.take_completed(mesh_id).unwrap();
```

### Example 5: Asset Bundles

```rust
use engine_assets::{AssetBundle, AssetManifest, CompressionFormat};

// Create bundle
let manifest = create_manifest();
let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Lz4);

// Add assets
bundle.add_asset(mesh_id, mesh_bytes)?;
bundle.add_asset(texture_id, texture_bytes)?;

// Pack and save
let packed = bundle.pack()?;
std::fs::write("assets/level1.bundle.lz4", packed)?;

// Load and unpack
let packed = std::fs::read("assets/level1.bundle.lz4")?;
let bundle = AssetBundle::unpack(&packed)?;

// Extract assets
for entry in bundle.manifest().assets {
    if let Some(data) = bundle.get_asset(entry.id) {
        println!("Loaded: {:?} ({} bytes)", entry.path, data.len());
    }
}
```

---

## See Also

- **Implementation**: `engine/assets/src/`
- **Tests**: `engine/assets/tests/`
- **Benchmarks**: `engine/assets/benches/`
- **Tutorial**: `docs/tutorials/asset-system-tutorial.md`
- **Task Spec**: `docs/tasks/phase1-7-asset-system.md`
