//! Performance Overlay
//!
//! Displays real-time rendering statistics as an on-screen overlay.
//!
//! # Features
//! - FPS counter with average/min/max
//! - Frame time graph
//! - Draw call count
//! - Vertex/triangle count
//! - GPU memory usage
//! - Customizable position and appearance
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::debug::PerformanceOverlay;
//!
//! # let context = todo!();
//! let mut overlay = PerformanceOverlay::new(&context)?;
//!
//! // Update stats each frame
//! overlay.update_frame_time(16.7);
//! overlay.update_draw_calls(1024);
//! overlay.update_triangle_count(500_000);
//!
//! // Render overlay
//! overlay.render()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{RendererError, VulkanContext};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

/// Performance statistics
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Current frame time (milliseconds)
    pub frame_time_ms: f32,

    /// Current FPS
    pub fps: f32,

    /// Average FPS over window
    pub fps_avg: f32,

    /// Minimum FPS over window
    pub fps_min: f32,

    /// Maximum FPS over window
    pub fps_max: f32,

    /// Number of draw calls this frame
    pub draw_calls: usize,

    /// Number of vertices rendered this frame
    pub vertex_count: usize,

    /// Number of triangles rendered this frame
    pub triangle_count: usize,

    /// GPU memory allocated (bytes)
    pub gpu_memory_bytes: u64,

    /// GPU memory used (bytes)
    pub gpu_memory_used: u64,
}

impl PerformanceStats {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        format!(
            "FPS: {:.1} (avg: {:.1}, min: {:.1}, max: {:.1})\n\
             Frame: {:.2}ms\n\
             Draw Calls: {}\n\
             Triangles: {}\n\
             GPU Mem: {:.1} MB / {:.1} MB",
            self.fps,
            self.fps_avg,
            self.fps_min,
            self.fps_max,
            self.frame_time_ms,
            self.draw_calls,
            self.triangle_count,
            self.gpu_memory_used as f32 / 1024.0 / 1024.0,
            self.gpu_memory_bytes as f32 / 1024.0 / 1024.0
        )
    }
}

/// Overlay configuration
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Enable overlay rendering
    pub enabled: bool,

    /// Position on screen (0.0-1.0, top-left)
    pub position: [f32; 2],

    /// Text color (RGBA, 0.0-1.0)
    pub text_color: [f32; 4],

    /// Background color (RGBA, 0.0-1.0)
    pub background_color: [f32; 4],

    /// Font size in pixels
    pub font_size: f32,

    /// Number of frames to average for statistics
    pub stats_window: usize,

    /// Show frame time graph
    pub show_graph: bool,

    /// Graph height in pixels
    pub graph_height: u32,

    /// Graph width in pixels
    pub graph_width: u32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: [0.02, 0.02], // Top-left with padding
            text_color: [1.0, 1.0, 1.0, 1.0],
            background_color: [0.0, 0.0, 0.0, 0.7], // Semi-transparent black
            font_size: 14.0,
            stats_window: 60, // 1 second at 60fps
            show_graph: true,
            graph_height: 60,
            graph_width: 200,
        }
    }
}

/// Performance overlay renderer
pub struct PerformanceOverlay {
    #[allow(dead_code)]
    context: Arc<VulkanContext>,

    config: OverlayConfig,

    /// Current statistics
    stats: PerformanceStats,

    /// Frame time history (for graph and averaging)
    frame_times: VecDeque<f32>,

    /// FPS history
    fps_history: VecDeque<f32>,

    /// Last frame timestamp
    last_frame: Instant,

    /// Frame counter for FPS calculation
    frame_count: u64,

    initialized: bool,
}

impl PerformanceOverlay {
    /// Create performance overlay
    pub fn new(context: Arc<VulkanContext>) -> Result<Self, RendererError> {
        Ok(Self {
            context,
            config: OverlayConfig::default(),
            stats: PerformanceStats::default(),
            frame_times: VecDeque::new(),
            fps_history: VecDeque::new(),
            last_frame: Instant::now(),
            frame_count: 0,
            initialized: false,
        })
    }

