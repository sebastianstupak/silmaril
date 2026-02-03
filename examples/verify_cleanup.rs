//! Quick verification that renderer cleanup works without leaks/crashes
//!
//! Run with: cargo run --release --bin verify_cleanup

use engine_renderer::{Renderer, WindowConfig};

fn main() {
    // Initialize tracing to see cleanup logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("\n========================================");
    println!("  Cleanup Verification Test");
    println!("========================================\n");

    let config = WindowConfig {
        title: "Cleanup Verification".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false, // Headless
    };

    println!("Creating renderer...");
    let mut renderer = match Renderer::new(config, "CleanupVerify") {
        Ok(r) => {
            println!("✓ Renderer created");
            r
        }
        Err(e) => {
            eprintln!("✗ Failed to create renderer: {:?}", e);
            return;
        }
    };

    println!("Rendering 5 frames...");
    for i in 0..5 {
        renderer.set_clear_color(i as f32 * 0.2, 1.0 - i as f32 * 0.2, 0.5, 1.0);
        if let Err(e) = renderer.render_frame() {
            eprintln!("✗ Frame {} failed: {:?}", i, e);
            return;
        }
    }
    println!("✓ 5 frames rendered");

    println!("Waiting for GPU idle...");
    if let Err(e) = renderer.wait_idle() {
        eprintln!("✗ wait_idle failed: {:?}", e);
        return;
    }
    println!("✓ GPU idle");

    println!("Dropping renderer (cleanup happening now)...");
    drop(renderer);

    println!("\n========================================");
    println!("✓ Cleanup completed successfully!");
    println!();
    println!("Verify no errors above:");
    println!("  - No 'leak detected' warnings");
    println!("  - No STATUS_ACCESS_VIOLATION");
    println!("  - 'Renderer destroyed' message appeared");
    println!("========================================\n");
}
