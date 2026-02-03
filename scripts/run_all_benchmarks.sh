#!/usr/bin/env bash
#
# Run all benchmarks across all engine crates
# Generates comprehensive performance report
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

echo "========================================"
echo "Running All Engine Benchmarks"
echo "========================================"
echo

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Timestamp for this run
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_DIR="target/benchmark_results_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}Results will be saved to: $RESULTS_DIR${NC}"
echo

# Function to run benchmarks for a crate
run_crate_benchmarks() {
    local crate=$1
    local crate_name="engine-$crate"

    if [ ! -d "engine/$crate/benches" ]; then
        echo -e "${YELLOW}No benchmarks found for $crate_name${NC}"
        return
    fi

    echo -e "${GREEN}Running benchmarks for $crate_name...${NC}"

    # Run criterion benchmarks
    cargo bench --package "$crate_name" -- --save-baseline "$TIMESTAMP" 2>&1 | tee "$RESULTS_DIR/${crate}_bench.log"

    echo
}

# Core crates
echo "=== Core Systems ==="
run_crate_benchmarks "core"
run_crate_benchmarks "math"
run_crate_benchmarks "macros"

echo "=== Rendering & Assets ==="
run_crate_benchmarks "renderer"
run_crate_benchmarks "assets"

echo "=== Networking & Physics ==="
run_crate_benchmarks "networking"
run_crate_benchmarks "physics"
run_crate_benchmarks "interest"

echo "=== Systems ==="
run_crate_benchmarks "audio"
run_crate_benchmarks "auth"
run_crate_benchmarks "auto-update"
run_crate_benchmarks "observability"
run_crate_benchmarks "profiling"

echo "=== Cross-Crate Integration Benchmarks ==="
echo -e "${GREEN}Running shared benchmarks...${NC}"
cargo bench --package engine-shared-tests -- --save-baseline "$TIMESTAMP" 2>&1 | tee "$RESULTS_DIR/shared_bench.log"
echo

echo "========================================"
echo "Benchmark Summary"
echo "========================================"
echo

# Generate summary report
SUMMARY_FILE="$RESULTS_DIR/summary.txt"
echo "Benchmark Run: $TIMESTAMP" > "$SUMMARY_FILE"
echo "Timestamp: $(date)" >> "$SUMMARY_FILE"
echo "Git Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"
echo "Benchmark Logs:" >> "$SUMMARY_FILE"
ls -1 "$RESULTS_DIR"/*.log >> "$SUMMARY_FILE"

cat "$SUMMARY_FILE"

echo
echo -e "${GREEN}✓ All benchmarks complete!${NC}"
echo -e "${BLUE}Results saved to: $RESULTS_DIR${NC}"
echo -e "${BLUE}Criterion HTML report: target/criterion/report/index.html${NC}"
echo

# Open HTML report if on macOS or Linux with xdg-open
if command -v open &> /dev/null; then
    echo "Opening HTML report..."
    open target/criterion/report/index.html
elif command -v xdg-open &> /dev/null; then
    echo "Opening HTML report..."
    xdg-open target/criterion/report/index.html
fi
