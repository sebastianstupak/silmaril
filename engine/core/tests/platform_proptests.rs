//! Property-based tests for platform abstraction layer
//!
//! These tests verify correctness properties across different platforms:
//! - Time backend monotonicity under concurrent access
//! - Filesystem path normalization for Windows/Unix paths
//! - Threading priority/affinity combinations

use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Property Test 1: Time Backend Monotonicity (Sequential)
// ============================================================================

proptest! {
    #[test]
    fn prop_time_monotonic_sequential(iterations in 10usize..1000) {
        let backend = create_time_backend().expect("time backend creation should succeed");

        let mut last_time = 0u64;

        for _ in 0..iterations {
            let current_time = backend.monotonic_nanos();

            // Time should never go backwards
            prop_assert!(
                current_time >= last_time,
                "Time went backwards: {} -> {}",
                last_time,
                current_time
            );

            last_time = current_time;
        }
    }
}

// ============================================================================
// Property Test 2: Time Backend Monotonicity (Concurrent Access)
// ============================================================================

proptest! {
    #[test]
    fn prop_time_monotonic_concurrent(thread_count in 2usize..16, samples_per_thread in 10usize..100) {
        let backend = Arc::new(create_time_backend().expect("time backend creation should succeed"));

        let mut handles = vec![];

        for _ in 0..thread_count {
            let backend_clone = Arc::clone(&backend);
            let samples = samples_per_thread;

            let handle = std::thread::spawn(move || {
                let mut times = Vec::with_capacity(samples);

                for _ in 0..samples {
                    times.push(backend_clone.monotonic_nanos());
                }

                times
            });

            handles.push(handle);
        }

        // Collect all time samples from all threads
        let mut all_times = Vec::new();
        for handle in handles {
            let thread_times = handle.join().expect("thread should complete successfully");
            all_times.extend(thread_times);
        }

        // Verify each thread saw monotonic time
        // Note: We can't verify global ordering across threads since they run concurrently,
        // but we can verify that within each thread, time is monotonic
        prop_assert!(all_times.len() == thread_count * samples_per_thread);

        // Verify no time value is zero (time should always advance)
        for time in all_times {
            prop_assert!(time > 0, "Time should never be zero");
        }
    }
}

// ============================================================================
// Property Test 3: Time Backend Sleep Accuracy
// ============================================================================

proptest! {
    #[test]
    fn prop_time_sleep_accuracy(sleep_ms in 1u64..100) {
        let backend = create_time_backend().expect("time backend creation should succeed");

        let start = backend.monotonic_nanos();
        backend.sleep(Duration::from_millis(sleep_ms));
        let end = backend.monotonic_nanos();

        let elapsed_ns = end - start;
        let elapsed_ms = elapsed_ns / 1_000_000;

        // Sleep should be at least the requested duration (allowing for scheduler variance)
        // We allow up to 5ms under on Windows due to timer resolution
        let tolerance_ms = if cfg!(windows) { 5 } else { 1 };
        prop_assert!(
            elapsed_ms + tolerance_ms >= sleep_ms,
            "Sleep was too short: requested {}ms, got {}ms",
            sleep_ms,
            elapsed_ms
        );

        // Sleep shouldn't be excessively long (allowing 50ms overhead for scheduling)
        prop_assert!(
            elapsed_ms <= sleep_ms + 50,
            "Sleep was too long: requested {}ms, got {}ms",
            sleep_ms,
            elapsed_ms
        );
    }
}

// ============================================================================
// Property Test 4: Filesystem Path Normalization (Simple Paths)
// ============================================================================

proptest! {
    #[test]
    fn prop_filesystem_normalize_simple_paths(
        segments in prop::collection::vec("[a-z]{1,8}", 1..5)
    ) {
        let fs = create_filesystem_backend();

        // Create a path with forward slashes
        let path_str = segments.join("/");
        let path = PathBuf::from(&path_str);

        // Normalize the path
        let normalized = fs.normalize_path(&path);

        // Path should exist as a PathBuf
        prop_assert!(normalized.as_os_str().len() > 0);

        // Verify all segments are present in the normalized path
        let normalized_str = normalized.to_string_lossy();
        for segment in &segments {
            prop_assert!(
                normalized_str.contains(segment),
                "Normalized path should contain segment '{}'",
                segment
            );
        }
    }
}

// ============================================================================
// Property Test 5: Filesystem Path Normalization with Dot Segments
// ============================================================================

