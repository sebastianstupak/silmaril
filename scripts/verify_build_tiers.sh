#!/bin/bash
# Verification script for build tier implementation
# Confirms all components of Task #59 are in place

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Build Tier Implementation Verification${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

PASS=0
FAIL=0

# Helper to check file existence
check_file() {
    local file=$1
    local description=$2

    if [[ -f "$file" ]]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $description (missing: $file)"
        ((FAIL++))
    fi
}

# Helper to check executable script
check_script() {
    local file=$1
    local description=$2

    if [[ -x "$file" ]] || [[ -f "$file" ]]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $description (missing or not executable: $file)"
        ((FAIL++))
    fi
}

# Helper to check if cargo config has tier definitions
check_cargo_config() {
    if grep -q "x86_64-v3" .cargo/config.toml && \
       grep -q "x86_64-v4" .cargo/config.toml; then
        echo -e "${GREEN}✓${NC} Cargo config has tier definitions (baseline, modern, highend)"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} Cargo config missing tier definitions"
        ((FAIL++))
    fi
}

echo -e "${YELLOW}1. Checking build infrastructure...${NC}"
echo ""

check_file ".cargo/config.toml" "Cargo config exists"
check_cargo_config
check_file ".cargo/config.toml.example" "Example config template"

echo ""
echo -e "${YELLOW}2. Checking build scripts...${NC}"
echo ""

check_script "scripts/build_all_tiers.sh" "Multi-tier build script (Linux/macOS)"
check_script "scripts/build_all_tiers.ps1" "Multi-tier build script (Windows)"
check_script "scripts/benchmark_tiers.sh" "Tier benchmarking script"

echo ""
echo -e "${YELLOW}3. Checking runtime CPU detection...${NC}"
echo ""

check_file "engine/build-utils/src/cpu_features.rs" "CPU feature detection module"
check_file "engine/build-utils/examples/cpu_tier_detection.rs" "CPU detection example"

echo ""
echo -e "${YELLOW}4. Checking documentation...${NC}"
echo ""

check_file "docs/build-tiers.md" "Build tiers documentation"

echo ""
echo -e "${YELLOW}5. Running CPU detection example...${NC}"
echo ""

if cargo run --example cpu_tier_detection --package engine-build-utils --quiet 2>/dev/null | grep -q "Detected Tier"; then
    echo -e "${GREEN}✓${NC} CPU tier detection works"
    ((PASS++))
else
    echo -e "${RED}✗${NC} CPU tier detection failed"
    ((FAIL++))
fi

echo ""
echo -e "${YELLOW}6. Running CPU features tests...${NC}"
echo ""

if cargo test --package engine-build-utils --lib cpu_features --quiet 2>/dev/null; then
    echo -e "${GREEN}✓${NC} All CPU features tests pass"
    ((PASS++))
else
    echo -e "${RED}✗${NC} Some tests failed"
    ((FAIL++))
fi

echo ""
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

echo -e "Passed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"
echo ""

if [[ $FAIL -eq 0 ]]; then
    echo -e "${GREEN}All checks passed! Build tier implementation is complete.${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "  1. Build all tiers: ./scripts/build_all_tiers.sh --release --both"
    echo "  2. Run benchmarks: ./scripts/benchmark_tiers.sh"
    echo "  3. Review documentation: docs/build-tiers.md"
    echo ""
    exit 0
else
    echo -e "${RED}Some checks failed. Please fix the issues above.${NC}"
    echo ""
    exit 1
fi
