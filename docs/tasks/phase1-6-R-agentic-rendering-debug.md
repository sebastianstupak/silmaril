# Phase 1.6.R: Agentic Rendering Debug Infrastructure

**Status:** Not Started
**Priority:** HIGH (blocks efficient rendering development)
**Duration:** 3 weeks (15 working days)
**Dependencies:** Phase 1.5 (Vulkan Context) ✅, Phase 1.6.1-1.6.3 (Window, Surface, RenderPass) ✅

---

## Overview

Following the successful physics agentic debugging approach, this phase implements machine-readable rendering debug infrastructure to enable AI agents to autonomously debug rendering issues. This infrastructure is implemented BEFORE completing the rendering pipeline (Phase 1.6.4-1.6.8) to enable debugging from first triangle render.

### Philosophy: AI-First Debugging

**Traditional Approach (Human-Centric):**
- Visual debuggers (RenderDoc, PIX)
- Manual frame inspection
- Human interpretation required
- Time: Hours to days per bug

**Agentic Approach (AI-First):**
- Machine-readable data export (JSONL, SQLite, PNG)
- Programmatic query API
- Autonomous analysis by AI agents
- Time: Seconds to minutes per bug

### Key Benefits

1. **Autonomous Debugging:** AI agents diagnose rendering issues without human intervention
2. **Historical Analysis:** Query past frames without re-running simulation
3. **Regression Testing:** Visual diff enables automated regression detection in CI
4. **Performance Validation:** GPU timestamp queries enable performance tracking from day one
5. **Resource Leak Detection:** Automatic tracking of texture/buffer lifecycle
6. **Consistent Approach:** Matches physics debugging infrastructure (unified workflow)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Vulkan Renderer                          │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Context │  │ Pipeline │  │ Commands │  │ Swapchain│    │
│  └────┬────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │            │             │             │            │
│       ▼            ▼             ▼             ▼            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Rendering Debug Infrastructure (R.1-R.5)     │  │
│  │                                                        │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐     │  │
│  │  │  Snapshot  │  │   Events   │  │   Capture  │     │  │
│  │  │   System   │  │  Recorder  │  │   System   │     │  │
│  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘     │  │
│  └────────┼───────────────┼───────────────┼────────────┘  │
└───────────┼───────────────┼───────────────┼───────────────┘
            │               │               │
            ▼               ▼               ▼
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │ JSONL Export │  │ SQLite Export│  │  PNG Export  │
    └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
           │                 │                  │
           └─────────────────┴──────────────────┘
                            │
                            ▼
                  ┌──────────────────────┐
                  │  Query API (R.4)     │
                  │  - Resource queries  │
                  │  - Performance       │
                  │  - Error detection   │
                  │  - Visual diff       │
                  └──────────┬───────────┘
                             │
                             ▼
                   ┌─────────────────────┐
                   │   AI Agent Debugger │
                   │   - Detect leaks    │
                   │   - Find bottlenecks│
                   │   - Diagnose errors │
                   └─────────────────────┘
```

---

## Task Breakdown

### R.1: Render State Snapshot System
**Duration:** 3-4 days
**LOC:** ~300
**Files:** `engine/renderer/src/debug/snapshot.rs`

#### Data Structures

```rust
/// Complete render state snapshot for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderDebugSnapshot {
    pub frame: u64,
    pub timestamp: f64,

    // Pipeline state
    pub active_pipeline: Option<String>,
    pub shader_stages: Vec<ShaderStageInfo>,
    pub viewport: Viewport,
    pub scissor: Rect2D,
    pub depth_test_enabled: bool,
    pub blend_enabled: bool,

    // Resources
    pub render_targets: Vec<RenderTargetInfo>,
    pub framebuffers: Vec<FramebufferInfo>,
    pub textures: Vec<TextureInfo>,
    pub buffers: Vec<BufferInfo>,

    // Draw calls
    pub draw_calls: Vec<DrawCallInfo>,

    // GPU state
    pub gpu_memory: GpuMemoryStats,
    pub queue_states: Vec<QueueStateInfo>,
}

