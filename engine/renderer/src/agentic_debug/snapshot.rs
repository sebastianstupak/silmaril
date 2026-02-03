//! Rendering State Snapshot System
//!
//! Captures complete GPU rendering state per frame for offline analysis by AI agents.

use serde::{Deserialize, Serialize};

/// Complete rendering state snapshot for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingDebugSnapshot {
    /// Frame number
    pub frame: u64,
    /// Timestamp (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Frame time (milliseconds)
    pub frame_time_ms: f64,
    /// GPU time (milliseconds, if available)
    pub gpu_time_ms: Option<f64>,
    /// CPU time (milliseconds)
    pub cpu_time_ms: f64,

    /// Render passes executed this frame
    pub render_passes: Vec<RenderPassState>,
    /// Command buffers recorded this frame
    pub command_buffers: Vec<CommandBufferState>,
    /// Pipeline states bound this frame
    pub pipelines: Vec<PipelineState>,
    /// Buffers allocated/used this frame
    pub buffers: Vec<BufferState>,
    /// Images/textures used this frame
    pub images: Vec<ImageState>,
    /// Descriptor sets bound this frame
    pub descriptor_sets: Vec<DescriptorSetState>,
    /// Framebuffers used this frame
    pub framebuffers: Vec<FramebufferState>,

    /// Synchronization state
    pub synchronization: SynchronizationState,
    /// Resource usage summary
    pub resources: ResourceState,

    /// Validation layer warnings/errors
    pub validation_messages: Vec<ValidationMessage>,
}

/// Render pass execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPassState {
    /// Render pass ID (for correlation)
    pub id: String,
    /// Format: "color-depth", "color-only", "depth-only"
    pub format: String,
    /// Number of subpasses
    pub subpass_count: u32,
    /// Color attachment count
    pub color_attachments: u32,
    /// Has depth attachment
    pub has_depth: bool,
    /// Has stencil attachment
    pub has_stencil: bool,
    /// Load operation: "clear", "load", "dont_care"
    pub load_op: String,
    /// Store operation: "store", "dont_care"
    pub store_op: String,
    /// Sample count (MSAA)
    pub samples: u32,
    /// Framebuffer width
    pub width: u32,
    /// Framebuffer height
    pub height: u32,
    /// Draw call count in this render pass
    pub draw_calls: u32,
    /// Time spent in this render pass (microseconds)
    pub duration_us: Option<f64>,
}

/// Command buffer recording state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBufferState {
    /// Command buffer ID (for correlation)
    pub id: String,
    /// Level: "primary", "secondary"
    pub level: String,
    /// State: "initial", "recording", "executable", "pending", "invalid"
    pub state: String,
    /// Draw call count
    pub draw_calls: u32,
    /// Compute dispatch count
    pub compute_dispatches: u32,
    /// Pipeline bind count
    pub pipeline_binds: u32,
    /// Descriptor set bind count
    pub descriptor_binds: u32,
    /// Buffer bind count
    pub buffer_binds: u32,
    /// Barrier count
    pub barriers: u32,
    /// Render pass begin/end count
    pub render_pass_count: u32,
    /// Recording time (microseconds)
    pub recording_time_us: Option<f64>,
}

/// Graphics pipeline state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Pipeline ID (for correlation)
    pub id: String,
    /// Type: "graphics", "compute"
    pub pipeline_type: String,
    /// Shader stages: ["vertex", "fragment"], ["compute"], etc.
    pub shader_stages: Vec<String>,
    /// Vertex input binding count
    pub vertex_bindings: u32,
    /// Descriptor set layout count
    pub descriptor_layouts: u32,
    /// Push constant ranges count
    pub push_constants: u32,
    /// Topology: "triangle_list", "triangle_strip", "point_list", etc.
    pub topology: String,
    /// Cull mode: "none", "front", "back", "front_and_back"
    pub cull_mode: String,
    /// Front face: "clockwise", "counter_clockwise"
    pub front_face: String,
    /// Polygon mode: "fill", "line", "point"
    pub polygon_mode: String,
    /// Depth test enabled
    pub depth_test: bool,
    /// Depth write enabled
    pub depth_write: bool,
    /// Depth compare op: "less", "less_or_equal", etc.
    pub depth_compare_op: Option<String>,
    /// Blend enabled
    pub blend_enabled: bool,
    /// Times bound this frame
    pub bind_count: u32,
}

