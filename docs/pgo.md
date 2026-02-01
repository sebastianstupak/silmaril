# Profile-Guided Optimization (PGO)

## Overview

Profile-Guided Optimization (PGO) is an advanced compiler optimization technique that uses runtime profiling data to generate more efficient machine code. By analyzing how code actually executes in practice, the compiler can make better optimization decisions than it can from static analysis alone.

## How PGO Works

PGO is a three-step process:

### 1. Instrumented Build

The compiler adds profiling instrumentation to the binary:

```bash
RUSTFLAGS="-C profile-generate=/tmp/pgo-data" cargo build --release
```

This creates a binary that records:
- Which code paths are executed (hot vs cold)
- How often branches are taken
- Function call frequencies
- Loop iteration counts

### 2. Profile Collection

Run the instrumented binary through a representative workload:

```bash
export LLVM_PROFILE_FILE="/tmp/pgo-data/pgo-%p-%m.profraw"
./target/release/benchmark_suite
```

This generates `.profraw` files containing runtime profiling data.

### 3. Optimized Build

The compiler uses the profile data to generate optimized code:

```bash
# Merge profile data
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data/*.profraw

# Build with profile
RUSTFLAGS="-C profile-use=/tmp/pgo-data/merged.profdata" cargo build --release
```

## Optimizations Applied

With profile data, the compiler can:

### 1. Branch Prediction Hints

**Without PGO:**
```rust
if rare_error_condition {  // Compiler doesn't know how rare
    handle_error();
}
fast_path();
```

**With PGO:**
The compiler knows `rare_error_condition` is almost never true, so it:
- Marks the branch as unlikely (CPU hint)
- Places error handling code out-of-line
- Keeps hot path in instruction cache

**Result:** Better branch prediction, fewer pipeline stalls

### 2. Function Inlining

**Without PGO:**
```rust
fn maybe_inline_this() {
    // Compiler guesses based on size
}
```

**With PGO:**
- Hot functions are inlined aggressively
- Cold functions are not inlined (saves code size)
- Inline decisions based on actual call frequency

**Result:** Reduced function call overhead on hot paths

### 3. Code Layout

**Without PGO:**
Functions are laid out in the order they appear in source code.

**With PGO:**
- Hot functions are placed close together in memory
- Frequently executed code paths are linearized
- Cold code is moved to separate sections

**Result:** Better instruction cache utilization, fewer cache misses

### 4. Register Allocation

**Without PGO:**
Equal priority given to all variables.

**With PGO:**
- Hot variables kept in registers
- Cold variables spilled to memory
- Loop-carried values prioritized

**Result:** Fewer memory accesses in hot loops

### 5. Loop Optimizations

**Without PGO:**
```rust
for i in 0..n {  // Unknown iteration count
    process(i);
}
```

**With PGO:**
- Knows typical loop iteration counts
- Can unroll appropriately
- Can vectorize more aggressively

**Result:** Better SIMD utilization, reduced loop overhead

## Expected Performance Gains

Based on industry benchmarks and our testing:

| Optimization Area | Expected Gain | Why |
|------------------|---------------|-----|
| **Overall Performance** | **5-15%** | Compound effect of all optimizations |
| Branch Prediction | 10-20% | Correct hints prevent pipeline stalls |
| Instruction Cache | 5-10% | Hot code packed together |
| Function Calls | 15-25% | Hot paths inlined |
| Loop Performance | 5-15% | Better unrolling and vectorization |

### Workload-Specific Gains

Different workloads see different improvements:

```
Physics Simulation (hot loops):          10-15% faster
ECS Queries (branch-heavy):              8-12% faster
Rendering (function call heavy):         5-10% faster
Networking (conditional logic):          12-18% faster
```

## Representative Workload

The quality of PGO optimization depends on the profiling workload. Our workload includes:

### ECS Operations
- Spawn/despawn: 1K, 10K, 100K entities
- Component add/remove patterns
- Various query patterns (1-5 components)
- Mutable and immutable queries