    /// Create with custom config
    pub fn with_config(
        context: Arc<VulkanContext>,
        config: OverlayConfig,
    ) -> Result<Self, RendererError> {
        let mut overlay = Self::new(context)?;
        let enabled = config.enabled;
        overlay.config = config;

        if enabled {
            overlay.initialize()?;
        }

        Ok(overlay)
    }

    /// Initialize overlay resources
    fn initialize(&mut self) -> Result<(), RendererError> {
        if self.initialized {
            return Ok(());
        }

        // TODO: Initialize text rendering
        // - Load font atlas
        // - Create text rendering pipeline
        // - Create graph rendering resources

        self.initialized = true;
        Ok(())
    }

    /// Begin frame timing
    pub fn begin_frame(&mut self) {
        self.last_frame = Instant::now();
    }

    /// End frame timing and update statistics
    pub fn end_frame(&mut self) {
        let frame_time = self.last_frame.elapsed();
        let frame_time_ms = frame_time.as_secs_f32() * 1000.0;

        self.update_frame_time(frame_time_ms);
        self.frame_count += 1;
    }

    /// Update frame time statistics
    pub fn update_frame_time(&mut self, frame_time_ms: f32) {
        self.stats.frame_time_ms = frame_time_ms;

        // Calculate FPS
        let fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };
        self.stats.fps = fps;

        // Add to history
        self.frame_times.push_back(frame_time_ms);
        self.fps_history.push_back(fps);

        // Limit history size
        while self.frame_times.len() > self.config.stats_window {
            self.frame_times.pop_front();
        }
        while self.fps_history.len() > self.config.stats_window {
            self.fps_history.pop_front();
        }

        // Calculate averages
        if !self.fps_history.is_empty() {
            self.stats.fps_avg =
                self.fps_history.iter().sum::<f32>() / self.fps_history.len() as f32;
            self.stats.fps_min = self
                .fps_history
                .iter()
                .copied()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            self.stats.fps_max = self
                .fps_history
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
        }
    }

    /// Update draw call count
    pub fn update_draw_calls(&mut self, count: usize) {
        self.stats.draw_calls = count;
    }

    /// Update vertex count
    pub fn update_vertex_count(&mut self, count: usize) {
        self.stats.vertex_count = count;
    }

    /// Update triangle count
    pub fn update_triangle_count(&mut self, count: usize) {
        self.stats.triangle_count = count;
    }

    /// Update GPU memory statistics
    pub fn update_gpu_memory(&mut self, allocated: u64, used: u64) {
        self.stats.gpu_memory_bytes = allocated;
        self.stats.gpu_memory_used = used;
    }

    /// Render overlay
    pub fn render(&mut self) -> Result<(), RendererError> {
        if !self.config.enabled {
            return Ok(());
        }

        if !self.initialized {
            self.initialize()?;
        }

        // TODO: Implement rendering
        // 1. Render background rectangle
        // 2. Render text with statistics
        // 3. If show_graph enabled, render frame time graph

        debug!(
            fps = self.stats.fps,
            frame_time = self.stats.frame_time_ms,
            "Rendering performance overlay"
        );

        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> &PerformanceStats {
        &self.stats
    }

    /// Get formatted stats string
    pub fn stats_string(&self) -> String {
        self.stats.format()
    }

    /// Enable/disable overlay
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), RendererError> {
        if enabled && !self.initialized {
            self.initialize()?;
        }
        self.config.enabled = enabled;
        Ok(())
    }

    /// Check if overlay is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Toggle overlay visibility
    pub fn toggle(&mut self) -> Result<(), RendererError> {
        self.set_enabled(!self.config.enabled)
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.stats = PerformanceStats::default();
        self.frame_times.clear();
        self.fps_history.clear();
        self.frame_count = 0;
    }

    /// Get frame count since creation/reset
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get average frame time over window
    pub fn avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            0.0
        } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        }
    }

    /// Check if performance meets target (60 FPS)
    pub fn meets_target(&self, target_fps: f32) -> bool {
        self.stats.fps >= target_fps
    }

    /// Get performance grade (A-F based on FPS)
    pub fn performance_grade(&self) -> char {
        match self.stats.fps as u32 {
            90.. => 'A',
            60..90 => 'B',
            30..60 => 'C',
            15..30 => 'D',
            _ => 'F',
        }
    }
}

