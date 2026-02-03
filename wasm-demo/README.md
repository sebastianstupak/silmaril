# WASM SIMD Performance Demo

This demo showcases the performance benefits of SIMD (Single Instruction, Multiple Data) operations in WebAssembly for the Silmaril's math library.

## What It Does

Compares three physics integration approaches:
1. **Scalar**: Traditional single-entity processing (baseline)
2. **SIMD 4-wide**: Process 4 entities simultaneously using 128-bit SIMD
3. **SIMD 8-wide**: Process 8 entities simultaneously using 256-bit SIMD

## Expected Performance

- **Chrome 91+**: 2-4x speedup with SIMD
- **Firefox 89+**: 2-4x speedup with SIMD
- **Safari 16.4+**: 2-4x speedup with SIMD (requires enabling SIMD)

## Build Instructions

### Prerequisites

```bash
# Install wasm-pack
cargo install wasm-pack

# Or use npm
npm install -g wasm-pack
```

### Build for Web

```bash
# Build optimized WASM with SIMD support
./build.sh

# Or manually:
wasm-pack build --target web --release -- --features simd
```

This generates:
- `pkg/wasm_simd_demo_bg.wasm` - The WASM binary with SIMD instructions
- `pkg/wasm_simd_demo.js` - JavaScript bindings

### Verify SIMD Instructions

Check that the WASM binary contains v128 SIMD instructions:

```bash
# Install wabt (WebAssembly Binary Toolkit)
# Windows: Download from https://github.com/WebAssembly/wabt/releases
# macOS: brew install wabt
# Linux: apt install wabt

# Disassemble and check for v128 instructions
wasm-objdump -d pkg/wasm_simd_demo_bg.wasm | grep "v128"
```

You should see instructions like:
- `v128.load` - Load 128-bit vectors
- `v128.store` - Store 128-bit vectors
- `f32x4.add` - Add four f32s simultaneously
- `f32x4.mul` - Multiply four f32s simultaneously

## Running the Demo

### Local Development Server

```bash
# Python 3
python -m http.server 8000

# Or use any static file server
# npm install -g http-server
# http-server -p 8000
```

Then open http://localhost:8000 in your browser.

### Browser Requirements

**Chrome/Edge:**
- Version 91+ (SIMD enabled by default)

**Firefox:**
- Version 89+ (SIMD enabled by default)

**Safari:**
- Version 16.4+ (requires enabling SIMD in Experimental Features)
- Enable: Develop > Experimental Features > WebAssembly SIMD

## Benchmark Details

The benchmark performs physics integration:

```rust
position = position + velocity * dt
```

For N entities over M iterations, comparing:
- Scalar: N * M individual operations
- SIMD 4-wide: (N/4) * M operations (4x parallelism)
- SIMD 8-wide: (N/8) * M operations (8x parallelism)

## Verification

After building, verify SIMD instructions are present:

```bash
# Install wasm-tools if not already installed
cargo install wasm-tools

# Convert WASM to text format and count SIMD instructions
wasm-tools print pkg/wasm_simd_demo_bg.wasm > output.wat
grep -c "f32x4\|v128\|i32x4" output.wat

# Expected: 200+ SIMD instructions
# Actual (2026-02-01): 253 instructions ✅
```

## Troubleshooting

### SIMD Not Working?

1. **Check browser version**: Ensure you're using a recent browser with SIMD support
2. **Enable SIMD in Safari**: Safari requires manual enablement in Experimental Features
3. **Check console**: Look for WASM loading errors in browser console
4. **Verify WASM binary**: Use `wasm-tools` to confirm v128 instructions are present (see Verification above)

### Build Errors?

```bash
# Clean build
cargo clean
rm -rf pkg/

# Rebuild
wasm-pack build --target web --release -- --features simd
```

### No Speedup?

- SIMD overhead: For very small datasets (<100 entities), conversion overhead may negate benefits
- Browser optimization: Some browsers may auto-vectorize scalar code
- Memory bandwidth: SIMD speedup is memory-bound; expect 2-4x, not 4-8x

## Performance Tips

### Optimal Configuration

- **Entity count**: 1000-10000 (enough to amortize conversion overhead)
- **Iterations**: 100-1000 (enough to see consistent timing)
- **SIMD width**: 4-wide typically performs best due to lower conversion overhead

### Browser-Specific Notes

**Chrome**: Best overall SIMD performance, fastest execution
**Firefox**: Good SIMD performance, slightly slower than Chrome
**Safari**: SIMD works but requires manual enablement, performance varies

## Technical Details

### WASM Features Used

- `simd128`: 128-bit SIMD operations (v128 instructions)
- `bulk-memory`: Fast memory operations
- `mutable-globals`: Mutable global state

### Rust SIMD Library

Uses `wide` crate for portable SIMD:
- Compiles to native SIMD on x86/ARM
- Compiles to WASM SIMD v128 instructions
- Provides safe, high-level SIMD API

## Next Steps

1. **Optimize build size**: Use `opt-level = "s"` or `"z"` for smaller WASM
2. **Add more benchmarks**: Test quaternion operations, matrix math
3. **Compare browsers**: Run on Chrome, Firefox, Safari and compare
4. **Profile with DevTools**: Use browser profiler to identify bottlenecks

## Resources

- [WebAssembly SIMD Proposal](https://github.com/WebAssembly/simd)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [wide Crate](https://docs.rs/wide/)
- [Silmaril](https://github.com/your-org/silmaril)
