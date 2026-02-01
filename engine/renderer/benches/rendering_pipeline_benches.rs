//! Benchmarks for Phase 1.6 rendering pipeline modules
//!
//! Measures performance of critical rendering operations:
//! - Framebuffer creation/destruction
//! - Command buffer allocation
//! - Synchronization object creation
//! - Render pass operations

#![allow(clippy::print_stdout)] // Benchmarks need to output results

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_renderer::*;

/// Helper to create a test Vulkan context
fn create_test_context() -> Option<VulkanContext> {
    VulkanContext::new("BenchmarkContext", None, None).ok()
}

// =============================================================================
// FRAMEBUFFER BENCHMARKS
// =============================================================================

fn bench_framebuffer_single_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    // Setup render pass
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

    // Create a test image view
    let offscreen = OffscreenTarget::new(&context, 1920, 1080, None, false)
        .expect("Failed to create offscreen target");

    c.bench_function("framebuffer_single_creation", |b| {
        b.iter(|| {
            let fb = Framebuffer::new(
                &context.device,
                render_pass.handle(),
                offscreen.image_view(),
                ash::vk::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            )
            .expect("Failed to create framebuffer");
            black_box(fb);
        });
    });
}

fn bench_framebuffer_batch_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
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

    let mut group = c.benchmark_group("framebuffer_batch_creation");

    for count in [2, 3, 5, 10].iter() {
        // Create image views
        let targets: Vec<_> = (0..*count)
            .map(|_| {
                OffscreenTarget::new(&context, 1920, 1080, None, false)
                    .expect("Failed to create offscreen target")
            })
            .collect();

        let image_views: Vec<_> = targets.iter().map(|t| t.image_view()).collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let framebuffers = create_framebuffers(
                    &context.device,
                    render_pass.handle(),
                    &image_views,
                    ash::vk::Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                )
                .expect("Failed to create framebuffers");
                black_box(framebuffers);
            });
        });
    }

    group.finish();
}

// =============================================================================
// SYNCHRONIZATION BENCHMARKS
// =============================================================================

fn bench_sync_objects_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("sync_single_creation", |b| {
        b.iter(|| {
            let sync = FrameSyncObjects::new(&context.device)
                .expect("Failed to create sync objects");
            black_box(sync);
        });
    });

    let mut group = c.benchmark_group("sync_batch_creation");

    for frames in [2, 3, 5].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(frames), frames, |b, &frames| {
            b.iter(|| {
                let sync_objects = create_sync_objects(&context.device, frames)
                    .expect("Failed to create sync objects");
                black_box(sync_objects);
            });
        });
    }

    group.finish();
}

fn bench_fence_operations(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync objects");

    c.bench_function("fence_wait_and_reset", |b| {
        b.iter(|| {
            sync.wait(&context.device, u64::MAX)
                .expect("Failed to wait");
            sync.reset(&context.device).expect("Failed to reset");
            black_box(&sync);
        });
    });
}

// =============================================================================
// COMMAND BUFFER BENCHMARKS
// =============================================================================

fn bench_command_pool_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("command_pool_creation", |b| {
        b.iter(|| {
            let pool = CommandPool::new(&context.device, context.queue_families.graphics)
                .expect("Failed to create command pool");
            black_box(pool);
        });
    });
}

fn bench_command_buffer_allocation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(&context.device, context.queue_families.graphics)
        .expect("Failed to create command pool");

    let mut group = c.benchmark_group("command_buffer_allocation");

    for count in [1, 2, 5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let buffers: Vec<_> = (0..count)
                    .map(|_| {
                        pool.allocate(&context.device)
                            .expect("Failed to allocate command buffer")
                    })
                    .collect();
                black_box(buffers);
            });
        });
    }

    group.finish();
}

