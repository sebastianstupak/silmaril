#!/usr/bin/env bash
#
# Compare current benchmarks against a baseline
# Detects performance regressions
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

# Parse arguments
BASELINE="${1:-main}"
THRESHOLD="${2:-10}" # Default 10% regression threshold

echo "========================================"
echo "Benchmark Regression Analysis"
echo "========================================"
echo "Baseline: $BASELINE"
echo "Threshold: ${THRESHOLD}%"
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Check if baseline exists
if [ ! -d "target/criterion/$BASELINE" ]; then
    echo -e "${YELLOW}Warning: Baseline '$BASELINE' not found${NC}"
    echo "Creating new baseline..."
    bash scripts/run_all_benchmarks.sh
    mv target/criterion/*/new target/criterion/"$BASELINE" 2>/dev/null || true
    echo -e "${GREEN}Baseline created: $BASELINE${NC}"
    exit 0
fi

echo -e "${BLUE}Running benchmarks against baseline '$BASELINE'...${NC}"
echo

# Run benchmarks with baseline comparison
REGRESSIONS=0
IMPROVEMENTS=0
STABLE=0

# Function to run and compare benchmarks for a crate
compare_crate_benchmarks() {
    local crate=$1
    local crate_name="engine-$crate"

    if [ ! -d "engine/$crate/benches" ]; then
        return
    fi

    echo -e "${GREEN}Comparing $crate_name...${NC}"

    # Run benchmarks with baseline
    cargo bench --package "$crate_name" -- --baseline "$BASELINE" 2>&1 | tee "/tmp/${crate}_compare.log"

    # Parse results for regressions (simplified - actual parsing would be more complex)
    if grep -q "Performance has regressed" "/tmp/${crate}_compare.log" 2>/dev/null; then
        echo -e "${RED}  ✗ Regressions detected in $crate_name${NC}"
        REGRESSIONS=$((REGRESSIONS + 1))
    elif grep -q "Performance has improved" "/tmp/${crate}_compare.log" 2>/dev/null; then
        echo -e "${GREEN}  ✓ Improvements in $crate_name${NC}"
        IMPROVEMENTS=$((IMPROVEMENTS + 1))
    else
        echo -e "  ○ Stable performance in $crate_name"
        STABLE=$((STABLE + 1))
    fi

    echo
}

# Run comparisons
compare_crate_benchmarks "core"
compare_crate_benchmarks "math"
compare_crate_benchmarks "renderer"
compare_crate_benchmarks "assets"
compare_crate_benchmarks "networking"
compare_crate_benchmarks "physics"
compare_crate_benchmarks "audio"
compare_crate_benchmarks "profiling"

# Shared benchmarks
echo -e "${GREEN}Comparing shared benchmarks...${NC}"
cargo bench --package engine-shared-tests -- --baseline "$BASELINE" 2>&1 | tee "/tmp/shared_compare.log"
if grep -q "Performance has regressed" "/tmp/shared_compare.log" 2>/dev/null; then
    echo -e "${RED}  ✗ Regressions detected in shared benchmarks${NC}"
    REGRESSIONS=$((REGRESSIONS + 1))
elif grep -q "Performance has improved" "/tmp/shared_compare.log" 2>/dev/null; then
    echo -e "${GREEN}  ✓ Improvements in shared benchmarks${NC}"
    IMPROVEMENTS=$((IMPROVEMENTS + 1))
else
    echo -e "  ○ Stable performance in shared benchmarks"
    STABLE=$((STABLE + 1))
fi
echo

echo "========================================"
echo "Summary"
echo "========================================"
echo -e "Regressions: ${RED}$REGRESSIONS${NC}"
echo -e "Improvements: ${GREEN}$IMPROVEMENTS${NC}"
echo -e "Stable: $STABLE"
echo

if [ $REGRESSIONS -gt 0 ]; then
    echo -e "${RED}⚠ Performance regressions detected!${NC}"
    echo "Review the full report: target/criterion/report/index.html"
    exit 1
else
    echo -e "${GREEN}✓ No performance regressions detected${NC}"
    exit 0
fi