/// Buffer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferState {
    /// Buffer ID (for correlation)
    pub id: String,
    /// Size in bytes
    pub size: u64,
    /// Usage flags: ["vertex", "index", "uniform", "storage", "transfer_src", etc.]
    pub usage: Vec<String>,
    /// Memory type: "device_local", "host_visible", "host_cached"
    pub memory_type: String,
    /// Currently mapped
    pub is_mapped: bool,
    /// Times bound this frame
    pub bind_count: u32,
}

/// Image/texture state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageState {
    /// Image ID (for correlation)
    pub id: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth (1 for 2D images)
    pub depth: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers (1 for non-array images)
    pub array_layers: u32,
    /// Format: "R8G8B8A8_SRGB", "D32_SFLOAT", etc.
    pub format: String,
    /// Usage flags: ["color_attachment", "depth_stencil_attachment", "sampled", etc.]
    pub usage: Vec<String>,
    /// Sample count (MSAA)
    pub samples: u32,
    /// Current layout: "undefined", "general", "color_attachment_optimal", etc.
    pub layout: String,
    /// Memory size in bytes
    pub memory_size: u64,
    /// Times bound this frame
    pub bind_count: u32,
}

/// Descriptor set state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptorSetState {
    /// Descriptor set ID (for correlation)
    pub id: String,
    /// Number of bindings
    pub binding_count: u32,
    /// Descriptor types: ["uniform_buffer", "sampled_image", "storage_buffer", etc.]
    pub descriptor_types: Vec<String>,
    /// Times bound this frame
    pub bind_count: u32,
}

/// Framebuffer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramebufferState {
    /// Framebuffer ID (for correlation)
    pub id: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Layers
    pub layers: u32,
    /// Attachment count
    pub attachments: u32,
    /// Compatible render pass ID
    pub render_pass_id: String,
    /// Times used this frame
    pub use_count: u32,
}

/// GPU synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchronizationState {
    /// Fence count (total alive)
    pub fence_count: u32,
    /// Fences signaled this frame
    pub fences_signaled: u32,
    /// Fences waited this frame
    pub fences_waited: u32,
    /// Fence wait time (milliseconds)
    pub fence_wait_time_ms: f64,

    /// Semaphore count (total alive)
    pub semaphore_count: u32,
    /// Semaphores signaled this frame
    pub semaphores_signaled: u32,
    /// Semaphores waited this frame
    pub semaphores_waited: u32,

    /// Pipeline barriers this frame
    pub pipeline_barriers: u32,
    /// Memory barriers this frame
    pub memory_barriers: u32,
    /// Image layout transitions this frame
    pub layout_transitions: u32,

    /// Command buffer submissions this frame
    pub queue_submissions: u32,
    /// Device idle waits this frame
    pub device_idle_waits: u32,
}

/// Resource usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    /// Total GPU memory allocated (bytes)
    pub gpu_memory_allocated: u64,
    /// GPU memory used (bytes)
    pub gpu_memory_used: u64,
    /// Host visible memory allocated (bytes)
    pub host_memory_allocated: u64,
    /// Host visible memory used (bytes)
    pub host_memory_used: u64,

    /// Buffer count (total alive)
    pub buffer_count: u32,
    /// Buffers allocated this frame
    pub buffers_allocated: u32,
    /// Buffers freed this frame
    pub buffers_freed: u32,

    /// Image count (total alive)
    pub image_count: u32,
    /// Images allocated this frame
    pub images_allocated: u32,
    /// Images freed this frame
    pub images_freed: u32,

    /// Pipeline count (total alive)
    pub pipeline_count: u32,
    /// Pipelines created this frame
    pub pipelines_created: u32,
    /// Pipelines destroyed this frame
    pub pipelines_destroyed: u32,

    /// Descriptor pool count
    pub descriptor_pool_count: u32,
    /// Descriptor sets allocated
    pub descriptor_set_count: u32,
}

/// Validation layer message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity: "error", "warning", "info", "verbose"
    pub severity: String,
    /// Message type: "general", "validation", "performance"
    pub message_type: String,
    /// Message ID (from Vulkan validation layers)
    pub message_id: i32,
    /// Message ID name (human-readable)
    pub message_id_name: String,
    /// Message text
    pub message: String,
    /// Timestamp (milliseconds since frame start)
    pub timestamp_ms: f64,
}

