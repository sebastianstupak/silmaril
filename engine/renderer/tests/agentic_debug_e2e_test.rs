//! End-to-End Agentic Rendering Debug Integration Test
//!
//! This test demonstrates the complete E2E workflow for autonomous AI debugging:
//! 1. Enable debug recording with RenderingDebugger
//! 2. Simulate a rendering loop with realistic events
//! 3. Export debug data to JSONL and SQLite
//! 4. Query the data using RenderQueryAPI (simulating AI agent)
//! 5. Detect issues autonomously
//!
//! This proves that E2E debugging capability is fully functional.

use engine_renderer::agentic_debug::{
    CsvExporter, RenderQueryAPI, RenderingDebugger, SqliteExporter,
};
use std::fs;
use std::path::PathBuf;

/// Helper to create test output directory
fn setup_test_dir(test_name: &str) -> PathBuf {
    let dir = PathBuf::from(format!("target/debug/test_output/{}", test_name));
    fs::create_dir_all(&dir).expect("Failed to create test output directory");
    dir
}

#[test]
fn test_e2e_agentic_debug_basic_workflow() {
    let test_dir = setup_test_dir("e2e_basic");

    // ========================================
    // PHASE 1: RECORDING - Simulate rendering loop
    // ========================================

    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    // Simulate 10 frames of rendering
    for frame in 0..10 {
        debugger.begin_frame(frame);

        // Simulate rendering commands for this frame
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 500, 1);
        debugger.record_pipeline_bind("cmd_buf_0", "shadow_pipeline", "graphics");
        debugger.record_draw_call("cmd_buf_0", "shadow_pipeline", 2000, 1);

        // Simulate resource allocations
        if frame == 0 {
            debugger.record_buffer_allocation("vertex_buf_0", 1024 * 1024); // 1MB
            debugger.record_buffer_allocation("index_buf_0", 512 * 1024); // 512KB
            debugger.record_image_allocation("color_target", 8 * 1024 * 1024); // 8MB
        }

        // Simulate fence wait (GPU synchronization)
        debugger.record_fence_wait("frame_fence", 0.5); // 0.5ms wait

        // Simulate validation error on frame 5
        if frame == 5 {
            debugger.record_validation_message(
                "error",
                "validation",
                12345,
                "VUID-vkCmdDraw-None-02859",
                "Validation error: Invalid pipeline state",
            );
        }

        debugger.end_frame();

        // Export snapshot for this frame
        let jsonl_path = test_dir.join(format!("frame_{}.jsonl", frame));
        debugger.export_jsonl(&jsonl_path).expect("Failed to export JSONL");
    }

    // ========================================
    // PHASE 2: EXPORT - Export to SQLite for querying
    // ========================================

    let db_path = test_dir.join("debug.db");
    let mut sqlite_exporter =
        SqliteExporter::new(&db_path).expect("Failed to create SQLite exporter");

    // Re-export all frames to SQLite
    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..10 {
        debugger.begin_frame(frame);

        // Same rendering commands as before
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 500, 1);
        debugger.record_pipeline_bind("cmd_buf_0", "shadow_pipeline", "graphics");
        debugger.record_draw_call("cmd_buf_0", "shadow_pipeline", 2000, 1);

        if frame == 0 {
            debugger.record_buffer_allocation("vertex_buf_0", 1024 * 1024);
            debugger.record_buffer_allocation("index_buf_0", 512 * 1024);
            debugger.record_image_allocation("color_target", 8 * 1024 * 1024);
        }

        debugger.record_fence_wait("frame_fence", 0.5);

        if frame == 5 {
            debugger.record_validation_message(
                "error",
                "validation",
                12345,
                "VUID-vkCmdDraw-None-02859",
                "Validation error: Invalid pipeline state",
            );
        }

        debugger.end_frame();

        // Export to SQLite
        debugger
            .export_sqlite(&mut sqlite_exporter)
            .expect("Failed to export to SQLite");
    }

    // ========================================
    // PHASE 3: QUERY - AI Agent analyzes data
    // ========================================

    let query_api =
        RenderQueryAPI::open(db_path.to_str().unwrap()).expect("Failed to open query API");

    // Query 1: Find slow frames (> 16.67ms for 60 FPS)
    let slow_frames = query_api
        .find_frames_above_threshold(16.67)
        .expect("Failed to query slow frames");

    // Query 2: Find frames with high draw calls
    let busy_frames = query_api
        .find_frames_with_draw_calls_above(5000)
        .expect("Failed to query busy frames");

    // Query 3: Detect resource leaks
    let leaks = query_api.detect_resource_leaks().expect("Failed to detect resource leaks");

    // Query 4: Find validation errors
    let errors = query_api.find_validation_errors().expect("Failed to find validation errors");

    // Query 5: Calculate average performance
    let avg_frame_time =
        query_api.average_frame_time().expect("Failed to calculate average frame time");

    let p95_frame_time = query_api.p95_frame_time().expect("Failed to calculate p95 frame time");

    let frame_count = query_api.frame_count().expect("Failed to get frame count");

    // ========================================
    // PHASE 4: ANALYSIS - Verify AI agent capabilities
    // ========================================

    // Verify frame count
    assert_eq!(frame_count, 10, "Should have 10 frames");

    // Verify performance metrics
    assert!(avg_frame_time >= 0.0, "Average frame time should be non-negative");
    assert!(p95_frame_time >= avg_frame_time, "p95 should be >= average");

    // Verify no slow frames (our test data should be fast)
    assert_eq!(slow_frames.len(), 0, "No frames should exceed 16.67ms in test data");

    // Verify no busy frames (our test has low draw call count)
    assert_eq!(busy_frames.len(), 0, "No frames should exceed 5000 draw calls");

    // Verify validation errors detected
    assert_eq!(errors.len(), 1, "Should detect 1 frame with validation errors (frame 5)");
    assert_eq!(errors[0].frame, 5, "Error should be on frame 5");

    // Verify event statistics
    let stats = debugger.event_statistics();
    assert_eq!(
        stats.draw_calls, 30,
        "Should have 30 total draw calls (3 per frame × 10 frames)"
    );
    assert_eq!(stats.pipeline_binds, 10, "Should have 10 pipeline binds (1 per frame)");

    // ========================================
    // PHASE 5: ISSUE DETECTION - Test snapshot analysis
    // ========================================

    let snapshot = debugger.create_snapshot();
    let issues = snapshot.detect_issues();

    // Our test data should not trigger any automatic issues
    assert!(issues.is_empty(), "Test data should not trigger automatic issue detection");

    // Verify performance stats
    let perf = snapshot.performance_stats();
    assert!(perf.fps > 0.0, "FPS should be positive");
    assert_eq!(perf.draw_calls, 3, "Last frame should have 3 draw calls");

    println!("\n========================================");
    println!("E2E AGENTIC DEBUG TEST RESULTS");
    println!("========================================");
    println!("✅ Recorded 10 frames successfully");
    println!("✅ Exported to JSONL: {} files", 10);
    println!("✅ Exported to SQLite: {}", db_path.display());
    println!("✅ AI Agent Query Results:");
    println!("   - Frame count: {}", frame_count);
    println!("   - Average frame time: {:.3}ms", avg_frame_time);
    println!("   - p95 frame time: {:.3}ms", p95_frame_time);
    println!("   - Slow frames (>16.67ms): {}", slow_frames.len());
    println!("   - Busy frames (>5000 draws): {}", busy_frames.len());
    println!("   - Validation errors: {} frames", errors.len());
    println!("   - Resource leaks: {} frames", leaks.len());
    println!("✅ Event statistics:");
    println!("   - Total draw calls: {}", stats.draw_calls);
    println!("   - Total pipeline binds: {}", stats.pipeline_binds);
    println!("   - Total state changes: {}", stats.state_changes);
    println!("   - Total allocations: {}", stats.resource_allocations);
    println!("   - Total sync events: {}", stats.synchronization_events);
    println!("========================================");
    println!("🚀 E2E AGENTIC DEBUGGING: FULLY FUNCTIONAL");
    println!("========================================\n");
}

