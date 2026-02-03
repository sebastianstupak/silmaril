# macOS Testing Quick Start Guide

This guide provides instructions for testing the macOS platform optimizations on actual macOS hardware.

---

## Prerequisites

### Required

- macOS 10.15 (Catalina) or newer
- Rust 1.70+ (`rustup update`)
- Xcode Command Line Tools (`xcode-select --install`)

### Recommended

- macOS 13+ (Ventura) for best Apple Silicon support
- Access to both Intel and Apple Silicon Macs (if possible)

---

## Quick Test (5 minutes)

### 1. Clone and Build

```bash
git clone https://github.com/your-org/silmaril.git
cd silmaril

# Build the core library
cargo build -p engine-core --lib --release
```

### 2. Run Unit Tests

```bash
# Run all platform tests
cargo test -p engine-core platform -- --nocapture

# Expected output:
# test result: ok. 38 passed; 0 failed
```

### 3. Run Benchmarks (Quick)

```bash
# Run time benchmarks only (fast)
cargo bench -p engine-core --bench platform_benches -- time/monotonic

# Look for:
# - time/monotonic_nanos/single: ~15-20ns (Apple Silicon) or ~25-30ns (Intel)
```

---

## Full Testing (30 minutes)

### 1. Platform Information

First, identify your hardware:

```bash
# Check CPU architecture
uname -m
# x86_64 = Intel Mac
# arm64 = Apple Silicon Mac

# Check macOS version
sw_vers

# Check CPU details
sysctl -n machdep.cpu.brand_string  # Intel
sysctl -n machdep.cpu.core_count    # All

# For Apple Silicon, check core configuration
system_profiler SPHardwareDataType | grep "Chip"
```

### 2. Run All Tests

```bash
# All platform tests (verbose)
cargo test -p engine-core platform -- --nocapture --test-threads=1

# Property tests (10,000 iterations)
cargo test -p engine-core --test platform_proptests -- --nocapture

# Integration tests
cargo test -p engine-core --test platform_integration -- --nocapture
```

### 3. Run All Benchmarks

```bash
# Full benchmark suite (~10 minutes)
cargo bench -p engine-core --bench platform_benches

# Save results as baseline
cargo bench -p engine-core --bench platform_benches -- --save-baseline macos-baseline

# Results will be in: target/criterion/
```

### 4. Verify Performance Targets

Check benchmark results against targets:

| Benchmark | Apple Silicon Target | Intel Target | Your Result |
|-----------|---------------------|--------------|-------------|
| time/monotonic_nanos/single | <20ns | <30ns | ? |
| time/monotonic_nanos/batch_1000 | <20us | <30us | ? |
| filesystem/normalize_path/simple | <200ns | <500ns | ? |
| filesystem/normalize_path/complex | <1.5us | <2us | ? |
| threading/set_priority/normal | <2us | <5us | ? |

### 5. Test-Specific Checks

#### A. Time Backend

```bash
# Test timebase ratio
cargo test -p engine-core test_macos_timebase_ratio -- --nocapture

# Expected output (Apple Silicon):
# timebase_info: numer=1, denom=1

# Expected output (Intel):
# timebase_info: numer and denom vary (e.g., 1:1000000000)
```

#### B. Sleep Accuracy

```bash
# Test sleep accuracy
cargo test -p engine-core test_macos_sleep_accuracy -- --nocapture --ignored

# Should pass with tolerance (90-150% of requested duration)
```

#### C. Threading Affinity

```bash
# Verify affinity returns error (expected behavior)
cargo test -p engine-core test_macos_affinity_not_supported -- --nocapture

# Expected: PlatformNotSupported error
```

---

## Advanced Testing

### 1. Cross-Compilation Testing

If you have both Intel and Apple Silicon Macs:

```bash
# On Intel Mac, test both architectures
cargo test -p engine-core --target x86_64-apple-darwin platform
cargo test -p engine-core --target aarch64-apple-darwin platform  # Via Rosetta

# On Apple Silicon Mac, test both architectures
cargo test -p engine-core --target aarch64-apple-darwin platform
cargo test -p engine-core --target x86_64-apple-darwin platform   # Via Rosetta
```

### 2. Release vs Debug Performance

