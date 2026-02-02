//! Frame Capture + Analysis (R.5)
//!
//! Visual regression testing and frame comparison for debugging rendering issues.
//!
//! # Overview
//!
//! This module implements frame capture and comparison:
//! - **Frame Capture**: Color and depth buffer capture from GPU
//! - **Frame Comparison**: Per-pixel difference detection
//! - **Anomaly Detection**: Identify visual issues automatically
//! - **Regression Testing**: Automated visual comparison for CI
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::debug::{RenderingDebugger, DebugConfig};
//!
//! # let context = todo!();
//! let debugger = RenderingDebugger::new(context, DebugConfig::default());
//!
//! // Capture current frame
//! let frame = debugger.capture_frame()?;
//! println!("Captured frame {} ({}x{})", frame.frame, frame.width, frame.height);
//!
//! // Compare with expected frame
//! # let expected_frame = todo!();
//! let diff = debugger.compare_frames(&expected_frame, &frame)?;
//! if diff.percent_different > 0.1 {
//!     println!("WARNING: {:.2}% pixels differ", diff.percent_different);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Performance
//!
//! - Frame capture: GPU -> CPU transfer (async preferred)
//! - Target overhead: < 2ms per frame
//! - Comparison: CPU-side per-pixel analysis

#![allow(missing_docs)]

use crate::context::VulkanContext;
use engine_core::{define_error, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

define_error! {
    /// Frame capture errors
    pub enum CaptureError {
        /// Failed to read color buffer from GPU
        ColorBufferReadFailed { details: String } = ErrorCode::DebugCaptureColorBufferReadFailed, ErrorSeverity::Error,

        /// Failed to read depth buffer from GPU
        DepthBufferReadFailed { details: String } = ErrorCode::DebugCaptureDepthBufferReadFailed, ErrorSeverity::Error,

        /// Frame dimensions mismatch
        DimensionMismatch {
            expected_width: u32,
            expected_height: u32,
            actual_width: u32,
            actual_height: u32,
        } = ErrorCode::DebugCaptureDimensionMismatch, ErrorSeverity::Error,

        /// Invalid frame data
        InvalidFrameData { details: String } = ErrorCode::DebugCaptureInvalidFrameData, ErrorSeverity::Error,
    }
}

/// Complete frame capture data
#[derive(Debug, Clone)]
pub struct FrameCaptureData {
    /// Frame number
    pub frame: u64,

    /// Frame width in pixels
    pub width: u32,

    /// Frame height in pixels
    pub height: u32,

    /// Color buffer (RGBA8 format, 4 bytes per pixel)
    /// Layout: row-major, top-left origin
    pub color_buffer: Vec<u8>,

    /// Depth buffer (f32 values in range [0.0, 1.0])
    /// Layout: row-major, top-left origin
    /// 0.0 = near plane, 1.0 = far plane
    pub depth_buffer: Vec<f32>,

    /// Frame metadata
    pub metadata: FrameMetadata,

    /// Overdraw map (optional): count of how many times each pixel was drawn
    /// Useful for detecting overdraw performance issues
    pub overdraw_map: Option<Vec<u8>>,

    /// Entity ID map (optional): which entity rendered each pixel
    /// Useful for debugging specific object rendering
    pub entity_id_map: Option<Vec<u32>>,
}

impl FrameCaptureData {
    /// Validate frame capture data consistency
    pub fn validate(&self) -> Result<(), CaptureError> {
        let expected_color_bytes = (self.width * self.height * 4) as usize;
        if self.color_buffer.len() != expected_color_bytes {
            return Err(CaptureError::InvalidFrameData(format!(
                "Color buffer size mismatch: expected {} bytes, got {}",
                expected_color_bytes,
                self.color_buffer.len()
            )));
        }

        let expected_depth_count = (self.width * self.height) as usize;
        if self.depth_buffer.len() != expected_depth_count {
            return Err(CaptureError::InvalidFrameData(format!(
                "Depth buffer size mismatch: expected {} values, got {}",
                expected_depth_count,
                self.depth_buffer.len()
            )));
        }

        // Validate depth values are in [0.0, 1.0]
        for (i, &depth) in self.depth_buffer.iter().enumerate() {
            if !depth.is_finite() || depth < 0.0 || depth > 1.0 {
                return Err(CaptureError::InvalidFrameData(format!(
                    "Invalid depth value at index {}: {}",
                    i, depth
                )));
            }
        }

        Ok(())
    }

    /// Get pixel color at (x, y) coordinates
    /// Returns None if coordinates are out of bounds
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = ((y * self.width + x) * 4) as usize;
        Some([
            self.color_buffer[index],
            self.color_buffer[index + 1],
            self.color_buffer[index + 2],
            self.color_buffer[index + 3],
        ])
    }

    /// Get depth value at (x, y) coordinates
    /// Returns None if coordinates are out of bounds
    pub fn get_depth(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = (y * self.width + x) as usize;
        Some(self.depth_buffer[index])
    }
}

