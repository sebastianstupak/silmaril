#!/usr/bin/env bash
# Compare PGO-optimized vs regular release build performance
#
# Usage:
#   ./scripts/compare_pgo_performance.sh
#
# This script builds both regular and PGO-optimized binaries,
# runs benchmarks on both, and compares the results.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}PGO Performance Comparison${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from repository root${NC}"
    exit 1
fi

# Handle Windows and Unix
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    PROFILE_DIR="$TEMP/pgo-data"
    BASELINE_DIR="$TEMP/pgo-baseline"
else
    PROFILE_DIR="/tmp/pgo-data"
    BASELINE_DIR="/tmp/pgo-baseline"
fi

# Step 1: Build and benchmark baseline (no PGO)
echo -e "${BLUE}Step 1/5: Building baseline release binary (no PGO)${NC}"
echo ""

# Clear RUSTFLAGS to ensure clean build
unset RUSTFLAGS
cargo clean
cargo build --release --all-targets

echo ""
echo -e "${BLUE}Step 2/5: Running baseline benchmarks${NC}"
echo ""

# Run benchmarks and save baseline
cargo bench -- --save-baseline no-pgo

# Step 2: Build PGO instrumented
echo ""
echo -e "${BLUE}Step 3/5: Building PGO workflow${NC}"
echo ""

./scripts/build_pgo_instrumented.sh "$PROFILE_DIR"

# Step 3: Run workload
echo ""
echo -e "${BLUE}Step 4/5: Collecting profile data${NC}"
echo ""

./scripts/run_pgo_workload.sh "$PROFILE_DIR"

# Step 4: Build PGO optimized
echo ""
echo -e "${BLUE}Step 5/5: Building PGO-optimized binary${NC}"
echo ""

./scripts/build_pgo_optimized.sh "$PROFILE_DIR"

# Step 5: Run benchmarks again and compare
echo ""
echo -e "${BLUE}Running PGO-optimized benchmarks and comparing${NC}"
echo ""

cargo bench -- --baseline no-pgo

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}Performance Comparison Complete!${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${BLUE}Results:${NC}"
echo -e "  - Baseline results: target/criterion (no-pgo baseline)"
echo -e "  - PGO results: target/criterion (compared against baseline)"
echo ""
echo -e "${YELLOW}Look for:${NC}"
echo -e "  - ${GREEN}Improvements${NC} in hot paths (physics, ECS queries)"
echo -e "  - Expected gain: ${GREEN}5-15%${NC} on typical workloads"
echo -e "  - Check Criterion output above for specific benchmark comparisons"
echo ""
echo -e "${BLUE}Detailed reports:${NC}"
echo -e "  HTML reports: ${YELLOW}target/criterion/report/index.html${NC}"
echo ""
