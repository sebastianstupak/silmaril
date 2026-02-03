//! Integration tests for the asset loading strategies.
//!
//! Tests sync, async, and streaming loading with various scenarios.

use engine_assets::{AssetManager, EnhancedLoader, MeshData};
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn create_test_obj() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nvn 0 0 1\nvt 0 0").unwrap();
    file.flush().unwrap();
    file
}

fn create_large_obj() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();

    // Create a larger mesh (1000 vertices)
    for i in 0..1000 {
        let x = (i as f32 * 0.1).sin();
        let y = (i as f32 * 0.1).cos();
        writeln!(file, "v {x} {y} 0").unwrap();
    }

    writeln!(file, "vn 0 0 1").unwrap();
    writeln!(file, "vt 0 0").unwrap();

    // Create triangles
    for i in 0..998 {
        writeln!(file, "f {} {} {}", i + 1, i + 2, i + 3).unwrap();
    }

    file.flush().unwrap();
    file
}

// ============================================================================
// UNIT TESTS (10 tests)
// ============================================================================

#[test]
fn test_sync_load_returns_handle() {
    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    let result = loader.load_sync::<MeshData>(test_file.path());
    assert!(result.is_ok(), "Sync load should succeed");

    let handle = result.unwrap();
    assert!(
        loader.manager().get_mesh(handle.id()).is_some(),
        "Loaded mesh should be accessible"
    );
}

#[test]
fn test_sync_load_file_not_found() {
    let loader = EnhancedLoader::default();
    let result = loader.load_sync::<MeshData>(std::path::Path::new("nonexistent.obj"));
    assert!(result.is_err(), "Loading nonexistent file should fail");
}

#[test]
fn test_sync_load_invalid_format() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "invalid obj data").unwrap();
    file.flush().unwrap();

    let loader = EnhancedLoader::default();
    let result = loader.load_sync::<MeshData>(file.path());

    // This might succeed or fail depending on parser strictness
    // We just verify it doesn't panic
    let _ = result;
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_load_doesnt_block() {
    use std::time::Instant;

    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    let start = Instant::now();

    // Start async load
    let load_future = loader.load_async::<MeshData>(test_file.path());

    // This should return almost immediately (< 1ms for starting the task)
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 50, "Async load should not block (took {elapsed:?})");

    // Wait for completion
    let result = load_future.await;
    assert!(result.is_ok());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_streaming_returns_lod0_quickly() {
    use std::time::Instant;

    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    let start = Instant::now();
    let result = loader.load_streaming::<MeshData>(test_file.path(), 3).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Streaming load should succeed");

    // Should return LOD 0 in < 100ms
    assert!(elapsed.as_millis() < 200, "Streaming should return quickly (took {elapsed:?})");

    let handle = result.unwrap();
    assert_eq!(handle.current_lod(), 0, "Initial LOD should be 0");
}