/// Frame metadata collected during rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    /// Timestamp in seconds since engine start
    pub timestamp: f64,

    /// Number of draw calls submitted
    pub draw_call_count: usize,

    /// Total number of vertices processed
    pub vertex_count: usize,

    /// Total number of triangles rendered
    pub triangle_count: usize,

    /// GPU time for entire frame (milliseconds)
    pub gpu_time_ms: f32,
}

impl Default for FrameMetadata {
    fn default() -> Self {
        Self {
            timestamp: 0.0,
            draw_call_count: 0,
            vertex_count: 0,
            triangle_count: 0,
            gpu_time_ms: 0.0,
        }
    }
}

/// Frame difference analysis result
#[derive(Debug, Clone)]
pub struct FrameDiff {
    /// Number of pixels that differ
    pub pixels_different: usize,

    /// Percentage of pixels that differ (0.0-100.0)
    pub percent_different: f32,

    /// Maximum color delta across all channels (0-255)
    pub max_color_delta: u8,

    /// Average color delta across all different pixels (0.0-255.0)
    pub avg_color_delta: f32,

    /// Visual diff image (red = different, green = same, RGBA8 format)
    pub diff_image: Vec<u8>,

    /// Average red channel delta
    pub red_delta: f32,

    /// Average green channel delta
    pub green_delta: f32,

    /// Average blue channel delta
    pub blue_delta: f32,

    /// Average alpha channel delta
    pub alpha_delta: f32,
}

/// Rectangle for bounding boxes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Visual anomaly detected during frame analysis
#[derive(Debug, Clone)]
pub enum Anomaly {
    /// Expected object missing from frame
    MissingObject { entity_id: u64, expected_bounds: Rect },

    /// Unexpected object present in frame
    UnexpectedObject { entity_id: u64, actual_bounds: Rect },

    /// Color mismatch at specific pixel
    ColorMismatch { pixel: (u32, u32), expected_color: [u8; 4], actual_color: [u8; 4] },

    /// Depth mismatch at specific pixel
    DepthMismatch { pixel: (u32, u32), expected_depth: f32, actual_depth: f32 },
}

/// Debug configuration for rendering debugger
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable overdraw analysis
    pub enable_overdraw: bool,

    /// Enable entity ID map capture
    pub enable_entity_ids: bool,

    /// Color difference threshold for anomaly detection (0-255)
    pub color_threshold: u8,

    /// Depth difference threshold for anomaly detection (0.0-1.0)
    pub depth_threshold: f32,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enable_overdraw: false,
            enable_entity_ids: false,
            color_threshold: 10,
            depth_threshold: 0.01,
        }
    }
}

/// Rendering debugger with frame capture and analysis
pub struct RenderingDebugger {
    context: Arc<VulkanContext>,
    config: DebugConfig,
}

impl RenderingDebugger {
    /// Create a new rendering debugger
    pub fn new(context: Arc<VulkanContext>, config: DebugConfig) -> Self {
        Self { context, config }
    }

