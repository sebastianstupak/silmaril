//! Visual Test Framework
//!
//! Provides test helpers for automated visual regression testing of rendered frames.
//!
//! # Example
//!
//! ```no_run
//! use helpers::visual_test::VisualTest;
//!
//! let mut test = VisualTest::new("cube_rendering");
//!
//! // Render frame
//! let frame = render_test_scene();
//!
//! // Compare with baseline (auto-creates if missing)
//! test.assert_matches_baseline(&frame, 1.0)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use engine_renderer::debug::{
    CaptureError, ComparisonConfig, ComparisonResult, FrameCaptureData, FrameMetadata,
    VisualValidator,
};
use std::path::PathBuf;

/// Visual regression test helper
pub struct VisualTest {
    /// Test name
    name: String,

    /// Visual validator
    validator: VisualValidator,

    /// Baseline directory (relative to project root)
    baseline_dir: PathBuf,
}

impl VisualTest {
    /// Create new visual test
    ///
    /// # Arguments
    /// * `name` - Test name (used for baseline filename)
    pub fn new(name: &str) -> Self {
        // Baselines stored in tests/baselines/
        let baseline_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("baselines");

        let validator = VisualValidator::new(&baseline_dir);

        Self { name: name.to_string(), validator, baseline_dir }
    }

    /// Assert frame matches baseline within threshold
    ///
    /// If baseline doesn't exist, creates it automatically (controlled by env var).
    ///
    /// # Arguments
    /// * `frame` - Captured frame to compare
    /// * `threshold_percent` - Maximum allowed difference percentage (0.0-100.0)
    pub fn assert_matches_baseline(
        &mut self,
        frame: &FrameCaptureData,
        threshold_percent: f32,
    ) -> Result<ComparisonResult, CaptureError> {
        // Check if baseline exists
        if !self.validator.has_baseline(&self.name) {
            // Auto-create baseline if UPDATE_BASELINES env var is set
            if std::env::var("UPDATE_BASELINES").is_ok() {
                self.validator.save_baseline(&self.name, frame)?;
                println!(
                    "✓ Created baseline: {} ({}x{})",
                    self.name, frame.width, frame.height
                );

                return Ok(ComparisonResult {
                    test_name: self.name.clone(),
                    is_match: true,
                    percent_different: 0.0,
                    max_color_delta: 0,
                    avg_color_delta: 0.0,
                    perceptual_distance: None,
                    diff_image_path: None,
                });
            } else {
                return Err(CaptureError::invalidframedata(format!(
                    "Baseline not found: {}. Run with UPDATE_BASELINES=1 to create it.",
                    self.name
                )));
            }
        }

        // Compare with baseline
        let config = ComparisonConfig {
            percent_threshold: threshold_percent,
            save_diff_on_mismatch: true,
            ..Default::default()
        };

        let result = self.validator.compare_with_baseline(&self.name, frame, &config)?;

        // Print result
        if result.is_match {
            println!("✓ {}", result.summary());
        } else {
            eprintln!("✗ {}", result.summary());
            if let Some(ref diff_path) = result.diff_image_path {
                eprintln!("  Diff saved to: {}", diff_path.display());
            }
        }

        // Assert match
        if !result.is_match {
            return Err(CaptureError::invalidframedata(format!(
                "Visual regression: {} ({:.2}% different, threshold: {:.2}%)",
                self.name, result.percent_different, threshold_percent
            )));
        }

        Ok(result)
    }

