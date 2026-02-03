//! Bounding Box Visualization
//!
//! Renders axis-aligned bounding boxes (AABB) and oriented bounding boxes (OBB)
//! for debugging collision volumes and spatial queries.
//!
//! # Features
//! - AABB rendering (axis-aligned boxes)
//! - OBB rendering (oriented boxes)
//! - Configurable colors per box
//! - Batch rendering for performance
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::debug::BoundingBoxRenderer;
//!
//! # let context = todo!();
//! let mut renderer = BoundingBoxRenderer::new(&context)?;
//!
//! // Add AABB to render
//! renderer.add_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 0.0, 1.0]);
//!
//! // Render all boxes
//! renderer.render()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{RendererError, VulkanContext};
use ash::vk;
use std::sync::Arc;
use tracing::debug;

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    /// Minimum corner (x, y, z)
    pub min: [f32; 3],

    /// Maximum corner (x, y, z)
    pub max: [f32; 3],

    /// Render color (RGBA, 0.0-1.0)
    pub color: [f32; 4],
}

impl Aabb {
    /// Create AABB from min/max corners
    pub fn new(min: [f32; 3], max: [f32; 3], color: [f32; 4]) -> Self {
        Self { min, max, color }
    }

    /// Get center point
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Get extents (half-sizes)
    pub fn extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }
}

/// Oriented bounding box
#[derive(Debug, Clone, Copy)]
pub struct Obb {
    /// Center position (x, y, z)
    pub center: [f32; 3],

    /// Half-extents (width, height, depth)
    pub extents: [f32; 3],

    /// Rotation quaternion (x, y, z, w)
    pub rotation: [f32; 4],

    /// Render color (RGBA, 0.0-1.0)
    pub color: [f32; 4],
}

impl Obb {
    /// Create OBB from center, extents, and rotation
    pub fn new(
        center: [f32; 3],
        extents: [f32; 3],
        rotation: [f32; 4],
        color: [f32; 4],
    ) -> Self {
        Self { center, extents, rotation, color }
    }
}

/// Bounding box renderer configuration
#[derive(Debug, Clone)]
pub struct BoundsConfig {
    /// Enable rendering
    pub enabled: bool,

    /// Line width for box edges
    pub line_width: f32,

    /// Default AABB color
    pub default_aabb_color: [f32; 4],

    /// Default OBB color
    pub default_obb_color: [f32; 4],
}

impl Default for BoundsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            line_width: 1.0,
            default_aabb_color: [0.0, 1.0, 0.0, 1.0], // Green
            default_obb_color: [0.0, 0.0, 1.0, 1.0],  // Blue
        }
    }
}

/// Bounding box renderer
///
/// Batches and renders AABBs and OBBs as colored wireframe boxes.
pub struct BoundingBoxRenderer {
    #[allow(dead_code)]
    context: Arc<VulkanContext>,

    #[allow(dead_code)]
    config: BoundsConfig,

    /// AABBs to render this frame
    aabbs: Vec<Aabb>,

    /// OBBs to render this frame
    obbs: Vec<Obb>,

    /// Vertex buffer for box geometry
    #[allow(dead_code)]
    vertex_buffer: Option<vk::Buffer>,

    /// Index buffer for box edges
    #[allow(dead_code)]
    index_buffer: Option<vk::Buffer>,

    /// Pipeline for rendering
    #[allow(dead_code)]
    pipeline: Option<vk::Pipeline>,

    initialized: bool,
}

impl BoundingBoxRenderer {
    /// Create bounding box renderer
    pub fn new(context: Arc<VulkanContext>) -> Result<Self, RendererError> {
        Ok(Self {
            context,
            config: BoundsConfig::default(),
            aabbs: Vec::new(),
            obbs: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            pipeline: None,
            initialized: false,
        })
    }

    /// Create with custom config
    pub fn with_config(
        context: Arc<VulkanContext>,
        config: BoundsConfig,
    ) -> Result<Self, RendererError> {
        let mut renderer = Self::new(context)?;
        let enabled = config.enabled;
        renderer.config = config;

        if enabled {
            renderer.initialize()?;
        }

        Ok(renderer)
    }

