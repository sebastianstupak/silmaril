//! Manual test for renderer cleanup
//!
//! Run this with: cargo test --package engine-renderer renderer_cleanup_manual_test -- --ignored --nocapture
//!
//! This test is marked #[ignore] because it creates a window and requires manual verification
//! that no memory leaks or access violations occur during cleanup.

use engine_renderer::{Renderer, WindowConfig};

#[test]
#[ignore] // Manual test - requires human verification
fn test_renderer_cleanup_manual() {
    println!("\n========================================");
    println!("Manual Cleanup Test");
    println!("========================================\n");

    let config = WindowConfig {
        title: "Cleanup Manual Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false, // Headless
    };

    println!("Creating renderer...");
    let mut renderer =
        Renderer::new(config, "CleanupManualTest").expect("Failed to create renderer");

    println!("Rendering 5 frames...");
    for i in 0..5 {
        renderer.set_clear_color(i as f32 * 0.2, 1.0 - i as f32 * 0.2, 0.5, 1.0);
        renderer.render_frame().expect("Failed to render frame");
    }

    println!("Waiting for GPU idle...");
    renderer.wait_idle().expect("Failed to wait for idle");

    println!("Dropping renderer (cleanup should happen now)...");
    drop(renderer);

    println!("\n✓ Cleanup completed");
    println!("Check above for:");
    println!("  - No 'leak detected' warnings");
    println!("  - No access violations (STATUS_ACCESS_VIOLATION)");
    println!("  - 'Renderer destroyed' log message appeared");
    println!("========================================\n");
}
