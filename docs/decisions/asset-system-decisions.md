# Asset System Design Decisions

**Document Version**: 1.0
**Date**: 2026-02-01
**Status**: Final
**Approvers**: User (Sebastian)

---

## Purpose

This document captures all architectural decisions made during the asset system design process. Each decision includes the question asked, the answer chosen, the rationale, and trade-offs considered.

---

## Decision Log

### Decision 1: Asset ID Strategy

**Question**: Content-addressable IDs (hash-based) vs sequential IDs vs GUIDs?

**Answer**: **Content-addressable IDs using Blake3 hashing**

**Rationale**:
- **Deterministic**: Same content → same ID (enables deduplication)
- **Fast**: Blake3 is extremely fast (GB/s throughput)
- **Collision Resistant**: 128-bit security (practically impossible to collide)
- **Network Efficient**: Same asset on client and server = same ID
- **Cache Friendly**: Can check if asset exists without loading

**Trade-offs Considered**:
- ❌ Sequential IDs: Not deterministic, requires central authority
- ❌ GUIDs: Random, no content deduplication
- ✅ Blake3: Best balance of speed, security, determinism

**Implementation**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId([u8; 32]);

impl AssetId {
    pub fn from_content<T: AsRef<[u8]>>(data: T) -> Self {
        let hash = blake3::hash(data.as_ref());
        Self(*hash.as_bytes())
    }
}
```

---

### Decision 2: Reference Counting Strategy

**Question**: Manual memory management vs reference counting vs garbage collection?

**Answer**: **Reference counting with hard/soft references**

**Rationale**:
- **Automatic Cleanup**: Zero refcount → automatic unload
- **Predictable**: No GC pauses (critical for games)
- **Lifetime Policies**: Hard refs prevent eviction, soft refs allow LRU
- **Thread-Safe**: Atomic refcounts for concurrent access
- **Industry Standard**: Used by Unity, Unreal, Godot

**Hard vs Soft References**:
- **Hard Reference**: Never evicted by LRU (critical assets like player character, UI)
- **Soft Reference**: Can be evicted when memory budget exceeded (distant objects, cached assets)

**Implementation**:
```rust
pub enum ReferenceType {
    Hard,  // Prevents LRU eviction
    Soft,  // Can be evicted by LRU
}

impl<T> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        self.registry.lock().increment_refcount(self.id);
        // ...
    }
}

impl<T> Drop for AssetHandle<T> {
    fn drop(&mut self) {
        let refcount = self.registry.lock().decrement_refcount(self.id);
        if refcount == 0 {
            self.registry.lock().unload(self.id); // Auto-cleanup
        }
    }
}
```

**Trade-offs**:
- ❌ Manual: Error-prone, easy to leak or double-free
- ❌ GC: Unpredictable pauses, not suitable for games
- ✅ RefCount: Predictable, automatic, thread-safe

---

### Decision 3: Dependency Tracking

**Question**: Automatic dependency resolution vs manual dependency management?

**Answer**: **Automatic dependency tracking with topological sort**

**Rationale**:
- **User-Friendly**: Load one asset, dependencies load automatically
- **Correctness**: Dependencies always loaded before dependents
- **Efficiency**: Parallel loading of independent dependencies
- **Industry Standard**: Unity Asset Bundles, Unreal Asset Manager

**Example**:
```yaml
# Material depends on textures
material:
  id: "materials/brick.mat"
  dependencies:
    - "textures/brick_albedo.png"
    - "textures/brick_normal.png"
    - "textures/brick_roughness.png"
```

**Implementation**:
```rust
impl AssetManager {
    pub async fn load_with_dependencies<T>(&mut self, id: AssetId) -> Result<AssetHandle<T>, AssetError> {
        // Get dependency list from manifest
        let deps = self.manifest.get_dependencies(id)?;

        // Load dependencies in parallel
        let dep_handles = futures::future::join_all(
            deps.iter().map(|dep_id| self.load_async(dep_id))
        ).await;

        // Load main asset
        self.load_async(id).await
    }
}
```

**Trade-offs**:
- ❌ Manual: Error-prone, easy to forget dependencies
- ✅ Automatic: Correct, user-friendly, efficient

---

### Decision 4: Ownership Model (Server vs Client Authority)

**Question**: Server authoritative vs client authoritative vs hybrid?

**Answer**: **Hybrid ownership model (configurable per asset type)**

**Rationale**:
- **Flexibility**: Different games have different needs
- **MMORPG**: Server owns NPC meshes, client owns UI textures
- **Singleplayer**: Client owns everything
- **Co-op**: Server owns gameplay assets, client owns cosmetics

**Configuration**:
```rust
pub struct AssetOwnership {
    pub mesh: Authority,      // Server (prevents client tampering)
    pub texture: Authority,   // Hybrid (client can override for modding)
    pub audio: Authority,     // Client (local preference)
    pub material: Authority,  // Server (gameplay-affecting)
}

