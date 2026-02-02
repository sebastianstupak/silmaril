# Phase 1.7 - Complete Asset Management System

**Estimated Time**: 12-15 days
**Status**: Not Started
**Dependencies**: Phase 1.6 complete (Vulkan context, render pass)

---

## Overview

Implement a production-grade asset management system that supports all asset types, multiple loading strategies, hot-reload, network transfer, memory management, and procedural generation. This system must work for both client and server, with server-side procedural generation not requiring any rendering dependencies.

**Key Architectural Principle**: Asset data structures (MeshData, TextureData, etc.) are pure data in `engine-assets` crate. GPU upload happens in `engine-renderer` crate.

---

## Task Breakdown

### Task 1: Asset Handle System (2 days)

**Goal**: Implement type-safe, reference-counted asset handles with lifetime policies.

**Sub-tasks**:
1. Create `AssetId` type (Blake3 hash, 32 bytes)
   - Implement content-addressable ID generation
   - Add Display/Debug traits
   - Add serialization (bincode + flatbuffers)
   - Test collision resistance (property-based tests)

2. Create `AssetHandle<T>` generic type
   - Hard references (prevent eviction)
   - Soft references (LRU evictable)
   - Reference counting (atomic)
   - Weak references for dependency tracking
   - Clone trait (increment refcount)
   - Drop trait (decrement refcount, auto-cleanup)

3. Create `AssetRegistry<T>` per-type storage
   - HashMap<AssetId, (T, RefCount, Metadata)>
   - Thread-safe (RwLock or DashMap)
   - Query by ID
   - Iterator over all assets
   - Metrics (count, memory usage)

**Tests**:
- Unit: AssetId generation determinism
- Unit: RefCount increment/decrement
- Unit: Hard vs Soft reference behavior
- Integration: Handle lifecycle (create → use → drop)
- Property: AssetId collision resistance

