//! High-level rendering debugger API for E2E debugging
//!
//! Provides a simple interface for enabling rendering debug recording,
//! capturing snapshots, and exporting data for AI agent analysis.

use super::events::{
    DrawCallEvent, EventRecorder, PipelineBindEvent, ResourceAllocationEvent, StateChangeEvent,
    SynchronizationEvent,
};
use super::exporters::{CsvExporter, ExportError, JsonlExporter, SqliteExporter};
use super::snapshot::{
    BufferState, CommandBufferState, ImageState, PipelineState, RenderPassState,
    RenderingDebugSnapshot, ResourceState, SynchronizationState, ValidationMessage,
};
use std::path::Path;
use std::time::Instant;

/// High-level rendering debugger
///
/// Wraps rendering operations and records debug data for offline analysis by AI agents.
///
/// # Example
///
/// ```no_run
/// use engine_renderer::agentic_debug::RenderingDebugger;
///
/// let mut debugger = RenderingDebugger::new();
/// debugger.enable_debug_recording();
///
/// for frame in 0..1000 {
///     debugger.begin_frame(frame);
///
///     // ... render scene ...
///     debugger.record_draw_call("main_pass", "mesh_pipeline", 36, 1);
///
///     let snapshot = debugger.create_snapshot();
///     snapshot.export_jsonl("debug.jsonl").ok();
///
///     debugger.end_frame();
/// }
/// ```
pub struct RenderingDebugger {
    /// Event recorder
    recorder: EventRecorder,
    /// Debug recording enabled
    enabled: bool,
    /// Current frame number
    current_frame: u64,
    /// Frame start time
    frame_start: Instant,
    /// Frame end time
    frame_end: Option<Instant>,

    // Accumulated state for current frame
    render_passes: Vec<RenderPassState>,
    command_buffers: Vec<CommandBufferState>,
    pipelines: Vec<PipelineState>,
    buffers: Vec<BufferState>,
    images: Vec<ImageState>,
    synchronization: SynchronizationState,
    resources: ResourceState,
    validation_messages: Vec<ValidationMessage>,
}

impl RenderingDebugger {
    /// Create a new rendering debugger
    pub fn new() -> Self {
        Self {
            recorder: EventRecorder::new(),
            enabled: false,
            current_frame: 0,
            frame_start: Instant::now(),
            frame_end: None,
            render_passes: Vec::new(),
            command_buffers: Vec::new(),
            pipelines: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
            synchronization: SynchronizationState::default(),
            resources: ResourceState::default(),
            validation_messages: Vec::new(),
        }
    }

    /// Enable debug recording
    pub fn enable_debug_recording(&mut self) {
        self.enabled = true;
    }

    /// Disable debug recording
    pub fn disable_debug_recording(&mut self) {
        self.enabled = false;
    }

    /// Check if debug recording is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self, frame: u64) {
        if !self.enabled {
            return;
        }

        self.current_frame = frame;
        self.frame_start = Instant::now();
        self.frame_end = None;

        // Clear frame state
        self.render_passes.clear();
        self.command_buffers.clear();
        self.pipelines.clear();
        self.buffers.clear();
        self.images.clear();
        self.validation_messages.clear();

        // Reset frame counters
        self.synchronization.fences_signaled = 0;
        self.synchronization.fences_waited = 0;
        self.synchronization.semaphores_signaled = 0;
        self.synchronization.semaphores_waited = 0;
        self.synchronization.pipeline_barriers = 0;
        self.synchronization.memory_barriers = 0;
        self.synchronization.layout_transitions = 0;
        self.synchronization.queue_submissions = 0;
        self.synchronization.device_idle_waits = 0;

        self.resources.buffers_allocated = 0;
        self.resources.buffers_freed = 0;
        self.resources.images_allocated = 0;
        self.resources.images_freed = 0;
        self.resources.pipelines_created = 0;
        self.resources.pipelines_destroyed = 0;