fn bench_command_buffer_begin_end(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(&context.device, context.queue_families.graphics)
        .expect("Failed to create command pool");

    let cmd = pool
        .allocate(&context.device)
        .expect("Failed to allocate command buffer");

    c.bench_function("command_buffer_begin_end", |b| {
        b.iter(|| {
            cmd.begin(&context.device).expect("Failed to begin");
            cmd.end(&context.device).expect("Failed to end");
            black_box(&cmd);
        });
    });
}

// =============================================================================
// RENDER PASS BENCHMARKS
// =============================================================================

fn bench_render_pass_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("render_pass_creation", |b| {
        b.iter(|| {
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
            black_box(render_pass);
        });
    });
}

fn bench_render_pass_with_depth(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("render_pass_with_depth", |b| {
        b.iter(|| {
            let render_pass = RenderPass::new(
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
            black_box(render_pass);
        });
    });
}

// =============================================================================
// OFFSCREEN TARGET BENCHMARKS
// =============================================================================

fn bench_offscreen_target_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    let mut group = c.benchmark_group("offscreen_target_creation");

    let resolutions = [
        ("720p", 1280, 720),
        ("1080p", 1920, 1080),
        ("1440p", 2560, 1440),
        ("4K", 3840, 2160),
    ];

    for (name, width, height) in resolutions.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter(|| {
                let target = OffscreenTarget::new(&context, *width, *height, None, false)
                    .expect("Failed to create offscreen target");
                black_box(target);
            });
        });
    }

    group.finish();
}

fn bench_offscreen_target_with_depth(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("offscreen_target_with_depth_1080p", |b| {
        b.iter(|| {
            let target = OffscreenTarget::new(&context, 1920, 1080, None, true)
                .expect("Failed to create offscreen target");
            black_box(target);
        });
    });
}

// =============================================================================
// FULL PIPELINE SETUP BENCHMARK
// =============================================================================

fn bench_full_pipeline_setup(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("full_pipeline_setup", |b| {
        b.iter(|| {
            // Create render pass
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

            // Create offscreen targets (simulating swapchain images)
            let targets: Vec<_> = (0..3)
                .map(|_| {
                    OffscreenTarget::new(&context, 1920, 1080, None, false)
                        .expect("Failed to create offscreen target")
                })
                .collect();

            let image_views: Vec<_> = targets.iter().map(|t| t.image_view()).collect();

            // Create framebuffers
            let _framebuffers = create_framebuffers(
                &context.device,
                render_pass.handle(),
                &image_views,
                ash::vk::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            )
            .expect("Failed to create framebuffers");

            // Create command pool
            let pool = CommandPool::new(&context.device, context.queue_families.graphics)
                .expect("Failed to create command pool");

            // Allocate command buffers
            let _command_buffers: Vec<_> = (0..3)
                .map(|_| {
                    pool.allocate(&context.device)
                        .expect("Failed to allocate command buffer")
                })
                .collect();

            // Create sync objects for 2 frames in flight
            let _sync_objects = create_sync_objects(&context.device, 2)
                .expect("Failed to create sync objects");

            black_box(&render_pass);
        });
    });
}

criterion_group!(
    framebuffer_benches,
    bench_framebuffer_single_creation,
    bench_framebuffer_batch_creation,
);

criterion_group!(
    sync_benches,
    bench_sync_objects_creation,
    bench_fence_operations,
);

criterion_group!(
    command_benches,
    bench_command_pool_creation,
    bench_command_buffer_allocation,
    bench_command_buffer_begin_end,
);

criterion_group!(
    render_pass_benches,
    bench_render_pass_creation,
    bench_render_pass_with_depth,
);

criterion_group!(
    offscreen_benches,
    bench_offscreen_target_creation,
    bench_offscreen_target_with_depth,
);

criterion_group!(pipeline_benches, bench_full_pipeline_setup,);

criterion_main!(
    framebuffer_benches,
    sync_benches,
    command_benches,
    render_pass_benches,
    offscreen_benches,
    pipeline_benches,
);
