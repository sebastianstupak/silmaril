#!/bin/bash
# Verify WASM SIMD compilation

set -e

echo "=================================="
echo "WASM SIMD Verification Script"
echo "=================================="
echo ""

# Check if pkg directory exists
if [ ! -d "pkg" ]; then
    echo "❌ Error: pkg/ directory not found"
    echo "Run 'wasm-pack build --target web --release' first"
    exit 1
fi

# Check if WASM binary exists
if [ ! -f "pkg/wasm_simd_demo_bg.wasm" ]; then
    echo "❌ Error: WASM binary not found"
    echo "Run 'wasm-pack build --target web --release' first"
    exit 1
fi

echo "✓ Found WASM binary: pkg/wasm_simd_demo_bg.wasm"
echo ""

# Check file size
SIZE=$(stat -c%s "pkg/wasm_simd_demo_bg.wasm" 2>/dev/null || stat -f%z "pkg/wasm_simd_demo_bg.wasm" 2>/dev/null || echo "unknown")
echo "Binary size: $SIZE bytes"
echo ""

# Check if wasm-tools is installed
if ! command -v wasm-tools &> /dev/null; then
    echo "⚠ Warning: wasm-tools not found"
    echo "Install with: cargo install wasm-tools"
    echo ""
    exit 0
fi

echo "✓ Found wasm-tools"
echo ""

# Convert WASM to WAT
echo "Converting WASM to text format..."
wasm-tools print pkg/wasm_simd_demo_bg.wasm > pkg/output.wat 2>&1

if [ $? -ne 0 ]; then
    echo "❌ Error: Failed to convert WASM to WAT"
    exit 1
fi

echo "✓ Converted to WAT format"
echo ""

# Count SIMD instructions
echo "Counting SIMD instructions..."
SIMD_COUNT=$(grep -c "f32x4\|v128\|i32x4" pkg/output.wat || echo "0")

echo ""
echo "=================================="
echo "SIMD Verification Results"
echo "=================================="
echo ""
echo "Total v128 SIMD instructions: $SIMD_COUNT"
echo ""

if [ "$SIMD_COUNT" -gt 200 ]; then
    echo "✅ SUCCESS: WASM SIMD is working!"
    echo ""
    echo "Sample SIMD instructions:"
    grep "f32x4\|v128" pkg/output.wat | head -10
    echo ""
    echo "Expected performance: 2-4x speedup over scalar code"
else
    echo "❌ FAILED: No SIMD instructions found!"
    echo ""
    echo "Troubleshooting:"
    echo "1. Check .cargo/config.toml has: rustflags = [\"-C\", \"target-feature=+simd128\"]"
    echo "2. Rebuild: wasm-pack build --target web --release"
    echo "3. Verify wide crate supports WASM SIMD"
fi

echo ""
echo "=================================="
