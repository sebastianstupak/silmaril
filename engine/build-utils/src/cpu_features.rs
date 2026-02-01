//! Runtime CPU feature detection for selecting optimal binary tier
//!
//! This module provides runtime detection of CPU features to select the best
//! available binary tier (baseline, modern, or high-end).
//!
//! # Tier Definitions
//!
//! - **Baseline (x86-64)**: SSE2 only (100% compatible)
//! - **Modern (x86-64-v3)**: SSE4.2 + AVX2 + FMA (~95% compatible)
//! - **High-end (x86-64-v4)**: AVX512 + AVX2 + FMA (~70% compatible)
//!
//! # Usage
//!
//! ```rust
//! use engine_build_utils::cpu_features::{detect_tier, CpuTier};
//!
//! let tier = detect_tier();
//! println!("Detected CPU tier: {:?}", tier);
//!
//! match tier {
//!     CpuTier::HighEnd => println!("Running optimized AVX512 binary"),
//!     CpuTier::Modern => println!("Running optimized AVX2 binary"),
//!     CpuTier::Baseline => println!("Running baseline SSE2 binary"),
//! }
//! ```

/// CPU tier based on supported instruction sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuTier {
    /// x86-64 baseline: SSE2 only (100% compatible)
    Baseline = 1,
    /// x86-64-v3: SSE4.2 + AVX2 + FMA (~95% compatible, 2013+ CPUs)
    Modern = 3,
    /// x86-64-v4: AVX512 + AVX2 + FMA (~70% compatible, 2017+ CPUs)
    HighEnd = 4,
}

impl CpuTier {
    /// Get the tier name as a string (matches build output directories)
    pub const fn name(self) -> &'static str {
        match self {
            CpuTier::Baseline => "baseline",
            CpuTier::Modern => "modern",
            CpuTier::HighEnd => "highend",
        }
    }

    /// Get the tier description
    pub const fn description(self) -> &'static str {
        match self {
            CpuTier::Baseline => "x86-64 with SSE2 (100% compatible)",
            CpuTier::Modern => "x86-64-v3 with AVX2+FMA (95% compatible)",
            CpuTier::HighEnd => "x86-64-v4 with AVX512 (70% compatible)",
        }
    }

    /// Get expected performance multiplier vs baseline
    pub const fn performance_multiplier(self) -> f32 {
        match self {
            CpuTier::Baseline => 1.0,
            CpuTier::Modern => 1.25,  // 15-30% faster avg
            CpuTier::HighEnd => 1.35, // 20-50% faster avg
        }
    }
}

impl std::fmt::Display for CpuTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.description())
    }
}

/// Detect the best CPU tier supported by the current CPU
///
/// This function checks CPU features at runtime and returns the highest
/// tier that the CPU can support.
///
/// # Examples
///
/// ```rust
/// let tier = detect_tier();
/// match tier {
///     CpuTier::HighEnd => launch_binary("highend/client.exe"),
///     CpuTier::Modern => launch_binary("modern/client.exe"),
///     CpuTier::Baseline => launch_binary("baseline/client.exe"),
/// }
/// ```
#[cfg(target_arch = "x86_64")]
pub fn detect_tier() -> CpuTier {
    use std::arch::is_x86_feature_detected;

    // Check for x86-64-v4 features (AVX512)
    // Requires: AVX512F, AVX512DQ, AVX512CD, AVX512BW, AVX512VL
    if is_x86_feature_detected!("avx512f")
        && is_x86_feature_detected!("avx512dq")
        && is_x86_feature_detected!("avx512cd")
        && is_x86_feature_detected!("avx512bw")
        && is_x86_feature_detected!("avx512vl")
    {
        return CpuTier::HighEnd;
    }

    // Check for x86-64-v3 features (AVX2 + FMA)
    // Requires: AVX2, FMA, SSE4.2, BMI1, BMI2
    if is_x86_feature_detected!("avx2")
        && is_x86_feature_detected!("fma")
        && is_x86_feature_detected!("sse4.2")
        && is_x86_feature_detected!("bmi1")
        && is_x86_feature_detected!("bmi2")
    {
        return CpuTier::Modern;
    }

    // Fallback to baseline (all x86-64 CPUs support SSE2)
    CpuTier::Baseline
}

/// Detect CPU tier on non-x86_64 architectures
///
/// Always returns Baseline since tiered builds are x86-64 specific
#[cfg(not(target_arch = "x86_64"))]
pub fn detect_tier() -> CpuTier {
    CpuTier::Baseline
}

/// Detailed CPU feature information
#[derive(Debug, Clone)]
pub struct CpuFeatures {
    /// Detected tier
    pub tier: CpuTier,
    /// CPU vendor (Intel, AMD, etc.)
    pub vendor: String,
    /// CPU brand string
    pub brand: String,
    /// Individual feature flags
    pub features: FeatureFlags,
}

/// Individual CPU feature flags
#[derive(Debug, Clone, Default)]
pub struct FeatureFlags {
    // SSE family
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,

