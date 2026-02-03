//! Edge case tests for platform abstraction layer.
//!
//! These tests verify correct behavior in unusual or boundary conditions:
//! - Edge cases in time measurement (overflow, precision limits)
//! - Edge cases in filesystem (empty files, special characters, permissions)
//! - Edge cases in threading (single CPU systems, invalid priorities)

use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, ThreadPriority,
};
use std::time::Duration;

// ============================================================================
// Time Backend Edge Cases
// ============================================================================

#[test]
fn edge_time_zero_duration_sleep() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Sleep for zero duration should not panic
    let start = backend.monotonic_nanos();
    backend.sleep(Duration::from_nanos(0));
    let end = backend.monotonic_nanos();

    // Should complete quickly (within 1ms)
    assert!((end - start) < 1_000_000);
}

#[test]
fn edge_time_very_short_sleep() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Sleep for 1 nanosecond (should be rounded up by OS)
    backend.sleep(Duration::from_nanos(1));

    // Should not panic
}

#[test]
fn edge_time_back_to_back_queries() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Query time twice in immediate succession
    let t1 = backend.monotonic_nanos();
    let t2 = backend.monotonic_nanos();

    // Time should either advance or stay the same (depending on precision)
    assert!(t2 >= t1);

    // The difference should be small (< 1ms)
    assert!((t2 - t1) < 1_000_000);
}

#[test]
fn edge_time_duration_addition() {
    let backend = create_time_backend().expect("Failed to create time backend");

    let d1 = backend.now();
    let d2 = backend.now();

    // Adding durations should not panic
    let sum = d1 + d2;
    assert!(sum >= d1);
    assert!(sum >= d2);
}

#[test]
fn edge_time_duration_subtraction() {
    let backend = create_time_backend().expect("Failed to create time backend");

    let d1 = backend.now();
    std::thread::sleep(Duration::from_millis(10));
    let d2 = backend.now();

    // Subtracting durations should work
    let diff = d2 - d1;
    assert!(diff >= Duration::from_millis(5));
}

// ============================================================================
// Filesystem Backend Edge Cases
// ============================================================================

#[test]
fn edge_filesystem_empty_file() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_empty_file.txt");

    // Write empty file
    fs.write_file(&test_file, b"").expect("Failed to write empty file");

    // Should exist
    assert!(fs.file_exists(&test_file));

    // Read should return empty vec
    let data = fs.read_file(&test_file).expect("Failed to read empty file");
    assert_eq!(data.len(), 0);

    // Read as string should return empty string
    let text = fs.read_to_string(&test_file).expect("Failed to read empty string");
    assert_eq!(text, "");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_single_byte_file() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_single_byte.bin");

    // Write single byte
    fs.write_file(&test_file, &[42]).expect("Failed to write single byte");

    let data = fs.read_file(&test_file).expect("Failed to read single byte");
    assert_eq!(data, vec![42]);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_special_characters_in_content() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_special_chars.txt");

    // Test content with special characters
    let special_content = "Line1\nLine2\r\nLine3\tTabbed\0Null";
    fs.write_string(&test_file, special_content)
        .expect("Failed to write special chars");

    let read_content = fs.read_to_string(&test_file).expect("Failed to read special chars");
    assert_eq!(read_content, special_content);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_emoji_in_content() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_emoji.txt");

    // Test content with emojis (multi-byte UTF-8)
    let emoji_content = "Hello 👋 World 🌍 Game 🎮 Engine 🚀";
    fs.write_string(&test_file, emoji_content).expect("Failed to write emoji");

    let read_content = fs.read_to_string(&test_file).expect("Failed to read emoji");
    assert_eq!(read_content, emoji_content);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_long_filename() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();

    // Create a long but valid filename (< 255 chars)
    let long_name = "a".repeat(200) + ".txt";
    let test_file = temp_dir.join(&long_name);

    // Should work on most platforms
    let result = fs.write_string(&test_file, "test");

    if result.is_ok() {
        assert!(fs.file_exists(&test_file));
        let content = fs.read_to_string(&test_file).expect("Failed to read long filename");
        assert_eq!(content, "test");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }
    // If it fails, that's also acceptable (filesystem limitation)
}

#[test]
fn edge_filesystem_path_with_dots() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();

    // Create a file with dots in the name
    let test_file = temp_dir.join("test.file.with.dots.txt");
    fs.write_string(&test_file, "dots").expect("Failed to write dotted filename");

    assert!(fs.file_exists(&test_file));
    let content = fs.read_to_string(&test_file).expect("Failed to read dotted filename");
    assert_eq!(content, "dots");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_normalize_current_dir() {
    let fs = create_filesystem_backend();

    // Normalize current directory reference
    let path = std::path::PathBuf::from(".");
    let normalized = fs.normalize_path(&path);

    // Should produce a valid path
    assert!(normalized.as_os_str().len() > 0);
}

#[test]
fn edge_filesystem_normalize_parent_dir() {
    let fs = create_filesystem_backend();

    // Normalize parent directory reference
    let path = std::path::PathBuf::from("..");
    let normalized = fs.normalize_path(&path);

    // Should produce a valid path
    assert!(normalized.as_os_str().len() > 0);
}

#[test]
fn edge_filesystem_normalize_empty_path() {
    let fs = create_filesystem_backend();

    // Normalize empty path
    let path = std::path::PathBuf::from("");
    let normalized = fs.normalize_path(&path);

    // Should handle gracefully (behavior may vary)
    // Just verify it doesn't panic
    let _ = normalized;
}