**Success Criteria**:
- Zero-cost abstraction (compiles to simple pointer ops)
- Thread-safe asset access
- Automatic cleanup on zero refcount
- Type-safe (can't mix Handle<Mesh> with Handle<Texture>)

---

### Task 2: Asset Types - Core Data Structures (3 days)

**Goal**: Define all asset data structures in `engine-assets` crate (no GPU dependencies).

**Sub-tasks**:

#### 2.1: Mesh Assets (0.5 days) - Already Done
- ✅ Vertex, MeshData, primitives, OBJ loader
- ✅ Tests and benchmarks passing
- Add: glTF loader (via `gltf` crate)
- Add: FBX loader (via `fbxcel-dom` crate, optional feature)
- Add: Custom binary format (fast deserialization)

#### 2.2: Texture Assets (0.5 days)
```rust
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat, // RGBA8, BC7, ASTC, etc.
    pub mip_levels: Vec<MipLevel>,
    pub data: Vec<u8>,
}

pub enum TextureFormat {
    RGBA8Unorm,
    RGBA8Srgb,
    BC7Unorm,      // Desktop
    ASTC4x4Unorm,  // Mobile
    // ...
}
```
- PNG/JPG loader (via `image` crate)
- DDS loader (BC compressed, via `ddsfile` crate)
- KTX2 loader (Basis Universal, via `ktx2` crate)
- Mipmap generation (via `image` crate)

#### 2.3: Material Assets (0.5 days)
```rust
pub struct MaterialData {
    pub name: String,
    pub base_color: AssetId,           // Texture
    pub metallic_roughness: AssetId,   // Texture
    pub normal: AssetId,               // Texture
    pub emissive: AssetId,             // Texture
    pub base_color_factor: Vec4,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}
```
- glTF material parsing
- Custom material format (YAML + binary)

#### 2.4: Audio Assets (0.5 days)
```rust
pub struct AudioData {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: AudioFormat, // PCM16, Vorbis, Opus
    pub data: Vec<u8>,
}
```
- WAV loader (via `hound` crate)
- OGG Vorbis loader (via `lewton` crate)
- MP3 loader (via `minimp3` crate, optional)

#### 2.5: Shader Assets (0.5 days)
```rust
pub struct ShaderData {
    pub stage: ShaderStage, // Vertex, Fragment, Compute
    pub source: ShaderSource,
}

pub enum ShaderSource {
    GLSL(String),
    SPIRV(Vec<u32>),
}
```
- GLSL parser (validation only)
- SPIR-V loader (pre-compiled)
- Shader include system (for shared code)

#### 2.6: Font Assets (0.5 days)
```rust
pub struct FontData {
    pub family: String,
    pub style: FontStyle, // Regular, Bold, Italic, etc.
    pub data: Vec<u8>,    // TTF/OTF bytes
}
```
- TTF/OTF loader (via `ttf-parser` crate)
- Font atlas generation (deferred to renderer)

**Tests**:
- Unit: Each loader with sample files
- Unit: Format conversions
- Integration: Load → validate → use
- Benchmark: Load times for each asset type

**Success Criteria**:
- All loaders working with real assets
- No GPU dependencies in `engine-assets` crate
- Server can use MeshData for procedural generation
- Fast deserialization (< 1ms for typical assets)

---

### Task 3: Loading Strategies (2 days)

**Goal**: Implement sync, async, and streaming loading with configurable strategies.

**Sub-tasks**:

#### 3.1: Synchronous Loader (0.5 days)
```rust
pub trait AssetLoader<T> {
    fn load_sync(&self, path: &Path) -> Result<T, AssetError>;
}

impl AssetManager {
    pub fn load_sync<T>(&mut self, path: &Path) -> Result<AssetHandle<T>, AssetError> {
        let data = T::Loader::load_sync(path)?;
        let id = AssetId::from_content(&data);
        let handle = self.insert(id, data);
        Ok(handle)
    }
}
```
- Blocking I/O
- Use for small assets (< 1MB)
- Use for critical startup assets

#### 3.2: Async Loader (1 day)
```rust
impl AssetManager {
    pub async fn load_async<T>(&mut self, path: &Path) -> Result<AssetHandle<T>, AssetError> {
        let data = tokio::fs::read(path).await?;
        let parsed = tokio::task::spawn_blocking(move || {
            T::Loader::parse(&data)
        }).await??;

        let id = AssetId::from_content(&parsed);
        let handle = self.insert(id, parsed);
        Ok(handle)
    }
}
```
- Tokio-based async I/O
- Use for large assets (> 1MB)
- Non-blocking loading screen

#### 3.3: Streaming Loader (0.5 days)
```rust
pub struct StreamingHandle<T> {
    lod_levels: Vec<AssetHandle<T>>, // LOD 0 (lowest) to LOD N (highest)
    current_lod: AtomicUsize,
}

impl AssetManager {
    pub async fn load_streaming<T>(&mut self, path: &Path) -> Result<StreamingHandle<T>, AssetError> {
        // Load LOD 0 immediately (low-res)
        let lod0 = self.load_async(&lod_path(path, 0)).await?;

        // Stream higher LODs in background
        let higher_lods = tokio::spawn(async move {
            // Load LOD 1, 2, 3... progressively
        });

        Ok(StreamingHandle {
            lod_levels: vec![lod0],
            current_lod: AtomicUsize::new(0),
        })
    }
}
```
- Progressive LOD streaming
- Use for textures, meshes
- Automatic upgrade as bandwidth allows

**Tests**:
- Unit: Each loader in isolation
- Integration: Load → use → unload
- Async: Test concurrent loads (no race conditions)
- Streaming: Test LOD progression
- Benchmark: Throughput (assets/sec)

**Success Criteria**:
- Sync loads block < 16ms for typical assets
- Async loads don't block main thread
- Streaming shows low-res asset immediately (< 100ms)
- No crashes on concurrent loads

---

### Task 4: Hot-Reload System (1.5 days)

**Goal**: Watch filesystem, detect changes, reload assets safely without crashes.

**Sub-tasks**:

#### 4.1: File Watcher (0.5 days)
```rust
pub struct AssetWatcher {
    watcher: notify::RecommendedWatcher,
    events: Receiver<AssetEvent>,
}

pub enum AssetEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}
```
- Use `notify` crate for cross-platform file watching
- Debounce events (ignore rapid successive writes)
- Filter by asset extensions (.obj, .png, .glsl, etc.)

#### 4.2: Safe Reload (1 day)
```rust
impl AssetManager {
    pub fn reload(&mut self, id: AssetId) -> Result<(), AssetError> {
        // 1. Load new version
        let new_data = self.load_new_version(id)?;

        // 2. Validate (don't crash if invalid)
        self.validate(&new_data)?;

        // 3. Double-buffer GPU resources (if applicable)
        if let Some(gpu_resource) = self.gpu_resources.get(&id) {
            let new_gpu = self.upload_to_gpu(&new_data)?;

            // 4. Atomic swap (use new, drop old)
            self.gpu_resources.insert(id, new_gpu);
        }

        // 5. Update CPU data
        self.assets.insert(id, new_data);

        Ok(())
    }
}
```
- Validation before reload (don't crash on syntax errors)
- GPU double-buffering (compile new shader, swap pipelines)
- Fallback on failure (keep old version working)
- Notifications (log reload success/failure)

**Tests**:
- Unit: File watcher detects changes
- Integration: Modify asset → auto-reload → see changes
- Error: Invalid asset → reload fails gracefully
- GPU: Shader reload doesn't crash pipeline

**Success Criteria**:
- Hot-reload works in < 1 second
- Invalid assets don't crash engine
- GPU resources properly cleaned up
- Works for all asset types

---

### Task 5: Network Transfer (2 days)

**Goal**: Transfer assets client ↔ server with full and delta compression.

**Sub-tasks**:

#### 5.1: Full Transfer (0.5 days)
```rust
#[derive(Serialize, Deserialize)]
pub struct AssetTransferFull {
    pub id: AssetId,
    pub asset_type: AssetType,
    pub data: Vec<u8>, // Compressed with zstd
}

impl AssetManager {
    pub fn serialize_asset(&self, id: AssetId) -> Result<Vec<u8>, AssetError> {
        let asset = self.get(id)?;
        let bytes = bincode::serialize(&asset)?;
        let compressed = zstd::encode_all(&bytes[..], 3)?; // Level 3 compression
        Ok(compressed)
    }
}
```
- zstd compression (fast, good ratio)
- FlatBuffers for network serialization (zero-copy)
- Use for new assets client doesn't have

#### 5.2: Delta Transfer (1 day)
```rust
pub struct AssetDelta {
    pub id: AssetId,
    pub base_version: u64,
    pub patches: Vec<BinaryPatch>,
}

impl AssetManager {
    pub fn compute_delta(&self, id: AssetId, old_version: u64) -> Result<AssetDelta, AssetError> {
        let old_bytes = self.get_version(id, old_version)?;
        let new_bytes = self.get_version(id, self.current_version(id))?;

        // Use bsdiff for binary diffing
        let patch = bsdiff::diff(&old_bytes, &new_bytes)?;

        Ok(AssetDelta {
            id,
            base_version: old_version,
            patches: vec![patch],
        })
    }
}
```
- Binary diffing (via `bsdiff` crate)
- Automatic selection: if delta > 50% of full, send full
- Versioning: track asset versions for delta base

#### 5.3: Transfer Protocol (0.5 days)
```rust
pub enum AssetNetworkMessage {
    Request { id: AssetId, have_version: Option<u64> },
    ResponseFull { data: AssetTransferFull },
    ResponseDelta { delta: AssetDelta },
    NotFound { id: AssetId },
}
```
- TCP for reliable transfer
- Request → Response model
- Chunked transfer for large assets (> 10MB)

**Tests**:
- Unit: Compression ratio
- Unit: Delta computation
- Integration: Client requests → server sends → client applies
- Network: Simulate packet loss (delta should still work)
- Benchmark: Transfer speed (MB/s)

**Success Criteria**:
- Full transfer: < 10ms for typical asset (1MB)
- Delta transfer: < 50% bandwidth of full (when applicable)
- Reliable delivery (TCP ensures no corruption)
- Works over real network (not just localhost)

---

### Task 6: Memory Management (2 days)

**Goal**: LRU eviction, memory budgets, hard/soft references.

**Sub-tasks**:

#### 6.1: Memory Tracking (0.5 days)
```rust
pub struct MemoryStats {
    pub total_allocated: usize,
    pub by_type: HashMap<AssetType, usize>,
}

impl AssetRegistry<T> {
    fn memory_usage(&self) -> usize {
        self.assets.values().map(|asset| asset.size_bytes()).sum()
    }
}
```
- Track per-asset size
- Track per-type totals
- Track system-wide total
- Metrics exposed via observability

#### 6.2: LRU Cache (1 day)
```rust
pub struct LruCache<T> {
    registry: AssetRegistry<T>,
    lru: LinkedHashMap<AssetId, ()>, // Insertion order = access order
    budget: usize,
}

impl<T> LruCache<T> {
    pub fn access(&mut self, id: AssetId) {
        // Move to front (most recently used)
        self.lru.remove(&id);
        self.lru.insert(id, ());
    }

    pub fn evict_if_needed(&mut self) {
        while self.memory_usage() > self.budget {
            // Find oldest soft-referenced asset
            let victim = self.lru.iter()
                .find(|(id, _)| self.registry.is_soft_referenced(id))
                .map(|(id, _)| *id);

            if let Some(id) = victim {
                self.registry.remove(id); // Unload asset
                self.lru.remove(&id);
            } else {
                break; // All assets are hard-referenced, can't evict
            }
        }
    }
}
```
- LinkedHashMap for O(1) LRU tracking
- Evict only soft-referenced assets
- Hard references prevent eviction
- Configurable budget per asset type

#### 6.3: Memory Budgets (0.5 days)
```rust
pub struct AssetManagerConfig {
    pub mesh_budget: usize,      // 100 MB
    pub texture_budget: usize,   // 500 MB
    pub audio_budget: usize,     // 200 MB
    pub total_budget: usize,     // 1 GB
}
```
- Per-type budgets
- Global budget (total limit)
- Warning logs when approaching budget
- Automatic eviction when budget exceeded

**Tests**:
- Unit: LRU eviction order
- Unit: Hard references prevent eviction
- Integration: Budget exceeded → evict → reload
- Stress: Load 1000 assets → evict down to budget

**Success Criteria**:
- Eviction works in < 1ms
- Hard-referenced assets never evicted
- Memory stays within budget
- LRU policy (least recently used evicted first)

---

### Task 7: Asset Manifest & Bundles (1.5 days)

**Goal**: Hybrid registry system (like Destiny 2) with asset bundles + streaming.

**Sub-tasks**:

#### 7.1: Manifest Format (0.5 days)
```yaml
# assets/manifest.yaml
version: 1
bundles:
  - id: "core"
    assets:
      - id: "mesh/cube.obj"
        hash: "blake3:abc123..."
        size: 1024
        dependencies: []
      - id: "texture/brick.png"
        hash: "blake3:def456..."
        size: 524288
        dependencies: []

  - id: "level_forest"
    assets:
      - id: "mesh/tree.glb"
        hash: "blake3:ghi789..."
        size: 102400
        dependencies: ["texture/bark.png", "texture/leaves.png"]
```
- YAML for human-readable manifests
- Content hashes for integrity
- Dependency tracking
- Bundle grouping (core, level-specific, etc.)

#### 7.2: Bundle Loading (0.5 days)
```rust
pub struct AssetBundle {
    pub id: String,
    pub assets: Vec<AssetManifestEntry>,
}

impl AssetManager {
    pub async fn load_bundle(&mut self, bundle_id: &str) -> Result<(), AssetError> {
        let manifest = self.load_manifest()?;
        let bundle = manifest.bundles.iter()
            .find(|b| b.id == bundle_id)
            .ok_or(AssetError::BundleNotFound)?;

        // Load all assets in bundle (parallel)
        let handles = futures::future::join_all(
            bundle.assets.iter().map(|entry| self.load_async(&entry.path))
        ).await;

        Ok(())
    }
}
```
- Parallel bundle loading
- Dependency resolution (topological sort)
- Progress tracking (loaded X / Y assets)

#### 7.3: Integrity Validation (0.5 days)
```rust
impl AssetManager {
    fn validate_integrity(&self, id: AssetId, expected_hash: &str) -> Result<(), AssetError> {
        let asset = self.get(id)?;
        let actual_hash = AssetId::from_content(&asset);

        if actual_hash.to_string() != expected_hash {
            return Err(AssetError::IntegrityCheckFailed { id, expected_hash, actual_hash });
        }

        Ok(())
    }
}
```
- Compare computed hash vs manifest hash
- Detect corrupted assets
- Warning on mismatch (don't crash, use anyway)

**Tests**:
- Unit: Manifest parsing
- Integration: Load bundle → all assets loaded
- Integration: Dependency resolution
- Error: Corrupted asset → integrity check fails

**Success Criteria**:
- Manifest loads in < 10ms
- Bundle loading parallelized (all cores used)
- Integrity checks prevent corrupted assets
- Dependencies loaded before dependents

---

### Task 8: Procedural Generation (1 day)

**Goal**: Server or client-side procedural asset generation with deterministic seeds.

**Sub-tasks**:

#### 8.1: Procedural API (0.5 days)
```rust
pub trait ProceduralGenerator<T> {
    fn generate(&self, seed: u64, params: &GeneratorParams) -> T;
}

pub struct ProceduralMeshGenerator;

impl ProceduralGenerator<MeshData> for ProceduralMeshGenerator {
    fn generate(&self, seed: u64, params: &GeneratorParams) -> MeshData {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        // Generate procedural mesh (deterministic)
        // Example: terrain from heightmap, building from rules
        todo!()
    }
}
```
- Deterministic RNG (ChaCha8)
- Same seed → same asset
- Configurable parameters

#### 8.2: Caching (0.5 days)
```rust
impl AssetManager {
    pub fn generate_or_load<T>(&mut self, seed: u64, params: &GeneratorParams) -> AssetHandle<T> {
        let id = AssetId::from_seed_and_params(seed, params);

        if let Some(handle) = self.get_handle(id) {
            return handle; // Already generated
        }

        // Generate new
        let generator = T::Generator::default();
        let asset = generator.generate(seed, params);
        self.insert(id, asset)
    }
}
```
- Cache generated assets
- Don't regenerate if already exists
- Content-addressable ID from seed+params

**Tests**:
- Unit: Same seed → same output
- Unit: Different seed → different output
- Integration: Generate → cache → reuse
- Benchmark: Generation time

**Success Criteria**:
- Deterministic (same seed = same asset)
- Fast generation (< 10ms for simple assets)
- Cached (don't regenerate)
- Works on server without GPU

---

### Task 9: Asset Cooker Tool (2 days)

**Goal**: Standalone CLI tool for asset optimization and preprocessing.

**Sub-tasks**:

#### 9.1: CLI Tool Structure (0.5 days)
```rust
// engine/tools/asset-cooker/src/main.rs
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,

    #[arg(long)]
    optimize: bool,

    #[arg(long)]
    generate_mipmaps: bool,
}

fn main() -> Result<(), AssetError> {
    let args = Args::parse();

    match asset_type(&args.input) {
        AssetType::Mesh => cook_mesh(&args),
        AssetType::Texture => cook_texture(&args),
        // ...
    }
}
```
- Clap for CLI parsing
- Progress bars (via `indicatif` crate)
- Parallel processing (via `rayon` crate)

#### 9.2: Mesh Optimization (0.5 days)
```rust
fn cook_mesh(args: &Args) -> Result<(), AssetError> {
    let mesh = MeshData::from_obj(&args.input)?;

    // Optimize vertex cache (via `meshopt` crate)
    let optimized = mesh.optimize_vertex_cache()?;

    // Optimize overdraw
    let optimized = optimized.optimize_overdraw()?;

    // Generate LODs
    let lods = optimized.generate_lods(&[0.75, 0.5, 0.25])?;

    // Save to binary format
    lods.save_binary(&args.output)?;

    Ok(())
}
```
- Vertex cache optimization (via `meshopt`)
- Overdraw optimization
- LOD generation (via `meshopt` simplification)
- Binary output (fast loading)

#### 9.3: Texture Processing (0.5 days)
```rust
fn cook_texture(args: &Args) -> Result<(), AssetError> {
    let image = image::open(&args.input)?;

    // Resize to power-of-2 if needed
    let resized = resize_to_pot(image)?;

    // Generate mipmaps
    let mipmaps = generate_mipmaps(&resized)?;

    // Compress (BC7 for desktop, ASTC for mobile)
    let compressed = compress_texture(&mipmaps, CompressionFormat::BC7)?;

    // Save to DDS
    compressed.save_dds(&args.output)?;

    Ok(())
}
```
- Mipmap generation
- Texture compression (BC7, ASTC via `intel-tex` or `basis-universal`)
- Power-of-2 resizing
- DDS/KTX2 output

#### 9.4: Batch Processing (0.5 days)
```bash
asset-cooker --input assets/ --output cooked/ --optimize --recursive
```
- Recursive directory processing
- Parallel cooking (all cores)
- Progress bars
- Error reporting (continue on error)

**Tests**:
- Integration: Cook mesh → load in engine
- Integration: Cook texture → render in engine
- Benchmark: Cooking speed (assets/sec)

**Success Criteria**:
- Mesh cooking: < 100ms per mesh
- Texture cooking: < 500ms per texture
- LODs generated correctly
- Optimized assets load faster

---

### Task 10: Validation System (1 day)

**Goal**: Multi-layer validation (format, data integrity, checksums).

**Sub-tasks**:

#### 10.1: Format Validation (0.5 days)
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
- Magic number checks
- Version checks
- Header validation

#### 10.2: Data Integrity (0.5 days)
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
- Data sanity checks
- Bounds checks
- NaN/Inf detection

**Tests**:
- Unit: Valid asset passes
- Unit: Invalid magic → error
- Unit: Out-of-bounds index → error
- Integration: Load corrupted asset → validation fails

**Success Criteria**:
- Validation in < 1ms
- Catches common errors
- Conditional compilation (skip in release for performance)

---

### Task 11: Integration with Renderer (1 day)

**Goal**: Connect asset system to GPU upload in `engine-renderer`.

**Sub-tasks**:

#### 11.1: GPU Upload (0.5 days)
```rust
// In engine-renderer
impl Renderer {
    pub fn upload_mesh(&mut self, mesh_data: &MeshData) -> Result<GpuMesh, RendererError> {
        GpuMesh::from_mesh_data(&self.context, mesh_data)
    }

    pub fn upload_texture(&mut self, texture_data: &TextureData) -> Result<GpuTexture, RendererError> {
        // Create Vulkan image, upload data, generate mipmaps if needed
        todo!()
    }
}
```
- Mesh upload (already implemented)
- Texture upload (create vk::Image, upload, transition layout)
- Material upload (create descriptor sets)

#### 11.2: Asset → GPU Handle Mapping (0.5 days)
```rust
pub struct GpuAssetCache {
    meshes: HashMap<AssetId, GpuMesh>,
    textures: HashMap<AssetId, GpuTexture>,
}

impl GpuAssetCache {
    pub fn get_or_upload_mesh(&mut self, id: AssetId, asset_manager: &AssetManager) -> Result<&GpuMesh, RendererError> {
        if !self.meshes.contains_key(&id) {
            let mesh_data = asset_manager.get(id)?;
            let gpu_mesh = self.renderer.upload_mesh(&mesh_data)?;
            self.meshes.insert(id, gpu_mesh);
        }
        Ok(&self.meshes[&id])
    }
}
```
- Lazy GPU upload
- Cache GPU handles
- Cleanup on asset eviction

**Tests**:
- Integration: Load MeshData → upload → render
- Integration: Load TextureData → upload → sample in shader
- GPU: Memory leaks (upload → evict → check GPU memory)

**Success Criteria**:
- GPU upload works for all asset types
- No GPU memory leaks
- Lazy upload (only upload when needed)

---

### Task 12: Documentation & Examples (1 day)

**Goal**: Complete documentation and working examples.

**Sub-tasks**:

#### 12.1: API Documentation (0.5 days)
- Rustdoc for all public APIs
- Examples in doc comments
- Architecture diagrams (mermaid.js)

#### 12.2: Examples (0.5 days)
- `examples/asset_loading.rs` - Load mesh, texture, material
- `examples/hot_reload.rs` - Modify asset, see hot-reload
- `examples/procedural_generation.rs` - Generate procedural mesh
- `examples/network_transfer.rs` - Server sends asset to client

**Tests**:
- Doc tests (all examples compile and run)

**Success Criteria**:
- All public APIs documented
- Examples run without errors
- Clear architecture documentation

---

## Testing Strategy

### Unit Tests
- Each asset type loader
- AssetId generation
- Reference counting
- LRU eviction
- Validation

### Integration Tests
- Load → use → unload lifecycle
- Hot-reload
- Network transfer (client ↔ server)
- Bundle loading
- Procedural generation

### Benchmarks
- Load time per asset type
- Compression ratio
- Delta vs full transfer
- LRU eviction overhead
- Cooking speed

### E2E Tests
- Load assets → render in engine
- Modify asset → hot-reload → see changes
- Server generates procedural asset → client receives → renders

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Mesh load (sync) | < 1ms | < 10ms |
| Texture load (sync) | < 5ms | < 50ms |
| Async load (doesn't block frame) | 0ms | 0ms |
| Hot-reload | < 1s | < 3s |
| Network transfer (1MB asset) | < 10ms | < 100ms |
| LRU eviction | < 1ms | < 10ms |
| Memory overhead (handles) | < 1% | < 5% |

---

## Dependencies

### New Crates
- `notify` - File watching
- `zstd` - Compression
- `bsdiff` / `qbsdiff` - Binary diffing
- `gltf` - glTF loader
- `image` - Image loading
- `ddsfile` - DDS texture format
- `ktx2` - KTX2 texture format
- `hound` - WAV audio loader
- `lewton` - Ogg Vorbis loader
- `ttf-parser` - Font loader
- `meshopt` - Mesh optimization
- `indicatif` - Progress bars
- `clap` - CLI parsing

### Updated Crates
- `engine-assets` - Add all asset types
- `engine-renderer` - GPU upload integration
- `engine-networking` - Asset transfer protocol

---

## Success Criteria

1. **All Asset Types Working**: Mesh, Texture, Material, Audio, Shader, Font
2. **All Loading Strategies Working**: Sync, Async, Streaming
3. **Hot-Reload Working**: Modify asset → see changes in < 1s
4. **Network Transfer Working**: Client ↔ Server with delta compression
5. **Memory Management Working**: LRU eviction stays within budget
6. **Procedural Generation Working**: Server generates meshes without GPU
7. **Asset Cooker Working**: Optimize meshes, compress textures, generate LODs
8. **Validation Working**: Catches corrupted assets
9. **Tests Passing**: 100% of unit/integration tests
10. **Performance Targets Met**: All metrics within targets
11. **Documentation Complete**: All public APIs documented with examples

---

## Risks & Mitigations

### Risk: Scope Too Large
- **Mitigation**: Implement incrementally (Task 1 → Task 2 → ... → Task 12)
- **Mitigation**: Each task is independently testable
- **Mitigation**: Can ship partial asset system (e.g., mesh + texture only)

### Risk: GPU Upload Complexity
- **Mitigation**: Already have GpuMesh working (Task 1.7 partial)
- **Mitigation**: Textures similar to meshes (create buffer, upload, transition)

### Risk: Network Transfer Performance
- **Mitigation**: Use proven compression (zstd, bsdiff)
- **Mitigation**: Benchmark early, optimize if needed

### Risk: Hot-Reload Crashes
- **Mitigation**: Validation before reload
- **Mitigation**: Double-buffering GPU resources
- **Mitigation**: Extensive testing with invalid assets

---

## Next Steps After Completion

1. **Phase 1.8**: Mesh Rendering (uses asset system)
2. **Phase 1.9**: Frame Capture
3. **Phase 2+**: Advanced rendering (PBR, shadows, etc.)
