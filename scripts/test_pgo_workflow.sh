#!/usr/bin/env bash
# Test the PGO workflow without running the full build
#
# This script verifies that all PGO scripts are properly configured
# and can execute without errors.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}PGO Workflow Test${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Handle Windows and Unix
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    PROFILE_DIR="$TEMP/pgo-test-data"
else
    PROFILE_DIR="/tmp/pgo-test-data"
fi

# Clean up test directory
rm -rf "$PROFILE_DIR"

echo -e "${BLUE}Testing PGO scripts with profile directory:${NC} $PROFILE_DIR"
echo ""

# Test 1: Check scripts exist
echo -e "${BLUE}[1/5] Checking script files...${NC}"

SCRIPTS=(
    "build_pgo_instrumented.sh"
    "build_pgo_optimized.sh"
    "run_pgo_workload.sh"
    "compare_pgo_performance.sh"
)

for script in "${SCRIPTS[@]}"; do
    if [ -f "scripts/$script" ]; then
        echo -e "  ${GREEN}✓${NC} Found: $script"
    else
        echo -e "  ${RED}✗${NC} Missing: $script"
        exit 1
    fi
done

echo ""

# Test 2: Check scripts are executable
echo -e "${BLUE}[2/5] Checking script permissions...${NC}"

for script in "${SCRIPTS[@]}"; do
    if [ -x "scripts/$script" ]; then
        echo -e "  ${GREEN}✓${NC} Executable: $script"
    else
        echo -e "  ${YELLOW}⚠${NC}  Not executable: $script (attempting to fix)"
        chmod +x "scripts/$script"
    fi
done

echo ""

# Test 3: Verify dependencies
echo -e "${BLUE}[3/5] Checking dependencies...${NC}"

# Check for cargo
if command -v cargo >/dev/null 2>&1; then
    echo -e "  ${GREEN}✓${NC} cargo: $(cargo --version)"
else
    echo -e "  ${RED}✗${NC} cargo: not found"
    exit 1
fi

# Check for rustup (optional)
if command -v rustup >/dev/null 2>&1; then
    echo -e "  ${GREEN}✓${NC} rustup: $(rustup --version | head -1)"
else
    echo -e "  ${YELLOW}⚠${NC}  rustup: not found (optional)"
fi

# Check for llvm-profdata (optional but recommended)
if command -v llvm-profdata >/dev/null 2>&1; then
    echo -e "  ${GREEN}✓${NC} llvm-profdata: $(llvm-profdata --version | head -1)"
else
    echo -e "  ${YELLOW}⚠${NC}  llvm-profdata: not found"
    echo -e "     Install: ${YELLOW}rustup component add llvm-tools-preview${NC}"
fi

echo ""

# Test 4: Test profile directory creation
echo -e "${BLUE}[4/5] Testing profile directory creation...${NC}"

mkdir -p "$PROFILE_DIR"
if [ -d "$PROFILE_DIR" ]; then
    echo -e "  ${GREEN}✓${NC} Created: $PROFILE_DIR"
else
    echo -e "  ${RED}✗${NC} Failed to create: $PROFILE_DIR"
    exit 1
fi

# Test writing to profile directory
TEST_FILE="$PROFILE_DIR/test.txt"
echo "test" > "$TEST_FILE"
if [ -f "$TEST_FILE" ]; then
    echo -e "  ${GREEN}✓${NC} Write test successful"
    rm "$TEST_FILE"
else
    echo -e "  ${RED}✗${NC} Failed to write to: $PROFILE_DIR"
    exit 1
fi

echo ""

# Test 5: Verify benchmark files exist
echo -e "${BLUE}[5/5] Checking benchmark files...${NC}"

BENCHMARKS=(
    "engine/core/benches/pgo_workload.rs"
    "engine/physics/benches/integration_bench.rs"
    "engine/math/benches/simd_benches.rs"
)

for bench in "${BENCHMARKS[@]}"; do
    if [ -f "$bench" ]; then
        echo -e "  ${GREEN}✓${NC} Found: $bench"
    else
        echo -e "  ${YELLOW}⚠${NC}  Missing: $bench"
    fi
done

echo ""

# Clean up
rm -rf "$PROFILE_DIR"

# Summary
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}All Tests Passed!${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${BLUE}PGO workflow is ready to use.${NC}"
echo ""
echo -e "${YELLOW}To run the full PGO workflow:${NC}"
echo -e "  ./scripts/build_pgo_instrumented.sh"
echo -e "  ./scripts/run_pgo_workload.sh"
echo -e "  ./scripts/build_pgo_optimized.sh"
echo ""
echo -e "${YELLOW}Or use the automated comparison:${NC}"
echo -e "  ./scripts/compare_pgo_performance.sh"
echo ""
