#!/bin/bash
# Build WASM SIMD demo for web browsers

set -e

echo "Building WASM SIMD demo..."
echo ""

# Build with wasm-pack (SIMD enabled via .cargo/config.toml)
echo "Building with WASM SIMD support (target-feature=+simd128)..."
wasm-pack build --target web --release

echo ""
echo "Build complete!"
echo ""
echo "Generated files:"
echo "  - pkg/wasm_simd_demo_bg.wasm (WASM binary)"
echo "  - pkg/wasm_simd_demo.js (JS bindings)"
echo ""

# Check for SIMD instructions (requires wabt tools)
if command -v wasm-objdump &> /dev/null; then
    echo "Checking for SIMD instructions..."
    SIMD_COUNT=$(wasm-objdump -d pkg/wasm_simd_demo_bg.wasm 2>/dev/null | grep -c "v128" || echo "0")
    echo "Found $SIMD_COUNT v128 SIMD instructions"
    echo ""

    if [ "$SIMD_COUNT" -gt 0 ]; then
        echo "✓ SIMD compilation successful!"
        echo ""
        echo "Sample SIMD instructions:"
        wasm-objdump -d pkg/wasm_simd_demo_bg.wasm 2>/dev/null | grep "v128" | head -5
    else
        echo "⚠ Warning: No v128 SIMD instructions found"
        echo "This may indicate SIMD is not being used"
    fi
else
    echo "Note: Install wabt tools to verify SIMD instructions"
    echo "  macOS: brew install wabt"
    echo "  Linux: apt install wabt"
    echo "  Windows: Download from https://github.com/WebAssembly/wabt/releases"
fi

echo ""
echo "To run the demo:"
echo "  1. Start a local server: python -m http.server 8000"
echo "  2. Open http://localhost:8000 in your browser"
echo ""