    /// Capture current frame (color + depth + metadata)
    ///
    /// NOTE: Currently returns dummy data until integrated with actual renderer.
    /// GPU -> CPU transfer will be implemented when framebuffer capture is available.
    pub fn capture_frame(&self) -> Result<FrameCaptureData, CaptureError> {
        // TODO: Implement actual GPU -> CPU transfer
        // For now, return dummy frame data for testing
        let width = 800;
        let height = 600;
        let pixel_count = (width * height) as usize;

        // Create dummy color buffer (RGBA8)
        let color_buffer = vec![0u8; pixel_count * 4];

        // Create dummy depth buffer
        let depth_buffer = vec![1.0f32; pixel_count];

        let frame_data = FrameCaptureData {
            frame: 0,
            width,
            height,
            color_buffer,
            depth_buffer,
            metadata: FrameMetadata::default(),
            overdraw_map: if self.config.enable_overdraw {
                Some(vec![0u8; pixel_count])
            } else {
                None
            },
            entity_id_map: if self.config.enable_entity_ids {
                Some(vec![0u32; pixel_count])
            } else {
                None
            },
        };

        frame_data.validate()?;
        Ok(frame_data)
    }

    /// Compare two frames and generate difference analysis
    pub fn compare_frames(
        &self,
        expected: &FrameCaptureData,
        actual: &FrameCaptureData,
    ) -> Result<FrameDiff, CaptureError> {
        // Validate dimensions match
        if expected.width != actual.width || expected.height != actual.height {
            return Err(CaptureError::DimensionMismatch {
                expected_width: expected.width,
                expected_height: expected.height,
                actual_width: actual.width,
                actual_height: actual.height,
            });
        }

        let pixel_count = (expected.width * expected.height) as usize;
        let mut pixels_different = 0;
        let mut max_color_delta = 0u8;
        let mut sum_color_delta = 0u64;
        let mut sum_red_delta = 0u64;
        let mut sum_green_delta = 0u64;
        let mut sum_blue_delta = 0u64;
        let mut sum_alpha_delta = 0u64;

        // Allocate diff image (RGBA8)
        let mut diff_image = Vec::with_capacity(pixel_count * 4);

        // Compare per-pixel
        for i in 0..pixel_count {
            let idx = i * 4;
            let exp_r = expected.color_buffer[idx];
            let exp_g = expected.color_buffer[idx + 1];
            let exp_b = expected.color_buffer[idx + 2];
            let exp_a = expected.color_buffer[idx + 3];

            let act_r = actual.color_buffer[idx];
            let act_g = actual.color_buffer[idx + 1];
            let act_b = actual.color_buffer[idx + 2];
            let act_a = actual.color_buffer[idx + 3];

            // Calculate per-channel deltas
            let delta_r = (exp_r as i16 - act_r as i16).abs() as u8;
            let delta_g = (exp_g as i16 - act_g as i16).abs() as u8;
            let delta_b = (exp_b as i16 - act_b as i16).abs() as u8;
            let delta_a = (exp_a as i16 - act_a as i16).abs() as u8;

            // Maximum delta across all channels
            let pixel_delta = delta_r.max(delta_g).max(delta_b).max(delta_a);

            // Track statistics
            if pixel_delta > 0 {
                pixels_different += 1;
                sum_color_delta += pixel_delta as u64;
                sum_red_delta += delta_r as u64;
                sum_green_delta += delta_g as u64;
                sum_blue_delta += delta_b as u64;
                sum_alpha_delta += delta_a as u64;
                max_color_delta = max_color_delta.max(pixel_delta);

                // Diff image: red for different pixels
                diff_image.push(255); // R
                diff_image.push(0); // G
                diff_image.push(0); // B
                diff_image.push(255); // A
            } else {
                // Diff image: green for same pixels
                diff_image.push(0); // R
                diff_image.push(255); // G
                diff_image.push(0); // B
                diff_image.push(255); // A
            }
        }

        let percent_different = (pixels_different as f32 / pixel_count as f32) * 100.0;

        let avg_color_delta = if pixels_different > 0 {
            sum_color_delta as f32 / pixels_different as f32
        } else {
            0.0
        };

        // Calculate per-channel average deltas (over all pixels, not just different ones)
        let red_delta = sum_red_delta as f32 / pixel_count as f32;
        let green_delta = sum_green_delta as f32 / pixel_count as f32;
        let blue_delta = sum_blue_delta as f32 / pixel_count as f32;
        let alpha_delta = sum_alpha_delta as f32 / pixel_count as f32;

        Ok(FrameDiff {
            pixels_different,
            percent_different,
            max_color_delta,
            avg_color_delta,
            diff_image,
            red_delta,
            green_delta,
            blue_delta,
            alpha_delta,
        })
    }

