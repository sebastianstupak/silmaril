//! Runtime tests for platform trait implementations.
//!
//! These tests validate that all platform backends:
//! - Implement required traits correctly
//! - Have proper Send + Sync bounds
//! - Can be created via factory functions
//! - Provide acceptable time precision

use engine_core::platform::{
    create_filesystem_backend, create_threading_backend, create_time_backend, FileSystemBackend,
    ThreadPriority, ThreadingBackend, TimeBackend,
};
use std::time::Duration;

#[test]
fn test_time_backend_implements_trait() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Verify trait implementation works
    let t1 = backend.monotonic_nanos();
    assert!(t1 > 0, "monotonic_nanos should return positive value");

    let now = backend.now();
    assert!(now.as_nanos() > 0, "now() should return positive duration");
}

#[test]
fn test_time_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn TimeBackend>>();

    // Verify the concrete backend is also Send + Sync
    let backend = create_time_backend().expect("Failed to create time backend");
    let _boxed: Box<dyn TimeBackend> = backend;
}

#[test]
fn test_filesystem_backend_implements_trait() {
    let backend = create_filesystem_backend();

    // Verify trait implementation works
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("arch_test_fs.txt");

    // Write and read operations
    backend.write_file(&test_file, b"test data").expect("Failed to write file");

    assert!(backend.file_exists(&test_file), "File should exist after write");

    let content = backend.read_file(&test_file).expect("Failed to read file");
    assert_eq!(content, b"test data");

    // String operations
    backend.write_string(&test_file, "hello world").expect("Failed to write string");
    let text = backend.read_to_string(&test_file).expect("Failed to read string");
    assert_eq!(text, "hello world");

    // Path normalization
    let normalized = backend.normalize_path(&test_file);
    assert!(normalized.is_absolute() || normalized.exists());

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_filesystem_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn FileSystemBackend>>();

    // Verify the concrete backend is also Send + Sync
    let backend = create_filesystem_backend();
    let _boxed: Box<dyn FileSystemBackend> = backend;
}

#[test]
fn test_threading_backend_implements_trait() {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    // Verify trait implementation works
    let num_cpus = backend.num_cpus();
    assert!(num_cpus > 0, "num_cpus should return positive value");
    assert!(num_cpus <= 1024, "num_cpus should be reasonable");

    // Test priority setting (Low/Normal should always work)
    backend
        .set_thread_priority(ThreadPriority::Normal)
        .expect("Failed to set Normal priority");

    backend
        .set_thread_priority(ThreadPriority::Low)
        .expect("Failed to set Low priority");

    // High priority may require permissions, so we just test it doesn't panic
    let _ = backend.set_thread_priority(ThreadPriority::High);

    // Test affinity (may fail without permissions, but shouldn't panic)
    let _ = backend.set_thread_affinity(&[0]);
}

#[test]
fn test_threading_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn ThreadingBackend>>();

    // Verify the concrete backend is also Send + Sync
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let _boxed: Box<dyn ThreadingBackend> = backend;
}

#[test]
fn test_factory_functions_work() {
    // All factory functions should succeed on supported platforms
    let time_result = create_time_backend();
    assert!(time_result.is_ok(), "Time backend creation failed: {:?}", time_result.err());

    let fs_backend = create_filesystem_backend();
    // Filesystem backend doesn't return Result, so just verify it's created
    assert!(fs_backend.file_exists(std::env::temp_dir().as_path()));

    let threading_result = create_threading_backend();
    assert!(
        threading_result.is_ok(),
        "Threading backend creation failed: {:?}",
        threading_result.err()
    );
}

#[test]
fn test_time_precision_is_acceptable() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Test precision by measuring a small sleep
    let t1 = backend.monotonic_nanos();
    std::thread::sleep(Duration::from_millis(1));
    let t2 = backend.monotonic_nanos();

    let diff = t2 - t1;

    // Should have at least microsecond precision (difference should be in reasonable range)
    assert!(diff >= 500_000, "Time precision too low: {}ns for 1ms sleep", diff);
    assert!(diff <= 100_000_000, "Time precision unreasonable: {}ns for 1ms sleep", diff);
}

#[test]
fn test_time_monotonicity_under_load() {
    let backend = create_time_backend().expect("Failed to create time backend");

    let mut last_time = backend.monotonic_nanos();

    // Rapidly query time to ensure it never goes backwards
    for _ in 0..10000 {
        let current_time = backend.monotonic_nanos();
        assert!(
            current_time >= last_time,
            "Time went backwards: {} -> {}",
            last_time,
            current_time
        );
        last_time = current_time;
    }
}

