//! Example: Runtime CPU Tier Detection
//!
//! This example demonstrates how to detect CPU capabilities at runtime
//! and select the appropriate binary tier.
//!
//! Run with: cargo run --example cpu_tier_detection --package engine-build-utils

use engine_build_utils::cpu_features::{detect_features, detect_tier, CpuTier};

fn main() {
    println!("===========================================");
    println!("CPU Tier Detection Example");
    println!("===========================================");
    println!();

    // Basic tier detection
    let tier = detect_tier();
    println!("Detected Tier: {}", tier);
    println!("  Performance: {:.0}% of native",
        tier.performance_multiplier() * 100.0);
    println!();

    // Detailed feature detection
    let features = detect_features();
    println!("CPU Details:");
    println!("  Vendor: {}", features.vendor);
    println!("  Brand:  {}", features.brand);
    println!();

    // Individual features
    #[cfg(target_arch = "x86_64")]
    {
        println!("SIMD Feature Support:");
        println!("  SSE2:    {} (required for x86-64)", check_mark(features.features.sse2));
        println!("  SSE4.1:  {}", check_mark(features.features.sse4_1));
        println!("  SSE4.2:  {} (required for Modern tier)", check_mark(features.features.sse4_2));
        println!("  AVX:     {}", check_mark(features.features.avx));
        println!("  AVX2:    {} (required for Modern tier)", check_mark(features.features.avx2));
        println!("  FMA:     {} (required for Modern tier)", check_mark(features.features.fma));
        println!("  AVX512F: {} (required for High-end tier)", check_mark(features.features.avx512f));
        println!("  AVX512DQ: {}", check_mark(features.features.avx512dq));
        println!("  AVX512BW: {}", check_mark(features.features.avx512bw));
        println!();

        println!("Other Features:");
        println!("  BMI1:    {} (Bit Manipulation)", check_mark(features.features.bmi1));
        println!("  BMI2:    {} (Bit Manipulation)", check_mark(features.features.bmi2));
        println!("  POPCNT:  {} (Population Count)", check_mark(features.features.popcnt));
        println!();
    }

    // Tier recommendations
    println!("===========================================");
    println!("Tier Compatibility & Performance");
    println!("===========================================");
    println!();

    println!("Baseline (x86-64 with SSE2):");
    println!("  ✓ Your CPU supports this tier");
    println!("  ✓ 100% compatibility (all x86-64 CPUs)");
    println!("  Performance: 1.0x (baseline)");
    println!();

    if tier >= CpuTier::Modern {
        println!("Modern (x86-64-v3 with AVX2 + FMA):");
        println!("  ✓ Your CPU supports this tier");
        println!("  ✓ ~95% compatibility (2013+ Intel, 2015+ AMD)");
        println!("  Performance: 1.15-1.30x faster than baseline");
        println!("  Recommended: Use this tier for best compatibility/performance");
        println!();
    } else {
        println!("Modern (x86-64-v3 with AVX2 + FMA):");
        println!("  ✗ Your CPU does NOT support this tier");
        println!("  Missing features:");
        if !features.features.avx2 {
            println!("    - AVX2");
        }
        if !features.features.fma {
            println!("    - FMA");
        }
        if !features.features.sse4_2 {
            println!("    - SSE4.2");
        }
        println!();
    }

    if tier >= CpuTier::HighEnd {
        println!("High-end (x86-64-v4 with AVX512):");
        println!("  ✓ Your CPU supports this tier");
        println!("  ✓ ~70% compatibility (2017+ Intel, 2022+ AMD)");
        println!("  Performance: 1.20-1.50x faster than baseline");
        println!("  Recommended: Use this tier for maximum performance");
        println!();
    } else {
        println!("High-end (x86-64-v4 with AVX512):");
        println!("  ✗ Your CPU does NOT support this tier");
        println!("  Missing features:");
        if !features.features.avx512f {
            println!("    - AVX512F");
        }
        if !features.features.avx512dq {
            println!("    - AVX512DQ");
        }
        if !features.features.avx512bw {
            println!("    - AVX512BW");
        }
        println!();
    }

    // Binary selection example
    println!("===========================================");
    println!("Binary Selection Example");
    println!("===========================================");
    println!();

    println!("In a launcher, you would select:");
    match tier {
        CpuTier::HighEnd => {
            println!("  → bin/highend/client.exe");
            println!("    (AVX512-optimized, ~50% faster)");
        }
        CpuTier::Modern => {
            println!("  → bin/modern/client.exe");
            println!("    (AVX2-optimized, ~30% faster)");
        }
        CpuTier::Baseline => {
            println!("  → bin/baseline/client.exe");
            println!("    (Universal compatibility)");
        }
    }
    println!();

    // Code example
    println!("Example launcher code:");
    println!("```rust");
    println!("let tier = detect_tier();");
    println!("let binary_path = match tier {{");
    println!("    CpuTier::HighEnd => \"bin/highend/client.exe\",");
    println!("    CpuTier::Modern => \"bin/modern/client.exe\",");
    println!("    CpuTier::Baseline => \"bin/baseline/client.exe\",");
    println!("}};");
    println!("launch_binary(binary_path);");
    println!("```");
}

fn check_mark(supported: bool) -> &'static str {
    if supported { "✓" } else { "✗" }
}
