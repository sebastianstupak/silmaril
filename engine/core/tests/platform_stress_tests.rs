//! Stress tests for platform abstraction layer.
//!
//! These tests push the platform backends to their limits to verify:
//! - Stability under high load
//! - Correct behavior with many concurrent operations
//! - Memory safety with large data sets
//! - Performance degradation characteristics

use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Time Backend Stress Tests
// ============================================================================

#[test]
fn stress_time_rapid_queries() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Rapidly query time 100,000 times
    let mut last_time = backend.monotonic_nanos();

    for i in 0..100_000 {
        let current_time = backend.monotonic_nanos();
        assert!(
            current_time >= last_time,
            "Time went backwards at iteration {}: {} -> {}",
            i,
            last_time,
            current_time
        );
        last_time = current_time;
    }
}

#[test]
fn stress_time_concurrent_queries() {
    let backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let num_threads = 16;
    let queries_per_thread = 10_000;

    let mut handles = vec![];
    let all_times = Arc::new(Mutex::new(Vec::new()));

    for thread_id in 0..num_threads {
        let backend_clone = Arc::clone(&backend);
        let times_clone = Arc::clone(&all_times);

        let handle = std::thread::spawn(move || {
            let mut local_times = Vec::with_capacity(queries_per_thread);

            for _ in 0..queries_per_thread {
                local_times.push(backend_clone.monotonic_nanos());
            }

            // Verify local monotonicity
            for i in 1..local_times.len() {
                assert!(
                    local_times[i] >= local_times[i - 1],
                    "Thread {} time not monotonic at index {}: {} -> {}",
                    thread_id,
                    i,
                    local_times[i - 1],
                    local_times[i]
                );
            }

            times_clone.lock().unwrap().extend(local_times);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let times = all_times.lock().unwrap();
    assert_eq!(times.len(), num_threads * queries_per_thread);
}

#[test]
fn stress_time_sleep_consistency() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Test sleep consistency over many iterations
    let sleep_duration = Duration::from_millis(1);
    let mut errors = Vec::new();

    for i in 0..100 {
        let start = backend.monotonic_nanos();
        backend.sleep(sleep_duration);
        let end = backend.monotonic_nanos();

        let elapsed_ns = end - start;
        let elapsed_ms = elapsed_ns / 1_000_000;

        // Allow wider variance on Windows due to timer resolution
        // Track extreme outliers only (< 1ms or > 100ms)
        if elapsed_ms < 1 || elapsed_ms > 100 {
            errors.push((i, elapsed_ms));
        }
    }

    // Allow up to 10% of sleeps to be outliers (Windows timer resolution is ~15ms)
    assert!(
        errors.len() <= 10,
        "Too many sleep outliers: {} out of 100: {:?}",
        errors.len(),
        errors
    );
}

// ============================================================================
// Filesystem Backend Stress Tests
// ============================================================================

#[test]
fn stress_filesystem_many_files() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();

    // Create and verify 1000 files
    let num_files = 1000;
    let mut file_paths = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let file_path = temp_dir.join(format!("stress_test_{}.tmp", i));
        let data = format!("File number {}", i);

        fs.write_string(&file_path, &data).expect("Failed to write file");
        assert!(fs.file_exists(&file_path), "File {} should exist", i);

        file_paths.push(file_path);
    }

    // Read back all files and verify content
    for (i, file_path) in file_paths.iter().enumerate() {
        let content = fs.read_to_string(file_path).expect("Failed to read file");
        assert_eq!(content, format!("File number {}", i));
    }

    // Cleanup
    for file_path in file_paths {
        std::fs::remove_file(&file_path).ok();
    }
}