#[test]
fn edge_filesystem_binary_with_all_bytes() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_all_bytes.bin");

    // Create data with all possible byte values
    let all_bytes: Vec<u8> = (0..=255).collect();
    fs.write_file(&test_file, &all_bytes).expect("Failed to write all bytes");

    let read_data = fs.read_file(&test_file).expect("Failed to read all bytes");
    assert_eq!(read_data, all_bytes);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_read_nonexistent_file() {
    let fs = create_filesystem_backend();

    // Try to read a file that definitely doesn't exist
    let nonexistent =
        std::path::PathBuf::from("/tmp/this_file_absolutely_does_not_exist_12345.tmp");
    let result = fs.read_file(&nonexistent);

    assert!(result.is_err(), "Reading nonexistent file should fail");
}

#[test]
fn edge_filesystem_overwrite_existing_file() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_overwrite.txt");

    // Write initial content
    fs.write_string(&test_file, "original").expect("Failed to write original");

    // Verify
    let content1 = fs.read_to_string(&test_file).expect("Failed to read original");
    assert_eq!(content1, "original");

    // Overwrite with new content
    fs.write_string(&test_file, "modified").expect("Failed to overwrite");

    // Verify new content
    let content2 = fs.read_to_string(&test_file).expect("Failed to read modified");
    assert_eq!(content2, "modified");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

// ============================================================================
// Threading Backend Edge Cases
// ============================================================================

#[test]
fn edge_threading_single_cpu_affinity() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Set affinity to just the first CPU
    let result = backend.set_thread_affinity(&[0]);

    // Should succeed or be ignored (platform-dependent)
    // Just verify it doesn't panic
    let _ = result;

    // Reset to all cores
    let num_cpus = backend.num_cpus();
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

#[test]
fn edge_threading_empty_affinity() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Try to set empty affinity (should fail or be ignored)
    let result = backend.set_thread_affinity(&[]);

    // Behavior is platform-dependent, just verify no panic
    let _ = result;

    // Reset to all cores
    let num_cpus = backend.num_cpus();
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

#[test]
fn edge_threading_duplicate_cores_in_affinity() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Try to set affinity with duplicate cores
    let result = backend.set_thread_affinity(&[0, 0, 1, 1]);

    // Should handle gracefully
    let _ = result;

    // Reset
    let num_cpus = backend.num_cpus();
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

#[test]
fn edge_threading_out_of_range_affinity() {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let num_cpus = backend.num_cpus();

    // Try to set affinity to cores beyond available CPUs
    let result = backend.set_thread_affinity(&[num_cpus + 1000]);

    // Should fail gracefully (not panic)
    let _ = result;

    // Reset
    let all_cores: Vec<usize> = (0..num_cpus).collect();
    let _ = backend.set_thread_affinity(&all_cores);
}

#[test]
fn edge_threading_priority_sequence() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Test all priority transitions
    let priorities = [
        ThreadPriority::Low,
        ThreadPriority::Normal,
        ThreadPriority::High,
        ThreadPriority::Realtime,
        ThreadPriority::High,
        ThreadPriority::Normal,
        ThreadPriority::Low,
    ];

    for priority in &priorities {
        // May fail for Realtime without permissions, that's okay
        let _ = backend.set_thread_priority(*priority);
    }

    // Reset
    let _ = backend.set_thread_priority(ThreadPriority::Normal);
}

#[test]
fn edge_threading_num_cpus_reasonable() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    let num_cpus = backend.num_cpus();

    // Should be at least 1
    assert!(num_cpus >= 1, "num_cpus should be at least 1");

    // Should be reasonable (< 1024 for now)
    assert!(num_cpus <= 1024, "num_cpus should be reasonable");
}

#[test]
fn edge_threading_num_cpus_consistent() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Query multiple times, should return same value
    let count1 = backend.num_cpus();
    let count2 = backend.num_cpus();
    let count3 = backend.num_cpus();

    assert_eq!(count1, count2);
    assert_eq!(count2, count3);
}

// ============================================================================
// Cross-Backend Edge Cases
// ============================================================================

#[test]
fn edge_combined_empty_operations() {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();
    let threading_backend = create_threading_backend().expect("Failed to create threading backend");

    // Do minimal operations with each backend
    let _time = time_backend.monotonic_nanos();
    let _exists = fs_backend.file_exists(std::env::temp_dir().as_path());
    let _cpus = threading_backend.num_cpus();

    // Should all complete without errors
}

#[test]
fn edge_combined_zero_sleep_with_file_check() {
    let time_backend = create_time_backend().expect("Failed to create time backend");
    let fs_backend = create_filesystem_backend();

    // Zero sleep followed by file existence check
    time_backend.sleep(Duration::from_nanos(0));
    let _exists = fs_backend.file_exists(std::env::temp_dir().as_path());

    // Should complete without issues
}

#[test]
fn edge_filesystem_write_then_immediate_read() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_immediate_read.txt");

    // Write and immediately read (no delay)
    fs.write_string(&test_file, "immediate").expect("Failed to write");
    let content = fs.read_to_string(&test_file).expect("Failed to read");

    assert_eq!(content, "immediate");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn edge_filesystem_rapid_write_read_cycle() {
    let fs = create_filesystem_backend();
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("edge_rapid_cycle.txt");

    // Rapidly write and read 100 times
    for i in 0..100 {
        let data = format!("iteration_{}", i);
        fs.write_string(&test_file, &data).expect("Failed to write");

        let read_data = fs.read_to_string(&test_file).expect("Failed to read");
        assert_eq!(read_data, data);
    }

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}
