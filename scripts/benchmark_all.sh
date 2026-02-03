#!/usr/bin/env bash
# Comprehensive benchmark suite for agent-game-engine
# Runs all benchmarks and generates performance reports

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "=================================================="
echo "  Agent Game Engine - Comprehensive Benchmarks"
echo "=================================================="
echo ""

# Parse arguments
SAVE_BASELINE=false
COMPARE_BASELINE=false
BASELINE_NAME="baseline"
QUICK_MODE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --save-baseline)
            SAVE_BASELINE=true
            shift
            ;;
        --compare-baseline)
            COMPARE_BASELINE=true
            shift
            ;;
        --baseline-name)
            BASELINE_NAME="$2"
            shift 2
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--save-baseline] [--compare-baseline] [--baseline-name NAME] [--quick]"
            exit 1
            ;;
    esac
done

# Create benchmark results directory
RESULTS_DIR="$PROJECT_ROOT/benchmark-results"
mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULT_FILE="$RESULTS_DIR/benchmark-$TIMESTAMP.txt"

# Benchmark configuration
if [ "$QUICK_MODE" = true ]; then
    BENCH_ARGS="--sample-size 10"
    echo -e "${YELLOW}Quick mode: Using reduced sample size${NC}"
else
    BENCH_ARGS=""
fi

# List of benchmark modules
BENCH_MODULES=(
    "engine/core"
    "engine/math"
    "engine/assets"
    "engine/renderer"
    "engine/networking"
    "engine/physics"
    "engine/audio"
    "engine/interest"
    "engine/auth"
    "engine/auto-update"
)

echo "Starting benchmark suite at $(date)"
echo "Results will be saved to: $RESULT_FILE"
echo ""

# Initialize result file
{
    echo "Agent Game Engine - Benchmark Results"
    echo "======================================"
    echo "Date: $(date)"
    echo "System: $(uname -a)"
    echo "Rust Version: $(rustc --version)"
    echo "Cargo Version: $(cargo --version)"
    echo ""
    echo "======================================"
    echo ""
} > "$RESULT_FILE"

# Run benchmarks for each module
total_modules=${#BENCH_MODULES[@]}
current=0

for module in "${BENCH_MODULES[@]}"; do
    current=$((current + 1))

    if [ ! -d "$PROJECT_ROOT/$module" ]; then
        echo -e "${YELLOW}[$current/$total_modules] Skipping $module (not found)${NC}"
        continue
    fi

    # Check if module has benchmarks
    if [ ! -d "$PROJECT_ROOT/$module/benches" ]; then
        echo -e "${YELLOW}[$current/$total_modules] Skipping $module (no benchmarks)${NC}"
        continue
    fi

    echo -e "${BLUE}[$current/$total_modules] Running benchmarks for $module${NC}"

    {
        echo ""
        echo "=================================================="
        echo "Module: $module"
        echo "=================================================="
        echo ""
    } >> "$RESULT_FILE"

    # Run cargo bench for this module
    if cargo bench --package "$(basename $module)" $BENCH_ARGS 2>&1 | tee -a "$RESULT_FILE"; then
        echo -e "${GREEN}✓ Completed $module${NC}"
    else
        echo -e "${RED}✗ Failed $module${NC}"
    fi

    echo ""
done

# Run workspace-wide benchmarks if they exist
if [ -d "$PROJECT_ROOT/benches" ]; then
    echo -e "${BLUE}Running workspace-wide benchmarks${NC}"

    {
        echo ""
        echo "=================================================="
        echo "Workspace Benchmarks"
        echo "=================================================="
        echo ""
    } >> "$RESULT_FILE"

    cargo bench --workspace $BENCH_ARGS 2>&1 | tee -a "$RESULT_FILE"
fi

echo ""
echo "=================================================="
echo "  Benchmark Suite Complete"
echo "=================================================="
echo ""
echo "Results saved to: $RESULT_FILE"

# Save baseline if requested
if [ "$SAVE_BASELINE" = true ]; then
    BASELINE_FILE="$RESULTS_DIR/baseline-$BASELINE_NAME.txt"
    cp "$RESULT_FILE" "$BASELINE_FILE"
    echo -e "${GREEN}Baseline saved to: $BASELINE_FILE${NC}"
fi

# Compare with baseline if requested
if [ "$COMPARE_BASELINE" = true ]; then
    BASELINE_FILE="$RESULTS_DIR/baseline-$BASELINE_NAME.txt"
    if [ -f "$BASELINE_FILE" ]; then
        echo ""
        echo "=================================================="
        echo "  Baseline Comparison"
        echo "=================================================="
        echo ""
        echo "Comparing with baseline: $BASELINE_FILE"
        echo ""
        echo "Note: Detailed comparison requires criterion's built-in comparison"
        echo "Re-run benchmarks with --save-baseline to establish new baseline"
    else
        echo -e "${YELLOW}No baseline found at: $BASELINE_FILE${NC}"
        echo "Run with --save-baseline to create one"
    fi
fi

echo ""
echo "To analyze results:"
echo "  cat $RESULT_FILE"
echo "  # or"
echo "  less $RESULT_FILE"
echo ""