    // AVX family
    pub avx: bool,
    pub avx2: bool,
    pub fma: bool,

    // AVX-512 family
    pub avx512f: bool,
    pub avx512dq: bool,
    pub avx512cd: bool,
    pub avx512bw: bool,
    pub avx512vl: bool,

    // Other
    pub bmi1: bool,
    pub bmi2: bool,
    pub popcnt: bool,
}

/// Detect detailed CPU features
#[cfg(target_arch = "x86_64")]
pub fn detect_features() -> CpuFeatures {
    use std::arch::is_x86_feature_detected;

    let features = FeatureFlags {
        // SSE
        sse2: is_x86_feature_detected!("sse2"),
        sse3: is_x86_feature_detected!("sse3"),
        ssse3: is_x86_feature_detected!("ssse3"),
        sse4_1: is_x86_feature_detected!("sse4.1"),
        sse4_2: is_x86_feature_detected!("sse4.2"),

        // AVX
        avx: is_x86_feature_detected!("avx"),
        avx2: is_x86_feature_detected!("avx2"),
        fma: is_x86_feature_detected!("fma"),

        // AVX-512
        avx512f: is_x86_feature_detected!("avx512f"),
        avx512dq: is_x86_feature_detected!("avx512dq"),
        avx512cd: is_x86_feature_detected!("avx512cd"),
        avx512bw: is_x86_feature_detected!("avx512bw"),
        avx512vl: is_x86_feature_detected!("avx512vl"),

        // Other
        bmi1: is_x86_feature_detected!("bmi1"),
        bmi2: is_x86_feature_detected!("bmi2"),
        popcnt: is_x86_feature_detected!("popcnt"),
    };

    // Get CPU vendor and brand using cpuid
    let (vendor, brand) = get_cpu_info();

    CpuFeatures { tier: detect_tier(), vendor, brand, features }
}

/// Get CPU vendor and brand string
#[cfg(target_arch = "x86_64")]
fn get_cpu_info() -> (String, String) {
    // Use raw_cpuid crate if available, otherwise return generic
    // Note: cpuid feature temporarily disabled
    ("Unknown".to_string(), "x86_64 CPU".to_string())
}

/// Detect CPU features on non-x86_64
#[cfg(not(target_arch = "x86_64"))]
pub fn detect_features() -> CpuFeatures {
    CpuFeatures {
        tier: CpuTier::Baseline,
        vendor: std::env::consts::ARCH.to_string(),
        brand: format!("{} CPU", std::env::consts::ARCH),
        features: FeatureFlags::default(),
    }
}

/// Print CPU features to stdout (useful for debugging)
pub fn print_cpu_info() {
    let features = detect_features();

    println!("CPU Information:");
    println!("  Vendor: {}", features.vendor);
    println!("  Brand:  {}", features.brand);
    println!("  Tier:   {}", features.tier);
    println!();
    println!("Feature Support:");

    #[cfg(target_arch = "x86_64")]
    {
        println!("  SSE2:    {}", check_mark(features.features.sse2));
        println!("  SSE4.2:  {}", check_mark(features.features.sse4_2));
        println!("  AVX:     {}", check_mark(features.features.avx));
        println!("  AVX2:    {}", check_mark(features.features.avx2));
        println!("  FMA:     {}", check_mark(features.features.fma));
        println!("  AVX512F: {}", check_mark(features.features.avx512f));
        println!("  BMI1:    {}", check_mark(features.features.bmi1));
        println!("  BMI2:    {}", check_mark(features.features.bmi2));
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        println!("  Architecture: {}", std::env::consts::ARCH);
        println!("  (Feature detection only available on x86_64)");
    }

    println!();
    println!("Recommended binary: {}", features.tier.name());
    println!(
        "Expected performance: {:.0}% of native",
        features.tier.performance_multiplier() * 100.0
    );
}

fn check_mark(supported: bool) -> &'static str {
    if supported {
        "✓"
    } else {
        "✗"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_ordering() {
        assert!(CpuTier::Baseline < CpuTier::Modern);
        assert!(CpuTier::Modern < CpuTier::HighEnd);
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(CpuTier::Baseline.name(), "baseline");
        assert_eq!(CpuTier::Modern.name(), "modern");
        assert_eq!(CpuTier::HighEnd.name(), "highend");
    }

    #[test]
    fn test_detect_tier() {
        let tier = detect_tier();
        // Should always detect at least baseline
        assert!(tier >= CpuTier::Baseline);
    }

    #[test]
    fn test_detect_features() {
        let features = detect_features();
        assert!(features.tier >= CpuTier::Baseline);

        #[cfg(target_arch = "x86_64")]
        {
            // All x86_64 CPUs support SSE2
            assert!(features.features.sse2);
        }
    }

    #[test]
    fn test_performance_multipliers() {
        assert_eq!(CpuTier::Baseline.performance_multiplier(), 1.0);
        assert!(CpuTier::Modern.performance_multiplier() > 1.0);
        assert!(
            CpuTier::HighEnd.performance_multiplier() > CpuTier::Modern.performance_multiplier()
        );
    }
}
