//! Simplified benchmarks for Phase 1.6 modules with correct APIs

#![allow(clippy::print_stdout)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_renderer::*;

fn create_test_context() -> Option<VulkanContext> {
    VulkanContext::new("BenchContext", None, None).ok()
}

fn bench_sync_creation(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("sync_creation", |b| {
        b.iter(|| {
            let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync");
            black_box(sync);
        });
    });
}

fn bench_framebuffer_creation(c: &mut Criterion) {
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

    let target =
        OffscreenTarget::new(&context, 1920, 1080, None, false).expect("Failed to create target");

    c.bench_function("framebuffer_creation", |b| {
        b.iter(|| {
            let fb = Framebuffer::new(
                &context.device,
                render_pass.handle(),
                target.color_image_view,
                ash::vk::Extent2D { width: 1920, height: 1080 },
            )
            .expect("Failed to create framebuffer");
            black_box(fb);
        });
    });
}

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
        });
    });
}

fn bench_offscreen_1080p(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("offscreen_1080p", |b| {
        b.iter(|| {
            let target = OffscreenTarget::new(&context, 1920, 1080, None, false)
                .expect("Failed to create offscreen target");
            black_box(target);
        });
    });
}

fn bench_command_pool(c: &mut Criterion) {
    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping benchmark: Vulkan not available");
            return;
        }
    };

    c.bench_function("command_pool_creation", |b| {
        b.iter(|| {
            let pool = CommandPool::new(
                &context.device,
                context.queue_families.graphics,
                ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .expect("Failed to create pool");
            black_box(pool);
        });
    });
}

criterion_group!(
    benches,
    bench_sync_creation,
    bench_framebuffer_creation,
    bench_render_pass_creation,
    bench_offscreen_1080p,
    bench_command_pool,
);

criterion_main!(benches);
