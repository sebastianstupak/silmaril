//! Manual performance measurement for Vulkan context operations
//! Run with: cargo run --release --example manual_bench

use engine_renderer::VulkanContext;
use std::time::Instant;

fn main() {
    println!("=== Vulkan Context Performance Measurements ===\n");

    // Warm up
    println!("[Warm-up] Creating context once to load libraries...");
    let _ = VulkanContext::new("Warmup", None, None);
    println!("[Warm-up] Complete\n");

    // Test 1: First context creation (cold start)
    println!("Test 1: First Context Creation (Cold Start)");
    let start = Instant::now();
    let context1 = VulkanContext::new("PerfTest1", None, None).expect("Failed to create context");
    let duration1 = start.elapsed();
    println!("  Time: {:?}", duration1);
    println!("  Device: {}", context1.device_name());
    drop(context1);
    println!();

    // Test 2: Second context creation (should hit device cache!)
    println!("Test 2: Second Context Creation (Cache Hit Expected)");
    let start = Instant::now();
    let context2 = VulkanContext::new("PerfTest2", None, None).expect("Failed to create context");
    let duration2 = start.elapsed();
    println!("  Time: {:?}", duration2);
    println!("  Device: {}", context2.device_name());

    // Calculate speedup
    let speedup = duration1.as_secs_f64() / duration2.as_secs_f64();
    println!(
        "\n  CACHE SPEEDUP: {:.2}x faster ({:.1}% improvement)",
        speedup,
        (speedup - 1.0) * 100.0
    );
    drop(context2);
    println!();

    // Test 3: Multiple rapid creations (test cache effectiveness)
    println!("Test 3: 10 Rapid Context Creations");
    let mut total_time = std::time::Duration::ZERO;
    for i in 1..=10 {
        let start = Instant::now();
        let ctx = VulkanContext::new(&format!("Rapid{}", i), None, None)
            .expect("Failed to create context");
        let duration = start.elapsed();
        total_time += duration;
        println!("  #{}: {:?}", i, duration);
        drop(ctx);
    }
    let avg_time = total_time / 10;
    println!("  Average: {:?}", avg_time);
    println!("  Total: {:?}", total_time);
    println!();

    // Test 4: Device wait idle performance
    println!("Test 4: Device Wait Idle (1000 iterations)");
    let context = VulkanContext::new("WaitIdleTest", None, None).expect("Failed to create context");

    let start = Instant::now();
    for _ in 0..1000 {
        context.wait_idle().expect("Wait idle failed");
    }
    let total = start.elapsed();
    let per_call = total / 1000;
    println!("  Total (1000 calls): {:?}", total);
    println!("  Per call: {:?}", per_call);
    drop(context);
    println!();

    println!("=== Performance Measurement Complete ===");
    println!("\nKEY FINDINGS:");
    println!("  1. First creation: {:?}", duration1);
    println!(
        "  2. Cached creation: {:?} ({:.1}% faster!)",
        duration2,
        (speedup - 1.0) * 100.0
    );
    println!("  3. Average (10 runs): {:?}", avg_time);
    println!("  4. Wait idle: {:?} per call", per_call);
}
