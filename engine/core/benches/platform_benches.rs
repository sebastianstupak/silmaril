//! Comprehensive platform backend benchmarks.
//!
//! This benchmark suite measures the performance of all platform abstraction layers:
//! - Time backend (monotonic_nanos, sleep accuracy)
//! - Filesystem backend (path normalization, file I/O)
//! - Threading backend (priority, affinity, CPU count)
//!
//! # Performance Targets
//!
//! ## Time Backend
//! - `monotonic_nanos`: < 50ns per call (target: 30ns)
//! - `monotonic_nanos` (1000 calls): < 50us total (target: 30us)
//! - `sleep(1ms)`: 1-2ms actual sleep (tolerance: +/-500us)
//! - `sleep(10ms)`: 10-11ms actual sleep (tolerance: +/-1ms)
//! - `sleep(100ms)`: 100-101ms actual sleep (tolerance: +/-2ms)
//!
//! ## Filesystem Backend
//! - `normalize_path`: < 500ns for simple paths (target: 200ns)
//! - `normalize_path` (complex): < 2us for paths with .. and . (target: 1us)
//! - `file_exists`: < 5us for cached results (target: 2us)
//! - `read_file` (1KB): < 20us (target: 10us)
//! - `read_file` (10KB): < 100us (target: 50us)
//! - `write_file` (1KB): < 50us (target: 30us)
//! - `write_file` (10KB): < 200us (target: 100us)
//!
//! ## Threading Backend
//! - `set_thread_priority`: < 5us per call (target: 2us)
//! - `set_thread_affinity` (1 core): < 10us (target: 5us)
//! - `set_thread_affinity` (4 cores): < 15us (target: 8us)
//! - `num_cpus`: < 1us (target: 100ns, should be cached)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use std::path::Path;
use std::time::Duration;

// ============================================================================
// Time Backend Benchmarks
// ============================================================================

/// Benchmark single monotonic_nanos call.
/// Target: < 50ns (30ns ideal)
fn bench_monotonic_nanos_single(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("time/monotonic_nanos/single", |b| {
        b.iter(|| {
            black_box(backend.monotonic_nanos());
        });
    });
}

/// Benchmark 1000 monotonic_nanos calls in a loop.
/// Target: < 50us total (30us ideal)
fn bench_monotonic_nanos_batch(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("time/monotonic_nanos/batch_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(backend.monotonic_nanos());
            }
        });
    });
}

/// Benchmark sleep accuracy for various durations.
/// We measure the actual sleep time vs requested to track accuracy.
fn bench_sleep_accuracy(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");
    let mut group = c.benchmark_group("time/sleep_accuracy");

    // Note: We can't directly benchmark sleep accuracy with criterion,
    // but we can measure the overhead of the sleep call itself.
    // Actual accuracy should be tested separately.

    for ms in [1, 10, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(ms), ms, |b, &ms| {
            b.iter(|| {
                backend.sleep(Duration::from_millis(ms));
            });
        });
    }

    group.finish();
}

/// Property: monotonic time should never decrease.
/// This is more of a stress test than a performance benchmark.
fn bench_time_never_decreases(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("time/never_decreases/stress", |b| {
        b.iter(|| {
            let mut last = 0u64;
            for _ in 0..1000 {
                let now = backend.monotonic_nanos();
                assert!(now >= last, "Time decreased: {} -> {}", last, now);
                last = now;
            }
        });
    });
}

/// Benchmark the now() helper method.
fn bench_time_now(c: &mut Criterion) {
    let backend = create_time_backend().expect("Failed to create time backend");

    c.bench_function("time/now", |b| {
        b.iter(|| {
            black_box(backend.now());
        });
    });
}

// ============================================================================
// Filesystem Backend Benchmarks
// ============================================================================

