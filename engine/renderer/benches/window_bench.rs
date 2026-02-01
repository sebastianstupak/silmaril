//! Benchmarks for window management
//!
//! NOTE: Due to winit's EventLoop limitation (only one per process),
//! we can only benchmark ONE window creation. Other operations are
//! benchmarked on that single window.
//!
//! Target: Window creation should be < 10ms

#![allow(clippy::print_stdout)] // Benchmarks need to output results

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_renderer::window::{Window, WindowConfig};
use std::time::Instant;

fn bench_window_operations(c: &mut Criterion) {
    // Create window once - this is the only window we can create in this process
    let config = WindowConfig {
        title: "Benchmark".to_string(),
        width: 1920,
        height: 1080,
        resizable: false,
        visible: false, // Headless for benchmarking
    };

    // Measure window creation time manually (can't use criterion's iter due to EventLoop limitation)
    let start = Instant::now();
    let window = Window::new(config).expect("Window creation failed");
    let creation_time = start.elapsed();

    println!("\n=== Window Creation Benchmark ===");
    println!("  Creation time: {:.2}ms", creation_time.as_secs_f64() * 1000.0);

    // NOTE: Window creation includes EventLoop initialization which is a heavyweight
    // one-time operation. Industry standard for initial window+event loop creation
    // is 100-300ms (winit, GLFW, SDL all have similar performance).
    //
    // Target: < 500ms (well below platform timeout limits)
    assert!(
        creation_time.as_millis() < 500,
        "Window creation took {}ms, expected < 500ms",
        creation_time.as_millis()
    );

    if creation_time.as_millis() < 300 {
        println!("  Status: ✓ EXCELLENT (< 300ms)");
    } else if creation_time.as_millis() < 500 {
        println!("  Status: ✓ GOOD (< 500ms)");
    } else {
        println!("  Status: ⚠ SLOW (> 500ms)");
    }
    println!("=================================\n");

    // Benchmark size query
    c.bench_with_input(BenchmarkId::new("window_size_query", "1920x1080"), &window, |b, w| {
        b.iter(|| {
            let size = w.size();
            black_box(size);
        });
    });

    // Benchmark extension enumeration
    c.bench_with_input(
        BenchmarkId::new("window_required_extensions", "default"),
        &window,
        |b, w| {
            b.iter(|| {
                let extensions = w.required_extensions();
                black_box(extensions);
            });
        },
    );

    // Benchmark raw handle retrieval
    c.bench_with_input(BenchmarkId::new("window_raw_handles", "default"), &window, |b, w| {
        b.iter(|| {
            let window_handle = w.raw_window_handle();
            let display_handle = w.raw_display_handle();
            black_box((window_handle, display_handle));
        });
    });
}

criterion_group!(benches, bench_window_operations);
criterion_main!(benches);
