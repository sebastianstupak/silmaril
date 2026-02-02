# Asset System Documentation Index

**Phase**: 1.7 - Complete Asset Management System
**Status**: Design Complete, Ready for Implementation
**Last Updated**: 2026-02-01

---

## Quick Start

**If you're implementing the asset system, read in this order:**

1. **[ROADMAP.md](../ROADMAP.md)** - High-level overview of Phase 1.7
2. **[decisions/asset-system-decisions.md](decisions/asset-system-decisions.md)** - All design decisions with rationale
3. **[architecture/asset-system.md](architecture/asset-system.md)** - Complete architecture documentation
4. **[tasks/phase1-7-asset-system.md](tasks/phase1-7-asset-system.md)** - Detailed task breakdown (12 tasks, 12-15 days)

---

## Document Purpose

### ROADMAP.md (Phase 1.7 Section)

**What**: High-level feature list and timeline
**When to Read**: Before starting Phase 1.7
**Key Info**:
- Feature list (handle system, all asset types, loading strategies, hot-reload, network transfer, etc.)
- 12-15 day estimate
- Dependencies (Phase 1.6 complete)

### decisions/asset-system-decisions.md

**What**: All 14 design decisions with rationale
**When to Read**: Before implementing any feature
**Key Info**:
- Decision 1: Content-addressable IDs (Blake3)
- Decision 2: Reference counting (Hard/Soft)
- Decision 3: Automatic dependency tracking
- Decision 4: Hybrid ownership (server/client configurable)
- Decision 5: Three loading strategies (Sync/Async/Streaming)
- Decision 6: Full + delta network transfer
- Decision 7: LRU memory management
- Decision 8: Hot-reload with validation
- Decision 9: Configurable procedural generation
- Decision 10: Hybrid manifest system (bundles)
- Decision 11: Standalone asset-cooker tool
- Decision 12: Multi-layer validation
- Decision 13: Multiple file formats
- Decision 14: Content-addressable conflict resolution

**AAA Research Included**:
- Destiny 2 asset system
- Unreal Engine Asset Manager
- Unity Asset Bundles
- Frostbite streaming

### architecture/asset-system.md

**What**: Complete technical architecture
**When to Read**: When implementing any module
**Key Info**:
- Architecture diagrams (ASCII art)
- Module structure (crates, files)
- Data flow diagrams
- Asset lifecycle (states: Unloaded → Loading → Loaded → On GPU → Evicted)
- Handle system implementation
- Loading strategies (sync, async, streaming)
- Memory management (LRU cache)
- Network transfer protocol
- Hot-reload process
- Procedural generation
- Integration points (renderer, networking, ECS)
- Performance characteristics
- Security considerations

### tasks/phase1-7-asset-system.md

**What**: Detailed task breakdown for implementation
**When to Read**: Before implementing each task
**Key Info**:

**Task 1** (2 days): Asset Handle System
- AssetId (Blake3 hash)
- AssetHandle<T> (hard/soft references)
- AssetRegistry<T> (storage)

**Task 2** (3 days): Asset Types - Core Data Structures
- 2.1: Mesh Assets (glTF, OBJ, FBX, custom binary)
- 2.2: Texture Assets (PNG, DDS, KTX2, mipmaps)
- 2.3: Material Assets (PBR parameters)
- 2.4: Audio Assets (WAV, OGG, MP3)
- 2.5: Shader Assets (GLSL, SPIR-V)
- 2.6: Font Assets (TTF, OTF)

**Task 3** (2 days): Loading Strategies
- 3.1: Synchronous loader
- 3.2: Asynchronous loader (tokio)
- 3.3: Streaming loader (progressive LOD)

**Task 4** (1.5 days): Hot-Reload System
- 4.1: File watcher (notify crate)
- 4.2: Safe reload (validation, double-buffering)

**Task 5** (2 days): Network Transfer
- 5.1: Full transfer (zstd compression)
- 5.2: Delta transfer (bsdiff)
- 5.3: Transfer protocol (TCP)

**Task 6** (2 days): Memory Management
- 6.1: Memory tracking
- 6.2: LRU cache (LinkedHashMap)
- 6.3: Memory budgets

**Task 7** (1.5 days): Asset Manifest & Bundles
- 7.1: Manifest format (YAML)
- 7.2: Bundle loading (parallel)
- 7.3: Integrity validation

**Task 8** (1 day): Procedural Generation
- 8.1: Procedural API (deterministic RNG)
- 8.2: Caching

**Task 9** (2 days): Asset Cooker Tool
- 9.1: CLI tool structure
- 9.2: Mesh optimization (meshopt)
- 9.3: Texture processing (BC7, ASTC)
- 9.4: Batch processing

