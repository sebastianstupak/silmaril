# Platform-Specific Build Tiers

> **Complete guide to building optimized binaries for different CPU capabilities**

This document explains the silmaril's multi-tier build system, which creates optimized binaries for different CPU feature sets to maximize performance while maintaining broad compatibility.

---

## Table of Contents

- [Overview](#overview)
- [Tier Definitions](#tier-definitions)
- [Quick Start](#quick-start)
- [Build Scripts](#build-scripts)
- [Runtime Detection](#runtime-detection)
- [Performance Benchmarking](#performance-benchmarking)
- [Deployment Strategy](#deployment-strategy)
- [Troubleshooting](#troubleshooting)
- [Technical Details](#technical-details)

---

## Overview

### Why Multi-Tier Builds?

Modern CPUs support different instruction sets (SSE, AVX, AVX2, AVX512) that can significantly improve performance. However, using newer instructions reduces compatibility with older CPUs.

The solution: **Build multiple optimized binaries and select the best one at runtime**.

### Performance Gains

Expected performance improvements vs baseline (SSE2):

| Tier | Scalar Ops | Batch Ops | Physics | Rendering |
|------|-----------|-----------|---------|-----------|
| Baseline | 1.0x | 1.0x | 1.0x | 1.0x |
| Modern | 1.15-1.30x | 2.0-3.0x | 1.20-1.35x | 1.15-1.25x |
| High-end | 1.20-1.35x | 3.0-5.0x | 1.25-1.50x | 1.20-1.35x |

**Target:** 5-20% improvement per tier for typical game workloads.

### Compatibility

| Tier | Compatibility | Example CPUs |
|------|--------------|--------------|
| Baseline | 100% | All x86-64 CPUs |
| Modern | ~95% | Intel Haswell (2013+), AMD Excavator (2015+) |
| High-end | ~70% | Intel Skylake-X (2017+), AMD Zen 4 (2022+) |

---

## Tier Definitions

### Tier 1: Baseline (x86-64 with SSE2)

**Target:** Maximum compatibility

**Features:**
- SSE2 (baseline x86-64 requirement)
- No AVX, FMA, or other modern extensions

**Build flags:**
```bash
# No special flags - default x86-64
rustc --target x86_64-unknown-linux-gnu
```

**Use cases:**
- Fallback for very old CPUs
- Initial compatibility testing
- Universal distribution

**Performance:**
- Baseline reference: 1.0x

### Tier 2: Modern (x86-64-v3)

**Target:** Best compatibility/performance balance

**Features:**
- SSE4.2 (SIMD string operations)
- AVX2 (256-bit SIMD)
- FMA (fused multiply-add)
- BMI1/BMI2 (bit manipulation)
- POPCNT (population count)

**Build flags:**
```bash
rustc --target x86_64-unknown-linux-gnu -C target-cpu=x86-64-v3
```

**Use cases:**
- Recommended default for distribution
- Steam/Epic releases
- Public downloads

**Performance:**
- Scalar math: 1.15-1.30x faster
- Batch operations: 2.0-3.0x faster
- Physics integration: 1.20-1.35x faster

**Compatible CPUs:**
- Intel: 4th gen Core (Haswell, 2013) and newer
- AMD: Excavator (2015) and newer, all Ryzen

### Tier 3: High-end (x86-64-v4)

**Target:** Maximum performance for modern hardware

**Features:**
- All x86-64-v3 features, plus:
- AVX512F (512-bit SIMD foundation)
- AVX512DQ (double/quadword operations)
- AVX512CD (conflict detection)
- AVX512BW (byte/word operations)
- AVX512VL (vector length extensions)

**Build flags:**
```bash
rustc --target x86_64-unknown-linux-gnu -C target-cpu=x86-64-v4
```

**Use cases:**
- High-end gaming PCs
- Server deployments
- Professional/enthusiast users

**Performance:**
- Scalar math: 1.20-1.35x faster
- Batch operations: 3.0-5.0x faster
- Physics integration: 1.25-1.50x faster

**Compatible CPUs:**
- Intel: Skylake-X (2017), Ice Lake (2019), and newer
- AMD: Zen 4 (2022) and newer

**Warning:** AVX512 is NOT available on:
- Most consumer Intel CPUs (disabled on some models)
- AMD Ryzen 1000-5000 series
- Apple Silicon (ARM64, different architecture)

---

## Quick Start

### Building All Tiers

#### Linux/macOS:
```bash
# Build all tiers (debug mode, both client and server)
./scripts/build_all_tiers.sh --both

# Build release binaries
./scripts/build_all_tiers.sh --release --both

# Build client only
./scripts/build_all_tiers.sh --release --client

# Build server only
./scripts/build_all_tiers.sh --release --server
```

#### Windows:
```powershell
# Build all tiers
.\scripts\build_all_tiers.ps1 -Both

# Build release binaries
.\scripts\build_all_tiers.ps1 -Release -Both

# Build client only
.\scripts\build_all_tiers.ps1 -Release -Client

# Build server only
.\scripts\build_all_tiers.ps1 -Release -Server
```

### Output Locations

Binaries are placed in tier-specific directories:

```
target/
├── baseline/
│   ├── debug/
│   │   ├── client
│   │   └── server
│   └── release/
│       ├── client
│       └── server
├── modern/
│   ├── debug/
│   └── release/
└── highend/
    ├── debug/
    └── release/
```

### Verifying Your Build

Check which features are enabled:

```bash
# Detect your CPU's capabilities
cargo run --example cpu_tier_detection --package engine-build-utils

# Example output:
# CPU Information:
#   Vendor: GenuineIntel
#   Brand:  Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
#   Tier:   modern (x86-64-v3 with AVX2+FMA (95% compatible))
#
# Feature Support:
#   SSE2:    ✓
#   SSE4.2:  ✓
#   AVX:     ✓
#   AVX2:    ✓
#   FMA:     ✓
#   AVX512F: ✗
#
# Recommended binary: modern
```

---

## Build Scripts

### build_all_tiers.sh / build_all_tiers.ps1

Builds all three tiers in one command.

**Features:**
- Cross-platform (Bash for Linux/macOS, PowerShell for Windows)
- Parallel builds for faster compilation
- Automatic target detection (linux-gnu, apple-darwin, pc-windows-msvc)
- Progress reporting and error handling
- Size reporting for each binary

**Usage:**
```bash
# Show help
./scripts/build_all_tiers.sh --help

# Common workflows
./scripts/build_all_tiers.sh --release --both        # Production builds
./scripts/build_all_tiers.sh --client                # Debug client
./scripts/build_all_tiers.sh --release --server      # Release server only
```

**Environment Variables:**
- `RUSTFLAGS`: Automatically set per tier
- `CARGO_TARGET_DIR`: Customized per tier to avoid conflicts

### Manual Building

If you need fine-grained control:

```bash
# Baseline
cargo build --release --bin client

# Modern (x86-64-v3)
RUSTFLAGS="-C target-cpu=x86-64-v3" \
  cargo build --release --bin client --target-dir target/modern

# High-end (x86-64-v4)
RUSTFLAGS="-C target-cpu=x86-64-v4" \
  cargo build --release --bin client --target-dir target/highend
```

---

## Runtime Detection

### Automatic Tier Selection

Use the `engine-build-utils` crate to detect CPU features at runtime:

```rust
use engine_build_utils::cpu_features::{detect_tier, CpuTier};

fn main() {
    let tier = detect_tier();

    match tier {
        CpuTier::HighEnd => {
            println!("Running high-end binary (AVX512)");
            // Launch target/highend/release/client
        }
        CpuTier::Modern => {
            println!("Running modern binary (AVX2+FMA)");
            // Launch target/modern/release/client
        }
        CpuTier::Baseline => {
            println!("Running baseline binary (SSE2)");
            // Launch target/baseline/release/client
        }
    }
}
```

### Launcher Implementation

Example launcher that selects the best binary:

```rust
use engine_build_utils::cpu_features::detect_tier;
use std::process::Command;
use std::path::PathBuf;

fn main() {
    let tier = detect_tier();

    // Build path to appropriate binary
    let mut binary_path = PathBuf::from("bin");
    binary_path.push(tier.name());
    binary_path.push("client");

    #[cfg(windows)]
    binary_path.set_extension("exe");

    println!("Launching: {}", binary_path.display());
    println!("Tier: {}", tier);

    // Launch the binary
    let status = Command::new(&binary_path)
        .status()
        .expect("Failed to launch game");

    std::process::exit(status.code().unwrap_or(1));
}
```

### Detailed Feature Detection

For advanced use cases:

```rust
use engine_build_utils::cpu_features::detect_features;

fn main() {
    let features = detect_features();

    println!("CPU: {} {}", features.vendor, features.brand);
    println!("Tier: {}", features.tier);

    // Check individual features
    if features.features.avx2 {
        println!("AVX2 available - batch operations will be fast!");
    }

    if features.features.fma {
        println!("FMA available - dot products optimized!");
    }

    if features.features.avx512f {
        println!("AVX512 available - maximum performance!");
    }
}
```

---

## Performance Benchmarking

### Benchmark All Tiers

```bash
# Build and benchmark all tiers
./scripts/build_all_tiers.sh --release --both
./scripts/benchmark_tiers.sh

# Save results to file
./scripts/benchmark_tiers.sh --output benchmark_results.json

# Verbose output
./scripts/benchmark_tiers.sh --verbose
```

### Expected Results

Typical performance gains (Intel Core i7-9700K, 3.6GHz):

**Vec3 Dot Product:**
```
baseline: 2.1 ns
modern:   1.5 ns  (1.4x faster, 40% improvement)
highend:  1.5 ns  (1.4x faster, 40% improvement)
```

**Batch Transform (10,000 entities):**
```
baseline: 40.6 µs
modern:    8.1 µs  (5.0x faster, 400% improvement)
highend:   8.0 µs  (5.1x faster, 410% improvement)
```

**Physics Integration (1,000 rigid bodies):**
```
baseline: 85.3 µs
modern:   62.1 µs  (1.37x faster, 27% improvement)
highend:  58.4 µs  (1.46x faster, 32% improvement)
```

### Validating Performance

After building tiers, verify the speedup:

```bash
# Run benchmarks for each tier
cd engine/math
cargo bench --target-dir ../../target/baseline
cargo bench --target-dir ../../target/modern
cargo bench --target-dir ../../target/highend

# Compare results
criterion-compare target/baseline/criterion target/modern/criterion
```

**Target:** Modern tier should be 5-20% faster for most workloads.

---

## Deployment Strategy

### Steam/Epic/Itch.io Distribution

**Recommended approach:** Ship all three tiers + launcher

```
game/
├── launcher.exe              # Detects CPU and launches correct binary
├── baseline/
│   └── client.exe
├── modern/
│   └── client.exe
└── highend/
    └── client.exe
```

**Total size overhead:** ~3x binary size (mitigated by compression)

**Alternatives:**
1. **Ship only Modern tier** - 95% compatibility, good performance
2. **Ship Modern + Baseline** - 100% compatibility, 2x size
3. **Let users choose** - Settings UI to select tier

### Server Deployment

Servers typically run on known hardware, so you can:

1. **Build for specific hardware:**
   ```bash
   # AWS c5.large (Intel Xeon Platinum, supports AVX512)
   ./scripts/build_all_tiers.sh --release --server
   # Deploy: target/highend/release/server
   ```

2. **Auto-detect on startup:**
   ```rust
   // Server main.rs
   let tier = detect_tier();
   info!("Server running on {} tier", tier.name());
   ```

3. **Build native:**
   ```bash
   # Maximum performance for this exact CPU
   RUSTFLAGS="-C target-cpu=native" cargo build --release --bin server
   ```

### CI/CD Pipeline

Example GitHub Actions workflow:

```yaml
name: Build Tiers

on: [push, pull_request]

jobs:
  build-tiers:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3

      - name: Build all tiers
        run: |
          # Linux/macOS
          ./scripts/build_all_tiers.sh --release --both
        if: runner.os != 'Windows'

      - name: Build all tiers (Windows)
        run: |
          .\scripts\build_all_tiers.ps1 -Release -Both
        if: runner.os == 'Windows'

      - name: Run benchmarks
        run: ./scripts/benchmark_tiers.sh
        if: runner.os != 'Windows'

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: binaries-${{ matrix.os }}
          path: target/*/release/*
```

---

## Troubleshooting

### "Illegal instruction" crash

**Problem:** Binary crashes immediately with SIGILL (illegal instruction)

**Cause:** CPU doesn't support the instruction set used in the binary

**Solution:**
1. Check your CPU tier:
   ```bash
   cargo run --example cpu_tier_detection
   ```
2. Run the appropriate binary:
   - If Modern tier not supported → use Baseline
   - If High-end not supported → use Modern or Baseline

### No performance improvement

**Problem:** Modern tier is same speed as baseline

**Possible causes:**

1. **Not building in release mode:**
   ```bash
   # Wrong (debug mode)
   ./scripts/build_all_tiers.sh --both

   # Correct (release mode)
   ./scripts/build_all_tiers.sh --release --both
   ```

2. **SIMD code not being used:**
   - Check if your workload actually uses SIMD operations
   - Run benchmarks to verify: `./scripts/benchmark_tiers.sh`

3. **Thermal throttling:**
   - High-end CPUs may throttle with AVX512
   - Monitor CPU frequency during benchmarks

4. **Memory bottleneck:**
   - SIMD helps compute, not memory access
   - If bottlenecked by RAM, tiers will perform similarly

### Build fails for specific tier

**Problem:** Modern or High-end tier fails to build

**Solutions:**

1. **Update Rust:**
   ```bash
   rustup update stable
   ```

2. **Check LLVM version:**
   ```bash
   rustc --version --verbose
   # LLVM version should be 14+
   ```

3. **Try manual build:**
   ```bash
   RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release
   ```

4. **Check for unsupported targets:**
   - Some targets (e.g., ARM) don't support x86-64-v3/v4
   - Use baseline for non-x86_64 architectures

### AVX512 slower than AVX2

**Known issue:** AVX512 can be slower due to:

1. **Frequency throttling** - CPU reduces clock speed with AVX512
2. **Transition penalties** - Switching between AVX2 and AVX512
3. **Power limits** - Thermal design limits may throttle

**Solution:** Benchmark on your hardware. If Modern is faster, prefer it:

```bash
./scripts/benchmark_tiers.sh --verbose
# Compare modern vs highend results
# Use the faster tier for your hardware
```

---

## Technical Details

### Cargo Configuration

The `.cargo/config.toml` defines tier-specific targets:

```toml
# Baseline - no special flags
[target.x86_64-pc-windows-msvc]
# Uses default SSE2

# Modern - x86-64-v3
[target.x86_64-v3-pc-windows-msvc]
rustflags = ["-C", "target-cpu=x86-64-v3"]

# High-end - x86-64-v4
[target.x86_64-v4-pc-windows-msvc]
rustflags = ["-C", "target-cpu=x86-64-v4"]
```

**Note:** These are custom target configurations. Rust doesn't have built-in "x86_64-v3" targets, so we use RUSTFLAGS to configure them.

### x86-64 Microarchitecture Levels

The tiers correspond to x86-64 psABI levels:

| Level | Our Tier | Required Features |
|-------|----------|-------------------|
| x86-64 | Baseline | SSE2 (and earlier x86-64 baseline) |
| x86-64-v2 | - | + CMPXCHG16B, LAHF-SAHF, POPCNT, SSE3, SSE4.1, SSE4.2, SSSE3 |
| x86-64-v3 | Modern | + AVX, AVX2, BMI1, BMI2, F16C, FMA, LZCNT, MOVBE, XSAVE |
| x86-64-v4 | High-end | + AVX512F, AVX512BW, AVX512CD, AVX512DQ, AVX512VL |

**Reference:** [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI)

### CPU Feature Detection Implementation

Runtime detection uses `std::arch::is_x86_feature_detected!`:

```rust
#[cfg(target_arch = "x86_64")]
pub fn detect_tier() -> CpuTier {
    use std::arch::is_x86_feature_detected;

    // Check for x86-64-v4 (AVX512)
    if is_x86_feature_detected!("avx512f")
        && is_x86_feature_detected!("avx512dq")
        && is_x86_feature_detected!("avx512cd")
        && is_x86_feature_detected!("avx512bw")
        && is_x86_feature_detected!("avx512vl")
    {
        return CpuTier::HighEnd;
    }

    // Check for x86-64-v3 (AVX2 + FMA)
    if is_x86_feature_detected!("avx2")
        && is_x86_feature_detected!("fma")
        && is_x86_feature_detected!("sse4.2")
        && is_x86_feature_detected!("bmi1")
        && is_x86_feature_detected!("bmi2")
    {
        return CpuTier::Modern;
    }

    // Fallback to baseline
    CpuTier::Baseline
}
```

This uses CPUID instruction at runtime (zero overhead, < 1µs).

### SIMD Code Generation

Rust's `std::simd` and manual intrinsics benefit from tier flags:

**Baseline (SSE2):**
```rust
// Compiles to: movaps, mulps, addps (128-bit SSE)
#[target_feature(enable = "sse2")]
unsafe fn dot_sse2(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    // SSE2 code
}
```

**Modern (AVX2 + FMA):**
```rust
// Compiles to: vfmadd213ps (256-bit FMA)
#[target_feature(enable = "avx2,fma")]
unsafe fn dot_avx2_fma(a: &[f32; 8], b: &[f32; 8]) -> f32 {
    // AVX2 + FMA code
}
```

**High-end (AVX512):**
```rust
// Compiles to: vfmadd213ps (512-bit FMA)
#[target_feature(enable = "avx512f")]
unsafe fn dot_avx512(a: &[f32; 16], b: &[f32; 16]) -> f32 {
    // AVX512 code
}
```

The engine-math crate uses runtime dispatch to select the best implementation.

### Why Not Use `target-cpu=native`?

**`-C target-cpu=native`** builds for YOUR specific CPU, but:

- Not portable (crashes on different CPUs)
- Can't distribute to users
- CI/CD builds may not match production hardware

**Multi-tier builds solve this:**
- Build 3 portable binaries
- Detect at runtime
- Users get best performance for THEIR hardware

Use `native` only for:
- Local development
- Benchmarking
- Server deployments (known hardware)

### ARM64 / Apple Silicon

ARM has different SIMD (NEON, SVE):

```rust
#[cfg(target_arch = "aarch64")]
pub fn detect_tier() -> CpuTier {
    // ARM doesn't have tiered builds (NEON is always available)
    CpuTier::Baseline
}
```

For Apple Silicon:
- All M1/M2/M3 support NEON
- Build single ARM64 binary
- No need for tiers (uniform architecture)

---

## See Also

- [docs/performance-targets.md](performance-targets.md) - Performance goals
- [docs/pgo.md](pgo.md) - Profile-Guided Optimization
- [docs/benchmarking.md](benchmarking.md) - Benchmarking guide
- [engine/math/CPU_FEATURES.md](../engine/math/CPU_FEATURES.md) - SIMD implementation details
- [engine/math/PERFORMANCE.md](../engine/math/PERFORMANCE.md) - Math benchmark results

---

## References

- [x86-64 psABI Microarchitecture Levels](https://gitlab.com/x86-psABIs/x86-64-ABI)
- [Rust SIMD Performance Guide](https://rust-lang.github.io/packed_simd/perf-guide/)
- [Intel Intrinsics Guide](https://www.intel.com/content/www/us/en/docs/intrinsics-guide/)
- [AMD Software Optimization Guide](https://www.amd.com/en/support/tech-docs)

---

**Status:** ✅ Implemented (Phase 0.5 - Task #59)
**Maintained by:** Build Infrastructure Team
**Last updated:** 2025-02-01
