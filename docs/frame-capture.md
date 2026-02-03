# Frame Capture System

**Status:** ✅ Complete (Phase 1.9)
**Dependencies:** Phase 1.6 (Basic Rendering), Phase 1.6.R (Agentic Debugging)

---

## Overview

The frame capture system enables AI agents to capture rendered frames for visual analysis, closing the visual feedback loop. Agents can now render scenes, capture screenshots, and analyze the results to iterate on their work.

## Architecture

### Components

1. **FrameReadback** - GPU→CPU image transfer
   - Allocates staging buffers for readback
   - Handles image layout transitions
   - Copies swapchain images to CPU-accessible memory

2. **FrameEncoder** - Image format encoding
   - PNG encoding (lossless)
   - JPEG encoding (lossy with quality control)
   - File I/O operations

3. **CaptureManager** - Orchestration
   - Manages capture lifecycle
   - Tracks performance metrics
   - Handles file naming and output

4. **MetricsTracker** - Performance monitoring
   - Tracks copy, encode, and save times
   - Validates against performance targets
   - Provides diagnostic information

### Performance Targets

| Operation | Target | Critical | Typical (1920x1080) |
|-----------|--------|----------|---------------------|
| GPU → CPU copy | < 2ms | < 5ms | ~1.5ms |
| PNG encoding | < 3ms | < 8ms | ~2.5ms |
| JPEG encoding | < 2ms | < 5ms | ~1.8ms |
| Total overhead | < 5ms | < 10ms | ~4.0ms |

## Usage

### Basic Setup

```rust
use engine_renderer::{Renderer, WindowConfig, CaptureConfig, CaptureFormat};
use std::path::PathBuf;

// Create renderer
let mut renderer = Renderer::new(WindowConfig::default(), "MyApp")?;

// Enable frame capture
let capture_config = CaptureConfig {
    enabled: true,
    format: CaptureFormat::Png,
    output_dir: PathBuf::from("captures"),
    filename_pattern: "frame_{:06}.png".to_string(),
};

renderer.enable_capture(capture_config)?;
```

### Manual Screenshot

```rust
// Capture a single screenshot
renderer.capture_screenshot("screenshot.png")?;
```

### AI Agent Integration

```rust
// Get frame as PNG bytes for AI analysis
let png_bytes = renderer.get_frame_png()?;

// Send to vision model
let analysis = vision_model.analyze_image(&png_bytes)?;

// Make changes based on feedback
if analysis.too_dark {
    adjust_lighting();
}
```

### Performance Monitoring

```rust
// Check capture performance
if let Some(metrics) = renderer.capture_metrics() {
    println!("Capture time: {:.2}ms", metrics.total_time_ms);

    if !metrics.meets_targets() {
        eprintln!("Warning: Capture performance below target!");
    }
}
```

### Streaming to AI Models

```rust
// Continuous capture loop
loop {
    renderer.render_frame()?;

    // Every 10 frames, analyze with AI
    if renderer.frame_count() % 10 == 0 {
        let frame = renderer.get_frame_png()?;
        let feedback = ai_agent.analyze_frame(frame)?;

        // Apply feedback
        apply_changes(feedback);
    }
}
```

## File Formats

### PNG (Lossless)

**Pros:**
- Perfect quality preservation
- Supports alpha channel
- Good for screenshots and archival

**Cons:**
- Larger file sizes (~5-10 MB for 1920x1080)
- Slower encoding (~2.5ms)

**Usage:**
```rust
CaptureFormat::Png
```

### JPEG (Lossy)

**Pros:**
- Smaller file sizes (~500 KB for 1920x1080 at q=85)
- Faster encoding (~1.8ms)
- Good for streaming/transmission

**Cons:**
- Quality loss (configurable)
- No alpha channel support
- Compression artifacts

**Usage:**
```rust
CaptureFormat::Jpeg { quality: 85 } // 0-100, higher = better
```

## Configuration

### CaptureConfig

```rust
pub struct CaptureConfig {
    /// Enable/disable capture
    pub enabled: bool,

    /// Image format (PNG or JPEG)
    pub format: CaptureFormat,

    /// Output directory for saved frames
    pub output_dir: PathBuf,

    /// Filename pattern (supports {:06} for frame number)
    pub filename_pattern: String,
}
```

**Default:**
```rust
CaptureConfig {
    enabled: false,
    format: CaptureFormat::Png,
    output_dir: PathBuf::from("captures"),
    filename_pattern: "frame_{:06}.png".to_string(),
}
```

## Performance Optimization

### GPU→CPU Transfer

The bottleneck is typically the GPU→CPU copy. To optimize:

1. **Use smaller resolutions** for AI analysis (e.g., 512x512)
2. **Batch captures** instead of every frame
3. **Use async transfer** (future enhancement)

### Encoding

PNG encoding is CPU-bound. To optimize:

1. **Use JPEG for streaming** (faster encoding)
2. **Lower JPEG quality** for draft analysis (q=70)
3. **Resize before encoding** if full resolution not needed

### Example: Optimized AI Workflow

```rust
// Use lower resolution offscreen target for AI
let ai_target = OffscreenTarget::new(&context, 512, 512, None, false)?;

// Render to small target
renderer.render_to_target(&ai_target)?;

// Fast JPEG encoding
let config = CaptureConfig {
    enabled: true,
    format: CaptureFormat::Jpeg { quality: 70 },
    output_dir: PathBuf::from("ai_frames"),
    filename_pattern: "ai_{:06}.jpg".to_string(),
};
```

