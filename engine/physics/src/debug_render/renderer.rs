//! Main debug renderer for physics visualization
//!
//! Manages debug rendering state and provides high-level API for drawing
//! physics debugging information.

use crate::debug_render::DebugLine;
use engine_math::Vec3;
use tracing::{debug, trace};

/// Debug renderer for physics visualization
///
/// This renderer accumulates debug draw calls per frame and batches them
/// for efficient rendering. It integrates with the main renderer to draw
/// debug overlays on top of the scene.
pub struct DebugRenderer {
    /// Lines to render this frame
    lines: Vec<DebugLine>,

    /// Maximum number of lines (prevent memory explosion)
    max_lines: usize,

    /// Whether debug rendering is enabled
    enabled: bool,
}

impl DebugRenderer {
    /// Create a new debug renderer
    ///
    /// # Arguments
    ///
    /// * `max_lines` - Maximum number of lines to render per frame (default: 100,000)
    pub fn new(max_lines: Option<usize>) -> Self {
        let max_lines = max_lines.unwrap_or(100_000);
        debug!(max_lines, "Creating physics debug renderer");

        Self { lines: Vec::with_capacity(1024), max_lines, enabled: true }
    }

    /// Begin a new frame
    ///
    /// Clears all accumulated debug lines from the previous frame.
    pub fn begin_frame(&mut self) {
        self.lines.clear();
    }

    /// End frame and return lines for rendering
    ///
    /// Returns the accumulated lines for this frame. The renderer should
    /// submit these to the GPU for drawing.
    pub fn end_frame(&mut self) -> &[DebugLine] {
        if !self.enabled {
            return &[];
        }

        trace!(line_count = self.lines.len(), "Debug render frame complete");
        &self.lines
    }

    /// Add a single line
    pub fn add_line(&mut self, start: Vec3, end: Vec3, color: [f32; 3]) {
        if !self.enabled || self.lines.len() >= self.max_lines {
            return;
        }

        self.lines.push(DebugLine::new(start, end, color));
    }

    /// Add multiple lines
    pub fn add_lines(&mut self, lines: &[DebugLine]) {
        if !self.enabled {
            return;
        }

        let remaining_capacity = self.max_lines.saturating_sub(self.lines.len());
        let lines_to_add = lines.len().min(remaining_capacity);

        self.lines.extend_from_slice(&lines[..lines_to_add]);

        if lines_to_add < lines.len() {
            tracing::warn!(
                requested = lines.len(),
                added = lines_to_add,
                "Debug renderer line limit reached"
            );
        }
    }

    /// Draw a wireframe box
    pub fn draw_box(&mut self, min: Vec3, max: Vec3, color: [f32; 3]) {
        let lines = super::wireframe_box(min, max, color);
        self.add_lines(&lines);
    }

    /// Draw an arrow
    pub fn draw_arrow(&mut self, start: Vec3, end: Vec3, color: [f32; 3]) {
        let lines = super::arrow(start, end, color);
        self.add_lines(&lines);
    }

    /// Enable debug rendering
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable debug rendering
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if debug rendering is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get current line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get maximum line capacity
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }
}

impl Default for DebugRenderer {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_renderer_creation() {
        let renderer = DebugRenderer::new(Some(1000));
        assert_eq!(renderer.max_lines(), 1000);
        assert!(renderer.is_enabled());
    }

    #[test]
    fn test_begin_end_frame() {
        let mut renderer = DebugRenderer::new(None);

        renderer.begin_frame();
        renderer.add_line(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(renderer.line_count(), 1);

        let lines = renderer.end_frame();
        assert_eq!(lines.len(), 1);

        // Next frame should start fresh
        renderer.begin_frame();
        assert_eq!(renderer.line_count(), 0);
    }

    #[test]
    fn test_line_limit() {
        let mut renderer = DebugRenderer::new(Some(10));

        renderer.begin_frame();

        // Add 20 lines (should only accept 10)
        for i in 0..20 {
            renderer.add_line(Vec3::ZERO, Vec3::new(i as f32, 0.0, 0.0), [1.0, 0.0, 0.0]);
        }

        assert_eq!(renderer.line_count(), 10, "Should enforce line limit");
    }

    #[test]
    fn test_enable_disable() {
        let mut renderer = DebugRenderer::new(None);

        renderer.begin_frame();
        renderer.add_line(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(renderer.line_count(), 1);

        // Disable and verify no lines added
        renderer.disable();
        renderer.begin_frame();
        renderer.add_line(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(renderer.line_count(), 0, "Should not add lines when disabled");

        // Re-enable
        renderer.enable();
        renderer.begin_frame();
        renderer.add_line(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(renderer.line_count(), 1, "Should add lines when re-enabled");
    }

    #[test]
    fn test_draw_box() {
        let mut renderer = DebugRenderer::new(None);
        renderer.begin_frame();

        renderer.draw_box(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(renderer.line_count(), 12, "Box should have 12 edges");
    }

    #[test]
    fn test_draw_arrow() {
        let mut renderer = DebugRenderer::new(None);
        renderer.begin_frame();

        renderer.draw_arrow(Vec3::ZERO, Vec3::Y, [0.0, 1.0, 0.0]);
        assert_eq!(renderer.line_count(), 4, "Arrow should have 4 lines");
    }
}