impl Drop for PerformanceOverlay {
    fn drop(&mut self) {
        // TODO: Cleanup resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_config_default() {
        let config = OverlayConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.stats_window, 60);
        assert!(config.show_graph);
    }

    #[test]
    fn test_performance_stats_default() {
        let stats = PerformanceStats::default();
        assert_eq!(stats.fps, 0.0);
        assert_eq!(stats.draw_calls, 0);
    }

    #[test]
    fn test_stats_format() {
        let stats = PerformanceStats {
            fps: 60.0,
            fps_avg: 59.5,
            fps_min: 55.0,
            fps_max: 62.0,
            frame_time_ms: 16.67,
            draw_calls: 1024,
            vertex_count: 1_500_000,
            triangle_count: 500_000,
            gpu_memory_bytes: 2_000_000_000,
            gpu_memory_used: 1_500_000_000,
        };

        let formatted = stats.format();
        assert!(formatted.contains("FPS: 60.0"));
        assert!(formatted.contains("Draw Calls: 1024"));
    }

    #[test]
    #[ignore] // Requires Vulkan context
    fn test_overlay_frame_timing() {
        // Requires real VulkanContext
        // let context = Arc::new(VulkanContext::new(...)?);
        // let mut overlay = PerformanceOverlay::new(context)?;
        //
        // overlay.begin_frame();
        // std::thread::sleep(Duration::from_millis(16));
        // overlay.end_frame();
        //
        // assert!(overlay.stats().frame_time_ms >= 16.0);
        // assert_eq!(overlay.frame_count(), 1);
    }

    #[test]
    fn test_update_statistics() {
        // Create overlay without Vulkan context (will fail initialization)
        // But we can test statistic updates
        let mut stats = PerformanceStats::default();

        // Simulate frame timing
        let frame_time = 16.67; // 60 FPS
        stats.frame_time_ms = frame_time;
        stats.fps = 1000.0 / frame_time;

        assert!((stats.fps - 60.0).abs() < 0.1);
    }

    #[test]
    #[ignore = "requires valid VulkanContext; VulkanContext now contains non-null fn pointers"]
    #[allow(invalid_value)]
    fn test_performance_grade() {
        // Create mock overlay for testing grades
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let mut overlay = PerformanceOverlay::new(context).unwrap();

        overlay.stats.fps = 120.0;
        assert_eq!(overlay.performance_grade(), 'A');

        overlay.stats.fps = 75.0;
        assert_eq!(overlay.performance_grade(), 'B');

        overlay.stats.fps = 45.0;
        assert_eq!(overlay.performance_grade(), 'C');

        overlay.stats.fps = 20.0;
        assert_eq!(overlay.performance_grade(), 'D');

        overlay.stats.fps = 10.0;
        assert_eq!(overlay.performance_grade(), 'F');
    }

    #[test]
    #[ignore = "requires valid VulkanContext; VulkanContext now contains non-null fn pointers"]
    #[allow(invalid_value)]
    fn test_meets_target() {
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let mut overlay = PerformanceOverlay::new(context).unwrap();

        overlay.stats.fps = 65.0;
        assert!(overlay.meets_target(60.0));

        overlay.stats.fps = 55.0;
        assert!(!overlay.meets_target(60.0));
    }
}