```bash
# Debug build (slower but with checks)
cargo bench -p engine-core --bench platform_benches

# Release build (production)
cargo bench -p engine-core --bench platform_benches --release
```

### 3. Profiling with Instruments

```bash
# Build with symbols
cargo build -p engine-core --lib --release

# Run benchmarks with instruments
instruments -t "Time Profiler" target/release/deps/platform_benches-* -- --bench
```

### 4. Memory Testing

```bash
# Check for leaks
cargo test -p engine-core platform
leaks --atExit -- cargo test -p engine-core platform
```

---

## Troubleshooting

### Issue: Tests Fail with Permission Errors

**Symptom**: `set_thread_priority` fails with EPERM

**Solution**: Realtime priority requires elevated privileges. This is expected for `ThreadPriority::Realtime`. Normal/Low/High should work without sudo.

```bash
# If you need to test realtime:
sudo cargo test -p engine-core test_macos_set_priority
```

### Issue: Benchmarks are Slow

**Symptom**: Benchmarks take >30 minutes

**Cause**: First run compiles dependencies

**Solution**: Use `--no-run` first, then run:

```bash
cargo bench -p engine-core --bench platform_benches --no-run
cargo bench -p engine-core --bench platform_benches
```

### Issue: Time Benchmark Fails Target

**Symptom**: `monotonic_nanos` is slower than target

**Possible Causes**:
1. Background processes consuming CPU
2. Thermal throttling
3. Power saving mode

**Solutions**:
```bash
# Check CPU frequency
sysctl hw.cpufrequency

# Disable Spotlight indexing temporarily
sudo mdutil -a -i off

# Close unnecessary apps
# Plug in power adapter (for laptops)
```

### Issue: Different Results on Rosetta

**Symptom**: x86_64 benchmarks on Apple Silicon differ from native Intel

**Explanation**: Rosetta 2 translation adds overhead. Compare:
- Native Apple Silicon (aarch64-apple-darwin)
- Native Intel (x86_64-apple-darwin on Intel Mac)
- Rosetta (x86_64-apple-darwin on Apple Silicon Mac) - will be slower

---

## Reporting Results

Please report results using this template:

### System Information

```
macOS Version: [e.g., 14.2]
Hardware: [e.g., MacBook Pro M2 Max, iMac Intel i9]
CPU: [from sysctl machdep.cpu.brand_string]
Cores: [Performance + Efficiency]
Memory: [e.g., 32GB]
```

### Test Results

```bash
# Copy output from:
cargo test -p engine-core platform

# Result: X passed, Y failed, Z ignored
```

### Benchmark Results

```bash
# Copy relevant lines from:
cargo bench -p engine-core --bench platform_benches -- time

# time/monotonic_nanos/single: [XX ns]
# time/monotonic_nanos/batch_1000: [XX us]
```

### Issues Encountered

- [List any failures, unexpected behavior, or performance issues]

---

## Expected Results Summary

### Apple Silicon (M1/M2/M3/M4)

```
✅ All 38+ platform tests should pass
✅ time/monotonic_nanos/single: 15-20ns
✅ time/monotonic_nanos/batch_1000: 15-25us
✅ filesystem/normalize_path/simple: 100-150ns
✅ filesystem/normalize_path/complex: 1.0-1.5us
✅ threading/set_priority: 1-3us
✅ threading/set_affinity: Returns PlatformNotSupported (expected)
```

### Intel Macs

```
✅ All 38+ platform tests should pass
✅ time/monotonic_nanos/single: 25-35ns
✅ time/monotonic_nanos/batch_1000: 25-40us
✅ filesystem/normalize_path/simple: 120-180ns
✅ filesystem/normalize_path/complex: 1.2-1.8us
✅ threading/set_priority: 1-3us
✅ threading/set_affinity: Returns PlatformNotSupported (expected)
```

---

## Next Steps After Testing

1. **Report results** to the team
2. **Save benchmark baseline** for future comparisons
3. **Document any platform-specific quirks** discovered
4. **Run integration tests** with MoltenVK renderer (Phase 2)

---

## Questions?

See detailed documentation in:
- `MACOS_OPTIMIZATION_RESULTS.md` - Comprehensive optimization details
- `MACOS_OPTIMIZATION_CHANGES.md` - Code changes summary
- `engine/core/src/platform/` - Implementation with inline docs

---

**Happy Testing!**