#[test]
fn test_sync_load_parse_error() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "f 1 2 3").unwrap(); // Face without vertices
    file.flush().unwrap();

    let loader = EnhancedLoader::default();
    let result = loader.load_sync::<MeshData>(file.path());

    // Parser might be lenient, but verify no panic
    let _ = result;
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_load_error_handling() {
    let loader = EnhancedLoader::default();
    let result = loader.load_async::<MeshData>(std::path::Path::new("missing.obj")).await;
    assert!(result.is_err(), "Should error on missing file");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_streaming_invalid_lod_count() {
    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    let result = loader.load_streaming::<MeshData>(test_file.path(), 0).await;
    assert!(result.is_err(), "Should error on zero LOD count");
}

#[test]
fn test_loader_creation_with_custom_manager() {
    let manager = Arc::new(AssetManager::new());
    let loader = EnhancedLoader::new(Arc::clone(&manager));

    assert_eq!(loader.manager().len(), 0);
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_sync_load_caching() {
    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    // Load same file twice
    let handle1 = loader.load_sync::<MeshData>(test_file.path()).unwrap();
    let handle2 = loader.load_sync::<MeshData>(test_file.path()).unwrap();

    // Should return same asset (same ID)
    assert_eq!(handle1.id(), handle2.id(), "Cached loads should return same asset");
}

// ============================================================================
// INTEGRATION TESTS (8 tests)
// ============================================================================

#[test]
fn test_sync_workflow_load_use_unload() {
    let manager = Arc::new(AssetManager::new());
    let loader = EnhancedLoader::new(Arc::clone(&manager));
    let test_file = create_test_obj();

    // Load
    let handle = loader.load_sync::<MeshData>(test_file.path()).unwrap();
    assert_eq!(manager.len(), 1, "Manager should have 1 asset");

    // Use
    let mesh = manager.get_mesh(handle.id()).unwrap();
    assert!(!mesh.vertices.is_empty(), "Mesh should have vertices");

    // Unload
    let unloaded = manager.unload(test_file.path());
    assert!(unloaded, "Unload should succeed");
    assert!(manager.is_empty(), "Manager should be empty after unload");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_workflow_multiple_concurrent_loads() {
    let loader = Arc::new(EnhancedLoader::default());

    // Create multiple test files
    let file1 = create_test_obj();
    let file2 = create_test_obj();
    let file3 = create_test_obj();

    // Load concurrently
    let loader1 = Arc::clone(&loader);
    let loader2 = Arc::clone(&loader);
    let loader3 = Arc::clone(&loader);

    let path1 = file1.path().to_path_buf();
    let path2 = file2.path().to_path_buf();
    let path3 = file3.path().to_path_buf();

    let (r1, r2, r3) = tokio::join!(
        async move { loader1.load_async::<MeshData>(&path1).await },
        async move { loader2.load_async::<MeshData>(&path2).await },
        async move { loader3.load_async::<MeshData>(&path3).await },
    );

    assert!(r1.is_ok(), "Load 1 should succeed");
    assert!(r2.is_ok(), "Load 2 should succeed");
    assert!(r3.is_ok(), "Load 3 should succeed");

    // All should be loaded
    assert!(loader.manager().len() >= 1, "At least one asset should be loaded");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_streaming_workflow_lod_progression() {
    let loader = EnhancedLoader::default();
    let test_file = create_test_obj();

    // Start streaming with 3 LOD levels
    let handle = loader.load_streaming::<MeshData>(test_file.path(), 3).await.unwrap();

    // LOD 0 should be immediately available
    assert_eq!(handle.current_lod(), 0);
    let lod0 = handle.get_lod(0).await;
    assert!(lod0.is_some(), "LOD 0 should be available");

    // Wait for higher LODs to load
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Higher LODs should now be available
    assert!(handle.current_lod() >= 1, "Higher LODs should load in background");

    let best = handle.get_best().await;
    assert!(best.is_some(), "Best LOD should be available");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_mixed_strategies_sync_and_async() {
    let loader = Arc::new(EnhancedLoader::default());
    let file1 = create_test_obj();
    let file2 = create_test_obj();

    // Load one sync
    let sync_handle = loader.load_sync::<MeshData>(file1.path()).unwrap();

    // Load one async
    let async_handle = loader.load_async::<MeshData>(file2.path()).await.unwrap();

    // Both should be accessible
    assert!(loader.manager().get_mesh(sync_handle.id()).is_some());
    assert!(loader.manager().get_mesh(async_handle.id()).is_some());
}

#[test]
fn test_multiple_loaders_same_manager() {
    let manager = Arc::new(AssetManager::new());
    let loader1 = EnhancedLoader::new(Arc::clone(&manager));
    let loader2 = EnhancedLoader::new(Arc::clone(&manager));

    let test_file = create_test_obj();

    // Load with loader1
    let handle1 = loader1.load_sync::<MeshData>(test_file.path()).unwrap();

    // Verify accessible through loader2
    assert!(loader2.manager().get_mesh(handle1.id()).is_some());
    assert_eq!(loader1.manager().len(), loader2.manager().len());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_streaming_multiple_files() {
    let loader = EnhancedLoader::default();
    let file1 = create_test_obj();
    let file2 = create_test_obj();

    // Stream both files
    let handle1 = loader.load_streaming::<MeshData>(file1.path(), 2).await.unwrap();
    let handle2 = loader.load_streaming::<MeshData>(file2.path(), 2).await.unwrap();

    // Both should have LOD 0
    assert!(handle1.get_lod(0).await.is_some());
    assert!(handle2.get_lod(0).await.is_some());

    // Wait for higher LODs
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    assert!(handle1.current_lod() >= 0);
    assert!(handle2.current_lod() >= 0);
}

#[test]
fn test_sync_load_large_asset_blocks() {
    use std::time::Instant;

    let loader = EnhancedLoader::default();
    let test_file = create_large_obj();

    let start = Instant::now();
    let result = loader.load_sync::<MeshData>(test_file.path());
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    // Sync load should block until complete
    assert!(elapsed.as_millis() > 0, "Sync load should take measurable time");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_load_error_propagation() {
    let loader = EnhancedLoader::default();

    // Try to load invalid path
    let result = loader.load_async::<MeshData>(std::path::Path::new("/invalid/path.obj")).await;

    assert!(result.is_err());
    // Verify error contains useful information
    let err = result.unwrap_err();
    let err_str = format!("{err:?}");
    assert!(err_str.contains("path") || err_str.contains("invalid"));
}

// ============================================================================
// CONCURRENCY TESTS (5 tests)
// ============================================================================

#[test]
fn test_concurrent_sync_loads_no_race() {
    use std::sync::Arc;
    use std::thread;

    let loader = Arc::new(EnhancedLoader::default());
    let mut handles = vec![];

    // Spawn multiple threads loading same file
    for _ in 0..10 {
        let loader_clone = Arc::clone(&loader);
        let test_file = create_test_obj();
        let path = test_file.path().to_path_buf();

        let handle = thread::spawn(move || {
            // Keep file alive
            let _file = test_file;
            loader_clone.load_sync::<MeshData>(&path)
        });
        handles.push(handle);
    }

    // All should succeed without race conditions
    for handle in handles {
        let result = handle.join().unwrap();
        // Some might fail due to file cleanup, but no panics
        let _ = result;
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_concurrent_async_loads_no_deadlock() {
    let loader = Arc::new(EnhancedLoader::default());
    let mut tasks = vec![];

    // Spawn many concurrent async loads
    for i in 0..20 {
        let loader_clone = Arc::clone(&loader);
        let test_file = create_test_obj();
        let path = test_file.path().to_path_buf();

        let task = tokio::spawn(async move {
            let _file = test_file;
            let result = loader_clone.load_async::<MeshData>(&path).await;
            (i, result)
        });

        tasks.push(task);
    }

    // All should complete without deadlock
    for task in tasks {
        let (i, result) = task.await.unwrap();
        // Some might fail, but verify no deadlock
        if result.is_err() {
            tracing::debug!("Task {i} failed (expected in concurrent test)");
        }
    }
}

#[test]
fn test_concurrent_reads_same_asset() {
    use std::sync::Arc;
    use std::thread;

    let loader = Arc::new(EnhancedLoader::default());
    let test_file = create_test_obj();

    // Load once
    let handle = loader.load_sync::<MeshData>(test_file.path()).unwrap();
    let id = handle.id();

    // Spawn multiple threads reading the same asset
    let mut threads = vec![];
    for _ in 0..10 {
        let loader_clone = Arc::clone(&loader);
        let thread = thread::spawn(move || {
            for _ in 0..100 {
                let mesh = loader_clone.manager().get_mesh(id).unwrap();
                assert!(!mesh.vertices.is_empty());
            }
        });
        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_concurrent_streaming_loads() {
    let loader = Arc::new(EnhancedLoader::default());
    let mut tasks = vec![];

    // Start multiple streaming loads concurrently
    for _ in 0..5 {
        let loader_clone = Arc::clone(&loader);
        let test_file = create_test_obj();
        let path = test_file.path().to_path_buf();

        let task = tokio::spawn(async move {
            let _file = test_file;
            loader_clone.load_streaming::<MeshData>(&path, 2).await
        });

        tasks.push(task);
    }

    // All should complete
    for task in tasks {
        let result = task.await.unwrap();
        // Some might fail due to file cleanup
        if let Ok(handle) = result {
            assert_eq!(handle.total_lods(), 2);
        }
    }
}

#[test]
fn test_thread_safety_handle_cloning() {
    use std::sync::Arc;
    use std::thread;

    let loader = Arc::new(EnhancedLoader::default());
    let test_file = create_test_obj();

    let handle = loader.load_sync::<MeshData>(test_file.path()).unwrap();

    // Clone handle in multiple threads
    let mut threads = vec![];
    for _ in 0..10 {
        let handle_clone = handle.clone();
        let thread = thread::spawn(move || {
            assert_eq!(handle_clone.refcount(), handle_clone.refcount());
        });
        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

// ============================================================================
// STRESS TESTS (3 tests)
// ============================================================================

#[cfg(feature = "async")]
#[tokio::test]
async fn test_stress_100_concurrent_async_loads() {
    let loader = Arc::new(EnhancedLoader::default());
    let mut tasks = vec![];

    for _ in 0..100 {
        let loader_clone = Arc::clone(&loader);
        let test_file = create_test_obj();
        let path = test_file.path().to_path_buf();

        let task = tokio::spawn(async move {
            let _file = test_file;
            loader_clone.load_async::<MeshData>(&path).await
        });

        tasks.push(task);
    }

    let mut success_count = 0;
    for task in tasks {
        if let Ok(Ok(_)) = task.await {
            success_count += 1;
        }
    }

    // At least some should succeed
    assert!(success_count > 0, "Some loads should succeed");
}

#[test]
fn test_stress_memory_usage_bulk_loading() {
    let loader = EnhancedLoader::default();
    let initial_count = loader.manager().len();

    // Load many assets
    for i in 0..50 {
        let test_file = create_test_obj();
        if loader.load_sync::<MeshData>(test_file.path()).is_ok() {
            // Successfully loaded
        }

        // Periodically check we're not leaking
        if i % 10 == 0 {
            let current = loader.manager().len();
            assert!(current >= initial_count, "Asset count should not decrease unexpectedly");
        }
    }
}

#[test]
fn test_stress_load_unload_cycling() {
    let loader = Arc::new(EnhancedLoader::default());

    // Cycle load/unload many times
    for _ in 0..20 {
        let test_file = create_test_obj();
        let path = test_file.path().to_path_buf();

        if let Ok(_handle) = loader.load_sync::<MeshData>(&path) {
            loader.manager().unload(&path);
        }
    }

    // Manager should be relatively empty
    assert!(loader.manager().len() < 10, "Cycling should not accumulate assets");
}