/// Benchmark path normalization with various patterns.
fn bench_normalize_path(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let mut group = c.benchmark_group("filesystem/normalize_path");

    // Simple path (no . or ..)
    group.bench_function("simple", |b| {
        let path = Path::new("foo/bar/baz.txt");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    // Path with . (current directory)
    group.bench_function("with_dot", |b| {
        let path = Path::new("foo/./bar/./baz.txt");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    // Path with .. (parent directory)
    group.bench_function("with_dotdot", |b| {
        let path = Path::new("foo/bar/../baz/qux.txt");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    // Complex path with multiple . and ..
    group.bench_function("complex", |b| {
        let path = Path::new("foo/./bar/../baz/../../qux/./final.txt");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    // Absolute path
    #[cfg(unix)]
    group.bench_function("absolute", |b| {
        let path = Path::new("/usr/local/bin/../lib/libfoo.so");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    #[cfg(windows)]
    group.bench_function("absolute", |b| {
        let path = Path::new("C:\\Program Files\\..\\Users\\test.txt");
        b.iter(|| {
            black_box(fs.normalize_path(path));
        });
    });

    group.finish();
}

/// Benchmark file_exists checks.
/// Note: Results may vary based on OS file cache.
fn bench_file_exists(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let mut group = c.benchmark_group("filesystem/file_exists");

    // Create a temporary file for testing
    let temp_dir = std::env::temp_dir();
    let existing_file = temp_dir.join("bench_file_exists.tmp");
    std::fs::write(&existing_file, b"test").expect("Failed to create test file");

    group.bench_function("existing_file", |b| {
        b.iter(|| {
            black_box(fs.file_exists(&existing_file));
        });
    });

    group.bench_function("non_existing_file", |b| {
        let non_existing = temp_dir.join("this_file_does_not_exist_12345.tmp");
        b.iter(|| {
            black_box(fs.file_exists(&non_existing));
        });
    });

    // Cleanup
    std::fs::remove_file(&existing_file).ok();

    group.finish();
}

/// Benchmark small file reads (1KB, 10KB).
fn bench_file_read(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let mut group = c.benchmark_group("filesystem/read_file");

    let temp_dir = std::env::temp_dir();

    // 1KB file
    let file_1kb = temp_dir.join("bench_read_1kb.tmp");
    let data_1kb = vec![0u8; 1024];
    std::fs::write(&file_1kb, &data_1kb).expect("Failed to create 1KB test file");

    group.bench_function("1kb", |b| {
        b.iter(|| {
            black_box(fs.read_file(&file_1kb).unwrap());
        });
    });

    // 10KB file
    let file_10kb = temp_dir.join("bench_read_10kb.tmp");
    let data_10kb = vec![0u8; 10240];
    std::fs::write(&file_10kb, &data_10kb).expect("Failed to create 10KB test file");

    group.bench_function("10kb", |b| {
        b.iter(|| {
            black_box(fs.read_file(&file_10kb).unwrap());
        });
    });

    // Cleanup
    std::fs::remove_file(&file_1kb).ok();
    std::fs::remove_file(&file_10kb).ok();

    group.finish();
}

/// Benchmark small file writes (1KB, 10KB).
fn bench_file_write(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let mut group = c.benchmark_group("filesystem/write_file");

    let temp_dir = std::env::temp_dir();

    // 1KB write
    let data_1kb = vec![0u8; 1024];
    group.bench_function("1kb", |b| {
        let file_path = temp_dir.join("bench_write_1kb.tmp");
        b.iter(|| {
            fs.write_file(&file_path, &data_1kb).unwrap();
        });
        std::fs::remove_file(&file_path).ok();
    });

    // 10KB write
    let data_10kb = vec![0u8; 10240];
    group.bench_function("10kb", |b| {
        let file_path = temp_dir.join("bench_write_10kb.tmp");
        b.iter(|| {
            fs.write_file(&file_path, &data_10kb).unwrap();
        });
        std::fs::remove_file(&file_path).ok();
    });

    group.finish();
}

/// Benchmark read_to_string for UTF-8 text files.
fn bench_read_to_string(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("bench_read_string.tmp");

    // Create a test file with some UTF-8 text
    let test_text = "Hello, World!\nこんにちは世界\n".repeat(100);
    std::fs::write(&file_path, &test_text).expect("Failed to create test file");

    c.bench_function("filesystem/read_to_string", |b| {
        b.iter(|| {
            black_box(fs.read_to_string(&file_path).unwrap());
        });
    });

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

/// Benchmark write_string for UTF-8 text files.
fn bench_write_string(c: &mut Criterion) {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("bench_write_string.tmp");

    let test_text = "Hello, World!\nこんにちは世界\n".repeat(100);

    c.bench_function("filesystem/write_string", |b| {
        b.iter(|| {
            fs.write_string(&file_path, &test_text).unwrap();
        });
    });

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

// ============================================================================
// Threading Backend Benchmarks
// ============================================================================

/// Benchmark set_thread_priority for all priority levels.
fn bench_set_thread_priority(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let mut group = c.benchmark_group("threading/set_priority");

    group.bench_function("low", |b| {
        b.iter(|| {
            // Note: May fail on some systems, but we're measuring the call overhead
            let _ = backend.set_thread_priority(ThreadPriority::Low);
        });
    });

    group.bench_function("normal", |b| {
        b.iter(|| {
            let _ = backend.set_thread_priority(ThreadPriority::Normal);
        });
    });

    group.bench_function("high", |b| {
        b.iter(|| {
            // May fail without privileges
            let _ = backend.set_thread_priority(ThreadPriority::High);
        });
    });

    // Note: Realtime priority typically requires elevated privileges,
    // so we skip benchmarking it to avoid test failures.

    group.finish();
}

/// Benchmark set_thread_affinity for different core counts.
fn bench_set_thread_affinity(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let num_cpus = backend.num_cpus();
    let mut group = c.benchmark_group("threading/set_affinity");

    // Single core
    group.bench_function("1_core", |b| {
        b.iter(|| {
            // Pin to first core
            let _ = backend.set_thread_affinity(&[0]);
        });
    });

    // 4 cores (if available)
    if num_cpus >= 4 {
        group.bench_function("4_cores", |b| {
            b.iter(|| {
                let _ = backend.set_thread_affinity(&[0, 1, 2, 3]);
            });
        });
    }

    // All cores
    group.bench_function("all_cores", |b| {
        let all_cores: Vec<usize> = (0..num_cpus).collect();
        b.iter(|| {
            let _ = backend.set_thread_affinity(&all_cores);
        });
    });

    // Reset to all cores after benchmarking
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);

    group.finish();
}

/// Benchmark num_cpus query.
/// This should be very fast as it's typically cached.
fn bench_num_cpus(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    c.bench_function("threading/num_cpus", |b| {
        b.iter(|| {
            black_box(backend.num_cpus());
        });
    });
}

/// Benchmark combined operations (priority + affinity).
fn bench_thread_setup(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    c.bench_function("threading/full_setup", |b| {
        b.iter(|| {
            // Typical thread setup: set priority and pin to core
            let _ = backend.set_thread_priority(ThreadPriority::High);
            let _ = backend.set_thread_affinity(&[0]);
        });
    });

    // Reset after benchmarking
    let _ = backend.set_thread_priority(ThreadPriority::Normal);
    let num_cpus = backend.num_cpus();
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

// ============================================================================
// Combined/Integration Benchmarks
// ============================================================================

/// Benchmark a realistic scenario: time measurement around file I/O.
fn bench_timed_file_operation(c: &mut Criterion) {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("bench_timed_io.tmp");
    let test_data = vec![0u8; 1024];

    c.bench_function("combined/timed_file_write", |b| {
        b.iter(|| {
            let start = time_backend.monotonic_nanos();
            fs_backend.write_file(&file_path, &test_data).unwrap();
            let end = time_backend.monotonic_nanos();
            black_box(end - start);
        });
    });

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

/// Benchmark creating all platform backends.
/// This tests the factory pattern overhead.
fn bench_backend_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform/backend_creation");

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

    group.bench_function("all_backends", |b| {
        b.iter(|| {
            let _ = black_box(create_time_backend().unwrap());
            let _ = black_box(create_filesystem_backend());
            let _ = black_box(create_threading_backend().unwrap());
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    time_benches,
    bench_monotonic_nanos_single,
    bench_monotonic_nanos_batch,
    bench_sleep_accuracy,
    bench_time_never_decreases,
    bench_time_now,
);

criterion_group!(
    filesystem_benches,
    bench_normalize_path,
    bench_file_exists,
    bench_file_read,
    bench_file_write,
    bench_read_to_string,
    bench_write_string,
);

criterion_group!(
    threading_benches,
    bench_set_thread_priority,
    bench_set_thread_affinity,
    bench_num_cpus,
    bench_thread_setup,
);

criterion_group!(combined_benches, bench_timed_file_operation, bench_backend_creation,);

criterion_main!(time_benches, filesystem_benches, threading_benches, combined_benches);
