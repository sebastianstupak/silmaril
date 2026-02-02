//! Agentic Rendering Debug Infrastructure
//!
//! Machine-readable rendering debug infrastructure enabling AI agents to
//! autonomously debug rendering issues, detect resource leaks, analyze
//! performance, and perform visual regression testing.
//!
//! # Overview
//!
//! This module implements rendering debug infrastructure following the
//! physics agentic debugging approach (Phase A.0):
//!
//! - **Snapshot System (R.1)**: Complete render state capture per frame
//! - **Event Stream (R.2)**: Resource lifecycle and error tracking
//! - **Export Infrastructure (R.3)**: JSONL, SQLite, PNG exporters
//! - **Query API (R.4)**: High-level queries for AI agents
//! - **Frame Capture (R.5)**: Visual comparison and regression testing
//!
//! # Example: Resource Leak Detection
//!
//! ```no_run
//! use engine_renderer::debug::{RenderingQueryAPI};
//!
//! // Open exported debug database
//! let api = RenderingQueryAPI::open("rendering_debug.db")?;
//!
//! // Query for leaked resources
//! let leaks = api.find_leaked_resources()?;
//!
//! // AI agent analyzes results
//! for leak in leaks {
//!     println!("Leaked {}: {} bytes at frame {}",
//!         leak.resource_type, leak.memory_size, leak.created_frame);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Performance
//!
//! - Snapshot overhead: < 1ms for 1000 draw calls
//! - Frame capture: < 2ms overhead
//! - Export: Async, non-blocking
//! - Query latency: < 10ms per query

pub mod capture;
pub mod events;
pub mod exporters;
pub mod query;
pub mod snapshot;

// Re-export main types
pub use snapshot::{
    BufferInfo, DrawCallInfo, FramebufferInfo, GpuMemoryStats, QueueStateInfo, RenderDebugSnapshot,
    RenderTargetInfo, ShaderStageInfo, TextureInfo, ValidationError,
};

pub use capture::{
    Anomaly, CaptureError, DebugConfig, FrameCaptureData, FrameDiff, FrameMetadata, Rect,
    RenderingDebugger,
};
pub use events::{EventRecorder, RenderEvent};
pub use exporters::{ExportError, JsonlExporter, PngExporter, SqliteExporter};
pub use query::{
    BufferLifecycle, DatabaseStats, DrawCallError, ImageDiff, LeakedResource, QueryError,
    RenderingQueryAPI, ShaderError, SwapchainEvent, TextureLifecycle, Value,
};