**Task 10** (1 day): Validation System
- 10.1: Format validation
- 10.2: Data integrity

**Task 11** (1 day): Integration with Renderer
- 11.1: GPU upload
- 11.2: Asset → GPU handle mapping

**Task 12** (1 day): Documentation & Examples
- 12.1: API documentation
- 12.2: Examples (loading, hot-reload, procedural, network)

### tasks/phase1-8-mesh-rendering.md

**What**: Mesh rendering implementation (uses asset system)
**When to Read**: After Phase 1.7 complete
**Key Info**:
- Graphics pipeline
- Transform component
- Camera system
- Depth buffer
- MeshRenderer component
- Rendering integration with ECS
- Examples and testing

---

## Key Concepts

### Asset Handle System

```rust
// Type-safe handles
let mesh: AssetHandle<MeshData> = asset_manager.load("cube.obj")?;
let texture: AssetHandle<TextureData> = asset_manager.load("brick.png")?;

// Automatic reference counting
let mesh2 = mesh.clone(); // RefCount = 2
drop(mesh2);              // RefCount = 1
drop(mesh);               // RefCount = 0, asset unloaded

// Hard vs Soft references
let critical = asset_manager.load_hard("player.glb")?;  // Never evicted
let optional = asset_manager.load_soft("tree.glb")?;    // Can be evicted by LRU
```

### Loading Strategies

```rust
// Sync: Blocks until loaded (< 16ms for small assets)
let config = asset_manager.load_sync("config.yaml")?;

// Async: Non-blocking, awaitable
let level = asset_manager.load_async("level.glb").await?;

// Streaming: Immediate low-res, upgrade progressively
let texture = asset_manager.load_streaming("terrain.png").await?;
```

### Network Transfer

```rust
// Client requests asset
client.send(AssetRequest { id, have_version: Some(42) });

// Server responds
if delta_size < full_size * 0.5 {
    server.send(AssetResponseDelta { patch });  // Bandwidth efficient
} else {
    server.send(AssetResponseFull { data });    // Simpler, faster
}
```

### Hot-Reload

```rust
// File watcher detects change
watcher.poll(); // Returns AssetEvent::Modified("mesh.obj")

// Safe reload (doesn't crash on invalid assets)
asset_manager.reload("mesh.obj")?; // Validates → uploads → swaps

// If validation fails: old version kept, error logged
```

### Procedural Generation

```rust
// Deterministic generation (same seed = same asset)
let terrain = asset_manager.generate_or_load::<MeshData>(
    seed: 42,
    params: GeneratorParams { size: 100, height: 10.0 },
)?;

// Cached (don't regenerate)
let terrain2 = asset_manager.generate_or_load::<MeshData>(
    seed: 42,
    params: GeneratorParams { size: 100, height: 10.0 },
)?;
assert_eq!(terrain.id(), terrain2.id()); // Same asset
```

---

## Performance Targets

| Metric | Target | Critical | Documented In |
|--------|--------|----------|---------------|
| Mesh load (sync, 1KB) | < 100 µs | < 1 ms | Tasks |
| Texture load (sync, 1MB) | < 5 ms | < 50 ms | Tasks |
| Hot-reload | < 1 s | < 3 s | Tasks |
| Network transfer (1MB) | < 10 ms | < 100 ms | Tasks |
| LRU eviction | < 1 ms | < 10 ms | Tasks |
| Handle overhead | < 1% | < 5% | Architecture |

---

## Testing Strategy

### Unit Tests
- AssetId generation (determinism, collision resistance)
- Reference counting (increment, decrement, auto-cleanup)
- LRU eviction (correct order)
- Each asset loader (OBJ, glTF, PNG, etc.)

### Integration Tests
- Load → use → unload lifecycle
- Hot-reload (modify → detect → reload → verify)
- Network transfer (client ↔ server)
- Bundle loading (dependencies resolved)
- Procedural generation (determinism)

### Benchmarks
- Load time per asset type
- Compression ratio (zstd, bsdiff)
- LRU eviction overhead
- Handle creation/access time

### E2E Tests
- Load assets → render in engine
- Modify asset → hot-reload → see changes
- Server generates → client receives → renders

---

## Dependencies

### Existing Crates
- `engine-core` - ECS integration, error handling
- `engine-renderer` - GPU upload (GpuMesh, GpuTexture)
- `engine-networking` - Asset transfer protocol

### New Crates
- `engine-assets` - Pure data structures (already created)
- `engine-asset-manager` - Lifecycle, caching, hot-reload
- `engine/tools/asset-cooker` - Standalone CLI tool

