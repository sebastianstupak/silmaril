//! Tests for proper resource cleanup and destruction order
//!
//! These tests verify that the renderer cleans up Vulkan resources
//! in the correct order without memory leaks or access violations.

use engine_renderer::{Renderer, WindowConfig};

/// Test that renderer can be created and destroyed cleanly
#[test]
fn test_renderer_clean_shutdown() {
    // Create renderer in headless mode
    let config = WindowConfig {
        title: "Cleanup Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false, // Headless for testing
    };

    let renderer = Renderer::new(config, "CleanupTest").expect("Failed to create renderer");

    // Wait for GPU to finish any pending work
    renderer.wait_idle().expect("Failed to wait for idle");

    // Drop renderer - this should clean up all resources
    drop(renderer);

    // If we get here without crashes or leaks, test passes
}

/// Test that renderer can render frames and then shut down cleanly
#[test]
fn test_renderer_render_then_shutdown() {
    let config = WindowConfig {
        title: "Render Cleanup Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false,
    };

    let mut renderer =
        Renderer::new(config, "RenderCleanupTest").expect("Failed to create renderer");

    // Render a few frames
    for _ in 0..3 {
        renderer.set_clear_color(1.0, 0.0, 0.0, 1.0);
        renderer.render_frame().expect("Failed to render frame");
    }

    // Wait for GPU to finish
    renderer.wait_idle().expect("Failed to wait for idle");

    // Drop - should clean up without leaks or crashes
    drop(renderer);
}

/// Test multiple create/destroy cycles
#[test]
fn test_multiple_renderer_cycles() {
    let config = WindowConfig {
        title: "Cycle Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false,
    };

    // Create and destroy renderer 3 times
    for i in 0..3 {
        let renderer = Renderer::new(config.clone(), &format!("CycleTest{}", i))
            .expect("Failed to create renderer");

        renderer.wait_idle().expect("Failed to wait for idle");
        drop(renderer);
    }
}

/// Test that depth buffer resources are properly cleaned up
#[test]
fn test_depth_buffer_cleanup() {
    let config = WindowConfig {
        title: "Depth Buffer Cleanup Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false,
    };

    let renderer = Renderer::new(config, "DepthCleanupTest").expect("Failed to create renderer");

    // The renderer creates a depth buffer internally
    // When dropped, it should clean up the depth buffer without leaks

    renderer.wait_idle().expect("Failed to wait for idle");
    drop(renderer);

    // This test specifically checks for the 4MB depth buffer leak
    // If the allocator reports leaks, this test should fail
}

/// Test that framebuffers are destroyed before depth buffer
#[test]
fn test_framebuffer_depth_destruction_order() {
    let config = WindowConfig {
        title: "Destruction Order Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false,
    };

    let renderer = Renderer::new(config, "OrderTest").expect("Failed to create renderer");

    // Framebuffers reference the depth buffer
    // They MUST be destroyed before the depth buffer
    // Otherwise we get access violations

    renderer.wait_idle().expect("Failed to wait for idle");
    drop(renderer);
}

/// Test cleanup after rendering with complex state
#[test]
fn test_cleanup_after_complex_rendering() {
    let config = WindowConfig {
        title: "Complex Cleanup Test".to_string(),
        width: 1280,
        height: 720,
        resizable: false,
        visible: false,
    };

    let mut renderer = Renderer::new(config, "ComplexTest").expect("Failed to create renderer");

    // Render multiple frames with different clear colors
    // This exercises more of the rendering pipeline
    for i in 0..10 {
        let phase = i as f32 * 0.1;
        renderer.set_clear_color(phase, 1.0 - phase, 0.5, 1.0);
        renderer.render_frame().expect("Failed to render frame");
    }

    renderer.wait_idle().expect("Failed to wait for idle");
    drop(renderer);
}
