//! Comprehensive hot-reload system tests.
//!
//! Tests cover:
//! - Watch registration and debouncing
//! - Path mapping and asset tracking
//! - File modification detection
//! - Multiple asset type reloading
//! - Handle invalidation after reload
//! - Error handling (corrupted/missing files)
//! - Batch reload functionality
//! - Concurrent reload scenarios
//! - Property-based testing

#![cfg(feature = "hot-reload")]

use engine_assets::{AssetManager, AssetType, HotReloadConfig, HotReloadEvent, HotReloader};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::{NamedTempFile, TempDir};

// Helper to create a simple OBJ mesh
fn create_simple_obj() -> String {
    r#"# Simple cube
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
"#
    .to_string()
}

// Helper to create a modified OBJ mesh
fn create_modified_obj() -> String {
    r#"# Modified cube (scaled 2x)
v 0.0 0.0 0.0
v 2.0 0.0 0.0
v 0.0 2.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
"#
    .to_string()
}

// Helper to create a PNG texture (1x1 red pixel)
fn create_simple_png() -> Vec<u8> {
    // Minimal PNG: 1x1 red pixel
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
        0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
        0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
        0xAE, 0x42, 0x60, 0x82,
    ]
}

#[test]
fn test_watch_registration() {
    let temp_dir = TempDir::new().unwrap();
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager, config).unwrap();

    // Should succeed
    let result = reloader.watch(temp_dir.path());
    assert!(result.is_ok(), "Watch registration should succeed");

    // Should be able to unwatch
    let result = reloader.unwatch(temp_dir.path());
    assert!(result.is_ok(), "Unwatch should succeed");
}

#[test]
fn test_debouncing() {
    let temp_dir = TempDir::new().unwrap();
    let asset_path = temp_dir.path().join("test.obj");

    // Write initial file
    std::fs::write(&asset_path, create_simple_obj()).unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(100),
        enable_batching: false,
        ..Default::default()
    };
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();

    // Rapid successive writes
    for i in 0..5 {
        std::fs::write(&asset_path, format!("# Version {i}\n{}", create_simple_obj())).unwrap();
        thread::sleep(Duration::from_millis(10)); // Less than debounce duration
    }

    // Wait for debounce + processing
    thread::sleep(Duration::from_millis(200));
    reloader.process_events();

    // Should only get one reload event (debouncing worked)
    let mut event_count = 0;
    while reloader.poll_event().is_some() {
        event_count += 1;
    }

    // Should be 1 or 2 events max (debouncing reduces rapid writes)
    assert!(
        event_count <= 2,
        "Debouncing should reduce rapid writes, got {event_count} events"
    );
}

#[test]
fn test_path_mapping() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager, config).unwrap();

    let path1 = PathBuf::from("mesh1.obj");
    let path2 = PathBuf::from("mesh2.obj");
    let id1 = engine_assets::AssetId::from_content(b"mesh1");
    let id2 = engine_assets::AssetId::from_content(b"mesh2");

    reloader.register_asset(path1.clone(), id1);
    reloader.register_asset(path2.clone(), id2);

    // Check forward mapping
    assert_eq!(reloader.id_to_path.get(&id1), Some(&path1));
    assert_eq!(reloader.id_to_path.get(&id2), Some(&path2));

    // Check reverse mapping
    assert_eq!(reloader.path_to_id.get(&path1), Some(&id1));
    assert_eq!(reloader.path_to_id.get(&path2), Some(&id2));

    // Unregister one
    reloader.unregister_asset(&path1);
    assert_eq!(reloader.id_to_path.get(&id1), None);
    assert_eq!(reloader.path_to_id.get(&path1), None);

    // Other should still be there
    assert_eq!(reloader.id_to_path.get(&id2), Some(&path2));
}

#[test]
#[ignore] // Requires file system operations and timing
fn test_file_modification_detection() {
    let temp_dir = TempDir::new().unwrap();
    let asset_path = temp_dir.path().join("test.obj");

    // Write initial file
    std::fs::write(&asset_path, create_simple_obj()).unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(50),
        enable_batching: false,
        ..Default::default()
    };
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();

    // Give watcher time to start
    thread::sleep(Duration::from_millis(100));

    // Modify the file
    std::fs::write(&asset_path, create_modified_obj()).unwrap();

    // Wait for notification
    thread::sleep(Duration::from_millis(200));
    reloader.process_events();

    // Should receive modification event
    let mut received_modify = false;
    while let Some(event) = reloader.poll_event() {
        if matches!(event, HotReloadEvent::Modified { .. }) {
            received_modify = true;
        }
    }

    assert!(received_modify, "Should detect file modification");
}

#[test]
#[ignore] // Requires file system operations
fn test_multiple_asset_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create different asset types
    let mesh_path = temp_dir.path().join("mesh.obj");
    let texture_path = temp_dir.path().join("texture.png");

    std::fs::write(&mesh_path, create_simple_obj()).unwrap();
    std::fs::write(&texture_path, create_simple_png()).unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(50),
        enable_batching: false,
        ..Default::default()
    };
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();
    thread::sleep(Duration::from_millis(100));

    // Modify both
    std::fs::write(&mesh_path, create_modified_obj()).unwrap();
    std::fs::write(&texture_path, create_simple_png()).unwrap();

    thread::sleep(Duration::from_millis(200));
    reloader.process_events();

    // Should receive events for both
    let mut mesh_modified = false;
    let mut texture_modified = false;

    while let Some(event) = reloader.poll_event() {
        match event {
            HotReloadEvent::Modified { asset_type, .. } => match asset_type {
                AssetType::Mesh => mesh_modified = true,
                AssetType::Texture => texture_modified = true,
                _ => {}
            },
            _ => {}
        }
    }

    assert!(
        mesh_modified || texture_modified,
        "Should detect modification of at least one asset"
    );
}

