//! Cross-platform performance benchmarks for platform abstraction layer.
//!
//! This benchmark suite compares performance across different platforms to ensure
//! consistent behavior and identify platform-specific optimizations.
//!
//! # Performance Comparison Targets
//!
//! These benchmarks should have similar performance across Windows, Linux, and macOS:
//! - Time queries should be within 2x of each other
//! - File I/O should be within 3x of each other (filesystem dependent)
//! - Threading operations should be within 2x of each other

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Time Backend Cross-Platform Benchmarks
// ============================================================================

/// Benchmark monotonic_nanos across platforms.
fn bench_time_monotonic_cross_platform(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("platform/time/monotonic_nanos", |b| {
        b.iter(|| {
            black_box(backend.monotonic_nanos());
        });
    });
}

/// Benchmark now() across platforms.
fn bench_time_now_cross_platform(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("platform/time/now", |b| {
        b.iter(|| {
            black_box(backend.now());
        });
    });
}

/// Benchmark sleep accuracy across platforms.
fn bench_time_sleep_cross_platform(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");
    let mut group = c.benchmark_group("platform/time/sleep");

    for sleep_ms in [1, 5, 10].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(sleep_ms), sleep_ms, |b, &ms| {
            b.iter(|| {
                backend.sleep(Duration::from_millis(ms));
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent time queries across platforms.
fn bench_time_concurrent_cross_platform(c: &mut Criterion) {
    let backend = Arc::new(create_time_backend().expect("Failed to create time backend"));

    c.bench_function("platform/time/concurrent_queries", |b| {
        b.iter(|| {
            let backend_clone = Arc::clone(&backend);
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let b = Arc::clone(&backend_clone);
                    std::thread::spawn(move || {
                        for _ in 0..100 {
                            black_box(b.monotonic_nanos());
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

// ============================================================================
// Filesystem Backend Cross-Platform Benchmarks
// ============================================================================

/// Benchmark file write across platforms.
fn bench_filesystem_write_cross_platform(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let mut group = c.benchmark_group("platform/filesystem/write");

    for size_kb in [1, 10, 100].iter() {
        let data = vec![0u8; size_kb * 1024];
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", size_kb)),
            &data,
            |b, data| {
                let file_path = temp_dir.join(format!("bench_write_{}kb.tmp", size_kb));
                b.iter(|| {
                    fs.write_file(&file_path, data).unwrap();
                });
                std::fs::remove_file(&file_path).ok();
            },
        );
    }

    group.finish();
}

/// Benchmark file read across platforms.
fn bench_filesystem_read_cross_platform(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let mut group = c.benchmark_group("platform/filesystem/read");

    for size_kb in [1, 10, 100].iter() {
        // Create test file
        let file_path = temp_dir.join(format!("bench_read_{}kb.tmp", size_kb));
        let data = vec![0u8; size_kb * 1024];
        std::fs::write(&file_path, &data).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", size_kb)),
            size_kb,
            |b, _| {
                b.iter(|| {
                    black_box(fs.read_file(&file_path).unwrap());
                });
            },
        );

        std::fs::remove_file(&file_path).ok();
    }

    group.finish();
}

/// Benchmark path normalization across platforms.
fn bench_filesystem_normalize_cross_platform(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let mut group = c.benchmark_group("platform/filesystem/normalize");

    let test_cases = vec![
        ("simple", "foo/bar/baz.txt"),
        ("with_dots", "foo/./bar/../baz.txt"),
        ("complex", "a/b/../c/./d/../../e/f.txt"),
    ];

    for (name, path_str) in test_cases {
        group.bench_function(name, |b| {
            let path = std::path::PathBuf::from(path_str);
            b.iter(|| {
                black_box(fs.normalize_path(&path));
            });
        });
    }

    group.finish();
}

/// Benchmark file_exists across platforms.
fn bench_filesystem_exists_cross_platform(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let mut group = c.benchmark_group("platform/filesystem/exists");

    // Create a test file
    let existing_file = temp_dir.join("bench_exists.tmp");
    std::fs::write(&existing_file, b"test").unwrap();

    group.bench_function("existing_file", |b| {
        b.iter(|| {
            black_box(fs.file_exists(&existing_file));
        });
    });

    group.bench_function("non_existing_file", |b| {
        let non_existing = temp_dir.join("does_not_exist_12345.tmp");
        b.iter(|| {
            black_box(fs.file_exists(&non_existing));
        });
    });

    std::fs::remove_file(&existing_file).ok();
    group.finish();
}

// ============================================================================
// Threading Backend Cross-Platform Benchmarks
// ============================================================================

/// Benchmark set_thread_priority across platforms.
fn bench_threading_priority_cross_platform(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let mut group = c.benchmark_group("platform/threading/set_priority");

    let priorities = vec![
        ("low", ThreadPriority::Low),
        ("normal", ThreadPriority::Normal),
        ("high", ThreadPriority::High),
    ];

    for (name, priority) in priorities {
        group.bench_function(name, |b| {
            b.iter(|| {
                let _ = backend.set_thread_priority(priority);
            });
        });
    }

    // Reset
    let _ = backend.set_thread_priority(ThreadPriority::Normal);
    group.finish();
}

/// Benchmark set_thread_affinity across platforms.
fn bench_threading_affinity_cross_platform(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let num_cpus = backend.num_cpus();
    let mut group = c.benchmark_group("platform/threading/set_affinity");

    group.bench_function("single_core", |b| {
        b.iter(|| {
            let _ = backend.set_thread_affinity(&[0]);
        });
    });

    if num_cpus >= 4 {
        group.bench_function("4_cores", |b| {
            b.iter(|| {
                let _ = backend.set_thread_affinity(&[0, 1, 2, 3]);
            });
        });
    }

    group.bench_function("all_cores", |b| {
        let all_cores: Vec<usize> = (0..num_cpus).collect();
        b.iter(|| {
            let _ = backend.set_thread_affinity(&all_cores);
        });
    });

    // Reset
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
    group.finish();
}

/// Benchmark num_cpus across platforms.
fn bench_threading_num_cpus_cross_platform(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    c.bench_function("platform/threading/num_cpus", |b| {
        b.iter(|| {
            black_box(backend.num_cpus());
        });
    });
}

// ============================================================================
// Combined Cross-Platform Benchmarks
// ============================================================================

/// Benchmark realistic game loop iteration across platforms.
fn bench_combined_game_loop_cross_platform(c: &mut Criterion) {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");

    let temp_dir = std::env::temp_dir();

    c.bench_function("platform/combined/game_loop_iteration", |b| {
        b.iter(|| {
            // Measure frame time
            let frame_start = time_backend.monotonic_nanos();

            // Check if save file exists
            let save_file = temp_dir.join("game_save.dat");
            let _exists = fs_backend.file_exists(&save_file);

            // Get CPU count (simulating workload distribution)
            let _num_cpus = threading_backend.num_cpus();

            // Measure frame end
            let frame_end = time_backend.monotonic_nanos();
            black_box(frame_end - frame_start);
        });
    });
}

/// Benchmark backend creation overhead across platforms.
fn bench_combined_backend_creation_cross_platform(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform/combined/backend_creation");

    group.bench_function("time", |b| {
        b.iter(|| {
            black_box(create_time_backend().unwrap());
        });
    });

    group.bench_function("filesystem", |b| {
        b.iter(|| {
            black_box(create_filesystem_backend());
        });
    });

    group.bench_function("threading", |b| {
        b.iter(|| {
            black_box(create_threading_backend().unwrap());
        });
    });

    group.bench_function("all_three", |b| {
        b.iter(|| {
            let _time = black_box(create_time_backend().unwrap());
            let _fs = black_box(create_filesystem_backend());
            let _threading = black_box(create_threading_backend().unwrap());
        });
    });

    group.finish();
}

/// Benchmark profiling scenario across platforms.
fn bench_combined_profiling_scenario_cross_platform(c: &mut Criterion) {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let profile_file = temp_dir.join("bench_profile.json");

    c.bench_function("platform/combined/profiling_scenario", |b| {
        b.iter(|| {
            // Start profiling event
            let event_start = time_backend.monotonic_nanos();

            // Simulate some work (file write)
            fs_backend.write_string(&profile_file, "{\"event\":\"test\"}").unwrap();

            // End profiling event
            let event_end = time_backend.monotonic_nanos();

            // Calculate duration
            black_box(event_end - event_start);
        });
    });

    // Cleanup
    std::fs::remove_file(&profile_file).ok();
}

// ============================================================================
// Platform-Specific Performance Characteristics
// ============================================================================

/// Benchmark to identify platform-specific overhead.
fn bench_platform_overhead_characterization(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform/overhead_characterization");

    // Time backend overhead
    let time_backend = create_time_backend().expect("Failed to create time backend");
    group.bench_function("time/call_overhead", |b| {
        b.iter(|| {
            // Measure the overhead of calling monotonic_nanos twice
            let t1 = time_backend.monotonic_nanos();
            let t2 = time_backend.monotonic_nanos();
            black_box(t2 - t1);
        });
    });

    // Filesystem backend overhead
    let fs_backend = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("overhead_test.tmp");

    // Create a small test file
    std::fs::write(&test_file, b"test").unwrap();

    group.bench_function("filesystem/exists_overhead", |b| {
        b.iter(|| {
            // Measure overhead of file existence check
            let exists = fs_backend.file_exists(&test_file);
            black_box(exists);
        });
    });

    std::fs::remove_file(&test_file).ok();

    // Threading backend overhead
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");
    group.bench_function("threading/num_cpus_overhead", |b| {
        b.iter(|| {
            // Measure overhead of querying CPU count
            let count = threading_backend.num_cpus();
            black_box(count);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    time_cross_platform,
    bench_time_monotonic_cross_platform,
    bench_time_now_cross_platform,
    bench_time_sleep_cross_platform,
    bench_time_concurrent_cross_platform,
);

criterion_group!(
    filesystem_cross_platform,
    bench_filesystem_write_cross_platform,
    bench_filesystem_read_cross_platform,
    bench_filesystem_normalize_cross_platform,
    bench_filesystem_exists_cross_platform,
);

criterion_group!(
    threading_cross_platform,
    bench_threading_priority_cross_platform,
    bench_threading_affinity_cross_platform,
    bench_threading_num_cpus_cross_platform,
);

criterion_group!(
    combined_cross_platform,
    bench_combined_game_loop_cross_platform,
    bench_combined_backend_creation_cross_platform,
    bench_combined_profiling_scenario_cross_platform,
    bench_platform_overhead_characterization,
);

criterion_main!(
    time_cross_platform,
    filesystem_cross_platform,
    threading_cross_platform,
    combined_cross_platform,
);