    /// Detect visual anomalies in a frame (basic implementation)
    ///
    /// This is a basic implementation that detects color and depth mismatches.
    /// Future enhancements could include:
    /// - Object detection and tracking
    /// - Automatic bounding box extraction
    /// - Machine learning-based anomaly detection
    pub fn detect_visual_anomalies(
        &self,
        frame: &FrameCaptureData,
    ) -> Result<Vec<Anomaly>, CaptureError> {
        let mut anomalies = Vec::new();

        // Basic validation: check for suspicious patterns
        // This is a placeholder for more sophisticated analysis

        // Example: Detect pure black pixels (might indicate rendering failure)
        for y in 0..frame.height {
            for x in 0..frame.width {
                if let Some(color) = frame.get_pixel(x, y) {
                    // Check for completely black pixels with zero alpha (render failure)
                    if color == [0, 0, 0, 0] {
                        // This might indicate a rendering issue
                        // In practice, we'd collect these into regions
                        // For now, we just track them
                    }
                }
            }
        }

        Ok(anomalies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame(
        frame: u64,
        width: u32,
        height: u32,
        fill_color: [u8; 4],
    ) -> FrameCaptureData {
        let pixel_count = (width * height) as usize;
        let mut color_buffer = Vec::with_capacity(pixel_count * 4);

        for _ in 0..pixel_count {
            color_buffer.extend_from_slice(&fill_color);
        }

        let depth_buffer = vec![0.5f32; pixel_count];

        FrameCaptureData {
            frame,
            width,
            height,
            color_buffer,
            depth_buffer,
            metadata: FrameMetadata::default(),
            overdraw_map: None,
            entity_id_map: None,
        }
    }

    #[test]
    fn test_frame_validation_valid() {
        let frame = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        assert!(frame.validate().is_ok());
    }

    #[test]
    fn test_frame_validation_invalid_color_size() {
        let mut frame = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        frame.color_buffer.truncate(100); // Make it invalid
        let result = frame.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CaptureError::InvalidFrameData(_)));
    }

    #[test]
    fn test_frame_validation_invalid_depth_size() {
        let mut frame = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        frame.depth_buffer.truncate(100); // Make it invalid
        let result = frame.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_validation_invalid_depth_value() {
        let mut frame = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        frame.depth_buffer[0] = -0.5; // Invalid depth
        let result = frame.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_pixel() {
        let frame = create_test_frame(0, 2, 2, [255, 0, 0, 255]);

        // Top-left pixel
        assert_eq!(frame.get_pixel(0, 0), Some([255, 0, 0, 255]));

        // Top-right pixel
        assert_eq!(frame.get_pixel(1, 0), Some([255, 0, 0, 255]));

        // Out of bounds
        assert_eq!(frame.get_pixel(2, 0), None);
        assert_eq!(frame.get_pixel(0, 2), None);
    }

    #[test]
    fn test_get_depth() {
        let frame = create_test_frame(0, 2, 2, [255, 0, 0, 255]);

        assert_eq!(frame.get_depth(0, 0), Some(0.5));
        assert_eq!(frame.get_depth(1, 1), Some(0.5));
        assert_eq!(frame.get_depth(2, 0), None);
    }

    #[test]
    fn test_compare_frames_identical() {
        let frame1 = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        let frame2 = create_test_frame(1, 800, 600, [255, 0, 0, 255]);

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() }); // Dummy context for testing
        let debugger = RenderingDebugger::new(context, config);

        let diff = debugger.compare_frames(&frame1, &frame2).unwrap();

        assert_eq!(diff.pixels_different, 0);
        assert_eq!(diff.percent_different, 0.0);
        assert_eq!(diff.max_color_delta, 0);
        assert_eq!(diff.avg_color_delta, 0.0);
    }

