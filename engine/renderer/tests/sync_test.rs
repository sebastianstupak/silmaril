//! Integration tests for synchronization primitives
//!
//! Tests the FrameSync API for managing frames in flight.

use ash::vk;
use engine_renderer::{FrameSync, VulkanContext};

/// Helper to create a test Vulkan device
fn create_test_device() -> (VulkanContext, ash::Device) {
    let context =
        VulkanContext::new("SyncTest", None, None).expect("Failed to create Vulkan context");
    let device = context.device.clone();
    (context, device)
}

#[test]
fn test_frame_sync_creation() {
    let (_context, device) = create_test_device();

    // Create FrameSync with 2 frames
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Verify initial state
    assert_eq!(sync.current_frame, 0, "Initial current_frame should be 0");
    assert_eq!(sync.frames_in_flight, 2, "frames_in_flight should be 2");
    assert_eq!(
        sync.image_available_semaphores.len(),
        2,
        "Should have 2 image_available semaphores"
    );
    assert_eq!(
        sync.render_finished_semaphores.len(),
        2,
        "Should have 2 render_finished semaphores"
    );
    assert_eq!(sync.in_flight_fences.len(), 2, "Should have 2 in_flight fences");

    // Verify semaphores are valid (not null)
    for (i, &semaphore) in sync.image_available_semaphores.iter().enumerate() {
        assert_ne!(semaphore, vk::Semaphore::null(), "image_available[{}] should not be null", i);
    }
    for (i, &semaphore) in sync.render_finished_semaphores.iter().enumerate() {
        assert_ne!(semaphore, vk::Semaphore::null(), "render_finished[{}] should not be null", i);
    }
    for (i, &fence) in sync.in_flight_fences.iter().enumerate() {
        assert_ne!(fence, vk::Fence::null(), "fence[{}] should not be null", i);
    }

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_frame_advancement() {
    let (_context, device) = create_test_device();
    let mut sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Initial state
    assert_eq!(sync.current_frame, 0);

    // Advance frame: 0 -> 1
    sync.advance_frame();
    assert_eq!(sync.current_frame, 1);

    // Advance frame: 1 -> 0 (cycling)
    sync.advance_frame();
    assert_eq!(sync.current_frame, 0);

    // Advance frame: 0 -> 1
    sync.advance_frame();
    assert_eq!(sync.current_frame, 1);

    // Verify cycling pattern continues
    sync.advance_frame();
    assert_eq!(sync.current_frame, 0);

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_frame_advancement_three_frames() {
    let (_context, device) = create_test_device();
    let mut sync = FrameSync::create(&device, 3).expect("Failed to create FrameSync with 3 frames");

    // Verify cycling: 0 -> 1 -> 2 -> 0
    assert_eq!(sync.current_frame, 0);

    sync.advance_frame();
    assert_eq!(sync.current_frame, 1);

    sync.advance_frame();
    assert_eq!(sync.current_frame, 2);

    sync.advance_frame();
    assert_eq!(sync.current_frame, 0);

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_wait_for_initial_frame() {
    let (_context, device) = create_test_device();
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Wait for frame should succeed immediately because fence starts signaled
    sync.wait_for_frame(&device)
        .expect("wait_for_frame() should succeed for initial signaled fence");

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_fence_reset() {
    let (_context, device) = create_test_device();
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Wait for frame (should succeed immediately - fence is signaled)
    sync.wait_for_frame(&device).expect("Failed to wait for frame");

    // Reset fence (unsignal it)
    sync.reset_fence(&device).expect("Failed to reset fence");

    // Verify fence is now unsignaled by checking fence status
    let fence = sync.in_flight_fences[sync.current_frame];
    let status = unsafe { device.get_fence_status(fence) };

    // Fence should be unsignaled after reset
    // get_fence_status returns Ok(true) if signaled, Ok(false) if unsignaled
    match status {
        Ok(is_signaled) => {
            assert!(!is_signaled, "Fence should be unsignaled after reset");
        }
        Err(e) => {
            panic!("Failed to query fence status: {:?}", e);
        }
    }

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_current_frame_resources() {
    let (_context, device) = create_test_device();
    let mut sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Get resources for frame 0
    let resources_0 = sync.current_frame_resources();
    assert_eq!(resources_0.image_available, sync.image_available_semaphores[0]);
    assert_eq!(resources_0.render_finished, sync.render_finished_semaphores[0]);
    assert_eq!(resources_0.in_flight_fence, sync.in_flight_fences[0]);

    // Advance to frame 1
    sync.advance_frame();
    let resources_1 = sync.current_frame_resources();
    assert_eq!(resources_1.image_available, sync.image_available_semaphores[1]);
    assert_eq!(resources_1.render_finished, sync.render_finished_semaphores[1]);
    assert_eq!(resources_1.in_flight_fence, sync.in_flight_fences[1]);

    // Verify different frames have different resources
    assert_ne!(resources_0.image_available, resources_1.image_available);
    assert_ne!(resources_0.render_finished, resources_1.render_finished);
    assert_ne!(resources_0.in_flight_fence, resources_1.in_flight_fence);

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_multiple_frames_in_flight() {
    let (_context, device) = create_test_device();

    // Test creating sync objects for different frame counts
    for frame_count in [1, 2, 3, 4] {
        let sync = FrameSync::create(&device, frame_count)
            .unwrap_or_else(|_| panic!("Failed to create FrameSync with {} frames", frame_count));

        assert_eq!(sync.frames_in_flight, frame_count);
        assert_eq!(sync.image_available_semaphores.len(), frame_count);
        assert_eq!(sync.render_finished_semaphores.len(), frame_count);
        assert_eq!(sync.in_flight_fences.len(), frame_count);

        sync.destroy(&device);
    }
}

#[test]
fn test_sync_cleanup() {
    let (_context, device) = create_test_device();

    // Create sync objects
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Store handles for verification
    let image_available_0 = sync.image_available_semaphores[0];
    let render_finished_0 = sync.render_finished_semaphores[0];
    let fence_0 = sync.in_flight_fences[0];

    // Destroy sync objects
    sync.destroy(&device);

    // After destroy, attempting to wait on the fence should fail (or behave as invalid)
    // We can't directly verify destruction without validation layers,
    // but we can at least verify the function executes without panic

    // This test mainly verifies that destroy() doesn't crash
    // Validation layers would catch use-after-free if we tried to use destroyed objects
    let _ = image_available_0;
    let _ = render_finished_0;
    let _ = fence_0;
}

#[test]
fn test_wait_and_reset_workflow() {
    let (_context, device) = create_test_device();
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Simulate the typical frame workflow

    // 1. Wait for frame (fence is initially signaled)
    sync.wait_for_frame(&device).expect("Failed to wait for frame");

    // 2. Reset fence for next submission
    sync.reset_fence(&device).expect("Failed to reset fence");

    // 3. In real rendering, we would:
    //    - acquire_next_image with image_available semaphore
    //    - record commands
    //    - submit with fence and signal render_finished semaphore
    //    - present with render_finished semaphore

    // For this test, we just verify the wait/reset pattern works

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_multiple_wait_on_signaled_fence() {
    let (_context, device) = create_test_device();
    let sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Waiting multiple times on a signaled fence should succeed
    sync.wait_for_frame(&device).expect("First wait should succeed");

    sync.wait_for_frame(&device)
        .expect("Second wait should succeed (fence still signaled)");

    sync.wait_for_frame(&device)
        .expect("Third wait should succeed (fence still signaled)");

    // Cleanup
    sync.destroy(&device);
}

#[test]
fn test_frame_cycling_with_resources() {
    let (_context, device) = create_test_device();
    let mut sync = FrameSync::create(&device, 2).expect("Failed to create FrameSync");

    // Collect resources for multiple frame cycles
    let mut seen_frames = vec![];

    for _ in 0..6 {
        let resources = sync.current_frame_resources();
        seen_frames.push((
            sync.current_frame,
            resources.image_available,
            resources.render_finished,
            resources.in_flight_fence,
        ));
        sync.advance_frame();
    }

    // Verify we cycled through frames: 0, 1, 0, 1, 0, 1
    assert_eq!(seen_frames[0].0, 0);
    assert_eq!(seen_frames[1].0, 1);
    assert_eq!(seen_frames[2].0, 0);
    assert_eq!(seen_frames[3].0, 1);
    assert_eq!(seen_frames[4].0, 0);
    assert_eq!(seen_frames[5].0, 1);

    // Verify resources match for same frame indices
    assert_eq!(seen_frames[0].1, seen_frames[2].1); // Frame 0 image_available
    assert_eq!(seen_frames[0].2, seen_frames[2].2); // Frame 0 render_finished
    assert_eq!(seen_frames[0].3, seen_frames[2].3); // Frame 0 fence

    assert_eq!(seen_frames[1].1, seen_frames[3].1); // Frame 1 image_available
    assert_eq!(seen_frames[1].2, seen_frames[3].2); // Frame 1 render_finished
    assert_eq!(seen_frames[1].3, seen_frames[3].3); // Frame 1 fence

    // Cleanup
    sync.destroy(&device);
}
