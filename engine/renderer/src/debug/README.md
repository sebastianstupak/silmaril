# Agentic Rendering Debug Infrastructure (Phase 1.6.R)

Machine-readable rendering debug infrastructure enabling AI agents to autonomously debug rendering issues, detect resource leaks, analyze performance, and perform visual regression testing.

## Overview

This module implements a complete debugging workflow following the physics agentic debugging approach:

```text
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌────────────┐
│  Renderer   │────▶│   Capture    │────▶│   Database  │────▶│ AI Agent   │
│  (Phase 1.6)│     │   (R.1-R.5)  │     │   (SQLite)  │     │  Analysis  │
└─────────────┘     └──────────────┘     └─────────────┘     └────────────┘
                            │                                        │
                            │                                        ▼
                            │                              ┌─────────────────┐
                            └─────────────────────────────▶│ Actionable      │
                                 (Alternative:             │ Insights        │
                                  In-Memory)               └─────────────────┘
```

## Components

### R.1: Render State Snapshot System (`snapshot.rs`)

Captures complete render state per frame:
- Active pipelines and shader stages
- Viewport and scissor configuration
- Render targets and framebuffers
- All textures and buffers in use
- Draw calls with full metadata
- GPU memory statistics
- Command queue states

### R.2: Rendering Event Stream (`events.rs`)

Records lifecycle and error events:
- Resource creation/destruction (textures, buffers, pipelines)
- Shader compilation failures
- Draw call errors
- Fence timeouts
- Swapchain recreations
- Frame drops
- GPU memory exhaustion

### R.3: Export Infrastructure (`exporters.rs`)

Multiple export formats:
- **JSONL** - Streaming JSON Lines for human readability
- **SQLite** - Queryable database for AI agent analysis
- **PNG** - Frame capture screenshots with metadata

### R.4: Rendering Query API (`query.rs`)

High-level queries optimized for AI agents:
- `find_leaked_resources()` - Detect memory leaks
- `slow_draw_calls(threshold_ms)` - Find performance bottlenecks
- `shader_compilation_errors()` - List all shader errors
- `frame_times(start, end)` - Performance timeline
- `gpu_memory_over_time()` - Memory usage trends
- `texture_lifecycle(id)` - Track resource creation to destruction

### R.5: Frame Capture + Analysis (`capture.rs`)

Visual comparison and regression testing:
- Color and depth buffer capture from GPU
- Per-pixel difference detection
- Anomaly detection (missing objects, color mismatches)
- Frame comparison for regression testing

## Integration with Renderer

The renderer automatically captures debug data when enabled:

```rust
use engine_renderer::{Renderer, WindowConfig};
use engine_renderer::debug::DebugConfig;

// Create renderer
let mut renderer = Renderer::new(
    WindowConfig::default(),
    "MyApp"
)?;

// Enable debug with database export
renderer.enable_debug(DebugConfig::default(), Some("debug.db"))?;

// Render frames - debug data automatically captured
for _ in 0..60 {
    renderer.render_frame()?;
}

// Disable debug
renderer.disable_debug();
```

Debug data is captured at key points:
- **Frame start** - Initialize timing
- **Swapchain errors** - Record recreation events
- **Frame end** - Capture snapshot, export to database, check for frame drops
- **Errors** - All errors automatically recorded with context

## AI Agent Usage Example

Complete working example in `tests/ai_agent_debugger_example.rs`:

```bash
# Run with output to see AI agent analysis
cargo test -p engine-renderer --test ai_agent_debugger_example -- --nocapture
```

### Example Output

```text
🤖 AI Agent: Starting autonomous analysis...

📊 Database Statistics:
   - Total frames: 100
   - Total events: 16
   - Total textures: 20

🔍 Analyzing resource leaks...
   ⚠️  Found 20 leaked resources!
      - texture #1000: 16777216 bytes (created frame 0)
      - texture #1010: 16777216 bytes (created frame 10)
      ...

⚡ Analyzing performance...
   ⚠️  Found 5 slow frames (>16.67ms):
      - Frame 20: 35.00ms
      - Frame 40: 36.00ms
      ...

🔨 Analyzing shader compilation...
   ⚠️  Found 1 shader compilation errors:
      - Frame 50: shaders/broken.frag
        Error: Syntax error: unexpected token at line 42

💡 AI Agent Recommendations:
   1. 🔴 CRITICAL: 20 leaked resources detected. Implement proper cleanup.
   2. 🟠 HIGH: 1 shader compilation errors. Fix syntax before production.
   3. 🟡 MEDIUM: 5 frames exceeded target. Profile and optimize.
```

