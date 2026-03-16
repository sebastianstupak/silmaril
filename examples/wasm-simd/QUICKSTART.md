# WASM SIMD Benchmark - Quick Start

## 1. Build the WASM Binary

```bash
# From wasm-demo directory
wasm-pack build --target web --release
```

**Expected output:**
```
[INFO]: ✨   Done in 5s
[INFO]: 📦   Your wasm pkg is ready at D:\...\wasm-demo\pkg
```

## 2. Verify SIMD Compilation

```bash
# Linux/macOS
./verify-simd.sh

# Windows
verify-simd.bat
```

**Expected output:**
```
✅ SUCCESS: WASM SIMD is working!
Total v128 SIMD instructions: 253
```

## 3. Start Development Server

```bash
# Python 3
python3 -m http.server 8000

# Or use the custom server (with CORS headers)
python3 test-server.py
```

## 4. Open in Browser

Navigate to: **http://localhost:8000**

## 5. Run Benchmarks

1. Configure parameters (defaults work well):
   - Iterations: 1000
   - Entity Count: 1000

2. Click "Run All Benchmarks"

3. View results:
   - Scalar: ~120 ms (baseline)
   - SIMD 4-wide: ~45 ms (2.5x faster)
   - SIMD 8-wide: ~50 ms (2.4x faster)

## Browser Requirements

- **Chrome 91+** ✅ Works by default
- **Firefox 89+** ✅ Works by default
- **Edge 91+** ✅ Works by default
- **Safari 16.4+** ⚠️ Requires enabling WASM SIMD in experimental features

### Safari Setup

1. Open Safari
2. **Develop → Experimental Features**
3. Enable **WebAssembly SIMD**
4. Reload the page

## Troubleshooting

### Build Fails

```bash
# Clean build
rm -rf pkg target
cargo clean
wasm-pack build --target web --release
```

### No SIMD Instructions Found

Check `.cargo/config.toml` has:
```toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "target-feature=+simd128"]
```

### Browser Shows Errors

1. Check browser console (F12)
2. Ensure server is running (localhost:8000)
3. Try a different browser (Chrome recommended)
4. Check browser version (must be recent)

## Expected Performance

**1000 entities, 1000 iterations:**
- Scalar: 100-120 ms
- SIMD 4-wide: 40-50 ms (2-3x faster)
- SIMD 8-wide: 50-60 ms (2-2.5x faster)

**Why is 4-wide often faster than 8-wide?**
- WASM SIMD is 128-bit (4 floats) native
- 8-wide uses two 128-bit operations (overhead)
- Conversion cost increases with width

## Next Steps

- See **[docs/wasm-simd.md](../docs/wasm-simd.md)** for complete documentation
- Experiment with different entity counts
- Compare across different browsers
- Integrate SIMD into your game logic