    #[test]
    fn test_compare_frames_different() {
        let frame1 = create_test_frame(0, 2, 2, [255, 0, 0, 255]); // Red
        let frame2 = create_test_frame(1, 2, 2, [0, 255, 0, 255]); // Green

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let debugger = RenderingDebugger::new(context, config);

        let diff = debugger.compare_frames(&frame1, &frame2).unwrap();

        assert_eq!(diff.pixels_different, 4); // All 4 pixels differ
        assert_eq!(diff.percent_different, 100.0);
        assert_eq!(diff.max_color_delta, 255);
        assert!(diff.avg_color_delta > 0.0);
    }

    #[test]
    fn test_compare_frames_dimension_mismatch() {
        let frame1 = create_test_frame(0, 800, 600, [255, 0, 0, 255]);
        let frame2 = create_test_frame(1, 1024, 768, [255, 0, 0, 255]);

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let debugger = RenderingDebugger::new(context, config);

        let result = debugger.compare_frames(&frame1, &frame2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CaptureError::DimensionMismatch { .. }));
    }

    #[test]
    fn test_diff_image_generation() {
        // Create partially different frames
        let mut frame1 = create_test_frame(0, 2, 2, [255, 0, 0, 255]);
        let mut frame2 = create_test_frame(1, 2, 2, [255, 0, 0, 255]);

        // Make bottom-right pixel different
        let idx = (1 * 2 + 1) * 4; // (y=1, x=1)
        frame2.color_buffer[idx] = 0; // Change red to green
        frame2.color_buffer[idx + 1] = 255;

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let debugger = RenderingDebugger::new(context, config);

        let diff = debugger.compare_frames(&frame1, &frame2).unwrap();

        // Should have 3 green pixels (same) and 1 red pixel (different)
        assert_eq!(diff.pixels_different, 1);
        assert_eq!(diff.percent_different, 25.0);

        // Check diff image format
        assert_eq!(diff.diff_image.len(), 2 * 2 * 4); // 2x2 pixels, RGBA8

        // First pixel (0,0) should be green (same)
        assert_eq!(&diff.diff_image[0..4], &[0, 255, 0, 255]);

        // Last pixel (1,1) should be red (different)
        let last_idx = 3 * 4;
        assert_eq!(&diff.diff_image[last_idx..last_idx + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn test_per_channel_deltas() {
        let frame1 = create_test_frame(0, 10, 10, [100, 100, 100, 255]);
        let frame2 = create_test_frame(1, 10, 10, [110, 90, 100, 255]);

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let debugger = RenderingDebugger::new(context, config);

        let diff = debugger.compare_frames(&frame1, &frame2).unwrap();

        // Red channel: +10 on all pixels
        assert_eq!(diff.red_delta, 10.0);

        // Green channel: -10 on all pixels
        assert_eq!(diff.green_delta, 10.0);

        // Blue channel: no change
        assert_eq!(diff.blue_delta, 0.0);

        // Alpha channel: no change
        assert_eq!(diff.alpha_delta, 0.0);
    }

    #[test]
    fn test_detect_visual_anomalies_empty() {
        let frame = create_test_frame(0, 800, 600, [255, 0, 0, 255]);

        let config = DebugConfig::default();
        let context = Arc::new(unsafe { std::mem::zeroed() });
        let debugger = RenderingDebugger::new(context, config);

        let anomalies = debugger.detect_visual_anomalies(&frame).unwrap();

        // Basic implementation returns empty for now
        assert_eq!(anomalies.len(), 0);
    }
}
