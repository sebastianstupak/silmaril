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

#![allow(missing_docs)] // Debug infrastructure - comprehensive docs not required

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};

define_error! {
    pub enum ValidationError {
        InvalidTimestamp { timestamp: f64 } = ErrorCode::InvalidTimestamp, ErrorSeverity::Error,
        InvalidViewport {} = ErrorCode::InvalidViewport, ErrorSeverity::Error,
        InvalidDrawCall { index: usize, message: String } = ErrorCode::InvalidDrawCall, ErrorSeverity::Error,
        InvalidTransform {} = ErrorCode::InvalidTransform, ErrorSeverity::Error,
        ZeroVertices {} = ErrorCode::ZeroVertices, ErrorSeverity::Error,
    }
}

// ============================================================================
// Core Snapshot Structure
// ============================================================================

/// Complete render debug snapshot for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderDebugSnapshot {
    /// Frame number
    pub frame: u64,

    /// Timestamp (seconds since start)
    pub timestamp: f64,

    /// Active graphics pipeline
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

    /// Active render targets
    pub render_targets: Vec<RenderTargetInfo>,

    /// Active framebuffers
    pub framebuffers: Vec<FramebufferInfo>,

    /// All textures in use this frame
    pub textures: Vec<TextureInfo>,

    /// All buffers in use this frame
    pub buffers: Vec<BufferInfo>,

    /// All draw calls executed this frame
    pub draw_calls: Vec<DrawCallInfo>,

    /// GPU memory statistics
    pub gpu_memory: GpuMemoryStats,

    /// Command queue states
    pub queue_states: Vec<QueueStateInfo>,
}

impl RenderDebugSnapshot {
    /// Create new snapshot for frame
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

    /// Validate snapshot data
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate timestamp
        if self.timestamp < 0.0 || !self.timestamp.is_finite() {
            return Err(ValidationError::invalidtimestamp(self.timestamp));
        }

        // Validate viewport
        if self.viewport.width == 0.0 || self.viewport.height == 0.0 {
            return Err(ValidationError::invalidviewport());
        }

        // Validate draw calls
        for (i, draw_call) in self.draw_calls.iter().enumerate() {
            if let Err(e) = draw_call.validate() {
                return Err(ValidationError::invaliddrawcall(i, e.to_string()));
            }
        }

        Ok(())
    }

    /// Total GPU time for all draw calls (nanoseconds)
    pub fn total_gpu_time_ns(&self) -> u64 {
        self.draw_calls.iter().map(|dc| dc.draw_time_gpu_ns).sum()
    }

    /// Total vertices across all draw calls
    pub fn total_vertices(&self) -> u64 {
        self.draw_calls
            .iter()
            .map(|dc| dc.vertex_count as u64 * dc.instance_count as u64)
            .sum()
    }

    /// Find draw calls slower than threshold (milliseconds)
    pub fn slow_draw_calls(&self, threshold_ms: f32) -> Vec<&DrawCallInfo> {
        let threshold_ns = (threshold_ms * 1_000_000.0) as u64;
        self.draw_calls.iter().filter(|dc| dc.draw_time_gpu_ns > threshold_ns).collect()
    }
}

// ============================================================================
// Shader Information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderStageInfo {
    pub stage: String, // "vertex", "fragment", "compute", etc.
    pub entry_point: String,
    pub shader_module_id: u64,
}

// ============================================================================
// Viewport & Scissor
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Rect2D {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// Render Target Information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderTargetInfo {
    pub attachment_index: u32,
    pub texture_id: u64,
    pub format: String,
    pub load_op: String,  // "clear", "load", "dont_care"
    pub store_op: String, // "store", "dont_care"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramebufferInfo {
    pub framebuffer_id: u64,
    pub width: u32,
    pub height: u32,
    pub attachment_count: u32,
}

// ============================================================================
// Texture Information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureInfo {
    pub texture_id: u64,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format: String,
    pub mip_levels: u32,
    pub sample_count: u32,
    pub memory_size: usize,
    pub created_frame: u64,
}