pub enum Authority {
    Server,   // Server sends to client, client cannot modify
    Client,   // Client loads locally, server doesn't care
    Hybrid,   // Server provides default, client can override
}
```

**Use Cases**:
- **Server Authority**: Anti-cheat (weapon damage, NPC stats)
- **Client Authority**: UI customization, audio settings
- **Hybrid**: Texture mods (server provides default, client overrides)

**Trade-offs**:
- ❌ Server-only: Can't support modding, high bandwidth
- ❌ Client-only: No anti-cheat, inconsistent state
- ✅ Hybrid: Flexible, supports all game types

---

### Decision 5: Loading Strategies

**Question**: Synchronous only vs asynchronous only vs both?

**Answer**: **Three strategies: Sync, Async, Streaming**

**Rationale**:
- **Sync**: Critical startup assets (must load before game starts)
- **Async**: Large assets during gameplay (loading screens)
- **Streaming**: Progressive LOD (show low-res immediately, upgrade later)

**When to Use Each**:

| Strategy | Use Case | Example |
|----------|----------|---------|
| Sync | Small critical assets | Startup logo, config files |
| Async | Large non-critical assets | Level geometry, textures |
| Streaming | Progressive quality | Distant terrain, high-res textures |

**Implementation**:
```rust
// Sync: Blocks until loaded
let config = asset_manager.load_sync("config.yaml")?;

// Async: Non-blocking, awaitable
let level = asset_manager.load_async("level.glb").await?;

// Streaming: Immediate low-res, upgrade progressively
let texture = asset_manager.load_streaming("terrain.png").await?;
```

**Trade-offs**:
- ❌ Sync-only: Blocks frame, poor UX for large assets
- ❌ Async-only: Complex startup (everything async)
- ✅ Both: Use right tool for job

---

### Decision 6: Network Transfer Strategy

**Question**: Full transfer only vs delta compression vs both?

**Answer**: **Hybrid: Full transfer + delta compression with automatic selection**

**Rationale**:
- **Full Transfer**: New assets, small assets (< 1MB)
- **Delta Compression**: Asset updates, large assets
- **Automatic Selection**: If delta > 50% of full, send full

**Algorithm**:
```rust
if client_has_asset {
    let delta = compute_delta(old_version, new_version);
    if delta.size < full.size * 0.5 {
        send_delta(delta);  // Bandwidth savings
    } else {
        send_full(full);    // Simpler, faster
    }
} else {
    send_full(full);  // Client doesn't have asset
}
```

**Compression**:
- **Full**: zstd level 3 (fast, good ratio)
- **Delta**: bsdiff (binary diffing)

**Performance**:
- Full transfer (1MB): ~10ms (compressed to ~500KB)
- Delta transfer (100KB patch): ~2ms

**Trade-offs**:
- ❌ Full-only: High bandwidth for updates
- ❌ Delta-only: Complex for new assets
- ✅ Hybrid: Best bandwidth, automatic

---

### Decision 7: Memory Management Strategy

**Question**: No eviction (unlimited memory) vs LRU eviction vs LFU eviction?

**Answer**: **LRU (Least Recently Used) eviction with memory budgets**

**Rationale**:
- **LRU**: Simple, effective, industry standard
- **Budgets**: Per-type and global limits
- **Hard Refs**: Prevent eviction of critical assets
- **Predictable**: Always evict least recently used

**Memory Budgets**:
```rust
pub struct AssetManagerConfig {
    pub mesh_budget: usize,      // 100 MB
    pub texture_budget: usize,   // 500 MB
    pub audio_budget: usize,     // 200 MB
    pub total_budget: usize,     // 1 GB
}
```

**Eviction Algorithm**:
1. Check if over budget
2. Find oldest soft-referenced asset
3. Unload asset
4. Repeat until under budget

**Trade-offs**:
- ❌ No eviction: OOM crash
- ❌ LFU: Complex, less predictable
- ✅ LRU: Simple, effective, well-tested

---

### Decision 8: Hot-Reload Strategy

**Question**: No hot-reload (restart required) vs full hot-reload vs partial hot-reload?

**Answer**: **Full hot-reload with validation and double-buffering**

**Rationale**:
- **Developer Experience**: Sub-second iteration times
- **Validation**: Don't crash on invalid assets
- **Double-Buffering**: Renderer keeps working during reload
- **Fallback**: Keep old version if reload fails

**Safe Reload Process**:
1. Detect file change (file watcher)
2. Load new version (don't crash if invalid)
3. Validate format and data integrity
4. If GPU resource: Upload new, keep old
5. Atomic swap (use new, drop old)
6. If any step fails: Keep old version, log error

**Benefits**:
- Modify shader → see changes in < 1 second
- Syntax error → old shader keeps working
- No crashes, no restarts

**Trade-offs**:
- ❌ No hot-reload: Slow iteration (restart game every change)
- ❌ Unsafe reload: Crashes on invalid assets
- ✅ Safe reload: Fast iteration, no crashes

---

### Decision 9: Procedural Generation Strategy

**Question**: Client-side only vs server-side only vs configurable?

**Answer**: **Configurable (server or client-side) with deterministic RNG**

**Rationale**:
- **Flexibility**: Different games have different needs
- **MMORPG**: Server generates terrain (all clients see same world)
- **Singleplayer**: Client generates (no server needed)
- **Hybrid**: Server generates gameplay, client generates cosmetics

**Deterministic RNG**:
```rust
pub trait ProceduralGenerator<T> {
    fn generate(&self, seed: u64, params: &GeneratorParams) -> T;
}