/// Single draw call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCallInfo {
    pub draw_call_id: u64,
    pub mesh_id: u64,
    pub material_id: u64,
    pub pipeline_id: u64,
    pub vertex_count: u32,
    pub index_count: u32,
    pub instance_count: u32,
    pub transform: [f32; 16],  // 4x4 matrix

    // GPU profiling (from timestamp queries)
    pub draw_time_gpu_ns: u64,
    pub vertices_processed: u64,
    pub fragments_processed: u64,
}

/// Texture resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureInfo {
    pub texture_id: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub mip_levels: u32,
    pub memory_size_bytes: usize,
    pub usage_flags: Vec<String>,
    pub created_frame: u64,
}

/// GPU memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMemoryStats {
    pub total_allocated: usize,
    pub textures: usize,
    pub buffers: usize,
    pub framebuffers: usize,
    pub device_local: usize,
    pub host_visible: usize,
}
```

#### Implementation Requirements

1. **Snapshot Creation:**
   - Capture at end of each frame
   - Zero-copy where possible (references, not clones)
   - Opt-in (configurable per frame)

2. **GPU Timestamp Queries:**
   - Query pool creation (per frame)
   - Timestamp before/after each draw call
   - Convert ticks to nanoseconds

3. **Validation:**
   - Check for NaN/Inf in transforms
   - Validate resource handles
   - Detect out-of-bounds indices

4. **Testing:**
   - Unit test: Snapshot creation
   - Unit test: GPU timestamp calculation
   - Unit test: Validation logic
   - Property test: Serialization roundtrip

#### Deliverables
- [ ] `RenderDebugSnapshot` struct with all fields
- [ ] GPU timestamp query implementation
- [ ] Validation methods
- [ ] 8+ unit tests
- [ ] Benchmarks (snapshot creation < 1ms for 1000 draw calls)

---

### R.2: Rendering Event Stream
**Duration:** 3 days
**LOC:** ~250
**Files:** `engine/renderer/src/debug/events.rs`

#### Event Types

```rust
/// Rendering events for debugging and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderEvent {
    // Resource lifecycle
    TextureCreated {
        texture_id: u64,
        width: u32,
        height: u32,
        format: String,
        memory_size: usize,
        frame: u64,
    },
    TextureDestroyed {
        texture_id: u64,
        frame: u64,
    },
    BufferCreated {
        buffer_id: u64,
        size: usize,
        usage: String,
        frame: u64,
    },
    BufferDestroyed {
        buffer_id: u64,
        frame: u64,
    },

    // Pipeline events
    PipelineCreated {
        pipeline_id: u64,
        vertex_shader: String,
        fragment_shader: String,
        frame: u64,
    },
    ShaderCompilationFailed {
        shader_path: String,
        error_message: String,
        frame: u64,
    },

    // Draw call events
    DrawCallSubmitted {
        draw_call_id: u64,
        mesh_id: u64,
        material_id: u64,
        vertex_count: u32,
        frame: u64,
    },
    DrawCallFailed {
        draw_call_id: u64,
        error: String,
        frame: u64,
    },

    // Synchronization events
    FenceWaitTimeout {
        fence_id: u64,
        timeout_ms: u64,
        frame: u64,
    },
    SwapchainRecreated {
        reason: String,
        old_width: u32,
        old_height: u32,
        new_width: u32,
        new_height: u32,
        frame: u64,
    },

    // Performance events
    FrameDropped {
        expected_frame_time_ms: f32,
        actual_frame_time_ms: f32,
        frame: u64,
    },
    GpuMemoryExhausted {
        requested_size: usize,
        available_size: usize,
        frame: u64,
    },
}

/// Event recorder for collecting rendering events
pub struct EventRecorder {
    events: Vec<RenderEvent>,
    enabled: bool,
    total_events: usize,
}

