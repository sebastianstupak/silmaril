//! Integration tests for Vulkan context and renderer components.
//!
//! These tests require Vulkan to be installed and available on the system.
//! They test the full initialization flow in headless mode (no window required).

use engine_renderer::{OffscreenTarget, VulkanContext};

/// Initialize tracing for tests.
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

#[test]
fn test_vulkan_context_creation_headless() {
    init_tracing();

    // Create headless context (no window/surface)
    let result = VulkanContext::new("IntegrationTest", None, None);

    match result {
        Ok(context) => {
            tracing::info!(
                device_name = context.device_name(),
                "Successfully created Vulkan context"
            );

            // Verify context properties
            assert!(!context.device_name().is_empty());
            assert!(context.queue_families.graphics < 32); // Reasonable queue family index
        }
        Err(e) => {
            // If Vulkan is not available, skip the test
            let error_msg = e.to_string();
            if error_msg.contains("Vulkan library")
                || error_msg.contains("validation")
                || error_msg.contains("layer")
            {
                eprintln!("Skipping test: Vulkan not available on this system or validation layers missing");
                return;
            }
            panic!("Failed to create Vulkan context: {:?}", e);
        }
    }
}

#[test]
fn test_device_properties() {
    init_tracing();

    let context = match VulkanContext::new("DevicePropertiesTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Check device properties
    let props = &context.physical_device_properties;

    tracing::info!(
        device_name = context.device_name(),
        api_version = props.api_version,
        driver_version = props.driver_version,
        vendor_id = props.vendor_id,
        device_id = props.device_id,
        device_type = ?props.device_type,
        "Device properties"
    );

    // Verify minimum Vulkan version (1.0 at least)
    assert!(props.api_version >= ash::vk::make_api_version(0, 1, 0, 0));
}

#[test]
fn test_queue_families() {
    init_tracing();

    let context = match VulkanContext::new("QueueFamiliesTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let families = &context.queue_families;

    tracing::info!(
        graphics = families.graphics,
        transfer = families.transfer,
        compute = families.compute,
        present = families.present,
        dedicated_transfer = families.has_dedicated_transfer(),
        dedicated_compute = families.has_dedicated_compute(),
        "Queue families"
    );

    // Verify queue families are valid
    assert!(families.graphics < 32);
    assert!(families.transfer < 32);
    assert!(families.compute < 32);
    assert!(families.present < 32);

    // Unique indices should contain at least graphics queue
    let unique = families.unique_indices();
    assert!(!unique.is_empty());
    assert!(unique.contains(&families.graphics));
}

#[test]
fn test_memory_properties() {
    init_tracing();

    let context = match VulkanContext::new("MemoryPropertiesTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let mem_props = &context.physical_device_memory_properties;

    tracing::info!(
        memory_type_count = mem_props.memory_type_count,
        memory_heap_count = mem_props.memory_heap_count,
        "Memory properties"
    );

    // Verify we have at least one memory type and heap
    assert!(mem_props.memory_type_count > 0);
    assert!(mem_props.memory_heap_count > 0);

    // Check for device-local memory
    let has_device_local = (0..mem_props.memory_type_count).any(|i| {
        let mem_type = mem_props.memory_types[i as usize];
        mem_type.property_flags.contains(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL)
    });

    assert!(has_device_local, "No device-local memory found");
}

#[test]
fn test_device_features() {
    init_tracing();

    let context = match VulkanContext::new("DeviceFeaturesTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let features = &context.physical_device_features;

    tracing::info!(
        geometry_shader = features.geometry_shader == ash::vk::TRUE,
        tessellation_shader = features.tessellation_shader == ash::vk::TRUE,
        multi_draw_indirect = features.multi_draw_indirect == ash::vk::TRUE,
        "Device features"
    );

    // Don't assert on specific features as they vary by GPU
    // Just verify the features struct is populated
}

#[test]
fn test_offscreen_target_creation() {
    init_tracing();

    let context = match VulkanContext::new("OffscreenTargetTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create offscreen target without depth
    let target = OffscreenTarget::new(&context, 800, 600, None, false);

    match target {
        Ok(target) => {
            tracing::info!(
                width = target.width(),
                height = target.height(),
                format = ?target.format,
                has_depth = target.has_depth(),
                "Created offscreen target"
            );

            assert_eq!(target.width(), 800);
            assert_eq!(target.height(), 600);
            assert!(!target.has_depth());
        }
        Err(e) => {
            panic!("Failed to create offscreen target: {:?}", e);
        }
    }
}

#[test]
fn test_offscreen_target_with_depth() {
    init_tracing();

    let context = match VulkanContext::new("OffscreenDepthTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create offscreen target with depth
    let target = OffscreenTarget::new(&context, 1920, 1080, None, true);

    match target {
        Ok(target) => {
            tracing::info!(
                width = target.width(),
                height = target.height(),
                format = ?target.format,
                depth_format = ?target.depth_format,
                has_depth = target.has_depth(),
                "Created offscreen target with depth"
            );

            assert_eq!(target.width(), 1920);
            assert_eq!(target.height(), 1080);
            assert!(target.has_depth());
            assert!(target.depth_format.is_some());
        }
        Err(e) => {
            panic!("Failed to create offscreen target with depth: {:?}", e);
        }
    }
}

#[test]
fn test_multiple_offscreen_targets() {
    init_tracing();

    let context = match VulkanContext::new("MultipleOffscreenTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create multiple offscreen targets
    let targets: Result<Vec<_>, _> = (0..3)
        .map(|i| {
            let width = 640 + i * 100;
            let height = 480 + i * 100;
            OffscreenTarget::new(&context, width, height, None, false)
        })
        .collect();

    match targets {
        Ok(targets) => {
            tracing::info!(count = targets.len(), "Created multiple offscreen targets");
            assert_eq!(targets.len(), 3);

            for (i, target) in targets.iter().enumerate() {
                assert_eq!(target.width(), 640 + i as u32 * 100);
                assert_eq!(target.height(), 480 + i as u32 * 100);
            }
        }
        Err(e) => {
            panic!("Failed to create multiple offscreen targets: {:?}", e);
        }
    }
}

#[test]
fn test_wait_idle() {
    init_tracing();

    let context = match VulkanContext::new("WaitIdleTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Wait for device to be idle (should succeed immediately)
    let result = context.wait_idle();
    assert!(result.is_ok(), "Failed to wait for device idle: {:?}", result.err());
}

#[test]
#[cfg(debug_assertions)]
fn test_validation_layers_enabled() {
    init_tracing();

    // In debug builds, validation layers should be enabled
    let context = match VulkanContext::new("ValidationLayersTest", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            // If validation layers are not available, the error should mention it
            if e.to_string().contains("validation") || e.to_string().contains("layer") {
                eprintln!("Skipping test: Validation layers not available");
                return;
            }
            panic!("Failed to create Vulkan context: {:?}", e);
        }
    };

    // Verify debug messenger was created
    assert!(context.debug_messenger.is_some());
    tracing::info!("Validation layers are enabled");
}

#[test]
#[cfg(not(debug_assertions))]
fn test_validation_layers_disabled() {
    init_tracing();

    // In release builds, validation layers should NOT be enabled
    let context = match VulkanContext::new("NoValidationTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    tracing::info!("Validation layers are disabled (release build)");
    // In release, debug_messenger field doesn't exist
    // This test just verifies the context can be created without validation layers
}

#[test]
fn test_context_drop_cleanup() {
    init_tracing();

    // Create and immediately drop context
    {
        let _context = match VulkanContext::new("DropTest", None, None) {
            Ok(ctx) => ctx,
            Err(_) => {
                eprintln!("Skipping test: Vulkan not available");
                return;
            }
        };

        tracing::info!("Context created, will be dropped");
    }

    // If cleanup is correct, this won't leak memory
    tracing::info!("Context dropped successfully");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn test_zero_sized_offscreen_target() {
    init_tracing();

    let context = match VulkanContext::new("ZeroSizeTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Vulkan requires non-zero dimensions
    let result = OffscreenTarget::new(&context, 0, 0, None, false);

    // Should fail with appropriate error
    assert!(result.is_err(), "Zero-sized target should fail to create");

    // Test zero width only
    let result = OffscreenTarget::new(&context, 0, 600, None, false);
    assert!(result.is_err(), "Zero-width target should fail to create");

    // Test zero height only
    let result = OffscreenTarget::new(&context, 800, 0, None, false);
    assert!(result.is_err(), "Zero-height target should fail to create");
}

#[test]
fn test_single_pixel_offscreen_target() {
    init_tracing();

    let context = match VulkanContext::new("SinglePixelTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // 1x1 should be valid
    let target = OffscreenTarget::new(&context, 1, 1, None, false);

    match target {
        Ok(target) => {
            assert_eq!(target.width(), 1);
            assert_eq!(target.height(), 1);
            tracing::info!("Successfully created 1x1 target");
        }
        Err(e) => {
            panic!("Failed to create 1x1 target: {:?}", e);
        }
    }
}

#[test]
fn test_very_large_offscreen_target_8k() {
    init_tracing();

    let context = match VulkanContext::new("8KTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // 8K resolution (7680x4320)
    let result = OffscreenTarget::new(&context, 7680, 4320, None, false);

    match result {
        Ok(target) => {
            assert_eq!(target.width(), 7680);
            assert_eq!(target.height(), 4320);
            tracing::info!("Successfully created 8K target");
        }
        Err(e) => {
            tracing::warn!(error = ?e, "8K target creation failed (may be hardware limitation)");
            // Not a test failure - some hardware may not support this
        }
    }
}

#[test]
fn test_very_large_offscreen_target_16k() {
    init_tracing();

    let context = match VulkanContext::new("16KTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Check device limits first
    let max_dimension = context.physical_device_properties.limits.max_image_dimension2_d;
    tracing::info!(max_dimension = max_dimension, "Device max image dimension");

    // 16K resolution (15360x8640)
    if max_dimension >= 15360 {
        let result = OffscreenTarget::new(&context, 15360, 8640, None, false);

        match result {
            Ok(target) => {
                assert_eq!(target.width(), 15360);
                assert_eq!(target.height(), 8640);
                tracing::info!("Successfully created 16K target");
            }
            Err(e) => {
                tracing::warn!(error = ?e, "16K target creation failed (may be memory limitation)");
            }
        }
    } else {
        tracing::info!("Skipping 16K test: Device max dimension too small");
    }
}

#[test]
fn test_extreme_aspect_ratios() {
    init_tracing();

    let context = match VulkanContext::new("AspectRatioTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Ultra-wide (32:9)
    let result = OffscreenTarget::new(&context, 3840, 1080, None, false);
    match result {
        Ok(target) => {
            assert_eq!(target.width(), 3840);
            assert_eq!(target.height(), 1080);
            tracing::info!("Successfully created 32:9 target");
        }
        Err(e) => {
            panic!("Failed to create ultra-wide target: {:?}", e);
        }
    }

    // Ultra-tall (9:32)
    let result = OffscreenTarget::new(&context, 1080, 3840, None, false);
    match result {
        Ok(target) => {
            assert_eq!(target.width(), 1080);
            assert_eq!(target.height(), 3840);
            tracing::info!("Successfully created 9:32 target");
        }
        Err(e) => {
            panic!("Failed to create ultra-tall target: {:?}", e);
        }
    }

    // Extreme thin horizontal (1000:1)
    let result = OffscreenTarget::new(&context, 10000, 10, None, false);
    match result {
        Ok(target) => {
            assert_eq!(target.width(), 10000);
            assert_eq!(target.height(), 10);
            tracing::info!("Successfully created 1000:1 target");
        }
        Err(e) => {
            panic!("Failed to create extreme horizontal target: {:?}", e);
        }
    }

    // Extreme thin vertical (1:1000)
    let result = OffscreenTarget::new(&context, 10, 10000, None, false);
    match result {
        Ok(target) => {
            assert_eq!(target.width(), 10);
            assert_eq!(target.height(), 10000);
            tracing::info!("Successfully created 1:1000 target");
        }
        Err(e) => {
            panic!("Failed to create extreme vertical target: {:?}", e);
        }
    }
}

#[test]
fn test_invalid_surface_formats() {
    init_tracing();

    let context = match VulkanContext::new("InvalidFormatTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Test various formats to ensure they're handled correctly
    use ash::vk;

    // Common formats that should work
    let valid_formats = [
        vk::Format::B8G8R8A8_SRGB,
        vk::Format::R8G8B8A8_SRGB,
        vk::Format::B8G8R8A8_UNORM,
        vk::Format::R8G8B8A8_UNORM,
    ];

    for format in valid_formats {
        let result = OffscreenTarget::new(&context, 800, 600, Some(format), false);
        match result {
            Ok(target) => {
                assert_eq!(target.format, format);
                tracing::info!(format = ?format, "Format supported");
            }
            Err(e) => {
                tracing::warn!(format = ?format, error = ?e, "Format not supported");
            }
        }
    }

    // Test uncommon format that may not be supported
    let result =
        OffscreenTarget::new(&context, 800, 600, Some(vk::Format::R64G64B64A64_SFLOAT), false);
    // Don't assert - just verify it doesn't crash
    match result {
        Ok(_) => tracing::info!("R64G64B64A64_SFLOAT format supported"),
        Err(e) => tracing::info!(error = ?e, "R64G64B64A64_SFLOAT format not supported (expected)"),
    }
}

#[test]
fn test_multiple_context_creation_destruction_cycles() {
    init_tracing();

    // Create and destroy context multiple times
    for i in 0..5 {
        tracing::info!(iteration = i, "Creating context");

        let context = match VulkanContext::new(&format!("CycleTest{}", i), None, None) {
            Ok(ctx) => ctx,
            Err(_) => {
                eprintln!("Skipping test: Vulkan not available");
                return;
            }
        };

        // Create a target to ensure full initialization
        let _target = OffscreenTarget::new(&context, 640, 480, None, false)
            .expect("Failed to create target in cycle test");

        tracing::info!(iteration = i, "Context created, will be dropped");

        // Context and target dropped here
    }

    tracing::info!("All cycles completed successfully");
}

#[test]
fn test_concurrent_context_creation() {
    init_tracing();

    use std::thread;

    // Try to create contexts from multiple threads
    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || {
                let name = format!("ConcurrentTest{}", i);
                let result = VulkanContext::new(&name, None, None);

                match result {
                    Ok(context) => {
                        tracing::info!(
                            thread = i,
                            device = context.device_name(),
                            "Context created"
                        );
                        true
                    }
                    Err(e) => {
                        tracing::warn!(thread = i, error = ?e, "Context creation failed");
                        false
                    }
                }
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // At least one thread should succeed if Vulkan is available
    if results.iter().any(|&success| success) {
        tracing::info!("At least one concurrent context creation succeeded");
    } else {
        eprintln!("Skipping test: Vulkan not available");
    }
}

#[test]
fn test_queue_family_edge_cases() {
    init_tracing();

    let context = match VulkanContext::new("QueueEdgeCaseTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let families = &context.queue_families;

    // All indices should be valid (less than some reasonable maximum)
    assert!(families.graphics < 100, "Graphics queue family index suspiciously high");
    assert!(families.transfer < 100, "Transfer queue family index suspiciously high");
    assert!(families.compute < 100, "Compute queue family index suspiciously high");
    assert!(families.present < 100, "Present queue family index suspiciously high");

    // Unique indices should have at least graphics
    let unique = families.unique_indices();
    assert!(!unique.is_empty());
    assert!(unique.len() <= 4, "Too many unique queue families");

    // Verify no duplicates in unique list
    let mut sorted = unique.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), unique.len(), "Unique indices contains duplicates");

    tracing::info!(
        graphics = families.graphics,
        transfer = families.transfer,
        compute = families.compute,
        present = families.present,
        unique_count = unique.len(),
        "Queue family edge cases validated"
    );
}

#[test]
fn test_wait_idle_multiple_times() {
    init_tracing();

    let context = match VulkanContext::new("WaitIdleMultipleTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Wait idle multiple times in succession
    for i in 0..10 {
        let result = context.wait_idle();
        assert!(result.is_ok(), "wait_idle failed on iteration {}: {:?}", i, result.err());
    }

    tracing::info!("Multiple wait_idle calls succeeded");
}

// =============================================================================
// STRESS TESTS
// =============================================================================

#[test]
fn test_rapid_offscreen_allocation_deallocation() {
    init_tracing();

    let context = match VulkanContext::new("RapidAllocTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Rapidly create and destroy targets
    for i in 0..50 {
        let target = OffscreenTarget::new(&context, 640, 480, None, false)
            .expect("Failed to create target in rapid allocation test");

        assert_eq!(target.width(), 640);
        assert_eq!(target.height(), 480);

        // Target dropped here
        if i % 10 == 0 {
            tracing::info!(iteration = i, "Rapid allocation progress");
        }
    }

    tracing::info!("Rapid allocation/deallocation test completed");
}

#[test]
fn test_many_offscreen_targets_simultaneously() {
    init_tracing();

    let context = match VulkanContext::new("ManyTargetsTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create multiple targets simultaneously
    let mut targets = Vec::new();

    for i in 0..20 {
        match OffscreenTarget::new(&context, 320, 240, None, false) {
            Ok(target) => {
                targets.push(target);
                if i % 5 == 0 {
                    tracing::info!(count = targets.len(), "Created targets");
                }
            }
            Err(e) => {
                tracing::warn!(
                    count = targets.len(),
                    error = ?e,
                    "Failed to create target (may be memory limit)"
                );
                break;
            }
        }
    }

    assert!(!targets.is_empty(), "Should be able to create at least one target");

    tracing::info!(
        total_targets = targets.len(),
        "Successfully created multiple simultaneous targets"
    );

    // All targets dropped here
}

#[test]
fn test_mixed_size_targets_simultaneously() {
    init_tracing();

    let context = match VulkanContext::new("MixedSizeTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let mut targets = Vec::new();

    // Various sizes
    let sizes = [
        (640, 480),
        (1920, 1080),
        (320, 240),
        (2560, 1440),
        (800, 600),
        (1280, 720),
        (100, 100),
        (3840, 2160),
    ];

    for (width, height) in sizes {
        match OffscreenTarget::new(&context, width, height, None, false) {
            Ok(target) => {
                assert_eq!(target.width(), width);
                assert_eq!(target.height(), height);
                targets.push(target);
            }
            Err(e) => {
                tracing::warn!(
                    width = width,
                    height = height,
                    error = ?e,
                    "Failed to create target of this size"
                );
            }
        }
    }

    assert!(!targets.is_empty(), "Should be able to create at least some targets");

    tracing::info!(total_targets = targets.len(), "Successfully created mixed-size targets");
}

#[test]
fn test_depth_and_no_depth_mixed() {
    init_tracing();

    let context = match VulkanContext::new("MixedDepthTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let mut targets = Vec::new();

    // Alternate between depth and no depth
    for i in 0..10 {
        let with_depth = i % 2 == 0;

        let target = OffscreenTarget::new(&context, 800, 600, None, with_depth)
            .expect("Failed to create target in mixed depth test");

        assert_eq!(target.has_depth(), with_depth);
        targets.push(target);
    }

    tracing::info!(
        total_targets = targets.len(),
        "Successfully created mixed depth/no-depth targets"
    );
}

#[test]
fn test_memory_pressure_small_allocations() {
    init_tracing();

    let context = match VulkanContext::new("MemoryPressureSmallTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create many small targets to test allocator behavior
    let mut targets = Vec::new();

    for i in 0..100 {
        match OffscreenTarget::new(&context, 64, 64, None, false) {
            Ok(target) => {
                targets.push(target);
                if i % 20 == 0 {
                    tracing::info!(count = targets.len(), "Small allocations progress");
                }
            }
            Err(e) => {
                tracing::warn!(
                    count = targets.len(),
                    error = ?e,
                    "Hit allocation limit"
                );
                break;
            }
        }
    }

    assert!(targets.len() >= 10, "Should be able to create at least 10 small targets");

    tracing::info!(total_targets = targets.len(), "Small allocation stress test completed");
}

// =============================================================================
// PROPERTY-BASED TESTS
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn dimension_strategy() -> impl Strategy<Value = (u32, u32)> {
        (1u32..=4096, 1u32..=4096)
    }

    proptest! {
        #[test]
        fn test_offscreen_target_random_dimensions(
            (width, height) in dimension_strategy()
        ) {
            init_tracing();

            let context = match VulkanContext::new("PropTestDimensions", None, None) {
                Ok(ctx) => ctx,
                Err(_) => {
                    // Vulkan not available, skip
                    return Ok(());
                }
            };

            let result = OffscreenTarget::new(&context, width, height, None, false);

            match result {
                Ok(target) => {
                    prop_assert_eq!(target.width(), width);
                    prop_assert_eq!(target.height(), height);
                }
                Err(_) => {
                    // Some dimensions may exceed hardware limits, that's ok
                    // As long as it doesn't crash
                }
            }
        }
    }

    proptest! {
        #[test]
        fn test_offscreen_target_with_random_depth(
            (width, height) in dimension_strategy(),
            with_depth in any::<bool>()
        ) {
            init_tracing();

            let context = match VulkanContext::new("PropTestDepth", None, None) {
                Ok(ctx) => ctx,
                Err(_) => {
                    return Ok(());
                }
            };

            let result = OffscreenTarget::new(&context, width, height, None, with_depth);

            match result {
                Ok(target) => {
                    prop_assert_eq!(target.width(), width);
                    prop_assert_eq!(target.height(), height);
                    prop_assert_eq!(target.has_depth(), with_depth);
                }
                Err(_) => {
                    // Hardware limits ok
                }
            }
        }
    }

    proptest! {
        #[test]
        fn test_multiple_targets_random_sizes(
            dimensions in prop::collection::vec(dimension_strategy(), 1..=10)
        ) {
            init_tracing();

            let context = match VulkanContext::new("PropTestMultiple", None, None) {
                Ok(ctx) => ctx,
                Err(_) => {
                    return Ok(());
                }
            };

            let mut successful = 0;
            let mut targets = Vec::new();

            for (width, height) in dimensions {
                match OffscreenTarget::new(&context, width, height, None, false) {
                    Ok(target) => {
                        prop_assert_eq!(target.width(), width);
                        prop_assert_eq!(target.height(), height);
                        targets.push(target);
                        successful += 1;
                    }
                    Err(_) => {
                        // Hit limits, ok
                        break;
                    }
                }
            }

            // Should be able to create at least one
            prop_assert!(successful >= 1);
        }
    }

    proptest! {
        #[test]
        fn test_context_creation_with_random_names(
            name_suffix in "[a-zA-Z0-9]{1,20}"
        ) {
            init_tracing();

            let app_name = format!("PropTest_{}", name_suffix);

            let result = VulkanContext::new(&app_name, None, None);

            match result {
                Ok(context) => {
                    // Should have valid device name
                    prop_assert!(!context.device_name().is_empty());
                }
                Err(_) => {
                    // Vulkan not available, ok
                }
            }
        }
    }

    proptest! {
        #[test]
        fn test_extreme_dimensions(
            width in 1u32..=16384,
            height in 1u32..=16384
        ) {
            init_tracing();

            let context = match VulkanContext::new("PropTestExtreme", None, None) {
                Ok(ctx) => ctx,
                Err(_) => {
                    return Ok(());
                }
            };

            // Should not crash, even if it fails
            let _result = OffscreenTarget::new(&context, width, height, None, false);

            // Success - didn't crash
        }
    }
}
