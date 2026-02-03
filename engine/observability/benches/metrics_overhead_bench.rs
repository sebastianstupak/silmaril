//! Benchmarks for Prometheus metrics overhead (Quick Win #2)
//!
//! Measures the performance impact of metrics recording.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

#[cfg(feature = "metrics")]
use engine_observability::metrics::MetricsRegistry;

fn bench_metrics_recording(c: &mut Criterion) {
    #[cfg(feature = "metrics")]
    {
        let mut group = c.benchmark_group("metrics_recording");

        let registry = MetricsRegistry::new();

        // Benchmark recording frame time
        group.bench_function("record_frame_time", |b| {
            b.iter(|| {
                registry.record_frame_time(black_box(16.67));
            });
        });

        // Benchmark recording tick duration
        group.bench_function("record_tick_duration", |b| {
            b.iter(|| {
                registry.record_tick_duration(black_box(15.5));
            });
        });

        // Benchmark setting entity count
        group.bench_function("set_entity_count", |b| {
            b.iter(|| {
                registry.set_entity_count(black_box(1000));
            });
        });

        // Benchmark recording network metrics
        group.bench_function("record_network_bytes_sent", |b| {
            b.iter(|| {
                registry.record_network_bytes_sent(black_box(1024));
            });
        });

        group.bench_function("record_network_latency", |b| {
            b.iter(|| {
                registry.record_network_latency(black_box(0.05));
            });
        });

        group.finish();
    }
}

fn bench_metrics_throughput(c: &mut Criterion) {
    #[cfg(feature = "metrics")]
    {
        let mut group = c.benchmark_group("metrics_throughput");
        group.throughput(Throughput::Elements(1000));

        let registry = MetricsRegistry::new();

        // Benchmark recording 1000 metrics
        group.bench_function("record_1000_frame_times", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    registry.record_frame_time(black_box(16.0 + (i as f64 % 10.0)));
                }
            });
        });

        // Benchmark mixed metrics
        group.bench_function("record_1000_mixed_metrics", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    match i % 5 {
                        0 => registry.record_frame_time(black_box(16.67)),
                        1 => registry.record_tick_duration(black_box(15.0)),
                        2 => registry.set_entity_count(black_box(1000 + i as i64)),
                        3 => registry.record_network_bytes_sent(black_box(1024)),
                        4 => registry.record_network_latency(black_box(0.05)),
                        _ => unreachable!(),
                    }
                }
            });
        });

        group.finish();
    }
}

fn bench_metrics_overhead_in_game_loop(c: &mut Criterion) {
    #[cfg(feature = "metrics")]
    {
        let mut group = c.benchmark_group("metrics_game_loop_overhead");

        let registry = MetricsRegistry::new();

        // Simulate a minimal game loop with metrics
        group.bench_function("game_loop_with_metrics", |b| {
            b.iter(|| {
                let start = std::time::Instant::now();

                // Simulate game logic
                let entity_count = black_box(1000);
                black_box(std::hint::black_box(entity_count * 2));

                // Record metrics
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                registry.record_frame_time(elapsed);
                registry.set_entity_count(entity_count);
            });
        });

        // Baseline without metrics
        group.bench_function("game_loop_without_metrics", |b| {
            b.iter(|| {
                let _start = std::time::Instant::now();

                // Same game logic
                let entity_count = black_box(1000);
                black_box(std::hint::black_box(entity_count * 2));

                // No metrics recording
            });
        });

        group.finish();
    }

    #[cfg(not(feature = "metrics"))]
    {
        // Metrics disabled - skip benchmarks
    }
}

criterion_group!(
    benches,
    bench_metrics_recording,
    bench_metrics_throughput,
    bench_metrics_overhead_in_game_loop
);
criterion_main!(benches);