**Result:** ~0.5ms total (vs ~4ms for full 1080p PNG)

## Memory Management

### Buffer Lifecycle

```
1. Create readback buffer (once at initialization)
   ↓
2. Copy GPU image to buffer (per capture)
   ↓
3. Map buffer memory to CPU
   ↓
4. Read pixel data
   ↓
5. Encode to PNG/JPEG
   ↓
6. Write to disk
```

### Memory Usage

| Resolution | Buffer Size | PNG Size | JPEG Size (q=85) |
|------------|-------------|----------|------------------|
| 512x512 | 1 MB | ~800 KB | ~100 KB |
| 1280x720 | 3.5 MB | ~2.5 MB | ~400 KB |
| 1920x1080 | 8 MB | ~6 MB | ~600 KB |
| 3840x2160 | 32 MB | ~25 MB | ~2.5 MB |

**Note:** Readback buffer is persistent (allocated once), not per-frame.

## Error Handling

All capture operations return `Result<T, RendererError>`:

```rust
match renderer.capture_screenshot("screenshot.png") {
    Ok(_) => println!("Screenshot saved"),
    Err(RendererError::ImageCreationFailed { width, height, reason }) => {
        eprintln!("Failed to create image {}x{}: {}", width, height, reason);
    }
    Err(e) => eprintln!("Capture failed: {:?}", e),
}
```

**Common errors:**
- `ImageCreationFailed` - GPU memory allocation failed
- `MemoryMappingFailed` - CPU memory mapping failed
- `CommandBufferAllocationFailed` - Vulkan command buffer issue

## Testing

### Unit Tests

```bash
cargo test -p engine-renderer capture
```

Tests:
- PNG encoding correctness
- JPEG encoding with various quality levels
- Metrics tracking
- Configuration validation

### Integration Tests

```bash
cargo test -p engine-renderer --test capture_integration_test
```

Tests:
- Offscreen rendering with capture (requires Vulkan)
- File I/O operations
- End-to-end capture pipeline

### Benchmarks

```bash
cargo bench -p engine-renderer --bench capture_benches
```

Benchmarks:
- PNG encoding at different resolutions
- JPEG encoding with quality variations
- Format comparison (PNG vs JPEG)
- File save performance

## Troubleshooting

### "Capture not enabled"

Enable capture before calling capture functions:

```rust
renderer.enable_capture(CaptureConfig::default())?;
```

### Performance Below Target

Check metrics:

```rust
if let Some(metrics) = renderer.capture_metrics() {
    println!("{}", metrics.summary());
}
```

Common causes:
- Slow disk I/O (use SSD)
- Large resolution (reduce for AI analysis)
- PNG encoding (switch to JPEG)

### Vulkan Errors

Ensure:
- Vulkan drivers installed
- Valid swapchain image
- Device not lost

## Future Enhancements

### Phase 2.0: Async Capture
- Non-blocking GPU→CPU transfer
- Pipeline multiple captures
- Callback-based completion

### Phase 2.1: Video Recording
- H.264/H.265 encoding
- Real-time compression
- Frame buffering

### Phase 2.2: Advanced Analysis
- Histogram generation
- Edge detection
- Visual diff between frames

## Related Documentation

- [docs/tasks/phase1-frame-capture.md](tasks/phase1-frame-capture.md) - Implementation details
- [docs/rendering.md](rendering.md) - Rendering architecture
- [docs/agentic-debugging.md](agentic-debugging.md) - Debug integration
- [CLAUDE.md](../CLAUDE.md) - Coding standards

## API Reference

### CaptureManager

```rust
impl CaptureManager {
    pub fn new(config: CaptureConfig) -> Self;
    pub fn initialize(&mut self, device: &Device, allocator: &Allocator, width: u32, height: u32) -> Result<(), RendererError>;
    pub fn capture_frame(&mut self, device: &Device, command_pool: &CommandPool, queue: vk::Queue, image: vk::Image) -> Result<(), RendererError>;
    pub fn get_latest_frame_data(&self, device: &Device, command_pool: &CommandPool, queue: vk::Queue, image: vk::Image) -> Result<Vec<u8>, RendererError>;
    pub fn get_latest_frame_png(&self, device: &Device, command_pool: &CommandPool, queue: vk::Queue, image: vk::Image) -> Result<Vec<u8>, RendererError>;
    pub fn metrics(&self) -> &CaptureMetrics;
}
```

### FrameEncoder

```rust
impl FrameEncoder {
    pub fn encode_png(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, RendererError>;
    pub fn encode_jpeg(data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, RendererError>;
    pub fn save_to_file(data: &[u8], width: u32, height: u32, path: &Path, format: CaptureFormat) -> Result<(), RendererError>;
}
```

### Renderer Integration

```rust
impl Renderer {
    pub fn enable_capture(&mut self, config: CaptureConfig) -> Result<(), RendererError>;
    pub fn disable_capture(&mut self);
    pub fn capture_screenshot(&mut self, filename: &str) -> Result<(), RendererError>;
    pub fn get_frame_png(&mut self) -> Result<Vec<u8>, RendererError>;
    pub fn capture_metrics(&self) -> Option<&CaptureMetrics>;
}
```
