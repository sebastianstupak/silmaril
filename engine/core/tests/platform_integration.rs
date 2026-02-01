//! Integration tests for platform abstraction layer.
//!
//! These tests verify real-world usage scenarios combining multiple platform backends.
//! They test cross-platform behavior, error handling, and practical use cases.

use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Integration Test 1: Timed File Operations
// ============================================================================

#[test]
fn integration_timed_file_write_and_read() {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("integration_timed_io.txt");

    // Measure write time
    let write_start = time_backend.monotonic_nanos();
    let test_data = "Integration test data for timed I/O".repeat(100);
    fs_backend
        .write_string(&test_file, &test_data)
        .expect("Failed to write file");
    let write_end = time_backend.monotonic_nanos();
    let write_duration_us = (write_end - write_start) / 1000;

    println!("Write took {}µs", write_duration_us);
    assert!(write_duration_us > 0, "Write should take measurable time");

    // Measure read time
    let read_start = time_backend.monotonic_nanos();
    let read_data = fs_backend.read_to_string(&test_file).expect("Failed to read file");
    let read_end = time_backend.monotonic_nanos();
    let read_duration_us = (read_end - read_start) / 1000;

    println!("Read took {}µs", read_duration_us);
    assert!(read_duration_us > 0, "Read should take measurable time");

    // Verify data integrity
    assert_eq!(test_data, read_data);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

// ============================================================================
// Integration Test 2: Multi-threaded File Access with Time Tracking
// ============================================================================

#[test]
fn integration_concurrent_file_access_with_timing() {
    let time_backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let fs_backend = Arc::new(create_filesystem_backend());
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");

    let temp_dir = std::env::temp_dir();
    let num_threads = 4.min(threading_backend.num_cpus());

    let mut handles = vec![];
    let timings = Arc::new(Mutex::new(Vec::new()));

    for i in 0..num_threads {
        let time_clone = Arc::clone(&time_backend);
        let fs_clone = Arc::clone(&fs_backend);
        let timings_clone = Arc::clone(&timings);
        let test_file = temp_dir.join(format!("integration_concurrent_{}.txt", i));

        let handle = std::thread::spawn(move || {
            let start = time_clone.monotonic_nanos();

            // Write data
            let data = format!("Thread {} data", i);
            fs_clone.write_string(&test_file, &data).unwrap();

            // Read it back
            let read_data = fs_clone.read_to_string(&test_file).unwrap();
            assert_eq!(data, read_data);

            let end = time_clone.monotonic_nanos();
            let duration_us = (end - start) / 1000;

            timings_clone.lock().unwrap().push(duration_us);

            // Cleanup
            std::fs::remove_file(&test_file).ok();
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify all threads completed and logged timings
    let timings = timings.lock().unwrap();
    assert_eq!(timings.len(), num_threads);

    for (i, &duration) in timings.iter().enumerate() {
        println!("Thread {} took {}µs", i, duration);
        assert!(duration > 0, "Thread {} should have measurable time", i);
    }
}

// ============================================================================
// Integration Test 3: High-Priority Thread with File I/O
// ============================================================================

#[test]
fn integration_high_priority_file_processing() {
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");
    let fs_backend = create_filesystem_backend();
    let time_backend = create_time_backend().expect("Failed to create time backend");

    // Try to set high priority (may fail without permissions)
    let _ = threading_backend.set_thread_priority(ThreadPriority::High);

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("integration_priority_test.bin");

    // Write a larger file
    let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

    let start = time_backend.monotonic_nanos();
    fs_backend.write_file(&test_file, &data).expect("Failed to write file");
    let write_time = time_backend.monotonic_nanos() - start;

    let start = time_backend.monotonic_nanos();
    let read_data = fs_backend.read_file(&test_file).expect("Failed to read file");
    let read_time = time_backend.monotonic_nanos() - start;

    println!("High priority I/O: write={}µs, read={}µs", write_time / 1000, read_time / 1000);

    assert_eq!(data, read_data);

    // Reset to normal priority
    let _ = threading_backend.set_thread_priority(ThreadPriority::Normal);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

// ============================================================================
// Integration Test 4: Path Normalization Across Platforms
// ============================================================================

#[test]
fn integration_cross_platform_path_handling() {
    let fs_backend = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();

    // Create a nested directory structure for testing
    let nested_dir = temp_dir.join("integration_path_test");
    std::fs::create_dir_all(&nested_dir).ok();

    // Test various path formats
    let paths_to_test = vec![
        "test.txt",
        "./test.txt",
        "foo/../test.txt",
        "foo/./bar/../test.txt",
        "foo/bar/../../test.txt",
    ];

    for path_str in paths_to_test {
        let path = nested_dir.join(path_str);
        let normalized = fs_backend.normalize_path(&path);

        println!("Original: {:?}", path);
        println!("Normalized: {:?}", normalized);

        // Normalized path should be valid
        assert!(normalized.as_os_str().len() > 0);

        // Test file operations with normalized path
        let test_data = b"cross-platform test";
        fs_backend.write_file(&normalized, test_data).expect("Failed to write with normalized path");

        assert!(
            fs_backend.file_exists(&normalized),
            "Normalized path {:?} should exist",
            normalized
        );

        let read_data = fs_backend.read_file(&normalized).expect("Failed to read with normalized path");
        assert_eq!(test_data, &read_data[..]);

        // Cleanup
        std::fs::remove_file(&normalized).ok();
    }

    // Cleanup directory
    std::fs::remove_dir_all(&nested_dir).ok();
}

// ============================================================================
// Integration Test 5: Sleep Accuracy with Different Priorities
// ============================================================================

#[test]
fn integration_sleep_accuracy_with_priority() {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");

    let priorities = vec![ThreadPriority::Low, ThreadPriority::Normal, ThreadPriority::High];

    for priority in priorities {
        // Try to set priority (may fail, that's okay)
        let _ = threading_backend.set_thread_priority(priority);

        let sleep_duration = Duration::from_millis(10);
        let start = time_backend.monotonic_nanos();
        time_backend.sleep(sleep_duration);
        let end = time_backend.monotonic_nanos();

        let actual_duration_ms = (end - start) / 1_000_000;
        let expected_ms = sleep_duration.as_millis() as u64;

        println!("Priority {:?}: requested {}ms, actual {}ms", priority, expected_ms, actual_duration_ms);

        // Sleep should be reasonably accurate (within 5ms)
        assert!(
            actual_duration_ms >= expected_ms.saturating_sub(5),
            "Sleep too short for {:?}: {}ms < {}ms",
            priority,
            actual_duration_ms,
            expected_ms
        );
    }

    // Reset to normal
    let _ = threading_backend.set_thread_priority(ThreadPriority::Normal);
}

// ============================================================================
// Integration Test 6: Concurrent Time Measurements
// ============================================================================

#[test]
fn integration_concurrent_time_measurements() {
    let time_backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let num_threads = 8;
    let samples_per_thread = 1000;

    let mut handles = vec![];
    let all_times = Arc::new(Mutex::new(Vec::new()));

    for _ in 0..num_threads {
        let time_clone = Arc::clone(&time_backend);
        let times_clone = Arc::clone(&all_times);

        let handle = std::thread::spawn(move || {
            let mut local_times = Vec::with_capacity(samples_per_thread);

            for _ in 0..samples_per_thread {
                local_times.push(time_clone.monotonic_nanos());
            }

            // Verify local monotonicity
            for i in 1..local_times.len() {
                assert!(
                    local_times[i] >= local_times[i - 1],
                    "Time not monotonic in thread: {} -> {}",
                    local_times[i - 1],
                    local_times[i]
                );
            }

            times_clone.lock().unwrap().extend(local_times);
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify we collected all samples
    let times = all_times.lock().unwrap();
    assert_eq!(times.len(), num_threads * samples_per_thread);

    // All times should be positive
    for &time in times.iter() {
        assert!(time > 0, "Time should always be positive");
    }
}

// ============================================================================
// Integration Test 7: File System Error Handling
// ============================================================================

#[test]
fn integration_filesystem_error_handling() {
    let fs_backend = create_filesystem_backend();

    // Test 1: Read nonexistent file
    let result = fs_backend.read_file(std::path::Path::new("/nonexistent/file.txt"));
    assert!(result.is_err(), "Reading nonexistent file should fail");

    // Test 2: Read nonexistent file as string
    let result = fs_backend.read_to_string(std::path::Path::new("/nonexistent/file.txt"));
    assert!(result.is_err(), "Reading nonexistent file as string should fail");

    // Test 3: Invalid UTF-8 handling
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("integration_invalid_utf8.bin");

    // Write invalid UTF-8 data
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    fs_backend.write_file(&test_file, &invalid_utf8).expect("Failed to write binary data");

    // Reading as string should fail
    let result = fs_backend.read_to_string(&test_file);
    assert!(result.is_err(), "Reading invalid UTF-8 as string should fail");

    // But reading as bytes should work
    let bytes = fs_backend.read_file(&test_file).expect("Failed to read as bytes");
    assert_eq!(bytes, invalid_utf8);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

// ============================================================================
// Integration Test 8: Combined Platform Backend Creation
// ============================================================================

#[test]
fn integration_create_all_backends() {
    // Create all backends at once (simulating real application startup)
    let time_start = std::time::Instant::now();

    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");

    let elapsed = time_start.elapsed();
    println!("Backend creation took {:?}", elapsed);

    // Verify all backends work
    let _time = time_backend.monotonic_nanos();
    let _exists = fs_backend.file_exists(std::env::temp_dir().as_path());
    let _num_cpus = threading_backend.num_cpus();

    // Creation should be fast (< 1ms)
    assert!(elapsed < Duration::from_millis(1), "Backend creation too slow: {:?}", elapsed);
}

// ============================================================================
// Integration Test 9: Real-World Profiling Scenario
// ============================================================================

#[test]
fn integration_realistic_profiling_workflow() {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();

    let temp_dir = std::env::temp_dir();
    let profile_file = temp_dir.join("integration_profile.json");

    // Simulate a profiling workflow
    struct ProfileEvent {
        name: String,
        start_ns: u64,
        duration_ns: u64,
    }

    let mut events = Vec::new();

    // Event 1: File write
    let start = time_backend.monotonic_nanos();
    fs_backend.write_string(&profile_file, "{\"events\":[]}").unwrap();
    let duration = time_backend.monotonic_nanos() - start;
    events.push(ProfileEvent {
        name: "write_profile".to_string(),
        start_ns: start,
        duration_ns: duration,
    });

    // Event 2: File read
    let start = time_backend.monotonic_nanos();
    let _data = fs_backend.read_to_string(&profile_file).unwrap();
    let duration = time_backend.monotonic_nanos() - start;
    events.push(ProfileEvent {
        name: "read_profile".to_string(),
        start_ns: start,
        duration_ns: duration,
    });

    // Event 3: Sleep (simulate workload)
    let start = time_backend.monotonic_nanos();
    time_backend.sleep(Duration::from_millis(5));
    let duration = time_backend.monotonic_nanos() - start;
    events.push(ProfileEvent {
        name: "workload".to_string(),
        start_ns: start,
        duration_ns: duration,
    });

    // Verify events
    for event in &events {
        println!("{}: start={}ns, duration={}µs", event.name, event.start_ns, event.duration_ns / 1000);
        assert!(event.duration_ns > 0, "Event {} should have positive duration", event.name);
    }

    // Events should be in chronological order
    for i in 1..events.len() {
        assert!(
            events[i].start_ns >= events[i - 1].start_ns,
            "Events should be chronological"
        );
    }

    // Cleanup
    std::fs::remove_file(&profile_file).ok();
}

// ============================================================================
// Integration Test 10: Thread Affinity with File I/O Benchmark
// ============================================================================

#[test]
fn integration_pinned_thread_performance() {
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");
    let fs_backend = create_filesystem_backend();
    let time_backend = create_time_backend().expect("Failed to create time backend");

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("integration_affinity_test.bin");
    let data = vec![0u8; 10000];

    // Test 1: Normal affinity (all cores)
    let num_cpus = threading_backend.num_cpus();
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = threading_backend.set_thread_affinity(&all_cores);

    let start = time_backend.monotonic_nanos();
    for _ in 0..10 {
        fs_backend.write_file(&test_file, &data).unwrap();
        let _ = fs_backend.read_file(&test_file).unwrap();
    }
    let all_cores_time = time_backend.monotonic_nanos() - start;

    // Test 2: Single core affinity
    let _ = threading_backend.set_thread_affinity(&[0]);

    let start = time_backend.monotonic_nanos();
    for _ in 0..10 {
        fs_backend.write_file(&test_file, &data).unwrap();
        let _ = fs_backend.read_file(&test_file).unwrap();
    }
    let single_core_time = time_backend.monotonic_nanos() - start;

    println!("All cores: {}µs, Single core: {}µs", all_cores_time / 1000, single_core_time / 1000);

    // Reset to all cores
    let _ = threading_backend.set_thread_affinity(&all_cores);

    // Both should complete successfully
    assert!(all_cores_time > 0);
    assert!(single_core_time > 0);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}