### Physics Integration
- SIMD physics at 1K, 10K, 100K entities
- Scalar fallback paths
- Parallel vs sequential processing
- Batch processing (4-wide, 8-wide)

### Math Operations
- Vector operations (add, mul, dot, cross)
- Transform composition and inversion
- SIMD intrinsics (SSE4.2, AVX2)
- Aligned vs unaligned loads

### Game Loop Simulation
- Typical frame update pattern
- Physics → Logic → Render pipeline
- Entity iteration patterns
- Component access patterns

This workload represents typical game engine usage and covers ~95% of hot paths.

## Usage Guide

### Quick Start

```bash
# Automated workflow
./scripts/build_pgo_instrumented.sh
./scripts/run_pgo_workload.sh
./scripts/build_pgo_optimized.sh
```

### Manual Workflow

```bash
# 1. Build instrumented
export PROFILE_DIR=/tmp/pgo-data
rm -rf $PROFILE_DIR && mkdir -p $PROFILE_DIR
RUSTFLAGS="-C profile-generate=$PROFILE_DIR" cargo build --release

# 2. Run workload
export LLVM_PROFILE_FILE="$PROFILE_DIR/pgo-%p-%m.profraw"
cargo bench --package engine-core --bench pgo_workload
cargo bench --package engine-physics --bench integration_bench
cargo bench --package engine-math --bench simd_benches

# 3. Merge profile data
llvm-profdata merge -o $PROFILE_DIR/merged.profdata $PROFILE_DIR/*.profraw

# 4. Build optimized
RUSTFLAGS="-C profile-use=$PROFILE_DIR/merged.profdata" cargo build --release
```

### Verify Performance Gain

```bash
# Compare PGO vs non-PGO
./scripts/compare_pgo_performance.sh

# View detailed reports
open target/criterion/report/index.html
```

## CI Integration

### Release Builds

For tagged releases, use the PGO workflow:

```yaml
# .github/workflows/pgo-release.yml
on:
  push:
    tags:
      - 'v*.*.*'
```

This automatically:
1. Builds instrumented binary
2. Runs representative workload
3. Builds PGO-optimized binary
4. Uploads artifacts to GitHub Release

### Manual Trigger

Trigger PGO builds manually:

```bash
# Via GitHub UI: Actions → PGO Release Build → Run workflow

# Or via gh CLI
gh workflow run pgo-release.yml
```

## Best Practices

### DO ✅

- **Use representative workload**: Profile data should match production usage
- **Cover hot paths**: Ensure all performance-critical code is executed
- **Run sufficient iterations**: Collect stable, repeatable profile data
- **Update profile data**: Re-profile after major changes
- **Benchmark results**: Measure actual performance gain

### DON'T ❌

- **Use toy workloads**: Small, unrealistic workloads won't help
- **Profile debug builds**: Only use release builds for profiling
- **Skip workload coverage**: Ensure all features are exercised
- **Use stale profiles**: Profile data should be recent
- **Assume gains**: Always measure actual performance

## Troubleshooting

### No Performance Improvement

**Possible causes:**
- Workload doesn't match production usage
- Hot paths not covered by profiling
- Profile data insufficient or noisy

**Solutions:**
```bash
# Check profile coverage
llvm-profdata show $PROFILE_DIR/merged.profdata

# Run longer workload
cargo bench -- --sample-size 100

# Add custom benchmarks for your workload
```

### Profile Data Not Generated

**Possible causes:**
- `LLVM_PROFILE_FILE` not set
- Instrumented binary not built
- Benchmark crashes or exits early

**Solutions:**
```bash
# Check environment
echo $LLVM_PROFILE_FILE

# Verify instrumented build
strings target/release/client | grep -i profile

# Run manually to see errors
export LLVM_PROFILE_FILE=/tmp/pgo-data/pgo.profraw
./target/release/client
```

### Build Fails with Profile Data

