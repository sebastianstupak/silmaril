# Phase 1.8: Frame Capture System

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (enables AI visual feedback loop)

---

## 🎯 **Objective**

Implement frame capture system to save rendered frames as images (PNG/JPEG) and provide streaming access for AI agents. This closes the visual feedback loop - agents can see what they're rendering.

**Capabilities:**
- Capture rendered frame to CPU
- Save as PNG/JPEG
- Stream frames to memory
- Performance monitoring

---

## 📋 **Detailed Tasks**

### **1. Image Readback** (Day 1-2)

**File:** `engine/renderer/src/capture/readback.rs`

```rust
use image::{ImageBuffer, Rgba};

/// Frame readback manager
pub struct FrameReadback {
    readback_buffer: vk::Buffer,
    readback_allocation: Allocation,
    width: u32,
    height: u32,
}

impl FrameReadback {
    /// Create readback buffer
    pub fn new(
        device: &ash::Device,
        allocator: &mut VulkanAllocator,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // Size for RGBA8 image
        let size = (width * height * 4) as u64;

        // Create CPU-accessible buffer
        let (readback_buffer, readback_allocation) = allocator.allocate_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuToCpu,
        )?;

        tracing::info!("Frame readback buffer created: {}x{}", width, height);

        Ok(Self {
            readback_buffer,
            readback_allocation,
            width,
            height,
        })
    }

    /// Copy swapchain image to readback buffer
    pub fn copy_image_to_buffer(
        &self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<(), RendererError> {
        // Create one-time command buffer
        let command_buffer = Self::begin_single_time_commands(device, command_pool)?;

        // Transition image to TRANSFER_SRC_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .build();

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        // Copy image to buffer
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: self.width,
                height: self.height,
                depth: 1,
            })
            .build();

        unsafe {
            device.cmd_copy_image_to_buffer(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.readback_buffer,
                &[region],
            );
        }

        // Transition back to PRESENT_SRC_KHR
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::empty())
            .build();

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        Self::end_single_time_commands(device, command_pool, queue, command_buffer)?;

        Ok(())
    }

    /// Get image data from readback buffer
    pub fn get_image_data(&self) -> Result<Vec<u8>, RendererError> {
        let size = (self.width * self.height * 4) as usize;
        let mut data = vec![0u8; size];

        unsafe {
            let mapped = self.readback_allocation.mapped_ptr().unwrap().as_ptr() as *const u8;
            std::ptr::copy_nonoverlapping(mapped, data.as_mut_ptr(), size);
        }

        Ok(data)
    }

    /// Helper: begin single-time command buffer
    fn begin_single_time_commands(
        device: &ash::Device,
        command_pool: &CommandPool,
    ) -> Result<vk::CommandBuffer, RendererError> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool.handle())
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&allocate_info)
                .map_err(|e| RendererError::CommandBufferAllocationFailed {
                    details: e.to_string(),
                })?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| RendererError::CommandBufferRecordingFailed {
                    details: e.to_string(),
                })?;
        }

        Ok(command_buffer)
    }

    /// Helper: end and submit single-time command buffer
    fn end_single_time_commands(
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), RendererError> {
        unsafe {
            device
                .end_command_buffer(command_buffer)
                .map_err(|e| RendererError::CommandBufferRecordingFailed {
                    details: e.to_string(),
                })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&[command_buffer])
                .build();

            device
                .queue_submit(queue, &[submit_info], vk::Fence::null())
                .map_err(|e| RendererError::QueueSubmitFailed {
                    details: e.to_string(),
                })?;

            device
                .queue_wait_idle(queue)
                .map_err(|e| RendererError::QueueWaitFailed {
                    details: e.to_string(),
                })?;

            device.free_command_buffers(command_pool.handle(), &[command_buffer]);
        }

        Ok(())
    }
}
```

---

### **2. Image Encoding** (Day 2)

**File:** `engine/renderer/src/capture/encoder.rs`