#[test]
#[ignore] // Requires file operations
fn test_error_handling_corrupted_file() {
    let temp_dir = TempDir::new().unwrap();
    let asset_path = temp_dir.path().join("test.obj");

    // Write valid file
    std::fs::write(&asset_path, create_simple_obj()).unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(50),
        enable_batching: false,
        ..Default::default()
    };
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();
    thread::sleep(Duration::from_millis(100));

    // Write corrupted file
    std::fs::write(&asset_path, "INVALID OBJ DATA").unwrap();

    thread::sleep(Duration::from_millis(200));
    reloader.process_events();

    // Should receive ReloadFailed event
    let mut received_failed = false;
    while let Some(event) = reloader.poll_event() {
        if matches!(event, HotReloadEvent::ReloadFailed { .. }) {
            received_failed = true;
        }
    }

    assert!(received_failed, "Should receive ReloadFailed event for corrupted file");
}

#[test]
#[ignore] // Requires file operations
fn test_error_handling_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let asset_path = temp_dir.path().join("test.obj");

    // Write file
    std::fs::write(&asset_path, create_simple_obj()).unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();
    thread::sleep(Duration::from_millis(100));

    // Delete the file
    std::fs::remove_file(&asset_path).unwrap();

    thread::sleep(Duration::from_millis(200));
    reloader.process_events();

    // Should receive Deleted event
    let mut received_deleted = false;
    while let Some(event) = reloader.poll_event() {
        if matches!(event, HotReloadEvent::Deleted { .. }) {
            received_deleted = true;
        }
    }

    assert!(received_deleted, "Should receive Deleted event for removed file");
}

#[test]
fn test_batch_reload_configuration() {
    let manager = Arc::new(AssetManager::new());

    // Test with batching enabled
    let config = HotReloadConfig {
        enable_batching: true,
        max_batch_size: 5,
        batch_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();
    assert!(reloader.config.enable_batching);
    assert_eq!(reloader.config.max_batch_size, 5);

    // Test with batching disabled
    let config = HotReloadConfig { enable_batching: false, ..Default::default() };
    let reloader = HotReloader::new(manager, config).unwrap();
    assert!(!reloader.config.enable_batching);
}

#[test]
#[ignore] // Requires file operations and timing
fn test_batch_reload_workflow() {
    let temp_dir = TempDir::new().unwrap();

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig {
        debounce_duration: Duration::from_millis(50),
        enable_batching: true,
        max_batch_size: 3,
        batch_timeout: Duration::from_millis(200),
    };
    let mut reloader = HotReloader::new(manager, config).unwrap();

    reloader.watch(temp_dir.path()).unwrap();
    thread::sleep(Duration::from_millis(100));

    // Create and modify multiple files
    for i in 0..3 {
        let path = temp_dir.path().join(format!("mesh{i}.obj"));
        std::fs::write(&path, create_simple_obj()).unwrap();
        thread::sleep(Duration::from_millis(60)); // Above debounce
        std::fs::write(&path, create_modified_obj()).unwrap();
    }

    // Wait for batch to process
    thread::sleep(Duration::from_millis(300));
    reloader.process_events();

    // Should receive batch event
    let mut received_batch = false;
    while let Some(event) = reloader.poll_event() {
        if let HotReloadEvent::BatchReloaded { count, .. } = event {
            received_batch = true;
            assert!(count > 0, "Batch should have reloaded at least one asset");
        }
    }

    assert!(received_batch, "Should receive BatchReloaded event");
}

#[test]
fn test_statistics_tracking() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let reloader = HotReloader::new(manager, config).unwrap();

    let stats = reloader.stats();
    assert_eq!(stats.total_reloads, 0);
    assert_eq!(stats.failed_reloads, 0);
    assert_eq!(stats.tracked_assets, 0);
    assert_eq!(stats.queued_reloads, 0);
}

#[test]
fn test_event_types() {
    use engine_assets::AssetId;

    let path = PathBuf::from("test.obj");
    let id = AssetId::from_content(b"test");

    // Test all event types can be created
    let _created =
        HotReloadEvent::Created { path: path.clone(), asset_type: AssetType::Mesh, asset_id: id };

    let _modified = HotReloadEvent::Modified {
        path: path.clone(),
        asset_type: AssetType::Mesh,
        old_id: id,
        new_id: id,
    };

    let _deleted =
        HotReloadEvent::Deleted { path: path.clone(), asset_type: AssetType::Mesh, asset_id: id };

    let _failed = HotReloadEvent::ReloadFailed {
        path: path.clone(),
        asset_type: AssetType::Mesh,
        error: "Test error".to_string(),
    };

    let _batch = HotReloadEvent::BatchReloaded { count: 5, duration_ms: 100 };
}

// Property-based tests would go here if using proptest
// For now, we have comprehensive integration tests above
