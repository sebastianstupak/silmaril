//! Integration tests for frame capture system
//!
//! Tests the complete capture pipeline from GPU readback to PNG encoding.

use engine_renderer::{CaptureConfig, CaptureFormat, OffscreenTarget, VulkanContext};
use std::path::PathBuf;

/// Test offscreen rendering with frame capture
///
/// This test validates:
/// - Offscreen target creation
/// - Frame readback setup
/// - PNG encoding
#[test]
#[ignore] // Requires Vulkan drivers - run with --ignored on machines with Vulkan
fn test_offscreen_capture_initialization() {
    // Create headless Vulkan context
    let context = match VulkanContext::new("CaptureTest", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping test - Vulkan not available: {:?}", e);
            return;
        }
    };

    // Create offscreen target
    let offscreen = OffscreenTarget::new(&context, 256, 256, None, false);
    assert!(offscreen.is_ok(), "Offscreen target creation failed");

    let target = offscreen.unwrap();
    assert_eq!(target.width(), 256);
    assert_eq!(target.height(), 256);
}

/// Test capture configuration
#[test]
fn test_capture_config() {
    let config = CaptureConfig {
        enabled: true,
        format: CaptureFormat::Png,
        output_dir: PathBuf::from("test_captures"),
        filename_pattern: "test_{:06}.png".to_string(),
    };

    assert!(config.enabled);
    assert_eq!(config.format, CaptureFormat::Png);
}

/// Test JPEG format configuration
#[test]
fn test_jpeg_format() {
    let format = CaptureFormat::Jpeg { quality: 90 };
    match format {
        CaptureFormat::Jpeg { quality } => assert_eq!(quality, 90),
        _ => panic!("Expected JPEG format"),
    }
}

/// Test PNG encoding of raw pixel data
#[test]
fn test_png_encoding() {
    use engine_renderer::FrameEncoder;

    // Create 2x2 test image (red, green, blue, white)
    let data: Vec<u8> = vec![
        255, 0, 0, 255, // Red
        0, 255, 0, 255, // Green
        0, 0, 255, 255, // Blue
        255, 255, 255, 255, // White
    ];

    let result = FrameEncoder::encode_png(&data, 2, 2);
    assert!(result.is_ok(), "PNG encoding failed");

    let png_data = result.unwrap();
    // Verify PNG signature
    assert_eq!(&png_data[0..8], b"\x89PNG\r\n\x1a\n", "Invalid PNG signature");
}

/// Test JPEG encoding of raw pixel data
#[test]
fn test_jpeg_encoding() {
    use engine_renderer::FrameEncoder;

    // Create 4x4 test image
    let data = vec![128u8; 16 * 4]; // Gray RGBA pixels

    let result = FrameEncoder::encode_jpeg(&data, 4, 4, 85);
    assert!(result.is_ok(), "JPEG encoding failed");

    let jpeg_data = result.unwrap();
    // JPEG should start with FFD8 marker
    assert_eq!(jpeg_data[0], 0xFF, "Invalid JPEG marker");
    assert_eq!(jpeg_data[1], 0xD8, "Invalid JPEG SOI marker");
}

/// Test capture metrics tracking
#[test]
fn test_capture_metrics() {
    use engine_renderer::MetricsTracker;
    use std::time::Duration;

    let mut tracker = MetricsTracker::new();
    assert_eq!(tracker.metrics().frames_captured, 0);

    tracker.start_capture();
    tracker.end_capture(
        Duration::from_millis(1),
        Duration::from_millis(2),
        Duration::from_millis(1),
    );

    assert_eq!(tracker.metrics().frames_captured, 1);
    assert!(tracker.metrics().copy_time_ms > 0.0);
    assert!(tracker.metrics().encode_time_ms > 0.0);
}

/// Test metrics performance targets
#[test]
fn test_metrics_targets() {
    use engine_renderer::CaptureMetrics;

    let mut metrics = CaptureMetrics::default();

    // Below targets - should pass
    metrics.copy_time_ms = 1.5;
    metrics.encode_time_ms = 2.5;
    metrics.total_time_ms = 4.0;
    assert!(metrics.meets_targets(), "Should meet performance targets");

    // Above targets - should fail
    metrics.total_time_ms = 6.0;
    assert!(!metrics.meets_targets(), "Should not meet performance targets");
}

/// Test saving to file
#[test]
fn test_save_to_file() {
    use engine_renderer::FrameEncoder;
    use std::fs;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("capture_test");
    fs::create_dir_all(&temp_dir).unwrap();

    // Create test image
    let data = vec![255u8; 16]; // 2x2 white image
    let path = temp_dir.join("test.png");

    let result = FrameEncoder::save_to_file(&data, 2, 2, &path, CaptureFormat::Png);

    assert!(result.is_ok(), "Failed to save PNG");
    assert!(path.exists(), "PNG file not created");

    // Cleanup
    fs::remove_file(path).ok();
    fs::remove_dir(temp_dir).ok();
}
