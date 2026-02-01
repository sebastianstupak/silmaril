//! Integration tests for Vulkan surface creation
//!
//! NOTE: Surface integration testing requires VulkanContext to be created with
//! surface extensions enabled. This creates a chicken-and-egg problem:
//! - Window provides required_extensions()
//! - But VulkanContext needs extensions during creation
//!
//! TODO: When implementing full Renderer in Phase 1.6, create proper integration
//! flow: Window → get extensions → create Instance with extensions → create Surface

use engine_renderer::surface::SurfaceError;

#[test]
fn test_surface_error_display() {
    let err = SurfaceError::CreationFailed { details: "test".to_string() };
    let msg = err.to_string();
    assert!(msg.contains("CreationFailed") || msg.contains("test"));

    let err2 = SurfaceError::QueryFailed { details: "query failed".to_string() };
    let msg2 = err2.to_string();
    assert!(msg2.contains("QueryFailed") || msg2.contains("query failed"));
}

// TODO: Full surface integration tests will be added when implementing Renderer module
// which will handle the proper initialization flow:
// 1. Create Window
// 2. Get required extensions from window
// 3. Create Vulkan Instance with those extensions
// 4. Create Surface from window + instance
// 5. Create VulkanContext with surface
//
// For now, surface.rs unit tests verify the error handling and public API.
// The surface creation logic is tested in the full renderer pipeline tests.
