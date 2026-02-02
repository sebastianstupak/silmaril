//! Benchmarks for depth buffer allocation and management
//!
//! Measures performance of depth buffer creation and destruction
//! to ensure it meets the <1ms allocation target.

use ash::vk;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_renderer::{DepthBuffer, VulkanContext};

/// Helper to create a benchmark Vulkan context
fn create_bench_context() -> VulkanContext {
    VulkanContext::new("DepthBufferBench", None, None).expect("Failed to create Vulkan context")
}

/// Benchmark depth buffer creation for various resolutions
fn bench_depth_buffer_creation(c: &mut Criterion) {
    let context = create_bench_context();
    let device = context.device();
    let allocator = context.allocator();

    let mut group = c.benchmark_group("depth_buffer_creation");

    let resolutions = [
        ("720p", 1280, 720),
        ("1080p", 1920, 1080),
        ("1440p", 2560, 1440),
        ("4K", 3840, 2160),
    ];

    for (name, width, height) in &resolutions {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let extent = vk::Extent2D { width: *w, height: *h };
                    let depth_buffer = DepthBuffer::new(device, allocator, extent)
                        .expect("Failed to create depth buffer");
                    black_box(depth_buffer);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark depth buffer destruction (via Drop)
fn bench_depth_buffer_destruction(c: &mut Criterion) {
    let context = create_bench_context();
    let device = context.device();
    let allocator = context.allocator();

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    c.bench_function("depth_buffer_destruction", |b| {
        b.iter_batched(
            || DepthBuffer::new(device, allocator, extent).expect("Failed to create depth buffer"),
            |depth_buffer| {
                drop(black_box(depth_buffer));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark multiple depth buffer allocations (simulating swapchain resize)
fn bench_depth_buffer_multiple_allocations(c: &mut Criterion) {
    let context = create_bench_context();
    let device = context.device();
    let allocator = context.allocator();

    c.bench_function("depth_buffer_10_allocations", |b| {
        b.iter(|| {
            let extent = vk::Extent2D { width: 1920, height: 1080 };
            let mut buffers = Vec::with_capacity(10);

            for _ in 0..10 {
                let depth_buffer = DepthBuffer::new(device, allocator, extent)
                    .expect("Failed to create depth buffer");
                buffers.push(depth_buffer);
            }

            black_box(buffers);
        });
    });
}

/// Benchmark depth buffer getter methods (should be zero-cost)
fn bench_depth_buffer_getters(c: &mut Criterion) {
    let context = create_bench_context();
    let device = context.device();
    let allocator = context.allocator();

    let extent = vk::Extent2D { width: 1920, height: 1080 };
    let depth_buffer =
        DepthBuffer::new(device, allocator, extent).expect("Failed to create depth buffer");

    c.bench_function("depth_buffer_image_getter", |b| {
        b.iter(|| {
            black_box(depth_buffer.image());
        });
    });

    c.bench_function("depth_buffer_image_view_getter", |b| {
        b.iter(|| {
            black_box(depth_buffer.image_view());
        });
    });

    c.bench_function("depth_buffer_format_getter", |b| {
        b.iter(|| {
            black_box(depth_buffer.format());
        });
    });
}

criterion_group!(
    benches,
    bench_depth_buffer_creation,
    bench_depth_buffer_destruction,
    bench_depth_buffer_multiple_allocations,
    bench_depth_buffer_getters
);

criterion_main!(benches);