    /// Initialize renderer resources
    fn initialize(&mut self) -> Result<(), RendererError> {
        if self.initialized {
            return Ok(());
        }

        // TODO: Create buffers and pipeline
        // - Vertex buffer for cube vertices (8 corners)
        // - Index buffer for 12 edges
        // - Pipeline for line rendering with instancing
        // - Per-instance data: transform, color

        self.initialized = true;
        Ok(())
    }

    /// Add AABB to render queue
    ///
    /// AABBs are cleared after each render() call.
    pub fn add_aabb(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 4]) {
        self.aabbs.push(Aabb::new(min, max, color));
    }

    /// Add AABB with default color
    pub fn add_aabb_default(&mut self, min: [f32; 3], max: [f32; 3]) {
        self.add_aabb(min, max, self.config.default_aabb_color);
    }

    /// Add OBB to render queue
    pub fn add_obb(
        &mut self,
        center: [f32; 3],
        extents: [f32; 3],
        rotation: [f32; 4],
        color: [f32; 4],
    ) {
        self.obbs.push(Obb::new(center, extents, rotation, color));
    }

    /// Add OBB with default color
    pub fn add_obb_default(
        &mut self,
        center: [f32; 3],
        extents: [f32; 3],
        rotation: [f32; 4],
    ) {
        self.add_obb(center, extents, rotation, self.config.default_obb_color);
    }

    /// Render all queued bounding boxes
    ///
    /// Clears the queue after rendering.
    pub fn render(&mut self) -> Result<(), RendererError> {
        if !self.config.enabled || (!self.initialized && !self.aabbs.is_empty() && !self.obbs.is_empty()) {
            self.clear();
            return Ok(());
        }

        if !self.initialized {
            self.initialize()?;
        }

        let aabb_count = self.aabbs.len();
        let obb_count = self.obbs.len();

        if aabb_count == 0 && obb_count == 0 {
            return Ok(());
        }

        // TODO: Implement batch rendering
        // 1. Update instance buffer with transforms and colors
        // 2. Bind pipeline and buffers
        // 3. Draw instanced (one draw call per type)

        debug!(
            aabb_count = aabb_count,
            obb_count = obb_count,
            "Rendering bounding boxes"
        );

        // Clear queue
        self.clear();

        Ok(())
    }

    /// Clear render queue without rendering
    pub fn clear(&mut self) {
        self.aabbs.clear();
        self.obbs.clear();
    }

    /// Get number of queued AABBs
    pub fn aabb_count(&self) -> usize {
        self.aabbs.len()
    }

    /// Get number of queued OBBs
    pub fn obb_count(&self) -> usize {
        self.obbs.len()
    }

    /// Enable/disable rendering
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), RendererError> {
        if enabled && !self.initialized {
            self.initialize()?;
        }
        self.config.enabled = enabled;
        Ok(())
    }

    /// Check if rendering is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Drop for BoundingBoxRenderer {
    fn drop(&mut self) {
        // TODO: Cleanup Vulkan resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_creation() {
        let aabb = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]);

        assert_eq!(aabb.min, [0.0, 0.0, 0.0]);
        assert_eq!(aabb.max, [1.0, 1.0, 1.0]);
        assert_eq!(aabb.center(), [0.5, 0.5, 0.5]);
        assert_eq!(aabb.extents(), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn test_obb_creation() {
        let obb = Obb::new(
            [1.0, 2.0, 3.0],
            [0.5, 0.5, 0.5],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
        );

        assert_eq!(obb.center, [1.0, 2.0, 3.0]);
        assert_eq!(obb.extents, [0.5, 0.5, 0.5]);
    }

    #[test]
    fn test_bounds_config_default() {
        let config = BoundsConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.line_width, 1.0);
    }

    #[test]
    #[ignore] // Requires Vulkan context
    fn test_renderer_add_aabb() {
        // Requires real VulkanContext
        // let context = Arc::new(VulkanContext::new(...)?);
        // let mut renderer = BoundingBoxRenderer::new(context)?;
        //
        // renderer.add_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]);
        // assert_eq!(renderer.aabb_count(), 1);
        //
        // renderer.clear();
        // assert_eq!(renderer.aabb_count(), 0);
    }
}