// Same seed + params → same output (server and client agree)
let terrain = generator.generate(seed: 42, params);
```

**Configuration**:
```yaml
procedural:
  terrain:
    authority: server  # Server generates, sends to clients
    cache: true        # Cache generated assets
  cosmetics:
    authority: client  # Client generates locally
    cache: false       # Regenerate each time
```

**Trade-offs**:
- ❌ Client-only: Inconsistent state (clients see different worlds)
- ❌ Server-only: High server CPU, bandwidth
- ✅ Configurable: Flexible, efficient

---

### Decision 10: Asset Manifest System

**Question**: No manifest (scan filesystem) vs simple list vs bundle system?

**Answer**: **Hybrid registry system (like Destiny 2) with asset bundles**

**Rationale**:
- **Bundles**: Group related assets (level assets, character assets)
- **Manifest**: Pre-computed metadata (size, hash, dependencies)
- **Fast Startup**: No filesystem scanning
- **Integrity**: Content hashes prevent corruption
- **Streaming**: Load bundles on-demand

**Manifest Format** (YAML):
```yaml
version: 1
bundles:
  - id: "core"
    assets:
      - id: "mesh/cube.obj"
        hash: "blake3:abc123..."
        size: 1024
        dependencies: []

  - id: "level_forest"
    assets:
      - id: "mesh/tree.glb"
        hash: "blake3:def456..."
        size: 102400
        dependencies: ["texture/bark.png"]
```

**Bundle Loading**:
```rust
// Load entire bundle (parallel)
asset_manager.load_bundle("level_forest").await?;

// All assets in bundle now available
let tree = asset_manager.get("mesh/tree.glb")?;
```

**Trade-offs**:
- ❌ No manifest: Slow startup, no integrity checks
- ❌ Simple list: No bundling, inefficient
- ✅ Bundles: Fast, organized, integrity-checked

---

### Decision 11: Build Pipeline

**Question**: No build pipeline (use source assets) vs standalone tool vs integrated tool?

**Answer**: **Standalone asset-cooker CLI tool**

**Rationale**:
- **Optimization**: Mesh vertex cache, texture compression, LOD generation
- **Cross-Platform**: Build on CI, deploy to all platforms
- **Batch Processing**: Cook all assets in parallel
- **Version Control**: Cooked assets separate from source

**Tool Features**:
- Mesh optimization (via meshopt)
- Texture compression (BC7 for desktop, ASTC for mobile)
- Mipmap generation
- LOD generation
- Binary output (fast loading)

**Usage**:
```bash
# Cook single asset
asset-cooker --input mesh.obj --output mesh.bin --optimize