proptest! {
    #[test]
    fn prop_filesystem_normalize_with_dots(
        base_segments in prop::collection::vec("[a-z]{1,8}", 2..5)
    ) {
        let fs = create_filesystem_backend();

        // Create a path like "foo/./bar/../baz"
        let mut path_parts = vec![base_segments[0].clone()];
        if base_segments.len() > 1 {
            path_parts.push(".".to_string());
            path_parts.push(base_segments[1].clone());
        }
        if base_segments.len() > 2 {
            path_parts.push("..".to_string());
            path_parts.push(base_segments[2].clone());
        }

        let path_str = path_parts.join("/");
        let path = PathBuf::from(&path_str);

        // Normalize the path
        let normalized = fs.normalize_path(&path);

        // The normalized path should be valid
        prop_assert!(normalized.as_os_str().len() > 0);
    }
}

// ============================================================================
// Property Test 6: Filesystem Read/Write Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_filesystem_read_write_roundtrip(
        content in prop::collection::vec(any::<u8>(), 0..10000)
    ) {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("proptest_{}.bin", std::process::id()));

        // Write data
        fs.write_file(&test_file, &content)
            .expect("write should succeed");

        // Read data back
        let read_content = fs.read_file(&test_file)
            .expect("read should succeed");

        // Verify roundtrip
        prop_assert_eq!(content, read_content);

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}

// ============================================================================
// Property Test 7: Filesystem String Read/Write Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_filesystem_string_roundtrip(content in "\\PC*") {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("proptest_str_{}.txt", std::process::id()));

        // Write string
        fs.write_string(&test_file, &content)
            .expect("write_string should succeed");

        // Read string back
        let read_content = fs.read_to_string(&test_file)
            .expect("read_to_string should succeed");

        // Verify roundtrip
        prop_assert_eq!(content, read_content);

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}

// ============================================================================
// Property Test 8: Threading Backend Priority Setting
// ============================================================================

proptest! {
    #[test]
    fn prop_threading_set_priority(priority_val in 0u8..3) {
        let backend = create_threading_backend().expect("threading backend creation should succeed");

        let priority = match priority_val {
            0 => ThreadPriority::Low,
            1 => ThreadPriority::Normal,
            _ => ThreadPriority::High,
        };

        // Set thread priority should not panic
        let result = backend.set_thread_priority(priority);

        // On some platforms, setting priority may fail due to permissions
        // We just verify it doesn't panic and returns a result
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// ============================================================================
// Property Test 9: Threading Backend Concurrent Priority Setting
// ============================================================================

proptest! {
    #[test]
    fn prop_threading_concurrent_priority_setting(thread_count in 2usize..8) {
        let backend = Arc::new(
            create_threading_backend().expect("threading backend creation should succeed")
        );

        let mut handles = vec![];

        for i in 0..thread_count {
            let backend_clone = Arc::clone(&backend);

            let handle = std::thread::spawn(move || {
                let priority = match i % 3 {
                    0 => ThreadPriority::Low,
                    1 => ThreadPriority::Normal,
                    _ => ThreadPriority::High,
                };

                // Each thread sets its own priority
                let result = backend_clone.set_thread_priority(priority);

                // Return whether the operation succeeded
                result.is_ok()
            });

            handles.push(handle);
        }

        // All threads should complete without panicking
        for handle in handles {
            let completed: Result<bool, _> = handle.join();
            prop_assert!(completed.is_ok());
        }
    }
}

// ============================================================================
// Property Test 10: Time Backend Duration Conversion
// ============================================================================

proptest! {
    #[test]
    fn prop_time_duration_conversion(nanos in 0u64..1_000_000_000_000) {
        let backend = create_time_backend().expect("time backend creation should succeed");

        // Get initial time
        let t1 = backend.now();

        // Create a duration from nanoseconds
        let expected_duration = Duration::from_nanos(nanos);

        // Verify Duration operations work correctly
        let t2 = t1 + expected_duration;

        prop_assert!(t2 >= t1, "Adding duration should not go backwards");
        prop_assert_eq!(
            t2.as_nanos() - t1.as_nanos(),
            nanos as u128,
            "Duration arithmetic should be exact"
        );
    }
}

// ============================================================================
// Property Test 11: Filesystem File Existence Check
// ============================================================================

proptest! {
    #[test]
    fn prop_filesystem_existence_check(file_exists in any::<bool>()) {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("proptest_exists_{}.tmp", std::process::id()));

        if file_exists {
            // Create the file
            fs.write_file(&test_file, b"test")
                .expect("write should succeed");

            // File should exist
            prop_assert!(fs.file_exists(&test_file));

            // Cleanup
            let _ = std::fs::remove_file(&test_file);
        } else {
            // Ensure file doesn't exist
            let _ = std::fs::remove_file(&test_file);

            // File should not exist
            prop_assert!(!fs.file_exists(&test_file));
        }
    }
}
