//! Robust benchmarks for Phase 1.6 rendering pipeline with validation layers disabled
//!
//! Based on research findings:
//! - Validation layers MUST be disabled for accurate benchmarks
//! - GPU clock variability affects measurements
//! - Proper synchronization needed between iterations
//!
//! References:
//! - https://mropert.github.io/2026/01/29/benchmarking_vulkan/
//! - https://vulkan-tutorial.com/Drawing_a_triangle/Setup/Validation_layers

#![allow(clippy::print_stdout)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_renderer::*;
use std::time::Duration;

/// Create a lightweight Vulkan context suitable for benchmarking
///
/// Key differences from test context:
/// - No validation layers (for accurate performance measurement)
/// - Minimal extensions
/// - No debug messenger overhead
fn create_bench_context() -> Option<VulkanContext> {
    // Use new_for_benchmarks() which disables validation layers
    // This is critical for accurate performance measurements
    match VulkanContext::new_for_benchmarks("BenchContext", None, None) {
        Ok(ctx) => {
            eprintln!("✅ Vulkan context created (validation layers: DISABLED)");
            Some(ctx)
        }
        Err(e) => {
            eprintln!("❌ Failed to create Vulkan context: {:?}", e);
            None
        }
    }
}

/// Helper to ensure GPU is idle between benchmark iterations
/// This prevents results from being affected by previous work
fn sync_gpu(device: &ash::Device) {
    unsafe {
        // Wait for all queues to be idle
        if let Err(e) = device.device_wait_idle() {
            eprintln!("⚠️  GPU sync failed: {:?}", e);
        }
    }
    // Longer delay to allow driver/GPU state to fully settle
    // This helps prevent driver crashes on Windows
    std::thread::sleep(Duration::from_millis(1));
}

// =============================================================================
// Synchronization Object Benchmarks
// =============================================================================

fn bench_sync_creation(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("sync_objects/create_frame_sync", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync");
                black_box(sync);
                // Sync objects are dropped here, cleaning up Vulkan resources
            }
            start.elapsed()
        });
    });
}

fn bench_sync_reuse(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("sync_objects/reset_fence", |b| {
        let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync");

        b.iter(|| {
            // Benchmark fence reset (common operation in frame loop)
            unsafe {
                context
                    .device
                    .reset_fences(&[sync.in_flight_fence])
                    .expect("Failed to reset fence");
            }
            black_box(&sync);
        });
    });
}

// =============================================================================
// Framebuffer Creation Benchmarks
// =============================================================================

fn bench_framebuffer_creation(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    // Pre-create render pass and offscreen target (one-time setup)
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    let target =
        OffscreenTarget::new(&context, 1920, 1080, None, false).expect("Failed to create target");

    let extent = ash::vk::Extent2D { width: 1920, height: 1080 };

    c.bench_function("framebuffer/create_1080p", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let fb = Framebuffer::new(
                    &context.device,
                    render_pass.handle(),
                    target.color_image_view,
                    extent,
                )
                .expect("Failed to create framebuffer");
                black_box(fb);
                // Framebuffer dropped here
            }
            start.elapsed()
        });
    });
}

fn bench_framebuffer_batch(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create multiple targets for batch test
    let targets: Vec<_> = (0..3)
        .map(|_| OffscreenTarget::new(&context, 1920, 1080, None, false).unwrap())
        .collect();

    let image_views: Vec<_> = targets.iter().map(|t| t.color_image_view).collect();

    let extent = ash::vk::Extent2D { width: 1920, height: 1080 };

    c.bench_function("framebuffer/batch_create_3x", |b| {
        b.iter(|| {
            let framebuffers =
                create_framebuffers(&context.device, render_pass.handle(), &image_views, extent)
                    .expect("Failed to create framebuffers");
            black_box(framebuffers);
        });
    });
}

// =============================================================================
// Render Pass Creation Benchmarks
// =============================================================================

