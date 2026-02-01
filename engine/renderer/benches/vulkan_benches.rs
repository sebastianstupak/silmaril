//! Benchmarks for Vulkan initialization and operations.
//!
//! Measures performance of critical paths:
//! - Context creation
//! - Device selection
//! - Offscreen target creation
//! - Memory allocation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_renderer::{OffscreenTarget, VulkanContext};

/// Benchmark Vulkan context creation (full initialization).
fn bench_context_creation(c: &mut Criterion) {
    c.bench_function("vulkan_context_creation", |b| {
        b.iter(|| {
            let context = VulkanContext::new("BenchApp", None, None);
            black_box(context)
        });
    });
}

/// Benchmark device wait idle.
fn bench_wait_idle(c: &mut Criterion) {
    // Create context once
    let context = match VulkanContext::new("WaitIdleBench", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping benchmark: Vulkan not available: {:?}", e);
            return;
        }
    };

    c.bench_function("device_wait_idle", |b| {
        b.iter(|| {
            let result = context.wait_idle();
            black_box(result)
        });
    });
}

/// Benchmark offscreen target creation for various resolutions.
fn bench_offscreen_creation(c: &mut Criterion) {
    let context = match VulkanContext::new("OffscreenBench", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping benchmark: Vulkan not available: {:?}", e);
            return;
        }
    };

    let mut group = c.benchmark_group("offscreen_target_creation");

    // Benchmark different resolutions
    for (width, height) in &[(640, 480), (1920, 1080), (3840, 2160)] {
        group.bench_with_input(
            BenchmarkId::new("no_depth", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let target = OffscreenTarget::new(&context, *w, *h, None, false);
                    black_box(target)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_depth", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let target = OffscreenTarget::new(&context, *w, *h, None, true);
                    black_box(target)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark offscreen target creation and destruction (memory allocation churn).
fn bench_offscreen_alloc_dealloc(c: &mut Criterion) {
    let context = match VulkanContext::new("AllocDeallocBench", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping benchmark: Vulkan not available: {:?}", e);
            return;
        }
    };

    c.bench_function("offscreen_alloc_dealloc_1080p", |b| {
        b.iter(|| {
            // Create and immediately drop (tests allocation + deallocation)
            let target = OffscreenTarget::new(&context, 1920, 1080, None, true).unwrap();
            black_box(target);
            // Drop happens here
        });
    });
}

/// Benchmark multiple offscreen target creation (batch allocation).
fn bench_multiple_offscreen_targets(c: &mut Criterion) {
    let context = match VulkanContext::new("MultipleOffscreenBench", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping benchmark: Vulkan not available: {:?}", e);
            return;
        }
    };

    let mut group = c.benchmark_group("multiple_offscreen_targets");

    for count in &[1, 4, 8, 16] {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let targets: Result<Vec<_>, _> = (0..count)
                    .map(|_| OffscreenTarget::new(&context, 800, 600, None, false))
                    .collect();
                black_box(targets)
            });
        });
    }

    group.finish();
}

/// Benchmark context recreation (measures cleanup + initialization).
fn bench_context_recreation(c: &mut Criterion) {
    c.bench_function("context_recreation", |b| {
        b.iter(|| {
            // Create context
            let context = VulkanContext::new("RecreationBench", None, None).unwrap();
            black_box(&context);
            // Drop context
            drop(context);
            // Create again
            let context2 = VulkanContext::new("RecreationBench", None, None).unwrap();
            black_box(context2)
        });
    });
}

/// Benchmark queue family lookup.
fn bench_queue_families(c: &mut Criterion) {
    let context = match VulkanContext::new("QueueFamilyBench", None, None) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping benchmark: Vulkan not available: {:?}", e);
            return;
        }
    };

    c.bench_function("queue_families_unique_indices", |b| {
        b.iter(|| {
            let unique = context.queue_families.unique_indices();
            black_box(unique)
        });
    });

    c.bench_function("queue_families_checks", |b| {
        b.iter(|| {
            let dedicated_transfer = context.queue_families.has_dedicated_transfer();
            let dedicated_compute = context.queue_families.has_dedicated_compute();
            black_box((dedicated_transfer, dedicated_compute))
        });
    });
}

criterion_group!(
    benches,
    bench_context_creation,
    bench_wait_idle,
    bench_offscreen_creation,
    bench_offscreen_alloc_dealloc,
    bench_multiple_offscreen_targets,
    bench_context_recreation,
    bench_queue_families,
);

criterion_main!(benches);
