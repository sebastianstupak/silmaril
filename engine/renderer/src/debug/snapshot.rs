//! Render State Snapshot System (R.1)
//!
//! Complete render state capture for debugging and analysis by AI agents.
//!
//! # Overview
//!
//! This module implements frame-by-frame snapshots of the rendering state:
//! - Pipeline state (active shaders, viewport, scissor)
//! - Resources (textures, buffers, framebuffers)
//! - Draw calls with GPU timing
//! - Memory statistics
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::debug::RenderDebugSnapshot;
//!
//! // Create snapshot at end of frame
//! let snapshot = RenderDebugSnapshot::new(frame_number, timestamp);
//!
//! // Add draw call information
//! snapshot.add_draw_call(DrawCallInfo {
//!     draw_call_id: 0,
//!     mesh_id: 42,
//!     vertex_count: 1024,
//!     draw_time_gpu_ns: 50000,
//!     // ...
//! });
//!
//! // Export to JSON
//! let json = serde_json::to_string(&snapshot)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};

// Validation errors using define_error! macro
define_error! {
    /// Validation errors for rendering debug snapshots
    pub enum ValidationError {
        /// Invalid timestamp in debug snapshot
        InvalidTimestamp { timestamp: f64 } =
            ErrorCode::InvalidTimestamp,
            ErrorSeverity::Error,

        /// Invalid viewport dimensions
        InvalidViewport {} =
            ErrorCode::InvalidViewport,
            ErrorSeverity::Error,

        /// Invalid draw call data
        InvalidDrawCall { index: usize, message: String } =
            ErrorCode::InvalidDrawCall,
            ErrorSeverity::Error,

        /// Invalid transform matrix (contains NaN or Inf)
        InvalidTransform {} =
            ErrorCode::InvalidTransform,
            ErrorSeverity::Error,

        /// Draw call has zero vertices
        ZeroVertices {} =
            ErrorCode::ZeroVertices,
            ErrorSeverity::Error,
    }
}

/// Complete render state snapshot for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderDebugSnapshot {
    /// Frame number
    pub frame: u64,

    /// Timestamp in seconds since engine start
    pub timestamp: f64,

    // Pipeline state
    /// Currently active pipeline name (None if no active pipeline)
    pub active_pipeline: Option<String>,

    /// Active shader stages
    pub shader_stages: Vec<ShaderStageInfo>,

    /// Viewport configuration
    pub viewport: Viewport,

    /// Scissor rectangle
    pub scissor: Rect2D,

    /// Depth test enabled
    pub depth_test_enabled: bool,

    /// Blending enabled
    pub blend_enabled: bool,

    // Resources
    /// Active render targets
    pub render_targets: Vec<RenderTargetInfo>,

    /// Framebuffers in use
    pub framebuffers: Vec<FramebufferInfo>,

    /// Textures loaded
    pub textures: Vec<TextureInfo>,

    /// Buffers allocated
    pub buffers: Vec<BufferInfo>,

    // Draw calls
    /// All draw calls submitted this frame
    pub draw_calls: Vec<DrawCallInfo>,

    // GPU state
    /// GPU memory statistics
    pub gpu_memory: GpuMemoryStats,

    /// Queue states (graphics, compute, transfer)
    pub queue_states: Vec<QueueStateInfo>,
}

impl RenderDebugSnapshot {
    /// Create a new empty snapshot
    pub fn new(frame: u64, timestamp: f64) -> Self {
        Self {
            frame,
            timestamp,
            active_pipeline: None,
            shader_stages: Vec::new(),
            viewport: Viewport::default(),
            scissor: Rect2D::default(),
            depth_test_enabled: false,
            blend_enabled: false,
            render_targets: Vec::new(),
            framebuffers: Vec::new(),
            textures: Vec::new(),
            buffers: Vec::new(),
            draw_calls: Vec::new(),
            gpu_memory: GpuMemoryStats::default(),
            queue_states: Vec::new(),
        }
    }

    /// Validate snapshot for NaN/Inf values and invalid data
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check timestamp
        if !self.timestamp.is_finite() {
            return Err(ValidationError::invalidtimestamp(self.timestamp));
        }

        // Validate viewport
        if !self.viewport.width.is_finite() || !self.viewport.height.is_finite() {
            return Err(ValidationError::invalidviewport());
        }

        // Validate draw calls
        for (i, draw_call) in self.draw_calls.iter().enumerate() {
            draw_call.validate().map_err(|e: ValidationError| {
                ValidationError::invaliddrawcall(i, e.to_string())
            })?;
        }