```rust
use image::{ImageBuffer, ImageFormat, Rgba};

/// Image format for saving
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFormat {
    Png,
    Jpeg { quality: u8 },
}

/// Frame encoder
pub struct FrameEncoder;

impl FrameEncoder {
    /// Encode raw RGBA data to PNG
    pub fn encode_png(
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RendererError> {
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, data.to_vec())
            .ok_or_else(|| RendererError::ImageEncodingFailed {
                details: "Failed to create image buffer".to_string(),
            })?;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        img.write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| RendererError::ImageEncodingFailed {
                details: e.to_string(),
            })?;

        Ok(output)
    }

    /// Encode raw RGBA data to JPEG
    pub fn encode_jpeg(
        data: &[u8],
        width: u32,
        height: u32,
        quality: u8,
    ) -> Result<Vec<u8>, RendererError> {
        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let rgb_data: Vec<u8> = data
            .chunks(4)
            .flat_map(|rgba| &rgba[0..3])
            .copied()
            .collect();

        let img: ImageBuffer<image::Rgb<u8>, _> =
            ImageBuffer::from_raw(width, height, rgb_data)
                .ok_or_else(|| RendererError::ImageEncodingFailed {
                    details: "Failed to create image buffer".to_string(),
                })?;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| RendererError::ImageEncodingFailed {
                details: e.to_string(),
            })?;

        Ok(output)
    }

    /// Save to file
    pub fn save_to_file(
        data: &[u8],
        width: u32,
        height: u32,
        path: &std::path::Path,
        format: CaptureFormat,
    ) -> Result<(), RendererError> {
        let encoded = match format {
            CaptureFormat::Png => Self::encode_png(data, width, height)?,
            CaptureFormat::Jpeg { quality } => Self::encode_jpeg(data, width, height, quality)?,
        };

        std::fs::write(path, encoded).map_err(|e| RendererError::ImageSaveFailed {
            details: e.to_string(),
        })?;

        tracing::info!("Frame saved to {}", path.display());

        Ok(())
    }
}
```

---

### **3. Capture Manager** (Day 2-3)

**File:** `engine/renderer/src/capture/mod.rs`

```rust
mod readback;
mod encoder;

pub use readback::FrameReadback;
pub use encoder::{FrameEncoder, CaptureFormat};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Frame capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub enabled: bool,
    pub format: CaptureFormat,
    pub output_dir: PathBuf,
    pub filename_pattern: String, // e.g., "frame_{:06}.png"
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

/// Frame capture manager
pub struct CaptureManager {
    config: CaptureConfig,
    readback: Option<FrameReadback>,
    frame_count: Arc<Mutex<u64>>,
}

impl CaptureManager {
    /// Create capture manager
    pub fn new(config: CaptureConfig) -> Self {
        // Create output directory
        if config.enabled {
            std::fs::create_dir_all(&config.output_dir).ok();
        }

        Self {
            config,
            readback: None,
            frame_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Initialize readback buffer
    pub fn initialize(
        &mut self,
        device: &ash::Device,
        allocator: &mut VulkanAllocator,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        if self.config.enabled {
            self.readback = Some(FrameReadback::new(device, allocator, width, height)?);
            tracing::info!("Frame capture initialized");
        }
        Ok(())
    }

    /// Capture current frame
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

        let readback = self.readback.as_ref().ok_or_else(|| {
            RendererError::CaptureNotInitialized {
                details: "Readback buffer not initialized".to_string(),
            }
        })?;

        // Copy image to buffer
        readback.copy_image_to_buffer(device, command_pool, queue, image)?;

        // Get image data
        let data = readback.get_image_data()?;

        // Generate filename
        let frame_num = {
            let mut count = self.frame_count.lock().unwrap();
            let num = *count;
            *count += 1;
            num
        };

        let filename = self.config.filename_pattern.replace("{:06}", &format!("{:06}", frame_num));
        let path = self.config.output_dir.join(filename);

        // Save to file
        FrameEncoder::save_to_file(
            &data,
            readback.width,
            readback.height,
            &path,
            self.config.format,
        )?;

        Ok(())
    }

    /// Get latest frame data (for streaming)
    pub fn get_latest_frame_data(
        &self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<Vec<u8>, RendererError> {
        if !self.config.enabled {
            return Err(RendererError::CaptureNotInitialized {
                details: "Capture not enabled".to_string(),
            });
        }

        let readback = self.readback.as_ref().ok_or_else(|| {
            RendererError::CaptureNotInitialized {
                details: "Readback buffer not initialized".to_string(),
            }
        })?;

        readback.copy_image_to_buffer(device, command_pool, queue, image)?;
        readback.get_image_data()
    }

    /// Get latest frame as PNG bytes (for API/streaming)
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
}
```

---

### **4. Integration with Renderer** (Day 3)

**File:** `engine/renderer/src/renderer.rs` (additions)

```rust
use crate::capture::{CaptureConfig, CaptureManager};

pub struct Renderer {
    // ... existing fields
    capture_manager: CaptureManager,
}

impl Renderer {
    pub fn new(window: &dyn WindowBackend, capture_config: CaptureConfig) -> Result<Self, RendererError> {
        // ... existing initialization

        let mut capture_manager = CaptureManager::new(capture_config);
        capture_manager.initialize(device.device(), &mut allocator, width, height)?;

        Ok(Self {
            // ... existing fields
            capture_manager,
        })
    }

    pub fn render_frame(&mut self, clear_color: [f32; 4]) -> Result<(), RendererError> {
        // ... existing render code

        // Capture frame after present
        if self.capture_manager.config.enabled {
            self.capture_manager.capture_frame(
                self.device.device(),
                &self.command_pool,
                self.device.graphics_queue(),
                self.swapchain.images()[image_index as usize],
            )?;
        }

        Ok(())
    }

    /// Get latest frame as PNG (for AI agents)
    pub fn get_frame_png(&self) -> Result<Vec<u8>, RendererError> {
        self.capture_manager.get_latest_frame_png(
            self.device.device(),
            &self.command_pool,
            self.device.graphics_queue(),
            self.swapchain.images()[0], // Current image
        )
    }
}
```

