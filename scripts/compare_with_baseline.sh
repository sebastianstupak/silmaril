#!/bin/bash
# Compare current benchmarks with saved baseline
#
# Usage:
#   ./scripts/compare_with_baseline.sh [baseline_name] [platform] [threshold]
#
# Arguments:
#   baseline_name: Name of baseline to compare against (default: main)
#   platform: Platform identifier (default: auto-detect)
#   threshold: Regression threshold percentage (default: 20)
#
# Examples:
#   ./scripts/compare_with_baseline.sh main
#   ./scripts/compare_with_baseline.sh develop linux-x64 15

set -e

# Configuration
BASELINE_NAME=${1:-main}
PLATFORM=${2:-$(uname -s)-$(uname -m)}
THRESHOLD=${3:-20}
BASELINE_DIR="benchmarks/baselines/${PLATFORM}/${BASELINE_NAME}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Comparing Benchmarks with Baseline"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Baseline: ${BASELINE_NAME}"
echo "Platform: ${PLATFORM}"
echo "Threshold: ${THRESHOLD}%"
echo ""

# Ensure we're in the repository root
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must be run from repository root"
    exit 1
fi

# Check if baseline exists
if [ ! -d "${BASELINE_DIR}/criterion" ]; then
    echo "❌ Error: Baseline not found at ${BASELINE_DIR}"
    echo ""
    echo "Available baselines:"
    find benchmarks/baselines -type d -name "criterion" | sed 's|/criterion||' | sed 's|benchmarks/baselines/||'
    echo ""
    echo "Create baseline with:"
    echo "  ./scripts/update_benchmark_baseline.sh ${BASELINE_NAME} ${PLATFORM}"
    exit 1
fi

# Show baseline info
if [ -f "${BASELINE_DIR}/baseline-info.json" ]; then
    echo "Baseline information:"
    cat "${BASELINE_DIR}/baseline-info.json" | head -20
    echo ""
fi

# Step 1: Copy baseline to target directory
echo "Step 1/3: Copying baseline to target directory..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
mkdir -p target
rm -rf target/criterion-baseline
cp -r "${BASELINE_DIR}/criterion" target/criterion-baseline
echo "✅ Baseline copied"
echo ""

# Step 2: Run benchmarks
echo "Step 2/3: Running benchmarks..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo bench --all-features -- --save-baseline current

# Step 3: Run regression checker
echo ""
echo "Step 3/3: Checking for regressions..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if Python is available
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    echo "⚠️  Python not found, skipping automated regression check"
    echo "ℹ️  Review benchmark output above manually"
    exit 0
fi

PYTHON_CMD=$(command -v python3 || command -v python)

# Install dependencies if needed
$PYTHON_CMD -m pip install --quiet pyyaml 2>/dev/null || true

# Run regression checker
echo ""
if $PYTHON_CMD scripts/check_benchmark_regression.py \
    --baseline target/criterion-baseline \
    --current target/criterion \
    --threshold "${THRESHOLD}" \
    --format criterion \
    --fail-on-regression; then

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✅ No Performance Regressions Detected!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "All benchmarks are within ${THRESHOLD}% of baseline."
    echo ""
    exit 0
else
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "❌ Performance Regressions Detected!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Review the regressions above and optimize before merging."
    echo ""
    echo "Next steps:"
    echo "1. Identify the slow benchmarks"
    echo "2. Profile with: cargo bench --features profiling"
    echo "3. Optimize the hot paths"
    echo "4. Re-run comparison: ./scripts/compare_with_baseline.sh"
    echo ""
    exit 1
fi