// ============================================================================
// Buffer Information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferInfo {
    pub buffer_id: u64,
    pub size_bytes: usize,
    pub usage: String,       // "vertex", "index", "uniform", "storage"
    pub memory_type: String, // "device_local", "host_visible", "host_cached"
}

// ============================================================================
// Draw Call Information
// ============================================================================

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

        // Validate vertex count
        if self.vertex_count == 0 {
            return Err(ValidationError::zerovertices());
        }

        Ok(())
    }
}

// ============================================================================
// GPU Memory Statistics
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
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
    /// Check if memory usage is within safe limits
    pub fn is_within_budget(&self, budget_bytes: usize) -> bool {
        self.total_allocated <= budget_bytes
    }

    /// Memory utilization percentage (0.0 to 1.0)
    pub fn utilization(&self, budget_bytes: usize) -> f32 {
        if budget_bytes == 0 {
            return 0.0;
        }
        (self.total_allocated as f32) / (budget_bytes as f32)
    }
}

// ============================================================================
// Command Queue State
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStateInfo {
    pub queue_family_index: u32,
    pub queue_index: u32,
    pub pending_commands: usize,
    pub last_submit_timestamp: f64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = RenderDebugSnapshot::new(1, 0.016);
        assert_eq!(snapshot.frame, 1);
        assert_eq!(snapshot.timestamp, 0.016);
        assert_eq!(snapshot.draw_calls.len(), 0);
    }

    #[test]
    fn test_snapshot_validation_valid() {
        let mut snapshot = RenderDebugSnapshot::new(1, 0.016);
        snapshot.viewport = Viewport {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn test_snapshot_validation_invalid_timestamp() {
        let mut snapshot = RenderDebugSnapshot::new(1, -1.0);
        snapshot.viewport.width = 1920.0;
        snapshot.viewport.height = 1080.0;

        let result = snapshot.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::InvalidTimestamp { .. }));
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
            fragments_processed: 5000,
        };

        assert!(draw_call.validate().is_ok());
    }

    #[test]
    fn test_draw_call_validation_zero_vertices() {
        let draw_call = DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 0, // Invalid!
            index_count: 0,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 0,
            fragments_processed: 0,
        };

        let result = draw_call.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::ZeroVertices { .. }));
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
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 5000,
        };

        draw_call.transform[0] = f32::NAN; // Invalid!

        let result = draw_call.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::InvalidTransform { .. }));
    }

    #[test]
    fn test_total_gpu_time() {
        let mut snapshot = RenderDebugSnapshot::new(1, 0.016);
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
            fragments_processed: 5000,
        });
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 1,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 200,
            index_count: 300,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 75000,
            vertices_processed: 200,
            fragments_processed: 10000,
        });

        assert_eq!(snapshot.total_gpu_time_ns(), 125000);
    }

    #[test]
    fn test_total_vertices() {
        let mut snapshot = RenderDebugSnapshot::new(1, 0.016);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 2, // Instanced!
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 200,
            fragments_processed: 5000,
        });

        assert_eq!(snapshot.total_vertices(), 200); // 100 * 2
    }

    #[test]
    fn test_slow_draw_calls() {
        let mut snapshot = RenderDebugSnapshot::new(1, 0.016);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000, // 0.05ms - fast
            vertices_processed: 100,
            fragments_processed: 5000,
        });
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 1,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 2_000_000, // 2ms - slow!
            vertices_processed: 100,
            fragments_processed: 5000,
        });

        let slow = snapshot.slow_draw_calls(1.0); // 1ms threshold
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].draw_call_id, 1);
    }

    #[test]
    fn test_gpu_memory_stats() {
        let stats = GpuMemoryStats {
            total_allocated: 500_000_000, // 500MB
            textures: 300_000_000,        // 300MB
            buffers: 150_000_000,         // 150MB
            framebuffers: 50_000_000,     // 50MB
            device_local: 500_000_000,
            host_visible: 0,
        };

        let budget = 1_000_000_000; // 1GB
        assert!(stats.is_within_budget(budget));
        assert_eq!(stats.utilization(budget), 0.5);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let snapshot = RenderDebugSnapshot::new(1, 0.016);
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: RenderDebugSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.frame, 1);
        assert_eq!(deserialized.timestamp, 0.016);
    }
}