# Cook entire directory (parallel)
asset-cooker --input assets/ --output cooked/ --recursive --optimize

# CI/CD integration
asset-cooker --input assets/ --output dist/assets/ --platform windows --optimize
```

**Trade-offs**:
- ❌ No pipeline: Slow loading, large file sizes
- ❌ Integrated: Tight coupling, hard to use in CI
- ✅ Standalone: Flexible, fast, CI-friendly

---

### Decision 12: Validation Strategy

**Question**: No validation vs format validation only vs multi-layer validation?

**Answer**: **Multi-layer validation (format + data integrity + checksums)**

**Rationale**:
- **Security**: Prevent malicious assets
- **Correctness**: Detect corrupted assets
- **Developer Experience**: Clear error messages

**Validation Layers**:

1. **Format Validation**: Magic number, version, header
2. **Data Integrity**: NaN/Inf check, bounds check
3. **Checksums**: Blake3 hash vs manifest

**Implementation**:
```rust
pub trait AssetValidator {
    fn validate_format(data: &[u8]) -> Result<(), ValidationError>;
    fn validate_data(&self) -> Result<(), ValidationError>;
}

impl AssetValidator for MeshData {
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        // Layer 1: Format validation
        if &data[0..4] != b"MESH" { return Err(ValidationError::InvalidMagic); }
        Ok(())
    }

    fn validate_data(&self) -> Result<(), ValidationError> {
        // Layer 2: Data integrity
        for vertex in &self.vertices {
            if !vertex.position.is_finite() { return Err(ValidationError::InvalidData); }
        }
        Ok(())
    }
}

// Layer 3: Checksum validation (against manifest)
asset_manager.verify_integrity(id, expected_hash)?;
```

**Conditional Compilation**:
```rust
#[cfg(debug_assertions)]
fn load_with_validation(data: &[u8]) -> Result<Asset, Error> {
    validate_format(data)?;  // Full validation in debug
    validate_data(data)?;
    parse(data)
}

#[cfg(not(debug_assertions))]
fn load_with_validation(data: &[u8]) -> Result<Asset, Error> {
    validate_format(data)?;  // Format only in release (performance)
    parse(data)
}
```

**Trade-offs**:
- ❌ No validation: Crashes on corrupted assets
- ❌ Format-only: Doesn't catch data corruption
- ✅ Multi-layer: Secure, correct, configurable

---

### Decision 13: File Formats

**Question**: Single format vs multiple formats vs extensible loader system?

**Answer**: **Multiple formats with extensible loader trait**

**Rationale**:
- **Interoperability**: glTF for meshes (industry standard)
- **Legacy Support**: OBJ for simple meshes
- **Performance**: Custom binary format for fast loading
- **Extensibility**: Easy to add new formats

**Supported Formats**:

| Asset Type | Formats | Notes |
|------------|---------|-------|
| Mesh | OBJ, glTF, FBX, Custom Binary | glTF preferred |
| Texture | PNG, JPG, DDS, KTX2 | DDS/KTX2 for compressed |
| Audio | WAV, OGG Vorbis, MP3 | OGG preferred |
| Material | glTF, Custom YAML | YAML for hand-authoring |
| Shader | GLSL, SPIR-V | SPIR-V for production |
| Font | TTF, OTF | TTF preferred |

**Extensible Loader**:
```rust
pub trait AssetLoader<T> {
    fn parse(&self, data: &[u8]) -> Result<T, AssetError>;
}

// Register loaders
asset_manager.register_loader("obj", ObjLoader);
asset_manager.register_loader("glb", GltfLoader);
asset_manager.register_loader("custom", CustomBinaryLoader);

