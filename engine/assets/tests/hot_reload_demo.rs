//! Hot-reload demonstration test.
//!
//! This test demonstrates the hot-reload functionality by creating temporary
//! asset files and watching them for changes.

#![cfg(feature = "hot-reload")]

use engine_assets::{AssetManager, HotReloadEvent, HotReloader};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
#[ignore] // This test requires file system operations and timing, run manually
fn demo_hot_reload_workflow() {
    // Create temporary directory for assets
    let temp_dir = TempDir::new().unwrap();
    let asset_path = temp_dir.path().join("test_mesh.obj");

    // Write initial asset
    std::fs::write(&asset_path, create_simple_obj()).unwrap();

    // Create asset manager and hot-reloader
    let manager = Arc::new(AssetManager::new());
    let mut hot_reloader = HotReloader::new(Arc::clone(&manager)).unwrap();

    // Start watching the directory
    hot_reloader.watch(temp_dir.path()).unwrap();

    println!("Watching directory: {:?}", temp_dir.path());

    // Modify the asset file
    thread::sleep(Duration::from_millis(200));
    std::fs::write(&asset_path, create_modified_obj()).unwrap();
    println!("Modified asset file");

    // Process events
    thread::sleep(Duration::from_millis(200));
    hot_reloader.process_events();

    // Check for hot-reload event
    let mut received_event = false;
    while let Some(event) = hot_reloader.poll_event() {
        match event {
            HotReloadEvent::Modified { path, asset_type } => {
                println!("Hot-reload event: {:?} modified (type: {:?})", path, asset_type);
                received_event = true;
            }
            HotReloadEvent::ReloadFailed { path, error } => {
                println!("Reload failed for {:?}: {}", path, error);
            }
            _ => {}
        }
    }

    assert!(received_event, "Should have received a hot-reload event");

    println!("Hot-reload demo completed successfully!");
}

fn create_simple_obj() -> String {
    r#"# Simple cube OBJ
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0

vn 0.0 0.0 1.0

vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0

f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
"#
    .to_string()
}

fn create_modified_obj() -> String {
    r#"# Modified cube OBJ (scaled)
v -2.0 -2.0  2.0
v  2.0 -2.0  2.0
v  2.0  2.0  2.0
v -2.0  2.0  2.0

vn 0.0 0.0 1.0

vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0

f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
"#
    .to_string()
}

#[test]
fn test_hot_reload_event_types() {
    use std::path::PathBuf;

    let created = HotReloadEvent::Created {
        path: PathBuf::from("test.obj"),
        asset_type: engine_assets::AssetType::Mesh,
    };

    let modified = HotReloadEvent::Modified {
        path: PathBuf::from("test.obj"),
        asset_type: engine_assets::AssetType::Mesh,
    };

    let deleted = HotReloadEvent::Deleted {
        path: PathBuf::from("test.obj"),
        asset_type: engine_assets::AssetType::Mesh,
    };

    let failed = HotReloadEvent::ReloadFailed {
        path: PathBuf::from("test.obj"),
        error: "Test error".to_string(),
    };

    // Just verify they can be created
    assert!(matches!(created, HotReloadEvent::Created { .. }));
    assert!(matches!(modified, HotReloadEvent::Modified { .. }));
    assert!(matches!(deleted, HotReloadEvent::Deleted { .. }));
    assert!(matches!(failed, HotReloadEvent::ReloadFailed { .. }));
}