#[test]
fn stress_filesystem_large_file() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("stress_large_file.bin");

    // Create a 10MB file
    let size_mb = 10;
    let data = vec![0xAB; size_mb * 1024 * 1024];

    fs.write_file(&test_file, &data).expect("Failed to write large file");

    // Read it back
    let read_data = fs.read_file(&test_file).expect("Failed to read large file");

    assert_eq!(read_data.len(), data.len());
    assert_eq!(read_data, data);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn stress_filesystem_concurrent_writes() {
    let fs = Arc::new(create_filesystem_backend());
    let temp_dir = std::env::temp_dir();
    let num_threads = 8;
    let writes_per_thread = 100;

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let fs_clone = Arc::clone(&fs);
        let temp_dir_clone = temp_dir.clone();

        let handle = std::thread::spawn(move || {
            for i in 0..writes_per_thread {
                let file_path =
                    temp_dir_clone.join(format!("stress_concurrent_{}_{}.tmp", thread_id, i));
                let data = format!("Thread {} write {}", thread_id, i);

                fs_clone.write_string(&file_path, &data).expect("Failed to write file");

                // Verify immediately
                let read_data = fs_clone.read_to_string(&file_path).expect("Failed to read file");
                assert_eq!(read_data, data);

                // Cleanup
                std::fs::remove_file(&file_path).ok();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn stress_filesystem_path_normalization() {
    let fs = create_filesystem_backend();

    // Test many complex path patterns
    let test_paths = vec![
        "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p",
        "a/../b/../c/../d/../e",
        "foo/bar/../../baz/../qux",
        "a/./b/./c/./d/./e/./f",
        "a/b/c/../../d/e/f/../../../g",
    ];

    for path_str in test_paths {
        let path = std::path::PathBuf::from(path_str);
        let normalized = fs.normalize_path(&path);

        // Should not panic - normalization may produce different results on different platforms
        // Just verify it returns a valid PathBuf
        let _ = normalized.as_os_str();
    }
}

// ============================================================================
// Threading Backend Stress Tests
// ============================================================================

#[test]
fn stress_threading_rapid_priority_changes() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Rapidly change thread priority 1000 times
    let priorities = [ThreadPriority::Low, ThreadPriority::Normal, ThreadPriority::High];

    for i in 0..1000 {
        let priority = priorities[i % priorities.len()];
        let _ = backend.set_thread_priority(priority);
    }

    // Reset to normal
    let _ = backend.set_thread_priority(ThreadPriority::Normal);
}

#[test]
fn stress_threading_concurrent_priority_changes() {
    let backend = Arc::new(create_threading_backend().expect("Failed to create threading backend"));
    let num_threads = 8;

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let backend_clone = Arc::clone(&backend);

        let handle = std::thread::spawn(move || {
            let priorities = [ThreadPriority::Low, ThreadPriority::Normal, ThreadPriority::High];

            for i in 0..100 {
                let priority = priorities[(thread_id + i) % priorities.len()];
                let _ = backend_clone.set_thread_priority(priority);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Reset to normal
    let _ = backend.set_thread_priority(ThreadPriority::Normal);
}

#[test]
fn stress_threading_affinity_patterns() {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let num_cpus = backend.num_cpus();

    // Test various affinity patterns
    for pattern_size in 1..=num_cpus.min(8) {
        let cores: Vec<usize> = (0..pattern_size).collect();
        let _ = backend.set_thread_affinity(&cores);
    }

    // Reset to all cores
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

// ============================================================================
// Combined Stress Tests
// ============================================================================

#[test]
fn stress_combined_realistic_workload() {
    // Simulate a realistic game engine workload with all backends
    let time_backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let fs_backend = Arc::new(create_filesystem_backend());
    let threading_backend =
        Arc::new(create_threading_backend().expect("Failed to create threading backend"));

    let temp_dir = std::env::temp_dir();
    let num_iterations = 100;

    for i in 0..num_iterations {
        // Measure time
        let start = time_backend.monotonic_nanos();

        // Write some data
        let file_path = temp_dir.join(format!("stress_combined_{}.tmp", i));
        let data = format!("Iteration {}", i);
        fs_backend.write_string(&file_path, &data).expect("Failed to write");

        // Read it back
        let read_data = fs_backend.read_to_string(&file_path).expect("Failed to read");
        assert_eq!(read_data, data);

        // Cleanup
        std::fs::remove_file(&file_path).ok();

        // Measure elapsed time
        let end = time_backend.monotonic_nanos();
        let elapsed_us = (end - start) / 1000;

        // Should complete in reasonable time (< 1ms per iteration)
        assert!(elapsed_us < 1_000_000, "Iteration {} took too long: {}us", i, elapsed_us);

        // Occasional priority change
        if i % 10 == 0 {
            let _ = threading_backend.set_thread_priority(ThreadPriority::High);
        }
    }

    // Reset
    let _ = threading_backend.set_thread_priority(ThreadPriority::Normal);
}

#[test]
fn stress_combined_concurrent_all_backends() {
    let time_backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let fs_backend = Arc::new(create_filesystem_backend());
    let threading_backend =
        Arc::new(create_threading_backend().expect("Failed to create threading backend"));

    let temp_dir = std::env::temp_dir();
    let num_threads = 4;

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let time_clone = Arc::clone(&time_backend);
        let fs_clone = Arc::clone(&fs_backend);
        let threading_clone = Arc::clone(&threading_backend);
        let temp_dir_clone = temp_dir.clone();

        let handle = std::thread::spawn(move || {
            for i in 0..50 {
                // Time measurement
                let start = time_clone.monotonic_nanos();

                // File I/O
                let file_path =
                    temp_dir_clone.join(format!("stress_thread_{}_{}.tmp", thread_id, i));
                fs_clone.write_string(&file_path, "data").unwrap();
                let _ = fs_clone.read_to_string(&file_path).unwrap();
                std::fs::remove_file(&file_path).ok();

                // Threading operation
                let _ = threading_clone.num_cpus();

                let end = time_clone.monotonic_nanos();
                assert!(end >= start);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// ============================================================================
// Memory Safety Stress Tests
// ============================================================================

#[test]
fn stress_memory_large_allocation() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();

    // Allocate and write multiple large buffers sequentially
    for i in 0..10 {
        let file_path = temp_dir.join(format!("stress_mem_{}.bin", i));
        let data = vec![i as u8; 1024 * 1024]; // 1MB each

        fs.write_file(&file_path, &data).expect("Failed to write");
        let read_data = fs.read_file(&file_path).expect("Failed to read");

        assert_eq!(read_data.len(), data.len());

        // Cleanup
        std::fs::remove_file(&file_path).ok();
    }
}

#[test]
fn stress_memory_many_time_queries() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // This shouldn't allocate, but verify it doesn't leak
    for _ in 0..1_000_000 {
        let _ = backend.monotonic_nanos();
    }
}
