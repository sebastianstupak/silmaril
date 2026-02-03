//! Frame capture system for AI agent visual feedback.
//!
//! Provides capabilities to capture rendered frames as images (PNG/JPEG) for AI analysis.
//! Essential for closing the visual feedback loop - agents can see what they're rendering.
//!
//! # Features
//! - GPU to CPU image readback
//! - PNG and JPEG encoding
//! - Configurable capture (enable/disable at runtime)
//! - Performance monitoring
//! - Memory-efficient streaming
//!
//! # Performance
//! - Target: < 2ms GPU→CPU copy
//! - Target: < 3ms PNG encoding
//! - Target: < 5ms total overhead per frame
//!
//! # Example
//! ```no_run
//! use engine_renderer::capture::{CaptureConfig, CaptureFormat};
//! use std::path::PathBuf;
//!
//! let config = CaptureConfig {
//!     enabled: true,
//!     format: CaptureFormat::Png,
//!     output_dir: PathBuf::from("captures"),
//!     filename_pattern: "frame_{:06}.png".to_string(),
//! };
//! ```

mod encoder;
mod metrics;
mod readback;

pub use encoder::{CaptureFormat, FrameEncoder};
pub use metrics::{CaptureMetrics, MetricsTracker};
pub use readback::FrameReadback;

use crate::{CommandPool, RendererError};
use ash::vk;
use std::path::PathBuf;
use tracing::{error, info};

/// Frame capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Whether capture is enabled
    pub enabled: bool,
    /// Image format for saving
    pub format: CaptureFormat,
    /// Output directory for captured frames
    pub output_dir: PathBuf,
    /// Filename pattern (e.g., "frame_{:06}.png")
    pub filename_pattern: String,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            format: CaptureFormat::Png,
            output_dir: PathBuf::from("captures"),
            filename_pattern: "frame_{:06}.png".to_string(),
        }
    }
}

/// Frame capture manager - orchestrates readback, encoding, and saving
pub struct CaptureManager {
    config: CaptureConfig,
    readback: Option<FrameReadback>,
    frame_count: u64,
    metrics_tracker: MetricsTracker,
}

impl CaptureManager {
    /// Create capture manager
    pub fn new(config: CaptureConfig) -> Self {
        // Create output directory if enabled
        if config.enabled {
            if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
                error!(
                    path = ?config.output_dir,
                    error = ?e,
                    "Failed to create capture output directory"
                );
            }
        }

        Self { config, readback: None, frame_count: 0, metrics_tracker: MetricsTracker::new() }
    }

    /// Initialize readback buffer
    ///
    /// Must be called after Vulkan device is created.
    pub fn initialize(
        &mut self,
        device: &ash::Device,
        allocator: &std::sync::Arc<std::sync::Mutex<gpu_allocator::vulkan::Allocator>>,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        if self.config.enabled {
            self.readback = Some(FrameReadback::new(device, allocator, width, height)?);
            info!(
                width = width,
                height = height,
                format = ?self.config.format,
                "Frame capture initialized"
            );
        }
        Ok(())
    }

    /// Capture current frame to disk
    ///
    /// Synchronous operation - blocks until frame is saved.
    pub fn capture_frame(
        &mut self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<(), RendererError> {
        if !self.config.enabled {
            return Ok(());
        }

        self.metrics_tracker.start_capture();

        let readback = self.readback.as_ref().ok_or_else(|| {
            RendererError::imagecreationfailed(0, 0, "Readback buffer not initialized".to_string())
        })?;

        // Copy image to buffer (GPU → CPU)
        let copy_start = std::time::Instant::now();
        readback.copy_image_to_buffer(device, command_pool, queue, image)?;
        let copy_time = copy_start.elapsed();

        // Get image data
        let data = readback.get_image_data()?;

        // Generate filename
        let filename = self
            .config
            .filename_pattern
            .replace("{:06}", &format!("{:06}", self.frame_count));
        let path = self.config.output_dir.join(filename);

        // Encode and save
        let encode_start = std::time::Instant::now();
        FrameEncoder::save_to_file(
            &data,
            readback.width,
            readback.height,
            &path,
            self.config.format,
        )?;
        let encode_time = encode_start.elapsed();

        self.metrics_tracker
            .end_capture(copy_time, encode_time, std::time::Duration::ZERO);
        self.frame_count += 1;

        Ok(())
    }

    /// Get latest frame data (for streaming to AI agents)
    ///
    /// Returns raw RGBA8 pixel data without encoding.
    pub fn get_latest_frame_data(
        &self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<Vec<u8>, RendererError> {
        if !self.config.enabled {
            return Err(RendererError::imagecreationfailed(
                0,
                0,
                "Capture not enabled".to_string(),
            ));
        }

        let readback = self.readback.as_ref().ok_or_else(|| {
            RendererError::imagecreationfailed(0, 0, "Readback buffer not initialized".to_string())
        })?;

        readback.copy_image_to_buffer(device, command_pool, queue, image)?;
        readback.get_image_data()
    }

    /// Get latest frame as PNG bytes (for API/streaming)
    ///
    /// Useful for sending frames over network or to AI models.
    pub fn get_latest_frame_png(
        &self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<Vec<u8>, RendererError> {
        let data = self.get_latest_frame_data(device, command_pool, queue, image)?;

        let readback = self.readback.as_ref().unwrap();
        FrameEncoder::encode_png(&data, readback.width, readback.height)
    }

    /// Get capture performance metrics
    pub fn metrics(&self) -> &CaptureMetrics {
        self.metrics_tracker.metrics()
    }

    /// Check if capture is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}
