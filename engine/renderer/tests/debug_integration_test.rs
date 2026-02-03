//! Debug integration test
//!
//! Demonstrates how to use the agentic debug infrastructure integrated into the renderer.

#[allow(unused_imports)]
use engine_renderer::debug::{DebugConfig, RenderingQueryAPI};
use tempfile::TempDir;

#[test]
#[ignore] // Requires Vulkan and windowing, run manually
fn test_debug_integration_example() {
    // This test demonstrates the debug integration but is ignored by default
    // since it requires Vulkan and windowing to be available.
    //
    // To run manually:
    // cargo test -p engine-renderer --test debug_integration_test -- --ignored
    //
    // Example usage:
    //
    // ```no_run
    // use engine_renderer::{Renderer, WindowConfig};
    // use engine_renderer::debug::DebugConfig;
    //
    // // Create renderer
    // let mut renderer = Renderer::new(
    //     WindowConfig::default().with_title("Debug Demo"),
    //     "DebugDemo"
    // )?;
    //
    // // Enable debug with database export
    // renderer.enable_debug(DebugConfig::default(), Some("debug.db"))?;
    //
    // // Render frames - debug data automatically captured
    // for _ in 0..60 {
    //     renderer.render_frame()?;
    // }
    //
    // // Disable debug (flushes data)
    // renderer.disable_debug();
    //
    // // Query captured data
    // let api = RenderingQueryAPI::open("debug.db")?;
    // let stats = api.statistics()?;
    // println!("Captured {} frames", stats.total_frames);
    // # Ok::<(), Box<dyn std::error::Error>>(())
    // ```
}

#[test]
fn test_debug_database_creation() {
    use engine_renderer::debug::SqliteExporter;

    // Test that we can create a debug database
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_debug.db");

    let exporter = SqliteExporter::create(&db_path);
    assert!(exporter.is_ok(), "Failed to create debug database");

    // Verify database exists
    assert!(db_path.exists(), "Database file was not created");
}

#[test]
fn test_debug_query_api() {
    use engine_renderer::debug::{RenderDebugSnapshot, SqliteExporter};

    // Create database and write test snapshot
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_query.db");

    let mut exporter = SqliteExporter::create(&db_path).unwrap();

    // Create test snapshot
    let snapshot = RenderDebugSnapshot::new(0, 0.0);

    exporter.write_snapshot(&snapshot).unwrap();

    // Query the data
    let api = RenderingQueryAPI::open(&db_path).unwrap();
    let stats = api.statistics().unwrap();

    assert_eq!(stats.total_frames, 1);
}