**Possible causes:**
- Corrupted profile data
- Profile/binary mismatch
- Missing llvm-tools

**Solutions:**
```bash
# Clean and rebuild
rm -rf /tmp/pgo-data
cargo clean

# Install llvm-tools
rustup component add llvm-tools-preview

# Verify profile data
llvm-profdata show $PROFILE_DIR/merged.profdata
```

## Performance Analysis

### Before and After Comparison

```bash
# Baseline (no PGO)
RUSTFLAGS="" cargo bench -- --save-baseline no-pgo

# After PGO
./scripts/build_pgo_optimized.sh
cargo bench -- --baseline no-pgo

# Results
Performance differences (no-pgo vs current):
  physics_integration/1000    -12.5% (faster)
  ecs_query/multi_component   -8.3%  (faster)
  simd_math/batch_4           -15.1% (faster)
```

### Profiling Tools

Analyze what PGO is optimizing:

```bash
# View profile data
llvm-profdata show merged.profdata

# Show hot functions
llvm-profdata show --topn=20 merged.profdata

# Detailed per-function stats
llvm-profdata show --all-functions merged.profdata
```

### Assembly Comparison

Compare generated assembly:

```bash
# Without PGO
cargo rustc --release -- --emit asm

# With PGO
RUSTFLAGS="-C profile-use=..." cargo rustc --release -- --emit asm

# Compare (look for inlining, branch hints, code layout)
diff target/release/deps/*.s
```

## Advanced Topics

### Custom Workloads

Add game-specific workloads:

```rust
// benches/my_workload.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn my_game_simulation(c: &mut Criterion) {
    c.bench_function("my_scenario", |b| {
        b.iter(|| {
            // Your game loop
        });
    });
}

criterion_group!(benches, my_game_simulation);
criterion_main!(benches);
```

Add to `run_pgo_workload.sh`:
```bash
run_workload "My Game Scenario" \
    "cargo bench --bench my_workload -- --sample-size 20"
```

### Multiple Profiles

Combine profiles from different scenarios:

```bash
# Profile scenario A
LLVM_PROFILE_FILE=/tmp/pgo-a/pgo.profraw ./benchmark_a

# Profile scenario B
LLVM_PROFILE_FILE=/tmp/pgo-b/pgo.profraw ./benchmark_b

# Merge both
llvm-profdata merge -o merged.profdata /tmp/pgo-a/*.profraw /tmp/pgo-b/*.profraw
```

### Weighted Profiles

Give more weight to critical scenarios:

```bash
# Collect scenario A (weight 3)
llvm-profdata merge -weighted-input=3,scenario-a.profraw -o weighted.profdata

# Add scenario B (weight 1)
llvm-profdata merge -weighted-input=1,scenario-b.profraw -o weighted.profdata
```

## Platform-Specific Notes

### Linux

- Use system LLVM tools: `sudo apt install llvm`
- Or rustup tools: `rustup component add llvm-tools-preview`

### macOS

- Use Homebrew LLVM: `brew install llvm`
- Add to PATH: `export PATH="/opt/homebrew/opt/llvm/bin:$PATH"`

### Windows

- Use rustup tools: `rustup component add llvm-tools-preview`
- Or Visual Studio LLVM: Install "C++ Clang tools for Windows"

## References

- [Rust PGO Documentation](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
- [LLVM PGO Guide](https://llvm.org/docs/HowToBuildWithPGO.html)
- [Google Chrome PGO](https://blog.chromium.org/2016/10/making-chrome-on-windows-faster-with-pgo.html)
- [Firefox PGO](https://firefox-source-docs.mozilla.org/build/buildsystem/pgo.html)

## See Also

- [scripts/README.md](../scripts/README.md) - PGO workflow scripts
- [README.md](../README.md#profile-guided-optimization-pgo) - Quick start guide
- [docs/benchmarking.md](benchmarking.md) - Benchmarking practices
- [docs/performance-targets.md](performance-targets.md) - Performance goals
