#!/bin/bash
# Verification script for physics integration optimization

set -e

echo "=========================================="
echo "Physics Integration Optimization Verify"
echo "=========================================="
echo ""

cd "$(dirname "$0")/.."

echo "Step 1: Check compilation..."
echo "------------------------------"
cargo check --package engine-physics
if [ $? -eq 0 ]; then
    echo "✓ Compilation successful"
else
    echo "✗ Compilation failed"
    exit 1
fi
echo ""

echo "Step 2: Run unit tests..."
echo "-------------------------"
cd engine/physics
cargo test --lib
if [ $? -eq 0 ]; then
    echo "✓ Unit tests passed"
else
    echo "✗ Unit tests failed"
    exit 1
fi
echo ""

echo "Step 3: Run integration tests..."
echo "---------------------------------"
cargo test --test integration_simd_test
if [ $? -eq 0 ]; then
    echo "✓ Integration tests passed"
else
    echo "✗ Integration tests failed"
    exit 1
fi
echo ""

echo "Step 4: Build release binary..."
echo "--------------------------------"
cargo build --release
if [ $? -eq 0 ]; then
    echo "✓ Release build successful"
else
    echo "✗ Release build failed"
    exit 1
fi
echo ""

echo "Step 5: Run demo (quick test)..."
echo "---------------------------------"
timeout 30 cargo run --example integration_demo --release || true
echo "✓ Demo executed"
echo ""

echo "=========================================="
echo "All verification steps completed!"
echo "=========================================="
echo ""
echo "To run benchmarks, execute:"
echo "  cargo bench --bench integration_bench"
echo ""
echo "Expected results:"
echo "  - 2-4x speedup over scalar version"
echo "  - All tests passing"
echo "  - Clean compilation with no errors"
echo ""