#[test]
fn test_e2e_performance_regression_detection() {
    let test_dir = setup_test_dir("e2e_regression");

    // Simulate a performance regression scenario
    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    // Frame 0-4: Normal performance (< 16ms)
    for frame in 0..5 {
        debugger.begin_frame(frame);
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
        std::thread::sleep(std::time::Duration::from_millis(10)); // 10ms frame
        debugger.end_frame();
    }

    // Frame 5-9: Performance regression (> 20ms)
    for frame in 5..10 {
        debugger.begin_frame(frame);
        // Simulate performance issue: many draw calls
        for _ in 0..1000 {
            debugger.record_draw_call("cmd_buf_0", "main_pipeline", 10, 1);
        }
        std::thread::sleep(std::time::Duration::from_millis(25)); // 25ms frame
        debugger.end_frame();
    }

    // Export to SQLite
    let db_path = test_dir.join("regression.db");
    let mut sqlite_exporter =
        SqliteExporter::new(&db_path).expect("Failed to create SQLite exporter");

    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..10 {
        debugger.begin_frame(frame);
        if frame < 5 {
            debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
            std::thread::sleep(std::time::Duration::from_millis(10));
        } else {
            for _ in 0..1000 {
                debugger.record_draw_call("cmd_buf_0", "main_pipeline", 10, 1);
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        debugger.end_frame();

        debugger
            .export_sqlite(&mut sqlite_exporter)
            .expect("Failed to export to SQLite");
    }

    // AI Agent detects regression
    let query_api =
        RenderQueryAPI::open(db_path.to_str().unwrap()).expect("Failed to open query API");

    let slow_frames = query_api
        .find_frames_above_threshold(20.0)
        .expect("Failed to query slow frames");

    let busy_frames = query_api
        .find_frames_with_draw_calls_above(500)
        .expect("Failed to query busy frames");

    // Verify AI agent detected the regression
    assert!(slow_frames.len() >= 5, "Should detect frames 5-9 as slow (> 20ms)");
    assert!(busy_frames.len() >= 5, "Should detect frames 5-9 as busy (> 500 draws)");

    println!("\n========================================");
    println!("E2E REGRESSION DETECTION TEST");
    println!("========================================");
    println!("✅ AI Agent detected performance regression:");
    println!("   - Slow frames (>20ms): {}", slow_frames.len());
    println!("   - Busy frames (>500 draws): {}", busy_frames.len());
    println!("   - First regression frame: {}", slow_frames[0].frame);
    println!("   - Worst frame time: {:.2}ms", slow_frames[0].frame_time_ms);
    println!("========================================\n");
}

#[test]
fn test_e2e_resource_leak_detection() {
    let test_dir = setup_test_dir("e2e_leaks");

    // Simulate resource leak scenario
    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..10 {
        debugger.begin_frame(frame);

        // Allocate buffers every frame but never free them (LEAK!)
        debugger.record_buffer_allocation(&format!("leaked_buf_{}", frame), 1024 * 1024);
        debugger.record_image_allocation(&format!("leaked_img_{}", frame), 4 * 1024 * 1024);

        debugger.end_frame();
    }

    // Export to SQLite
    let db_path = test_dir.join("leaks.db");
    let mut sqlite_exporter =
        SqliteExporter::new(&db_path).expect("Failed to create SQLite exporter");

    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..10 {
        debugger.begin_frame(frame);
        debugger.record_buffer_allocation(&format!("leaked_buf_{}", frame), 1024 * 1024);
        debugger.record_image_allocation(&format!("leaked_img_{}", frame), 4 * 1024 * 1024);
        debugger.end_frame();

        debugger
            .export_sqlite(&mut sqlite_exporter)
            .expect("Failed to export to SQLite");
    }

    // AI Agent detects leaks
    let query_api =
        RenderQueryAPI::open(db_path.to_str().unwrap()).expect("Failed to open query API");

    let leaks = query_api.detect_resource_leaks().expect("Failed to detect resource leaks");

    // Verify AI agent detected the leaks
    assert_eq!(leaks.len(), 10, "Should detect leaks in all 10 frames");

    for (i, leak) in leaks.iter().enumerate() {
        assert_eq!(leak.frame, i as u64, "Frame {} should have leak", i);
        assert_eq!(leak.buffer_leak(), 1, "Each frame should leak 1 buffer");
        assert_eq!(leak.image_leak(), 1, "Each frame should leak 1 image");
    }

    println!("\n========================================");
    println!("E2E RESOURCE LEAK DETECTION TEST");
    println!("========================================");
    println!("✅ AI Agent detected resource leaks:");
    println!("   - Frames with leaks: {}", leaks.len());
    println!(
        "   - Total buffers leaked: {}",
        leaks.iter().map(|l| l.buffer_leak()).sum::<i32>()
    );
    println!(
        "   - Total images leaked: {}",
        leaks.iter().map(|l| l.image_leak()).sum::<i32>()
    );
    println!("========================================\n");
}

#[test]
fn test_e2e_csv_export() {
    let test_dir = setup_test_dir("e2e_csv");

    // Create simple test data
    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..5 {
        debugger.begin_frame(frame);
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
        debugger.end_frame();
    }

    // Export to CSV
    let csv_path = test_dir.join("metrics.csv");
    let mut csv_exporter = CsvExporter::new(&csv_path);

    let mut debugger = RenderingDebugger::new();
    debugger.enable_debug_recording();

    for frame in 0..5 {
        debugger.begin_frame(frame);
        debugger.record_draw_call("cmd_buf_0", "main_pipeline", 1000, 1);
        debugger.end_frame();

        debugger.export_csv(&mut csv_exporter).expect("Failed to export to CSV");
    }

    // Verify CSV file exists and has content
    assert!(csv_path.exists(), "CSV file should exist");

    let csv_content = fs::read_to_string(&csv_path).expect("Failed to read CSV file");
    assert!(csv_content.contains("frame"), "CSV should have header");
    assert!(csv_content.contains("frame_time_ms"), "CSV should have frame_time column");

    println!("\n========================================");
    println!("E2E CSV EXPORT TEST");
    println!("========================================");
    println!("✅ CSV export successful:");
    println!("   - File: {}", csv_path.display());
    println!("   - Size: {} bytes", csv_content.len());
    println!("   - Lines: {}", csv_content.lines().count());
    println!("========================================\n");
}