    /// Assert frame matches baseline with custom config
    pub fn assert_matches_baseline_with_config(
        &mut self,
        frame: &FrameCaptureData,
        config: &ComparisonConfig,
    ) -> Result<ComparisonResult, CaptureError> {
        if !self.validator.has_baseline(&self.name) {
            if std::env::var("UPDATE_BASELINES").is_ok() {
                self.validator.save_baseline(&self.name, frame)?;
                return Ok(ComparisonResult {
                    test_name: self.name.clone(),
                    is_match: true,
                    percent_different: 0.0,
                    max_color_delta: 0,
                    avg_color_delta: 0.0,
                    perceptual_distance: None,
                    diff_image_path: None,
                });
            } else {
                return Err(CaptureError::invalidframedata(format!(
                    "Baseline not found: {}",
                    self.name
                )));
            }
        }

        let result = self.validator.compare_with_baseline(&self.name, frame, config)?;

        if !result.is_match {
            eprintln!("✗ {}", result.summary());
            return Err(CaptureError::invalidframedata(format!(
                "Visual regression: {}",
                self.name
            )));
        }

        println!("✓ {}", result.summary());
        Ok(result)
    }

    /// Update baseline (force overwrite)
    pub fn update_baseline(&self, frame: &FrameCaptureData) -> Result<(), CaptureError> {
        self.validator.save_baseline(&self.name, frame)?;
        println!("✓ Updated baseline: {}", self.name);
        Ok(())
    }

    /// Get baseline directory path
    pub fn baseline_dir(&self) -> &PathBuf {
        &self.baseline_dir
    }
}

/// Helper to create test frame data
pub fn create_test_frame(
    width: u32,
    height: u32,
    fill_color: [u8; 4],
) -> FrameCaptureData {
    let pixel_count = (width * height) as usize;
    let mut color_buffer = Vec::with_capacity(pixel_count * 4);

    for _ in 0..pixel_count {
        color_buffer.extend_from_slice(&fill_color);
    }

    let depth_buffer = vec![1.0f32; pixel_count];

    FrameCaptureData {
        frame: 0,
        width,
        height,
        color_buffer,
        depth_buffer,
        metadata: FrameMetadata::default(),
        overdraw_map: None,
        entity_id_map: None,
    }
}

/// Helper to create frame with gradient
pub fn create_gradient_frame(width: u32, height: u32) -> FrameCaptureData {
    let pixel_count = (width * height) as usize;
    let mut color_buffer = Vec::with_capacity(pixel_count * 4);

    for y in 0..height {
        for x in 0..width {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128;
            let a = 255;

            color_buffer.extend_from_slice(&[r, g, b, a]);
        }
    }

    let depth_buffer = vec![0.5f32; pixel_count];

    FrameCaptureData {
        frame: 0,
        width,
        height,
        color_buffer,
        depth_buffer,
        metadata: FrameMetadata::default(),
        overdraw_map: None,
        entity_id_map: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_frame() {
        let frame = create_test_frame(10, 10, [255, 0, 0, 255]);
        assert_eq!(frame.width, 10);
        assert_eq!(frame.height, 10);
        assert_eq!(frame.color_buffer.len(), 400); // 10*10*4
        assert!(frame.validate().is_ok());
    }

    #[test]
    fn test_create_gradient_frame() {
        let frame = create_gradient_frame(256, 256);
        assert_eq!(frame.width, 256);
        assert_eq!(frame.height, 256);
        assert!(frame.validate().is_ok());

        // Check gradient - top-left should be dark, bottom-right should be bright
        assert_eq!(frame.get_pixel(0, 0).unwrap()[0], 0); // Red at (0,0) = 0
        assert_eq!(frame.get_pixel(255, 255).unwrap()[0], 255); // Red at max = 255
    }

    #[test]
    #[ignore] // Only run manually - requires baseline directory
    fn test_visual_test_workflow() {
        // Create test frame
        let frame = create_test_frame(64, 64, [255, 128, 64, 255]);

        // Create visual test
        let mut test = VisualTest::new("test_workflow");

        // This would fail if baseline doesn't exist (unless UPDATE_BASELINES=1)
        // For manual testing: UPDATE_BASELINES=1 cargo test test_visual_test_workflow -- --ignored
        let result = test.assert_matches_baseline(&frame, 1.0);

        if result.is_ok() {
            println!("Test passed or baseline created");
        }
    }
}