        self.recorder.begin_frame(frame);
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_end = Some(Instant::now());
        self.recorder.end_frame();
    }

    /// Get or create command buffer state
    fn get_or_create_command_buffer(&mut self, command_buffer_id: &str) -> &mut CommandBufferState {
        if let Some(idx) = self.command_buffers.iter().position(|cb| cb.id == command_buffer_id) {
            &mut self.command_buffers[idx]
        } else {
            self.command_buffers.push(CommandBufferState {
                id: command_buffer_id.to_string(),
                level: "primary".to_string(),
                state: "recording".to_string(),
                draw_calls: 0,
                compute_dispatches: 0,
                pipeline_binds: 0,
                descriptor_binds: 0,
                buffer_binds: 0,
                barriers: 0,
                render_pass_count: 0,
                recording_time_us: None,
            });
            self.command_buffers.last_mut().unwrap()
        }
    }

    /// Record a draw call
    pub fn record_draw_call(
        &mut self,
        command_buffer_id: &str,
        pipeline_id: &str,
        vertex_count: u32,
        instance_count: u32,
    ) {
        if !self.enabled {
            return;
        }

        // Track command buffer state
        let cmd_buf = self.get_or_create_command_buffer(command_buffer_id);
        cmd_buf.draw_calls += 1;

        self.recorder.record_draw_call(DrawCallEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            command_buffer_id: command_buffer_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            draw_type: "draw".to_string(),
            vertex_count,
            instance_count,
            first_vertex: 0,
            vertex_offset: None,
        });
    }

    /// Record an indexed draw call
    pub fn record_draw_indexed(
        &mut self,
        command_buffer_id: &str,
        pipeline_id: &str,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) {
        if !self.enabled {
            return;
        }

        // Track command buffer state
        let cmd_buf = self.get_or_create_command_buffer(command_buffer_id);
        cmd_buf.draw_calls += 1;

        self.recorder.record_draw_call(DrawCallEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            command_buffer_id: command_buffer_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            draw_type: "draw_indexed".to_string(),
            vertex_count: index_count,
            instance_count,
            first_vertex: first_index,
            vertex_offset: Some(vertex_offset),
        });
    }

    /// Record a pipeline bind
    pub fn record_pipeline_bind(
        &mut self,
        command_buffer_id: &str,
        pipeline_id: &str,
        bind_point: &str,
    ) {
        if !self.enabled {
            return;
        }

        // Track command buffer state
        let cmd_buf = self.get_or_create_command_buffer(command_buffer_id);
        cmd_buf.pipeline_binds += 1;

        self.recorder.record_pipeline_bind(PipelineBindEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            command_buffer_id: command_buffer_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            bind_point: bind_point.to_string(),
        });
    }

    /// Record a state change
    pub fn record_state_change(&mut self, command_buffer_id: &str, state_type: &str, value: &str) {
        if !self.enabled {
            return;
        }

        self.recorder.record_state_change(StateChangeEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            command_buffer_id: command_buffer_id.to_string(),
            state_type: state_type.to_string(),
            value: value.to_string(),
        });
    }

    /// Record a buffer allocation
    pub fn record_buffer_allocation(&mut self, buffer_id: &str, size_bytes: u64) {
        if !self.enabled {
            return;
        }

        self.resources.buffers_allocated += 1;
        self.resources.buffer_count += 1;

        self.recorder.record_resource_allocation(ResourceAllocationEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            operation: "allocate".to_string(),
            resource_type: "buffer".to_string(),
            resource_id: buffer_id.to_string(),
            size_bytes: Some(size_bytes),
        });
    }

    /// Record a buffer deallocation
    pub fn record_buffer_free(&mut self, buffer_id: &str) {
        if !self.enabled {
            return;
        }

        self.resources.buffers_freed += 1;
        self.resources.buffer_count = self.resources.buffer_count.saturating_sub(1);

        self.recorder.record_resource_allocation(ResourceAllocationEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            operation: "free".to_string(),
            resource_type: "buffer".to_string(),
            resource_id: buffer_id.to_string(),
            size_bytes: None,
        });
    }

    /// Record an image allocation
    pub fn record_image_allocation(&mut self, image_id: &str, size_bytes: u64) {
        if !self.enabled {
            return;
        }

        self.resources.images_allocated += 1;
        self.resources.image_count += 1;

        self.recorder.record_resource_allocation(ResourceAllocationEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            operation: "allocate".to_string(),
            resource_type: "image".to_string(),
            resource_id: image_id.to_string(),
            size_bytes: Some(size_bytes),
        });
    }

    /// Record an image deallocation
    pub fn record_image_free(&mut self, image_id: &str) {
        if !self.enabled {
            return;
        }

        self.resources.images_freed += 1;
        self.resources.image_count = self.resources.image_count.saturating_sub(1);

        self.recorder.record_resource_allocation(ResourceAllocationEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            operation: "free".to_string(),
            resource_type: "image".to_string(),
            resource_id: image_id.to_string(),
            size_bytes: None,
        });
    }

    /// Record a fence wait
    pub fn record_fence_wait(&mut self, fence_id: &str, wait_time_ms: f64) {
        if !self.enabled {
            return;
        }

        self.synchronization.fences_waited += 1;
        self.synchronization.fence_wait_time_ms += wait_time_ms;

        self.recorder.record_synchronization(SynchronizationEvent {
            frame: self.current_frame,
            timestamp_us: self.recorder.current_timestamp_us(),
            command_buffer_id: None,
            sync_type: "fence_wait".to_string(),
            sync_object_id: Some(fence_id.to_string()),
            barrier_type: None,
            wait_time_ms: Some(wait_time_ms),
        });
    }

    /// Record a queue submission
    pub fn record_queue_submit(&mut self) {
        if !self.enabled {
            return;
        }

        self.synchronization.queue_submissions += 1;
    }

    /// Record a validation message
    pub fn record_validation_message(
        &mut self,
        severity: &str,
        message_type: &str,
        message_id: i32,
        message_id_name: &str,
        message: &str,
    ) {
        if !self.enabled {
            return;
        }

        self.validation_messages.push(ValidationMessage {
            severity: severity.to_string(),
            message_type: message_type.to_string(),
            message_id,
            message_id_name: message_id_name.to_string(),
            message: message.to_string(),
            timestamp_ms: self.frame_start.elapsed().as_millis() as f64,
        });
    }

    /// Create a snapshot of the current frame state
    pub fn create_snapshot(&self) -> RenderingDebugSnapshot {
        let frame_time_ms = if let Some(end) = self.frame_end {
            end.duration_since(self.frame_start).as_secs_f64() * 1000.0
        } else {
            self.frame_start.elapsed().as_secs_f64() * 1000.0
        };

        RenderingDebugSnapshot {
            frame: self.current_frame,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            frame_time_ms,
            gpu_time_ms: None,          // TODO: Add GPU timing queries
            cpu_time_ms: frame_time_ms, // For now, assume CPU = frame time
            render_passes: self.render_passes.clone(),
            command_buffers: self.command_buffers.clone(),
            pipelines: self.pipelines.clone(),
            buffers: self.buffers.clone(),
            images: self.images.clone(),
            descriptor_sets: Vec::new(), // TODO: Track descriptor sets
            framebuffers: Vec::new(),    // TODO: Track framebuffers
            synchronization: self.synchronization.clone(),
            resources: self.resources.clone(),
            validation_messages: self.validation_messages.clone(),
        }
    }

    /// Export current frame snapshot to JSONL
    pub fn export_jsonl<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        let snapshot = self.create_snapshot();
        let exporter = JsonlExporter::new(path);
        exporter.export(&snapshot)
    }

    /// Export current frame snapshot to SQLite
    pub fn export_sqlite(&self, exporter: &mut SqliteExporter) -> Result<(), ExportError> {
        let snapshot = self.create_snapshot();
        exporter.export(&snapshot)
    }

    /// Export current frame snapshot to CSV
    pub fn export_csv(&self, exporter: &mut CsvExporter) -> Result<(), ExportError> {
        let snapshot = self.create_snapshot();
        exporter.export(&snapshot)
    }

    /// Get event statistics
    pub fn event_statistics(&self) -> super::events::EventStatistics {
        self.recorder.statistics()
    }

    /// Get current frame number
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }
}

impl Default for RenderingDebugger {
    fn default() -> Self {
        Self::new()
    }
}