### External Dependencies
- `blake3` - Content-addressable IDs
- `notify` - File watching
- `zstd` - Compression
- `bsdiff` / `qbsdiff` - Binary diffing
- `gltf` - glTF loader
- `image` - Image loading
- `ddsfile` - DDS textures
- `ktx2` - KTX2 textures
- `hound` - WAV audio
- `lewton` - OGG Vorbis
- `ttf-parser` - Fonts
- `meshopt` - Mesh optimization
- `tokio` - Async I/O

---

## Implementation Checklist

Before starting each task:
- [ ] Read relevant decision in decisions/asset-system-decisions.md
- [ ] Read architecture section in architecture/asset-system.md
- [ ] Read task breakdown in tasks/phase1-7-asset-system.md
- [ ] Check performance targets
- [ ] Review testing requirements

During implementation:
- [ ] Follow CLAUDE.md coding standards
- [ ] Use structured logging (tracing, no println!)
- [ ] Custom error types (no anyhow)
- [ ] Write tests first (TDD)
- [ ] Benchmark performance-critical code
- [ ] Document public APIs (rustdoc)

Before marking task complete:
- [ ] All tests passing
- [ ] Performance targets met
- [ ] Documentation complete
- [ ] No regressions (all existing tests still pass)
- [ ] Code reviewed (follows CLAUDE.md)

---

## Common Pitfalls

### ❌ Don't: Mix asset data with GPU code
```rust
// BAD: MeshData in engine-renderer
pub struct MeshData { ... } // in engine/renderer/src/mesh.rs
```

### ✅ Do: Separate data from GPU
```rust
// GOOD: MeshData in engine-assets, GpuMesh in engine-renderer
pub struct MeshData { ... }    // in engine/assets/src/mesh.rs
pub struct GpuMesh { ... }      // in engine/renderer/src/buffer.rs
```

### ❌ Don't: Use anyhow or Box<dyn Error>
```rust
// BAD
fn load_asset() -> anyhow::Result<Asset> { ... }
```

### ✅ Do: Use custom error types
```rust
// GOOD
fn load_asset() -> Result<Asset, AssetError> { ... }
```

### ❌ Don't: Block main thread for large assets
```rust
// BAD: Sync load of 100MB texture blocks frame
let texture = asset_manager.load_sync("huge.png")?; // 200ms!
```

### ✅ Do: Use async for large assets
```rust
// GOOD: Async load doesn't block
let texture = asset_manager.load_async("huge.png").await?; // 0ms main thread
```

### ❌ Don't: Forget to validate before reload
```rust
// BAD: Crashes on invalid asset
let new_data = std::fs::read("mesh.obj")?;
self.assets.insert(id, new_data); // CRASH if invalid
```

### ✅ Do: Validate before swapping
```rust
// GOOD: Keep old version if validation fails
let new_data = std::fs::read("mesh.obj")?;
if validate(&new_data).is_ok() {
    self.assets.insert(id, new_data);
}
```

---

## Questions?

If you're implementing the asset system and have questions:

1. **Design Decision**: Check [decisions/asset-system-decisions.md](decisions/asset-system-decisions.md)
2. **Architecture**: Check [architecture/asset-system.md](architecture/asset-system.md)
3. **Implementation**: Check [tasks/phase1-7-asset-system.md](tasks/phase1-7-asset-system.md)
4. **Coding Standards**: Check [CLAUDE.md](../CLAUDE.md)
5. **Still unclear**: Ask user for clarification

---

## Success Criteria

Phase 1.7 is complete when:

1. ✅ All 12 tasks implemented
2. ✅ All asset types working (Mesh, Texture, Material, Audio, Shader, Font)
3. ✅ All loading strategies working (Sync, Async, Streaming)
4. ✅ Hot-reload working (< 1s reload time)
5. ✅ Network transfer working (full + delta)
6. ✅ Memory management working (LRU eviction)
7. ✅ Procedural generation working (deterministic)
8. ✅ Asset cooker working (optimize meshes, compress textures)
9. ✅ All tests passing (100% pass rate)
10. ✅ Performance targets met (see table above)
11. ✅ Documentation complete (all public APIs documented)
12. ✅ Examples running (loading, hot-reload, procedural, network)

---

## What's Next?

After Phase 1.7 complete:
- **Phase 1.8**: Mesh Rendering (uses asset system for MeshData)
- **Phase 1.9**: Frame Capture
- **Phase 2+**: Advanced rendering (PBR, shadows, lighting)

See [tasks/phase1-8-mesh-rendering.md](tasks/phase1-8-mesh-rendering.md) for mesh rendering implementation.

---

**Ready to implement? Start with Task 1: Asset Handle System in [tasks/phase1-7-asset-system.md](tasks/phase1-7-asset-system.md)!**