impl EventRecorder {
    pub fn new() -> Self;
    pub fn enable(&mut self);
    pub fn disable(&mut self);
    pub fn record(&mut self, event: RenderEvent);
    pub fn drain(&mut self) -> Vec<RenderEvent>;
    pub fn event_count(&self) -> usize;
}
```

#### Event Classification

```rust
impl RenderEvent {
    pub fn is_critical(&self) -> bool;
    pub fn is_resource_event(&self) -> bool;
    pub fn is_error_event(&self) -> bool;
    pub fn frame(&self) -> u64;
    pub fn involved_resources(&self) -> Vec<u64>;
}
```

#### Implementation Requirements

1. **Event Recording:**
   - Thread-safe event collection (Mutex or lock-free queue)
   - Enable/disable recording dynamically
   - Drain events (consume and return)

2. **Integration Points:**
   - Hook into VulkanContext resource creation/destruction
   - Hook into Renderer draw call submission
   - Hook into Swapchain recreation

3. **Testing:**
   - Unit test: Event creation and accessors
   - Unit test: Enable/disable recording
   - Unit test: Event draining
   - Unit test: Serialization

#### Deliverables
- [ ] `RenderEvent` enum with 12+ event types
- [ ] `EventRecorder` with thread-safe collection
- [ ] Integration hooks in renderer
- [ ] 6+ unit tests

---

### R.3: Export Infrastructure
**Duration:** 2-3 days
**LOC:** ~200
**Files:** `engine/renderer/src/debug/exporters.rs`

#### Exporters

```rust
/// JSONL (JSON Lines) exporter for streaming event export
pub struct JsonlExporter {
    writer: BufWriter<File>,
    objects_written: usize,
}

impl JsonlExporter {
    pub fn create(path: &Path) -> Result<Self, ExportError>;
    pub fn append(path: &Path) -> Result<Self, ExportError>;
    pub fn write_snapshot(&mut self, snapshot: &RenderDebugSnapshot) -> Result<(), ExportError>;
    pub fn write_event(&mut self, event: &RenderEvent) -> Result<(), ExportError>;
    pub fn flush(&mut self) -> Result<(), ExportError>;
    pub fn finish(self) -> Result<usize, ExportError>;
}

/// SQLite exporter for queryable database
pub struct SqliteExporter {
    conn: Connection,
}

impl SqliteExporter {
    pub fn create(path: &Path) -> Result<Self, ExportError>;

    /// Initialize database schema
    fn init_schema(&self) -> Result<(), ExportError>;

    /// Export snapshot and all related data
    pub fn write_snapshot(&mut self, snapshot: &RenderDebugSnapshot) -> Result<(), ExportError>;

    /// Export event
    pub fn write_event(&mut self, event: &RenderEvent) -> Result<(), ExportError>;
}
```

#### SQLite Schema

```sql
CREATE TABLE snapshots (
    frame INTEGER PRIMARY KEY,
    timestamp REAL NOT NULL,
    active_pipeline TEXT,
    gpu_memory_total INTEGER,
    draw_call_count INTEGER
);

CREATE TABLE draw_calls (
    draw_call_id INTEGER PRIMARY KEY,
    frame INTEGER NOT NULL,
    mesh_id INTEGER,
    material_id INTEGER,
    vertex_count INTEGER,
    index_count INTEGER,
    draw_time_gpu_ns INTEGER,
    FOREIGN KEY (frame) REFERENCES snapshots(frame)
);

CREATE TABLE textures (
    texture_id INTEGER PRIMARY KEY,
    frame_created INTEGER NOT NULL,
    frame_destroyed INTEGER,
    width INTEGER,
    height INTEGER,
    format TEXT,
    memory_size INTEGER,
    FOREIGN KEY (frame_created) REFERENCES snapshots(frame)
);

CREATE TABLE events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    frame INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,  -- JSON blob
    FOREIGN KEY (frame) REFERENCES snapshots(frame)
);