impl RenderingDebugSnapshot {
    /// Create a new empty snapshot
    pub fn new(frame: u64) -> Self {
        Self {
            frame,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            frame_time_ms: 0.0,
            gpu_time_ms: None,
            cpu_time_ms: 0.0,
            render_passes: Vec::new(),
            command_buffers: Vec::new(),
            pipelines: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
            descriptor_sets: Vec::new(),
            framebuffers: Vec::new(),
            synchronization: SynchronizationState::default(),
            resources: ResourceState::default(),
            validation_messages: Vec::new(),
        }
    }

    /// Calculate performance statistics
    pub fn performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            fps: if self.frame_time_ms > 0.0 { 1000.0 / self.frame_time_ms } else { 0.0 },
            frame_time_ms: self.frame_time_ms,
            gpu_time_ms: self.gpu_time_ms,
            cpu_time_ms: self.cpu_time_ms,
            draw_calls: self.command_buffers.iter().map(|cb| cb.draw_calls).sum(),
            pipeline_binds: self.command_buffers.iter().map(|cb| cb.pipeline_binds).sum(),
            barriers: self.command_buffers.iter().map(|cb| cb.barriers).sum(),
        }
    }

    /// Detect potential issues
    pub fn detect_issues(&self) -> Vec<RenderingIssue> {
        let mut issues = Vec::new();

        // Check frame time
        if self.frame_time_ms > 16.67 {
            issues.push(RenderingIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Performance,
                message: format!(
                    "Frame time {}ms exceeds 60 FPS target (16.67ms)",
                    self.frame_time_ms
                ),
            });
        }

        // Check draw call efficiency
        let total_draws: u32 = self.command_buffers.iter().map(|cb| cb.draw_calls).sum();
        if total_draws > 5000 {
            issues.push(RenderingIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Performance,
                message: format!("High draw call count: {} (consider batching)", total_draws),
            });
        }

        // Check pipeline binds
        let total_binds: u32 = self.command_buffers.iter().map(|cb| cb.pipeline_binds).sum();
        if total_binds > 1000 {
            issues.push(RenderingIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Performance,
                message: format!("High pipeline bind count: {} (state thrashing)", total_binds),
            });
        }

        // Check memory usage
        if self.resources.gpu_memory_used > self.resources.gpu_memory_allocated {
            issues.push(RenderingIssue {
                severity: IssueSeverity::Error,
                category: IssueCategory::Memory,
                message: "GPU memory usage exceeds allocation (memory corruption?)".to_string(),
            });
        }

        // Check validation messages
        for msg in &self.validation_messages {
            if msg.severity == "error" {
                issues.push(RenderingIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Validation,
                    message: format!("Vulkan validation error: {}", msg.message),
                });
            }
        }

        issues
    }
}

/// Performance statistics derived from snapshot
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub gpu_time_ms: Option<f64>,
    pub cpu_time_ms: f64,
    pub draw_calls: u32,
    pub pipeline_binds: u32,
    pub barriers: u32,
}

/// Detected rendering issue
#[derive(Debug, Clone)]
pub struct RenderingIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueCategory {
    Performance,
    Memory,
    Validation,
    Synchronization,
    State,
}

impl Default for SynchronizationState {
    fn default() -> Self {
        Self {
            fence_count: 0,
            fences_signaled: 0,
            fences_waited: 0,
            fence_wait_time_ms: 0.0,
            semaphore_count: 0,
            semaphores_signaled: 0,
            semaphores_waited: 0,
            pipeline_barriers: 0,
            memory_barriers: 0,
            layout_transitions: 0,
            queue_submissions: 0,
            device_idle_waits: 0,
        }
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self {
            gpu_memory_allocated: 0,
            gpu_memory_used: 0,
            host_memory_allocated: 0,
            host_memory_used: 0,
            buffer_count: 0,
            buffers_allocated: 0,
            buffers_freed: 0,
            image_count: 0,
            images_allocated: 0,
            images_freed: 0,
            pipeline_count: 0,
            pipelines_created: 0,
            pipelines_destroyed: 0,
            descriptor_pool_count: 0,
            descriptor_set_count: 0,
        }
    }
}
