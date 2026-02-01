//! Benchmark for Vulkan context operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_renderer::VulkanContext;

fn bench_context_creation(c: &mut Criterion) {
    c.bench_function("context_creation", |bench| {
        bench.iter(|| {
            let context =
                VulkanContext::new("BenchContext", None, None).expect("Failed to create context");
            black_box(&context);
            drop(context);
        });
    });
}

fn bench_wait_idle(c: &mut Criterion) {
    let context =
        VulkanContext::new("WaitIdleBench", None, None).expect("Failed to create context");

    c.bench_function("wait_idle", |bench| {
        bench.iter(|| {
            context.wait_idle().expect("Wait idle failed");
        });
    });

    drop(context);
}

criterion_group!(benches, bench_context_creation, bench_wait_idle);
criterion_main!(benches);