-- Indices for common queries
CREATE INDEX idx_draw_calls_frame ON draw_calls(frame);
CREATE INDEX idx_draw_calls_time ON draw_calls(draw_time_gpu_ns);
CREATE INDEX idx_textures_created ON textures(frame_created);
CREATE INDEX idx_events_frame ON events(frame);
CREATE INDEX idx_events_type ON events(event_type);
```

#### Frame Capture Export

```rust
/// PNG frame capture exporter
pub struct PngExporter;

impl PngExporter {
    pub fn export_frame(
        path: &Path,
        color_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), ExportError>;

    pub fn export_comparison(
        path: &Path,
        expected: &[u8],
        actual: &[u8],
        diff: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), ExportError>;
}
```

#### Implementation Requirements

1. **Performance:**
   - Buffered I/O (BufWriter)
   - Transaction batching for SQLite
   - Async export (optional, for production)

2. **Error Handling:**
   - Custom ExportError type
   - Graceful degradation (log errors, continue)

3. **Testing:**
   - Integration test: JSONL export/import roundtrip
   - Integration test: SQLite database creation and querying
   - Integration test: PNG frame export

#### Deliverables
- [ ] JsonlExporter with streaming writes
- [ ] SqliteExporter with full schema
- [ ] PngExporter for frame capture
- [ ] 4+ integration tests

---

### R.4: Rendering Query API
**Duration:** 3-4 days
**LOC:** ~300
**Files:** `engine/renderer/src/debug/query.rs`

#### Query API

```rust
/// High-level query API for AI agents
pub struct RenderingQueryAPI {
    conn: Connection,
}

impl RenderingQueryAPI {
    pub fn open(path: &Path) -> Result<Self, QueryError>;

    // Resource lifecycle queries
    pub fn texture_lifecycle(&self, texture_id: u64) -> Result<TextureLifecycle, QueryError>;
    pub fn find_leaked_resources(&self) -> Result<Vec<LeakedResource>, QueryError>;
    pub fn buffer_lifecycle(&self, buffer_id: u64) -> Result<BufferLifecycle, QueryError>;