// Auto-select loader based on extension
asset_manager.load("mesh.obj")?;  // Uses ObjLoader
asset_manager.load("mesh.glb")?;  // Uses GltfLoader
```

**Trade-offs**:
- ❌ Single format: Not interoperable
- ❌ Multiple hardcoded: Not extensible
- ✅ Extensible: Flexible, future-proof

---

### Decision 14: Conflict Resolution

**Question**: Last-write-wins vs version merging vs content-addressable?

**Answer**: **Content-addressable + semantic versioning**

**Rationale**:
- **Content-Addressable**: Different content → different ID (no conflicts)
- **Semantic Versioning**: Track asset versions for delta compression
- **No Merging Required**: Content hash is ground truth

**Version Tracking**:
```rust
pub struct AssetMetadata {
    pub id: AssetId,           // Content hash
    pub version: u64,          // Semantic version
    pub path: PathBuf,         // Source path
    pub last_modified: u64,    // Timestamp
}

// Loading new version
let old_id = AssetId::from_content(&old_data);
let new_id = AssetId::from_content(&new_data);

if old_id != new_id {
    // Content changed, new ID
    asset_manager.insert(new_id, new_data);
    asset_manager.increment_version(new_id);
}
```

**Benefits**:
- **No Conflicts**: Different content = different ID
- **Automatic Deduplication**: Same content = same ID
- **Delta Compression**: Version tracking enables deltas

**Trade-offs**:
- ❌ Last-write-wins: Race conditions, data loss
- ❌ Merging: Complex, error-prone
- ✅ Content-addressable: Simple, correct, efficient

---

## AAA Industry Research

### Referenced Systems

1. **Destiny 2 Asset System**
   - Hybrid manifest + streaming
   - Content-addressable IDs
   - Delta patching for updates
   - Inspired our manifest system

2. **Unreal Engine Asset Manager**
   - Async loading
   - Hard/soft references
   - Asset bundles
   - Inspired our reference counting

3. **Unity Asset Bundles**
   - Bundle grouping
   - Dependency tracking
   - LRU caching
   - Inspired our bundle system

4. **Frostbite Engine (Battlefield)**
   - Streaming LOD
   - Procedural generation
   - Hot-reload
   - Inspired our streaming strategy

---

## Implementation Priority

Based on dependencies and user value:

1. **Phase 1** (Critical): Handle system, basic loaders, sync loading
2. **Phase 2** (High Value): Async loading, LRU cache, hot-reload
3. **Phase 3** (Optimization): Streaming, network transfer, asset cooker
4. **Phase 4** (Polish): Validation, procedural generation, manifest

---

## Success Metrics

| Metric | Target | Rationale |
|--------|--------|-----------|
| Load time (mesh, 1KB) | < 100 µs | Fast startup |
| Load time (texture, 1MB) | < 5 ms | Acceptable delay |
| Hot-reload time | < 1 s | Fast iteration |
| Memory overhead | < 1% | Minimal waste |
| Network transfer (1MB) | < 10 ms | Smooth experience |
| LRU eviction | < 1 ms | No frame drops |

---

## Risks and Mitigations

### Risk 1: Scope Too Large
- **Mitigation**: Incremental implementation (12 tasks, each independently testable)
- **Mitigation**: Ship partial asset system if needed (mesh + texture only)

### Risk 2: Performance Regression
- **Mitigation**: Benchmarks for every operation
- **Mitigation**: Performance targets documented and tested

### Risk 3: GPU Upload Complexity
- **Mitigation**: Already have GpuMesh working (buffer.rs)
- **Mitigation**: Textures similar to meshes (create buffer, upload, transition)

### Risk 4: Hot-Reload Crashes
- **Mitigation**: Validation before reload
- **Mitigation**: Double-buffering GPU resources
- **Mitigation**: Extensive testing with invalid assets

---

## Conclusion

These decisions form the foundation of a production-grade asset system that:

✅ Scales from indie games to MMORPGs
✅ Supports all common asset types
✅ Provides fast iteration (hot-reload)
✅ Efficient network transfer (delta compression)
✅ Automatic memory management (LRU)
✅ Flexible ownership (hybrid server/client)
✅ Secure (validation + integrity checks)
✅ Extensible (plugin system for new formats)

All decisions are based on:
- **User Requirements**: Gathered through interview
- **AAA Best Practices**: Destiny 2, Unreal, Unity, Frostbite
- **Performance Targets**: Industry-standard metrics
- **Rust Ecosystem**: Leverage existing crates (notify, zstd, bsdiff, etc.)

Ready for implementation in Phase 1.7.