        Ok(())
    }

    /// Get total number of vertices drawn this frame
    pub fn total_vertices(&self) -> u64 {
        self.draw_calls
            .iter()
            .map(|dc| dc.vertex_count as u64)
            .sum()
    }

    /// Get total number of triangles drawn this frame
    pub fn total_triangles(&self) -> u64 {
        self.draw_calls
            .iter()
            .map(|dc| {
                if dc.index_count > 0 {
                    (dc.index_count / 3) as u64
                } else {
                    (dc.vertex_count / 3) as u64
                }
            })
            .sum()
    }

    /// Get total GPU time for all draw calls (nanoseconds)
    pub fn total_gpu_time_ns(&self) -> u64 {
        self.draw_calls.iter().map(|dc| dc.draw_time_gpu_ns).sum()
    }

    /// Get total GPU time in milliseconds
    pub fn total_gpu_time_ms(&self) -> f64 {
        self.total_gpu_time_ns() as f64 / 1_000_000.0
    }

    /// Find draw calls that took longer than threshold (milliseconds)
    pub fn slow_draw_calls(&self, threshold_ms: f32) -> Vec<&DrawCallInfo> {
        let threshold_ns = (threshold_ms * 1_000_000.0) as u64;
        self.draw_calls
            .iter()
            .filter(|dc| dc.draw_time_gpu_ns > threshold_ns)
            .collect()
    }
}

/// Single draw call information with GPU profiling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCallInfo {
    /// Unique draw call ID within frame
    pub draw_call_id: u64,

    /// Mesh ID being rendered
    pub mesh_id: u64,

    /// Material ID being used
    pub material_id: u64,

    /// Pipeline ID being used
    pub pipeline_id: u64,

    /// Number of vertices
    pub vertex_count: u32,

    /// Number of indices (0 if non-indexed)
    pub index_count: u32,

    /// Number of instances
    pub instance_count: u32,

    /// Model-View-Projection transform (4x4 matrix, column-major)
    pub transform: [f32; 16],

    /// GPU timestamp: time taken to execute this draw call (nanoseconds)
    pub draw_time_gpu_ns: u64,

    /// Number of vertices processed by vertex shader
    pub vertices_processed: u64,

    /// Number of fragments processed by fragment shader
    pub fragments_processed: u64,
}

impl DrawCallInfo {
    /// Validate draw call data
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check for NaN/Inf in transform
        for &value in &self.transform {
            if !value.is_finite() {
                return Err(ValidationError::invalidtransform());
            }
        }

        // Check counts are reasonable
        if self.vertex_count == 0 {
            return Err(ValidationError::zerovertices());
        }

        Ok(())
    }

    /// Get draw time in milliseconds
    pub fn draw_time_ms(&self) -> f64 {
        self.draw_time_gpu_ns as f64 / 1_000_000.0
    }
}

/// Shader stage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderStageInfo {
    /// Shader stage (vertex, fragment, compute, etc.)
    pub stage: String,

    /// Shader module name or path
    pub module_name: String,

    /// Entry point function name
    pub entry_point: String,
}

/// Viewport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

/// Scissor rectangle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect2D {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for Rect2D {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        }
    }
}

/// Render target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderTargetInfo {
    pub target_id: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub sample_count: u32,
}

/// Framebuffer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramebufferInfo {
    pub framebuffer_id: u64,
    pub width: u32,
    pub height: u32,
    pub attachment_count: u32,
    pub has_depth: bool,
}

/// Texture resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureInfo {
    /// Unique texture ID
    pub texture_id: u64,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Format (e.g., "RGBA8", "BGRA8", "Depth32F")
    pub format: String,

    /// Number of mip levels
    pub mip_levels: u32,

    /// Memory size in bytes
    pub memory_size_bytes: usize,

    /// Usage flags (e.g., ["SAMPLED", "TRANSFER_DST"])
    pub usage_flags: Vec<String>,

    /// Frame when texture was created
    pub created_frame: u64,
}

/// Buffer resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferInfo {
    /// Unique buffer ID
    pub buffer_id: u64,

    /// Buffer size in bytes
    pub size_bytes: usize,

    /// Usage (e.g., "VERTEX", "INDEX", "UNIFORM")
    pub usage: String,

    /// Memory type (e.g., "DEVICE_LOCAL", "HOST_VISIBLE")
    pub memory_type: String,

    /// Frame when buffer was created
    pub created_frame: u64,
}

/// GPU memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuMemoryStats {
    /// Total allocated memory (bytes)
    pub total_allocated: usize,

    /// Memory used by textures
    pub textures: usize,

    /// Memory used by buffers
    pub buffers: usize,

    /// Memory used by framebuffers
    pub framebuffers: usize,

    /// Device-local memory (GPU VRAM)
    pub device_local: usize,

    /// Host-visible memory (CPU-GPU shared)
    pub host_visible: usize,
}