    // Performance queries
    pub fn slow_draw_calls(
        &self,
        threshold_ms: f32,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<DrawCallInfo>, QueryError>;

    pub fn frame_times(
        &self,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<(u64, f32)>, QueryError>;

    pub fn gpu_memory_over_time(
        &self,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<(u64, GpuMemoryStats)>, QueryError>;

    // Error queries
    pub fn shader_compilation_errors(&self) -> Result<Vec<ShaderError>, QueryError>;
    pub fn swapchain_recreations(&self) -> Result<Vec<SwapchainEvent>, QueryError>;
    pub fn draw_call_failures(&self) -> Result<Vec<DrawCallError>, QueryError>;

    // Regression testing
    pub fn compare_render_outputs(
        &self,
        frame_a: u64,
        frame_b: u64,
    ) -> Result<ImageDiff, QueryError>;

    // Advanced queries
    pub fn raw_query(&self, sql: &str) -> Result<Vec<HashMap<String, Value>>, QueryError>;
    pub fn statistics(&self) -> Result<DatabaseStats, QueryError>;
}
```

#### Result Types

```rust
#[derive(Debug, Clone)]
pub struct TextureLifecycle {
    pub texture_id: u64,
    pub created_frame: u64,
    pub destroyed_frame: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub memory_size: usize,
    pub usage_count: usize,  // How many frames it was used
}

#[derive(Debug, Clone)]
pub struct LeakedResource {
    pub resource_type: String,  // "texture", "buffer", "framebuffer"
    pub resource_id: u64,
    pub created_frame: u64,
    pub memory_size: usize,
    pub last_used_frame: u64,
}

#[derive(Debug, Clone)]
pub struct ImageDiff {
    pub frame_a: u64,
    pub frame_b: u64,
    pub pixels_different: usize,
    pub percent_different: f32,
    pub max_color_delta: f32,
    pub avg_color_delta: f32,
}
```

#### Implementation Requirements

1. **Query Optimization:**
   - Use prepared statements
   - Index optimization
   - Connection pooling (if needed)

2. **Error Handling:**
   - Descriptive error messages
   - Handle missing data gracefully

3. **Testing:**
   - Unit test: Each query method
   - Integration test: Query results match expected data
   - Edge case test: No data, malformed database

#### Deliverables
- [ ] `RenderingQueryAPI` with 10+ query methods
- [ ] Result types for all queries
- [ ] 10+ unit tests
- [ ] Integration test with real database

---

### R.5: Frame Capture + Analysis
**Duration:** 4-5 days
**LOC:** ~400
**Files:** `engine/renderer/src/debug/capture.rs`

#### Frame Capture

```rust
/// Complete frame capture data
#[derive(Debug, Clone)]
pub struct FrameCaptureData {
    pub frame: u64,
    pub width: u32,
    pub height: u32,

    // Image data
    pub color_buffer: Vec<u8>,      // RGBA8
    pub depth_buffer: Vec<f32>,     // Depth values [0.0, 1.0]

    // Metadata
    pub metadata: FrameMetadata,

    // Advanced debug info
    pub overdraw_map: Option<Vec<u8>>,     // How many times each pixel was drawn
    pub entity_id_map: Option<Vec<u32>>,   // Which entity rendered each pixel
}

#[derive(Debug, Clone)]
pub struct FrameMetadata {
    pub timestamp: f64,
    pub draw_call_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub gpu_time_ms: f32,
}

/// Rendering debugger with frame capture and analysis
pub struct RenderingDebugger {
    context: Arc<VulkanContext>,
    config: DebugConfig,
}

impl RenderingDebugger {
    pub fn new(context: Arc<VulkanContext>, config: DebugConfig) -> Self;

    /// Capture current frame (color + depth + metadata)
    pub fn capture_frame(&self) -> Result<FrameCaptureData, CaptureError>;

    /// Compare two frames and generate diff
    pub fn compare_frames(
        &self,
        expected: &FrameCaptureData,
        actual: &FrameCaptureData,
    ) -> Result<FrameDiff, CaptureError>;

    /// Detect visual anomalies (missing objects, incorrect colors, etc.)
    pub fn detect_visual_anomalies(
        &self,
        frame: &FrameCaptureData,
    ) -> Result<Vec<Anomaly>, CaptureError>;
}
```

#### Frame Comparison

```rust
#[derive(Debug, Clone)]
pub struct FrameDiff {
    pub pixels_different: usize,
    pub percent_different: f32,
    pub max_color_delta: u8,
    pub avg_color_delta: f32,

    // Visual diff image (red = different, green = same)
    pub diff_image: Vec<u8>,

    // Per-channel differences
    pub red_delta: f32,
    pub green_delta: f32,
    pub blue_delta: f32,
    pub alpha_delta: f32,
}

#[derive(Debug, Clone)]
pub enum Anomaly {
    MissingObject {
        entity_id: u64,
        expected_bounds: Rect,
    },
    UnexpectedObject {
        entity_id: u64,
        actual_bounds: Rect,
    },
    ColorMismatch {
        pixel: (u32, u32),
        expected_color: [u8; 4],
        actual_color: [u8; 4],
    },
    DepthMismatch {
        pixel: (u32, u32),
        expected_depth: f32,
        actual_depth: f32,
    },
}
```

#### Implementation Requirements

1. **Capture Performance:**
   - Async GPU → CPU transfer (double buffering)
   - Benchmark: < 2ms overhead target

2. **Overdraw Analysis:**
   - Use atomic counter in shader (optional)
   - Or analyze draw call overlap (CPU-side)

3. **Entity ID Map:**
   - Render pass with entity ID as color output
   - Separate render target

4. **Visual Diff:**
   - Per-pixel color distance
   - Perceptual difference (optional)
   - Generate red/green diff image

5. **Testing:**
   - Unit test: Frame comparison logic
   - Integration test: Capture from real render
   - Benchmark: Capture overhead

#### Deliverables
- [ ] `FrameCaptureData` with color, depth, metadata
- [ ] `RenderingDebugger` with capture and analysis
- [ ] Frame comparison and diff generation
- [ ] Visual anomaly detection (basic)
- [ ] 6+ tests
- [ ] Benchmark validating < 2ms overhead

---

## Integration with Existing Renderer

### VulkanContext Integration

Add debug hooks to `engine/renderer/src/context.rs`:

```rust
impl VulkanContext {
    /// Enable debug snapshot recording
    pub fn enable_debug_snapshots(&mut self, config: DebugConfig);

    /// Create debug snapshot of current state
    pub fn create_debug_snapshot(&self, frame: u64) -> RenderDebugSnapshot;

    /// Get event recorder (mutable access)
    pub fn event_recorder_mut(&mut self) -> &mut EventRecorder;
}
```

### Renderer Integration

Add debug hooks to `engine/renderer/src/renderer.rs`:

```rust
impl Renderer {
    /// Enable agentic debug mode
    pub fn enable_agentic_debug(&mut self, config: DebugConfig);

    /// Export debug data for frame range
    pub fn export_debug_data(
        &self,
        start_frame: u64,
        end_frame: u64,
        path: &Path,
    ) -> Result<(), ExportError>;
}
```

---

## Testing Strategy

### Unit Tests (25+ tests)
- R.1: Snapshot creation, validation, serialization
- R.2: Event recording, classification, draining
- R.3: Export format correctness
- R.4: Query correctness, edge cases
- R.5: Frame comparison logic, anomaly detection

### Integration Tests (8+ tests)
- End-to-end: Render → Snapshot → Export → Query
- JSONL export/import roundtrip
- SQLite database creation and queries
- PNG frame capture and comparison
- Resource leak detection workflow
- Slow draw call detection workflow

### Benchmarks
- Snapshot creation overhead (< 1ms for 1000 draw calls)
- Export throughput (JSONL, SQLite)
- Query latency (< 10ms per query)
- Frame capture overhead (< 2ms target)

---

## Example AI Agent Workflow

### Scenario: Resource Leak Detection

**Bug:** GPU memory usage grows over time, eventually crashes

**AI Agent Workflow:**

```rust
// 1. Open exported debug database
let api = RenderingQueryAPI::open("rendering_debug.db")?;

// 2. Query for leaked resources
let leaks = api.find_leaked_resources()?;

// 3. Analyze results
for leak in leaks {
    println!("Leaked {}: ID {} created at frame {}, {} bytes",
        leak.resource_type, leak.resource_id,
        leak.created_frame, leak.memory_size);

    // 4. Query lifecycle
    let lifecycle = api.texture_lifecycle(leak.resource_id)?;
    println!("  Used in {} frames, never destroyed", lifecycle.usage_count);
}

// 5. Generate report
println!("ROOT CAUSE: Textures created but never destroyed");
println!("FIX: Add texture.destroy() call in cleanup code");
```

**Time to diagnosis:** 30 seconds (automated)

---

## Success Criteria

- ✅ Complete render state exportable (snapshots, events, frames)
- ✅ Query API supports 90% of common debugging scenarios
- ✅ Frame capture enables visual regression testing
- ✅ Performance overhead < 5% when recording enabled
- ✅ Exported data is AI agent-readable (JSON, SQL, PNG)
- ✅ Example AI agent can detect resource leaks, shader errors, performance bottlenecks autonomously

---

## Deliverables

1. **Code:**
   - `engine/renderer/src/debug/` module (R.1-R.5)
   - Integration hooks in VulkanContext and Renderer
   - 25+ unit tests, 8+ integration tests
   - Benchmarks validating performance targets

2. **Documentation:**
   - API reference (rustdoc)
   - Integration guide (how to enable debug mode)
   - Example workflows (resource leak detection, performance analysis)

3. **Examples:**
   - `examples/rendering_debugger_ai_agent.rs` - Autonomous debugging example
   - `examples/visual_regression_test.rs` - Frame comparison example

4. **Tooling:**
   - `scripts/export_rendering_debug.sh` - Export script
   - `scripts/analyze_rendering_debug.py` - Analysis script (Python, for AI agents)

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-2:** R.1 (Render State Snapshot System) - Data structures, GPU timestamps
- **Day 3:** R.1 (continued) - Validation, tests
- **Day 4:** R.2 (Rendering Event Stream) - Event types, recorder
- **Day 5:** R.2 (continued) - Integration hooks, tests

### Week 2 (Days 6-10)
- **Day 6-7:** R.3 (Export Infrastructure) - JSONL, SQLite exporters
- **Day 8:** R.3 (continued) - PNG exporter, tests
- **Day 9-10:** R.4 (Rendering Query API) - Query methods, result types

### Week 3 (Days 11-15)
- **Day 11-12:** R.4 (continued) - Advanced queries, tests
- **Day 13-14:** R.5 (Frame Capture + Analysis) - Capture system, frame comparison
- **Day 15:** R.5 (continued) - Visual anomaly detection, final testing

---

## Dependencies

### Crate Dependencies
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.30", features = ["bundled"] }
png = "0.17"  # For frame capture export

[dev-dependencies]
tempfile = "3.8"
proptest = "1.4"
```

### Phase Dependencies
- ✅ Phase 1.5: Vulkan Context (COMPLETE)
- ✅ Phase 1.6.1-1.6.3: Window, Surface, RenderPass (COMPLETE)
- ⚠️ GPU Timestamp Queries (requires Vulkan 1.1+, must validate device support)

---

## Risk Assessment

### Low Risk ✅
- Data structure design (similar to physics debugging)
- JSONL/SQLite export (proven approach)
- Query API (straightforward SQL queries)

### Medium Risk ⚠️
- GPU timestamp query availability (not all devices support)
- Frame capture performance (GPU → CPU transfer can be slow)
- Overdraw analysis (may require shader modifications)

### Mitigation Strategies
1. **GPU Timestamps:** Check device support, fall back to CPU timing if unavailable
2. **Frame Capture:** Use async transfer, double buffering, benchmark early
3. **Overdraw:** Make optional (nice-to-have, not critical)

---

## Comparison: Physics vs Rendering Debug

| Aspect | Physics Debug | Rendering Debug |
|--------|---------------|-----------------|
| **State Size** | Small (~1KB/frame) | Large (~10MB/frame with capture) |
| **Query Speed** | Fast | Moderate (large data) |
| **Visual Output** | Optional | Essential (frame capture) |
| **GPU Access** | N/A | Required (timestamp queries) |
| **Complexity** | Low | Medium |

**Key Difference:** Rendering debug generates MORE data, so careful attention to performance and storage optimization is critical.

---

## Next Steps After Completion

1. **Use during Phase 1.6.4-1.6.8:** Debug framebuffers, commands, sync, shaders
2. **Visual Regression Testing:** Integrate into CI pipeline
3. **Performance Optimization:** Use GPU timestamps to identify bottlenecks
4. **Extend to Phase 4:** Add PBR material debugging, lighting analysis

---

## Conclusion

Phase 1.6.R implements agentic rendering debug infrastructure following the proven physics debugging approach. This 3-week investment enables:

- **Autonomous debugging** by AI agents (30 seconds vs hours)
- **Historical analysis** without re-running simulations
- **Visual regression testing** in CI from day one
- **Performance validation** via GPU timestamp queries

By implementing debug infrastructure BEFORE completing rendering, any bugs in the remaining pipeline modules (framebuffers, commands, sync, shaders) will be trivial to diagnose and fix.

**Start Date:** TBD
**End Date:** TBD
**Status:** Ready to implement
