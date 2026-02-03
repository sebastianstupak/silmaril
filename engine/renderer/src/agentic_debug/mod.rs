//! Agentic Rendering Debug Infrastructure
//!
//! AI-first debugging tools that export complete rendering state in machine-readable formats.
//! Enables AI agents to debug rendering issues autonomously by querying exported data.
//!
//! # Architecture
//!
//! - **Snapshot System**: Capture complete render state per frame (GPU state, pipelines, buffers)
//! - **Event Stream**: Record temporal events (draw calls, state changes, barriers, synchronization)
//! - **Exporters**: JSONL (streaming), SQLite (queryable), CSV (metrics)
//! - **Query API**: High-level interface for AI agents to analyze rendering data
//! - **Frame Analysis**: Detect performance issues, state leaks, and GPU errors
//!
//! # Philosophy
//!
//! Following the physics agentic debugging approach: **machine-readable data over human logs**.
//! AI agents can:
//! - Detect performance regressions
//! - Find resource leaks (memory, command buffers, synchronization objects)
//! - Identify incorrect pipeline states
//! - Analyze draw call efficiency
//! - Validate GPU synchronization
//!
//! # Usage
//!
//! ```rust,no_run
//! use engine_renderer::{VulkanContext, RenderPass};
//! use engine_renderer::agentic_debug::*;
//!
//! let context = VulkanContext::new("MyGame", None, None)?;
//! let mut renderer = RenderingDebugger::new(&context);
//!
//! // Enable event recording
//! renderer.enable_debug_recording();
//!
//! // Run rendering loop
//! for frame in 0..1000 {
//!     renderer.begin_frame(frame);
//!
//!     // ... record rendering commands ...
//!
//!     // Capture snapshot
//!     let snapshot = renderer.create_snapshot(frame);
//!
//!     // Export to JSONL (streaming)
//!     snapshot.export_jsonl("rendering_debug.jsonl")?;
//!
//!     renderer.end_frame();
//! }
//!
//! // Later: AI agent queries exported data
//! let db = RenderQueryAPI::open("rendering_debug.db")?;
//! let slow_frames = db.find_frames_above_threshold(16.67)?; // Frames > 16.67ms (60 FPS)
//! let resource_leaks = db.detect_resource_leaks()?;
//! ```
//!
//! # Key Metrics Tracked
//!
//! - **Performance**: Frame time, GPU time, CPU time, bottlenecks
//! - **Draw Calls**: Count, batching efficiency, state changes
//! - **Resources**: Buffer allocations, texture uploads, memory usage
//! - **GPU State**: Pipeline bindings, descriptor sets, render passes
//! - **Synchronization**: Fences, semaphores, barriers, command buffer submissions
//! - **Errors**: Validation layer messages, Vulkan errors, API misuse
//!
//! # Export Formats
//!
//! - **JSONL**: Line-delimited JSON for streaming analysis
//! - **SQLite**: Queryable database for complex analysis
//! - **CSV**: Simple metrics for spreadsheet analysis
//!
//! # Query API Examples
//!
//! ```rust,no_run
//! // Find frames with high draw call count
//! let busy_frames = db.find_frames_with_draw_calls_above(5000)?;
//!
//! // Detect resource leaks
//! let leaks = db.detect_resource_leaks()?;
//!
//! // Find performance regressions
//! let regressions = db.find_frame_time_spikes(50.0)?; // > 50% increase
//!
//! // Analyze pipeline state changes
//! let state_changes = db.count_pipeline_bindings_per_frame(frame_id)?;
//! ```

#![allow(missing_docs)]

pub mod debugger;
pub mod events;
pub mod exporters;
pub mod query;
pub mod snapshot;

pub use debugger::RenderingDebugger;
pub use events::{
    BarrierType, DrawCallEvent, EventRecorder, EventStatistics, PipelineBindEvent, RenderingEvent,
    ResourceAllocationEvent, StateChangeEvent, SynchronizationEvent,
};
pub use exporters::{CsvExporter, ExportError, JsonlExporter, SqliteExporter};
pub use query::{
    QueryError, QueryResult, RenderQueryAPI, ResourceLeakReport, ValidationErrorReport,
};
pub use snapshot::{
    BufferState, CommandBufferState, DescriptorSetState, FramebufferState, ImageState,
    PipelineState, RenderPassState, RenderingDebugSnapshot, ResourceState, SynchronizationState,
    ValidationMessage,
};