#[test]
fn test_time_sleep_accuracy() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Test that sleep is reasonably accurate
    let sleep_duration = Duration::from_millis(50);
    let t1 = backend.monotonic_nanos();
    backend.sleep(sleep_duration);
    let t2 = backend.monotonic_nanos();

    let elapsed = Duration::from_nanos(t2 - t1);

    // Sleep should be at least the requested duration (minus a small margin)
    assert!(elapsed >= Duration::from_millis(45), "Sleep too short: {:?} < 45ms", elapsed);

    // Sleep should not be excessively long (allow up to 2x for scheduler variance)
    assert!(elapsed <= Duration::from_millis(200), "Sleep too long: {:?} > 200ms", elapsed);
}

#[test]
fn test_filesystem_unicode_support() {
    let backend = create_filesystem_backend();

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("unicode_test_架构.txt");

    // Test Unicode content
    let unicode_content = "Hello 世界! 🎮 Привет";
    backend
        .write_string(&test_file, unicode_content)
        .expect("Failed to write Unicode string");

    let read_content = backend.read_to_string(&test_file).expect("Failed to read Unicode string");

    assert_eq!(read_content, unicode_content);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_filesystem_binary_data() {
    let backend = create_filesystem_backend();

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("binary_test.bin");

    // Test binary data with all byte values
    let binary_data: Vec<u8> = (0..=255).collect();
    backend
        .write_file(&test_file, &binary_data)
        .expect("Failed to write binary data");

    let read_data = backend.read_file(&test_file).expect("Failed to read binary data");

    assert_eq!(read_data, binary_data);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_threading_priority_order() {
    // Verify ThreadPriority enum ordering is correct
    assert!(ThreadPriority::Low < ThreadPriority::Normal);
    assert!(ThreadPriority::Normal < ThreadPriority::High);
    assert!(ThreadPriority::High < ThreadPriority::Realtime);
}

#[test]
fn test_threading_affinity_invalid_cores() {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let num_cpus = backend.num_cpus();

    // Try to set affinity to a non-existent core
    let result = backend.set_thread_affinity(&[num_cpus + 100]);

    // This should either fail gracefully or be ignored on some platforms
    // Just verify it doesn't panic
    let _ = result;
}

#[test]
fn test_all_backends_can_be_used_in_threads() {
    use std::sync::Arc;
    use std::thread;

    // Create backends
    let time_backend = Arc::new(create_time_backend().expect("Failed to create time backend"));
    let fs_backend = Arc::new(create_filesystem_backend());
    let threading_backend =
        Arc::new(create_threading_backend().expect("Failed to create threading backend"));

    // Spawn threads that use the backends
    let time_clone = Arc::clone(&time_backend);
    let t1 = thread::spawn(move || {
        let _t = time_clone.monotonic_nanos();
    });

    let fs_clone = Arc::clone(&fs_backend);
    let t2 = thread::spawn(move || {
        let _exists = fs_clone.file_exists(std::env::temp_dir().as_path());
    });

    let threading_clone = Arc::clone(&threading_backend);
    let t3 = thread::spawn(move || {
        let _num = threading_clone.num_cpus();
    });

    // Wait for all threads
    t1.join().expect("Thread 1 panicked");
    t2.join().expect("Thread 2 panicked");
    t3.join().expect("Thread 3 panicked");
}

#[test]
fn test_filesystem_error_on_invalid_path() {
    let backend = create_filesystem_backend();

    // Try to read a file that doesn't exist
    let result = backend.read_file(std::path::Path::new("/nonexistent/path/file.txt"));

    assert!(result.is_err(), "Reading nonexistent file should fail");

    // Verify we get the right error type
    if let Err(e) = result {
        use engine_core::EngineError;
        let code = e.code();
        assert_eq!(code, engine_core::ErrorCode::FileSystemError);
    }
}

#[test]
fn test_time_duration_conversion() {
    let backend = create_time_backend().expect("Failed to create time backend");

    // Test that now() returns a valid Duration
    let duration1 = backend.now();
    std::thread::sleep(Duration::from_millis(10));
    let duration2 = backend.now();

    assert!(duration2 > duration1);
    assert!(duration2 - duration1 >= Duration::from_millis(9));
}