impl GpuMemoryStats {
    /// Create empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total memory in megabytes
    pub fn total_mb(&self) -> f64 {
        self.total_allocated as f64 / (1024.0 * 1024.0)
    }
}

/// Queue state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStateInfo {
    /// Queue family index
    pub queue_family: u32,

    /// Queue index within family
    pub queue_index: u32,

    /// Queue type (graphics, compute, transfer)
    pub queue_type: String,

    /// Number of pending submissions
    pub pending_submissions: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = RenderDebugSnapshot::new(42, 1.234);
        assert_eq!(snapshot.frame, 42);
        assert_eq!(snapshot.timestamp, 1.234);
        assert_eq!(snapshot.draw_calls.len(), 0);
    }

    #[test]
    fn test_snapshot_validation_valid() {
        let snapshot = RenderDebugSnapshot::new(1, 1.0);
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn test_snapshot_validation_invalid_timestamp() {
        let mut snapshot = RenderDebugSnapshot::new(1, f64::NAN);
        let result = snapshot.validate();
        assert!(result.is_err());
        // Check error code instead of variant match
        let err = result.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidTimestamp as u32);
    }

    #[test]
    fn test_draw_call_validation_valid() {
        let draw_call = DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        };
        assert!(draw_call.validate().is_ok());
    }

    #[test]
    fn test_draw_call_validation_invalid_transform() {
        let mut draw_call = DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [f32::NAN; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        };
        let result = draw_call.validate();
        assert!(result.is_err());
        // Check error code
        let err = result.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidTransform as u32);
    }

    #[test]
    fn test_draw_call_validation_zero_vertices() {
        let draw_call = DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 0,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 0,
            fragments_processed: 0,
        };
        let result = draw_call.validate();
        assert!(result.is_err());
        // Check error code
        let err = result.unwrap_err();
        assert_eq!(err.code(), ErrorCode::ZeroVertices as u32);
    }

    #[test]
    fn test_total_vertices() {
        let mut snapshot = RenderDebugSnapshot::new(1, 1.0);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        });
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 1,
            mesh_id: 2,
            material_id: 3,
            pipeline_id: 3,
            vertex_count: 200,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 100000,
            vertices_processed: 200,
            fragments_processed: 2000,
        });
        assert_eq!(snapshot.total_vertices(), 300);
    }

    #[test]
    fn test_total_gpu_time() {
        let mut snapshot = RenderDebugSnapshot::new(1, 1.0);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 1_000_000, // 1ms
            vertices_processed: 100,
            fragments_processed: 1000,
        });
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 1,
            mesh_id: 2,
            material_id: 3,
            pipeline_id: 3,
            vertex_count: 200,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 2_000_000, // 2ms
            vertices_processed: 200,
            fragments_processed: 2000,
        });
        assert_eq!(snapshot.total_gpu_time_ns(), 3_000_000);
        assert_eq!(snapshot.total_gpu_time_ms(), 3.0);
    }

    #[test]
    fn test_slow_draw_calls() {
        let mut snapshot = RenderDebugSnapshot::new(1, 1.0);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 500_000, // 0.5ms (fast)
            vertices_processed: 100,
            fragments_processed: 1000,
        });
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 1,
            mesh_id: 2,
            material_id: 3,
            pipeline_id: 3,
            vertex_count: 200,
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 5_000_000, // 5ms (slow)
            vertices_processed: 200,
            fragments_processed: 2000,
        });

        let slow = snapshot.slow_draw_calls(1.0); // > 1ms threshold
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].draw_call_id, 1);
    }

    #[test]
    fn test_gpu_memory_stats() {
        let stats = GpuMemoryStats {
            total_allocated: 10 * 1024 * 1024, // 10 MB
            textures: 5 * 1024 * 1024,
            buffers: 3 * 1024 * 1024,
            framebuffers: 2 * 1024 * 1024,
            device_local: 8 * 1024 * 1024,
            host_visible: 2 * 1024 * 1024,
        };
        assert_eq!(stats.total_mb(), 10.0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut snapshot = RenderDebugSnapshot::new(42, 1.234);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        });

        // Serialize to JSON
        let json = serde_json::to_string(&snapshot).unwrap();

        // Deserialize back
        let deserialized: RenderDebugSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.frame, snapshot.frame);
        assert_eq!(deserialized.timestamp, snapshot.timestamp);
        assert_eq!(deserialized.draw_calls.len(), snapshot.draw_calls.len());
        assert_eq!(
            deserialized.draw_calls[0].vertex_count,
            snapshot.draw_calls[0].vertex_count
        );
    }
}
