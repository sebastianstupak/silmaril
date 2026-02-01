/// Build script for engine-math
///
/// Enforces architectural rules at compile time and detects CPU features
///
/// # CLAUDE.md Requirements
/// 1. No `println!`/`eprintln!`/`dbg!` in production code
/// 2. Error types must use `define_error!` macro (if any error types exist)
/// 3. Detect and enable CPU SIMD features for optimal performance
use engine_build_utils::{ErrorCheckConfig, PrintCheckConfig};

fn main() {
    // Tell cargo to rerun if source files change
    engine_build_utils::rerun_if_src_changed();

    // Check for print statements in production code
    let print_config = PrintCheckConfig::default();
    engine_build_utils::check_no_print_statements(&print_config);

    // Check that error types use define_error! macro
    // Currently engine-math has no error types, but this will catch them if added
    let error_config = ErrorCheckConfig::default();
    engine_build_utils::check_error_types_use_macro(&error_config);

    // Detect and enable CPU features for SIMD optimizations
    detect_cpu_features();
}

/// Detect available CPU features and enable compiler optimizations
///
/// This function checks for SIMD instruction sets and sets cargo cfg flags
/// to enable conditional compilation based on available features.
///
/// Detected features:
/// - SSE4.2: Required for baseline SIMD operations
/// - FMA: Fused multiply-add for better performance and precision
/// - AVX2: 256-bit SIMD operations
/// - AVX512: 512-bit SIMD operations (future)
fn detect_cpu_features() {
    // Emit cargo instructions to enable features based on target
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");

    // Check if we're building for x86/x86_64
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch != "x86" && target_arch != "x86_64" {
        // Not x86, skip feature detection
        return;
    }

    // Get enabled target features
    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();

    // Check for specific SIMD features
    let has_sse42 = target_features.iter().any(|f| f == "sse4.2");
    let has_fma = target_features.iter().any(|f| f == "fma");
    let has_avx2 = target_features.iter().any(|f| f == "avx2");
    let has_avx512f = target_features.iter().any(|f| f == "avx512f");

    // Set custom cfg flags for conditional compilation
    if has_sse42 {
        println!("cargo:rustc-cfg=has_sse42");
    }
    if has_fma {
        println!("cargo:rustc-cfg=has_fma");
    }
    if has_avx2 {
        println!("cargo:rustc-cfg=has_avx2");
    }
    if has_avx512f {
        println!("cargo:rustc-cfg=has_avx512");
    }

    // Emit rustc flags for better optimization if target-cpu=native is not set
    let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    if !rustflags.contains("target-cpu") {
        // Suggest enabling target-cpu=native for best performance
        println!("cargo:warning=engine-math: For optimal SIMD performance, compile with RUSTFLAGS=\"-C target-cpu=native\"");

        // Enable baseline optimizations
        if has_sse42 && has_fma {
            println!("cargo:rustc-env=RUSTFLAGS=-C target-feature=+sse4.2,+fma");
        }
    }

    // Print feature summary (only during build, not at runtime)
    eprintln!("engine-math CPU features:");
    eprintln!("  SSE4.2: {}", if has_sse42 { "enabled" } else { "disabled" });
    eprintln!("  FMA:    {}", if has_fma { "enabled" } else { "disabled" });
    eprintln!("  AVX2:   {}", if has_avx2 { "enabled" } else { "disabled" });
    eprintln!("  AVX512: {}", if has_avx512f { "enabled" } else { "disabled" });
}