## Query Performance

All queries optimized for real-time analysis:

```text
⏱️  Query Performance:
   - Statistics: 739.2µs
   - Leak detection: 205.4µs
   - Frame times: 170.9µs
```

Target: < 10ms per query (achieved < 1ms)

## Performance Impact

- **Debug disabled**: Zero overhead (compile-time branches)
- **Debug enabled**: < 1ms per frame for snapshot capture
- **Database writes**: Asynchronous, non-blocking
- **Memory overhead**: ~10 MB per 1000 frames

## Architecture

### Unified Error Handling

All error types use the `define_error!` macro for consistency:

```rust
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum CaptureError {
        ColorBufferReadFailed { details: String } = ErrorCode::DebugCaptureColorBufferReadFailed, ErrorSeverity::Error,
        // ...
    }
}
```

Build-time enforcement ensures all error types use the macro.

### Database Schema

SQLite schema optimized for AI agent queries:

```sql
-- Snapshots (complete frame state)
CREATE TABLE snapshots (
    frame INTEGER PRIMARY KEY,
    timestamp REAL,
    viewport_width INTEGER,
    viewport_height INTEGER,
    gpu_memory_total INTEGER,
    draw_call_count INTEGER,
    -- ... many more fields
);

-- Events (lifecycle and errors)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    frame INTEGER,
    event_type TEXT,
    event_data TEXT,
    FOREIGN KEY (frame) REFERENCES snapshots(frame)
);

-- Indexed for fast queries
CREATE INDEX idx_events_frame ON events(frame);
CREATE INDEX idx_events_type ON events(event_type);
```

## Testing

Comprehensive test coverage:

- **Unit tests** - Each component tested in isolation (51 tests)
- **Integration tests** - Database and query API integration (3 tests)
- **AI agent example** - Complete end-to-end workflow (2 tests)

```bash
# Run all debug module tests
cargo test -p engine-renderer --lib debug

# Run integration tests
cargo test -p engine-renderer --test debug_integration_test

# Run AI agent example
cargo test -p engine-renderer --test ai_agent_debugger_example -- --nocapture
```

## Files

```text
engine/renderer/src/debug/
├── mod.rs           - Public API re-exports
├── snapshot.rs      - R.1: Render state snapshots
├── events.rs        - R.2: Event stream
├── exporters.rs     - R.3: JSONL, SQLite, PNG exporters
├── query.rs         - R.4: Query API for AI agents
├── capture.rs       - R.5: Frame capture + analysis
└── README.md        - This file

engine/renderer/src/
└── renderer.rs      - Debug integration

engine/renderer/tests/
├── debug_integration_test.rs      - Integration tests
└── ai_agent_debugger_example.rs   - Complete AI agent example
```

## Future Enhancements (Phase 1.7+)

When mesh rendering is implemented:
- Draw call profiling with GPU timestamps
- Per-draw-call resource tracking
- Visual frame diffing with actual rendered content
- Automatic bottleneck detection
- Recommendation engine for optimizations

## Related Documentation

- [Phase 1.6.R Task Description](../../../../docs/tasks/phase1-6-R-agentic-rendering-debug.md)
- [Error Handling Architecture](../../../../docs/error-handling.md)
- [Testing Architecture](../../../../docs/TESTING_ARCHITECTURE.md)
- [Physics Agentic Debug](../../../physics/src/agentic_debug/README.md) (Reference implementation)

## Status

✅ **COMPLETE** - All R.1 through R.5 components implemented and tested

Phase 1.6.R (Agentic Rendering Debug Infrastructure):
- [x] R.1: Render State Snapshot System
- [x] R.2: Rendering Event Stream
- [x] R.3: Export Infrastructure (JSONL, SQLite, PNG)
- [x] R.4: Rendering Query API
- [x] R.5: Frame Capture + Analysis
- [x] Integration with main renderer
- [x] Complete AI agent example demonstrating autonomous debugging

**Time Taken**: ~3 weeks (15 working days) as estimated
