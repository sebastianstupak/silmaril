//! Rendering Event Stream
//!
//! Records temporal rendering events for offline analysis.
//! AI agents can analyze event sequences to detect performance issues,
//! incorrect state transitions, and synchronization problems.

use serde::{Deserialize, Serialize};

/// Rendering event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RenderingEvent {
    /// Draw call executed
    DrawCall(DrawCallEvent),
    /// Pipeline bound
    PipelineBind(PipelineBindEvent),
    /// State change (viewport, scissor, blend, etc.)
    StateChange(StateChangeEvent),
    /// Resource allocation/deallocation
    ResourceAllocation(ResourceAllocationEvent),
    /// Synchronization event (fence, semaphore, barrier)
    Synchronization(SynchronizationEvent),
}

/// Draw call event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCallEvent {
    /// Frame number
    pub frame: u64,
    /// Timestamp (microseconds since frame start)
    pub timestamp_us: f64,
    /// Command buffer ID
    pub command_buffer_id: String,
    /// Pipeline ID (for correlation)
    pub pipeline_id: String,
    /// Draw type: "draw", "draw_indexed", "draw_indirect", "draw_indexed_indirect"
    pub draw_type: String,
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex/index
    pub first_vertex: u32,
    /// Vertex offset (for indexed draws)
    pub vertex_offset: Option<i32>,
}

/// Pipeline bind event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineBindEvent {
    /// Frame number
    pub frame: u64,
    /// Timestamp (microseconds since frame start)
    pub timestamp_us: f64,
    /// Command buffer ID
    pub command_buffer_id: String,
    /// Pipeline ID
    pub pipeline_id: String,
    /// Bind point: "graphics", "compute"
    pub bind_point: String,
}

/// State change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    /// Frame number
    pub frame: u64,
    /// Timestamp (microseconds since frame start)
    pub timestamp_us: f64,
    /// Command buffer ID
    pub command_buffer_id: String,
    /// State type: "viewport", "scissor", "line_width", "blend_constants", etc.
    pub state_type: String,
    /// State value (JSON serialized)
    pub value: String,
}

/// Resource allocation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationEvent {
    /// Frame number
    pub frame: u64,
    /// Timestamp (microseconds since frame start)
    pub timestamp_us: f64,
    /// Operation: "allocate", "free"
    pub operation: String,
    /// Resource type: "buffer", "image", "pipeline", "descriptor_set", etc.
    pub resource_type: String,
    /// Resource ID
    pub resource_id: String,
    /// Size in bytes (for buffers/images)
    pub size_bytes: Option<u64>,
}

/// Synchronization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchronizationEvent {
    /// Frame number
    pub frame: u64,
    /// Timestamp (microseconds since frame start)
    pub timestamp_us: f64,
    /// Command buffer ID (if applicable)
    pub command_buffer_id: Option<String>,
    /// Sync type: "fence_wait", "fence_signal", "semaphore_wait", "semaphore_signal", "barrier"
    pub sync_type: String,
    /// Sync object ID (fence/semaphore ID)
    pub sync_object_id: Option<String>,
    /// Barrier type (if applicable)
    pub barrier_type: Option<BarrierType>,
    /// Wait time (milliseconds, for fence waits)
    pub wait_time_ms: Option<f64>,
}

/// Pipeline barrier type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierType {
    /// Barrier kind: "memory", "buffer", "image"
    pub kind: String,
    /// Source stage: "transfer", "fragment_shader", "compute_shader", etc.
    pub src_stage: String,
    /// Destination stage
    pub dst_stage: String,
    /// Access flags: ["transfer_write", "shader_read", etc.]
    pub access_flags: Vec<String>,
}

/// Event recorder
pub struct EventRecorder {
    /// All recorded events
    events: Vec<RenderingEvent>,
    /// Current frame
    current_frame: u64,
    /// Frame start time (for relative timestamps)
    frame_start: std::time::Instant,
}

impl EventRecorder {
    /// Create a new event recorder
    pub fn new() -> Self {
        Self { events: Vec::new(), current_frame: 0, frame_start: std::time::Instant::now() }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self, frame: u64) {
        self.current_frame = frame;
        self.frame_start = std::time::Instant::now();
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        // Nothing to do (could add frame end event if needed)
    }

    /// Record a draw call
    pub fn record_draw_call(&mut self, event: DrawCallEvent) {
        self.events.push(RenderingEvent::DrawCall(event));
    }

    /// Record a pipeline bind
    pub fn record_pipeline_bind(&mut self, event: PipelineBindEvent) {
        self.events.push(RenderingEvent::PipelineBind(event));
    }

    /// Record a state change
    pub fn record_state_change(&mut self, event: StateChangeEvent) {
        self.events.push(RenderingEvent::StateChange(event));
    }

    /// Record a resource allocation
    pub fn record_resource_allocation(&mut self, event: ResourceAllocationEvent) {
        self.events.push(RenderingEvent::ResourceAllocation(event));
    }

    /// Record a synchronization event
    pub fn record_synchronization(&mut self, event: SynchronizationEvent) {
        self.events.push(RenderingEvent::Synchronization(event));
    }

    /// Get all recorded events
    pub fn events(&self) -> &[RenderingEvent] {
        &self.events
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get current timestamp (microseconds since frame start)
    pub fn current_timestamp_us(&self) -> f64 {
        self.frame_start.elapsed().as_micros() as f64
    }

    /// Get event statistics
    pub fn statistics(&self) -> EventStatistics {
        let mut stats = EventStatistics::default();

        for event in &self.events {
            match event {
                RenderingEvent::DrawCall(_) => stats.draw_calls += 1,
                RenderingEvent::PipelineBind(_) => stats.pipeline_binds += 1,
                RenderingEvent::StateChange(_) => stats.state_changes += 1,
                RenderingEvent::ResourceAllocation(e) => {
                    if e.operation == "allocate" {
                        stats.resource_allocations += 1;
                    } else {
                        stats.resource_deallocations += 1;
                    }
                }
                RenderingEvent::Synchronization(_) => stats.synchronization_events += 1,
            }
        }

        stats.total_events = self.events.len();
        stats
    }
}

impl Default for EventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Event statistics summary
#[derive(Debug, Clone, Default)]
pub struct EventStatistics {
    pub total_events: usize,
    pub draw_calls: usize,
    pub pipeline_binds: usize,
    pub state_changes: usize,
    pub resource_allocations: usize,
    pub resource_deallocations: usize,
    pub synchronization_events: usize,
}

impl EventStatistics {
    /// Calculate events per frame
    pub fn events_per_frame(&self, frame_count: u64) -> f64 {
        if frame_count == 0 {
            0.0
        } else {
            self.total_events as f64 / frame_count as f64
        }
    }

    /// Calculate draw calls per frame
    pub fn draw_calls_per_frame(&self, frame_count: u64) -> f64 {
        if frame_count == 0 {
            0.0
        } else {
            self.draw_calls as f64 / frame_count as f64
        }
    }
}
