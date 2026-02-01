//! Stable benchmarks that reuse a single Vulkan context
//!
//! This avoids crashes from repeated context creation/destruction

#![allow(clippy::print_stdout)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_renderer::*;
use std::sync::Once;
use std::time::{Duration, Instant};

/// Single shared context for all benchmarks (avoids repeated init)
static INIT: Once = Once::new();
static mut BENCH_CONTEXT: Option<VulkanContext> = None;

#[allow(static_mut_refs)]
fn get_or_create_context() -> &'static VulkanContext {
    INIT.call_once(|| {
        eprintln!("🔧 Creating shared Vulkan context (validation layers: DISABLED)...");
        match VulkanContext::new_for_benchmarks("StableBench", None, None) {
            Ok(ctx) => {
                eprintln!("✅ Vulkan context created successfully");
                unsafe {
                    BENCH_CONTEXT = Some(ctx);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to create Vulkan context: {:?}", e);
                panic!("Cannot run benchmarks without Vulkan");
            }
        }
    });

    unsafe { BENCH_CONTEXT.as_ref().unwrap() }
}

fn sync_gpu(device: &ash::Device) {
    unsafe {
        device.device_wait_idle().ok();
    }
    std::thread::sleep(Duration::from_millis(2));
}

fn bench_all_operations(c: &mut Criterion) {
    let context = get_or_create_context();

    // Benchmark 1: Sync object creation
    c.bench_function("01_sync_create", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let sync = FrameSyncObjects::new(&context.device)
                    .expect("Failed to create sync");
                black_box(sync);
            }
            start.elapsed()
        });
    });

    eprintln!("✅ Sync object benchmark complete");
    sync_gpu(&context.device);
    std::thread::sleep(Duration::from_secs(1));

    // Benchmark 2: Fence reset (common operation)
    let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync");
    c.bench_function("02_fence_reset", |b| {
        b.iter(|| {
            unsafe {
                context.device
                    .reset_fences(&[sync.in_flight_fence])
                    .expect("Failed to reset");
            }
            black_box(&sync);
        });
    });

    eprintln!("✅ Fence reset benchmark complete");
    sync_gpu(&context.device);
    std::thread::sleep(Duration::from_secs(1));

    // Benchmark 3: Framebuffer creation (reuse render pass and target)
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

    let target = OffscreenTarget::new(context, 1920, 1080, None, false)
        .expect("Failed to create target");

    let extent = ash::vk::Extent2D {
        width: 1920,
        height: 1080,
    };

    c.bench_function("03_framebuffer_create", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let fb = Framebuffer::new(
                    &context.device,
                    render_pass.handle(),
                    target.color_image_view,
                    extent,
                )
                .expect("Failed to create framebuffer");
                black_box(fb);
            }
            start.elapsed()
        });
    });

    eprintln!("✅ Framebuffer benchmark complete");
    sync_gpu(&context.device);
    std::thread::sleep(Duration::from_secs(1));

    // Benchmark 4: Render pass creation (minimal iterations)
    let mut group = c.benchmark_group("render_pass");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("04_renderpass_basic", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
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
                sync_gpu(&context.device);
            }
            start.elapsed()
        });
    });

    group.finish();
    eprintln!("✅ Render pass benchmark complete");
    sync_gpu(&context.device);
    std::thread::sleep(Duration::from_secs(2));

    // Benchmark 5: Command pool creation
    let mut group = c.benchmark_group("command_pool");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("05_pool_resettable", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let pool = CommandPool::new(
                    &context.device,
                    context.queue_families.graphics,
                    ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                )
                .expect("Failed to create pool");
                black_box(pool);
                sync_gpu(&context.device);
            }
            start.elapsed()
        });
    });

    group.finish();
    eprintln!("✅ Command pool benchmark complete");
    sync_gpu(&context.device);
    std::thread::sleep(Duration::from_secs(2));

    // Benchmark 6: Offscreen target creation (very conservative)
    let mut group = c.benchmark_group("offscreen");
    group.sample_size(5);
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("06_offscreen_1080p", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let target = OffscreenTarget::new(context, 1920, 1080, None, false)
                    .expect("Failed to create offscreen target");
                black_box(target);
                sync_gpu(&context.device);
                std::thread::sleep(Duration::from_millis(10));
            }
            start.elapsed()
        });
    });

    group.finish();
    eprintln!("✅ Offscreen target benchmark complete");
    eprintln!("🎉 ALL BENCHMARKS COMPLETE!");
}

criterion_group!(benches, bench_all_operations);
criterion_main!(benches);