---

### **5. Performance Monitoring** (Day 4)

**File:** `engine/renderer/src/capture/metrics.rs`

```rust
use std::time::{Duration, Instant};

/// Capture performance metrics
#[derive(Debug, Clone)]
pub struct CaptureMetrics {
    pub capture_time_ms: f32,
    pub encode_time_ms: f32,
    pub save_time_ms: f32,
    pub total_time_ms: f32,
    pub frames_captured: u64,
}

/// Metrics tracker
pub struct MetricsTracker {
    metrics: CaptureMetrics,
    last_capture_start: Option<Instant>,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self {
            metrics: CaptureMetrics {
                capture_time_ms: 0.0,
                encode_time_ms: 0.0,
                save_time_ms: 0.0,
                total_time_ms: 0.0,
                frames_captured: 0,
            },
            last_capture_start: None,
        }
    }

    pub fn start_capture(&mut self) {
        self.last_capture_start = Some(Instant::now());
    }

    pub fn end_capture(&mut self, encode_time: Duration, save_time: Duration) {
        if let Some(start) = self.last_capture_start.take() {
            let total = start.elapsed();

            self.metrics.capture_time_ms = (total - encode_time - save_time).as_secs_f32() * 1000.0;
            self.metrics.encode_time_ms = encode_time.as_secs_f32() * 1000.0;
            self.metrics.save_time_ms = save_time.as_secs_f32() * 1000.0;
            self.metrics.total_time_ms = total.as_secs_f32() * 1000.0;
            self.metrics.frames_captured += 1;

            tracing::debug!(
                "Frame capture: {:.2}ms (capture: {:.2}ms, encode: {:.2}ms, save: {:.2}ms)",
                self.metrics.total_time_ms,
                self.metrics.capture_time_ms,
                self.metrics.encode_time_ms,
                self.metrics.save_time_ms
            );
        }
    }

    pub fn metrics(&self) -> &CaptureMetrics {
        &self.metrics
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Frame readback from GPU to CPU works
- [ ] PNG encoding works
- [ ] JPEG encoding works (with quality settings)
- [ ] Frames saved to disk automatically (when enabled)
- [ ] Capture can be enabled/disabled at runtime
- [ ] Latest frame accessible via API (for AI agents)
- [ ] Performance impact < 5ms per frame
- [ ] No memory leaks after 1000+ captures
- [ ] Works on all platforms

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| GPU → CPU copy | < 2ms | < 5ms |
| PNG encoding | < 3ms | < 8ms |
| JPEG encoding | < 2ms | < 5ms |
| Total overhead | < 5ms | < 10ms |

**Throughput:**
- 60 FPS with capture enabled
- 1000+ frames without memory issues

---

## 🧪 **Tests**

```rust
#[test]
fn test_frame_capture() {
    let mut renderer = setup_test_renderer();

    let config = CaptureConfig {
        enabled: true,
        format: CaptureFormat::Png,
        output_dir: PathBuf::from("test_captures"),
        filename_pattern: "test_{:06}.png".to_string(),
    };

    renderer.set_capture_config(config);

    // Render and capture 10 frames
    for _ in 0..10 {
        renderer.render_frame([0.0, 0.0, 0.0, 1.0]).unwrap();
    }

    // Verify files exist
    assert!(PathBuf::from("test_captures/test_000000.png").exists());
    assert!(PathBuf::from("test_captures/test_000009.png").exists());
}

#[test]
fn test_frame_streaming() {
    let renderer = setup_test_renderer();

    // Get frame as PNG
    let png_data = renderer.get_frame_png().unwrap();

    // Should be valid PNG
    assert!(png_data.len() > 0);
    assert_eq!(&png_data[0..8], b"\x89PNG\r\n\x1a\n");
}
```

---

## 💡 **Usage Example (AI Agent)**

```rust
// In AI agent code:
let renderer = Renderer::new(&window, CaptureConfig::default())?;

loop {
    // Render frame
    renderer.render_frame([0.0, 0.0, 0.0, 1.0])?;

    // Get visual feedback
    let frame_png = renderer.get_frame_png()?;

    // Send to AI vision model
    let analysis = vision_model.analyze(&frame_png)?;

    // Make changes based on feedback
    if analysis.contains("too dark") {
        adjust_lighting();
    }
}
```

---

**Dependencies:** [phase1-mesh-rendering.md](phase1-mesh-rendering.md)
**Next:** Phase 2 (Networking)