fn bench_render_pass_creation(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let mut group = c.benchmark_group("render_pass");

    // Very low sample count to avoid driver crashes
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("create_basic", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let rp = RenderPass::new(
                    &context.device,
                    RenderPassConfig {
                        color_format: ash::vk::Format::B8G8R8A8_SRGB,
                        depth_format: None,
                        samples: ash::vk::SampleCountFlags::TYPE_1,
                        load_op: ash::vk::AttachmentLoadOp::CLEAR,
                        store_op: ash::vk::AttachmentStoreOp::STORE,
                    },
                )
                .expect("Failed to create render pass");
                black_box(rp);

                // Sync after every iteration to prevent crashes
                sync_gpu(&context.device);
            }
            start.elapsed()
        });
    });

    group.bench_function("create_with_depth", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let rp = RenderPass::new(
                    &context.device,
                    RenderPassConfig {
                        color_format: ash::vk::Format::B8G8R8A8_SRGB,
                        depth_format: Some(ash::vk::Format::D32_SFLOAT),
                        samples: ash::vk::SampleCountFlags::TYPE_1,
                        load_op: ash::vk::AttachmentLoadOp::CLEAR,
                        store_op: ash::vk::AttachmentStoreOp::STORE,
                    },
                )
                .expect("Failed to create render pass");
                black_box(rp);

                // Sync after every iteration
                sync_gpu(&context.device);
            }
            start.elapsed()
        });
    });

    group.finish();
}

// =============================================================================
// Offscreen Target Benchmarks
// =============================================================================

fn bench_offscreen_targets(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let mut group = c.benchmark_group("offscreen_target");

    // Extremely low sample count due to GPU memory allocation
    group.sample_size(5);
    group.measurement_time(Duration::from_secs(20));
    group.warm_up_time(Duration::from_secs(2));

    for (name, width, height) in [("720p", 1280, 720), ("1080p", 1920, 1080), ("1440p", 2560, 1440)]
    {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(width, height),
            |b, &(w, h)| {
                b.iter_custom(|iters| {
                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        let target = OffscreenTarget::new(&context, w, h, None, false)
                            .expect("Failed to create offscreen target");
                        black_box(target);

                        // GPU memory allocation needs time to settle
                        // Sync after EVERY iteration to prevent crashes
                        sync_gpu(&context.device);
                    }
                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

fn bench_offscreen_with_depth(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let mut group = c.benchmark_group("offscreen_target");
    group.sample_size(5);
    group.measurement_time(Duration::from_secs(20));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("1080p_with_depth", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let target = OffscreenTarget::new(&context, 1920, 1080, None, true)
                    .expect("Failed to create offscreen target with depth");
                black_box(target);

                // Sync after EVERY iteration
                sync_gpu(&context.device);
            }
            start.elapsed()
        });
    });

    group.finish();
}

// =============================================================================
// Command Pool Benchmarks
// =============================================================================

fn bench_command_pool_creation(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let mut group = c.benchmark_group("command_pool");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("create_resettable", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for i in 0..iters {
                let pool = CommandPool::new(
                    &context.device,
                    context.queue_families.graphics,
                    ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                )
                .expect("Failed to create pool");
                black_box(pool);

                if i % 10 == 9 {
                    sync_gpu(&context.device);
                }
            }
            start.elapsed()
        });
    });

    group.bench_function("create_transient", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for i in 0..iters {
                let pool = CommandPool::new(
                    &context.device,
                    context.queue_families.graphics,
                    ash::vk::CommandPoolCreateFlags::TRANSIENT,
                )
                .expect("Failed to create pool");
                black_box(pool);

                if i % 10 == 9 {
                    sync_gpu(&context.device);
                }
            }
            start.elapsed()
        });
    });

    group.finish();
}

fn bench_command_buffer_allocation(c: &mut Criterion) {
    let context = match create_bench_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("⚠️  Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create pool");

    let mut group = c.benchmark_group("command_buffer");

    group.bench_function("allocate_primary_1x", |b| {
        b.iter(|| {
            let buffers = pool
                .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 1)
                .expect("Failed to allocate");
            black_box(buffers);
            // Buffers are dropped and freed back to pool
        });
    });

    group.bench_function("allocate_primary_3x", |b| {
        b.iter(|| {
            let buffers = pool
                .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 3)
                .expect("Failed to allocate");
            black_box(buffers);
        });
    });

    group.bench_function("allocate_secondary_1x", |b| {
        b.iter(|| {
            let buffers = pool
                .allocate(&context.device, ash::vk::CommandBufferLevel::SECONDARY, 1)
                .expect("Failed to allocate");
            black_box(buffers);
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(sync_benches, bench_sync_creation, bench_sync_reuse,);

criterion_group!(framebuffer_benches, bench_framebuffer_creation, bench_framebuffer_batch,);

criterion_group!(render_pass_benches, bench_render_pass_creation,);

criterion_group!(offscreen_benches, bench_offscreen_targets, bench_offscreen_with_depth,);

criterion_group!(
    command_pool_benches,
    bench_command_pool_creation,
    bench_command_buffer_allocation,
);

criterion_main!(
    sync_benches,
    framebuffer_benches,
    render_pass_benches,
    offscreen_benches,
    command_pool_benches,
);
