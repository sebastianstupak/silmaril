#!/usr/bin/env bash
# Coverage analysis script for agent-game-engine
# Uses cargo-llvm-cov for accurate coverage reporting

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=================================================="
echo "  Agent Game Engine - Coverage Analysis"
echo "=================================================="
echo ""

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${YELLOW}cargo-llvm-cov not found. Installing...${NC}"
    cargo install cargo-llvm-cov
fi

# Clean previous coverage data
echo "Cleaning previous coverage data..."
cargo llvm-cov clean --workspace

# Run tests with coverage
echo ""
echo "Running tests with coverage instrumentation..."
echo "This may take a few minutes..."
echo ""

# Run coverage for the entire workspace
cargo llvm-cov \
    --workspace \
    --all-features \
    --lcov \
    --output-path coverage.lcov \
    --ignore-filename-regex '(tests?|benches?|examples?)/.*\.rs$' \
    -- --test-threads=1

# Generate HTML report
echo ""
echo "Generating HTML coverage report..."
cargo llvm-cov report --html --output-dir coverage-html

# Generate summary
echo ""
echo "=================================================="
echo "  Coverage Summary"
echo "=================================================="
cargo llvm-cov report --summary-only

# Generate detailed per-module report
echo ""
echo "=================================================="
echo "  Per-Module Coverage"
echo "=================================================="
cargo llvm-cov report | grep -E "^(engine|Filename)" | head -50

# Check coverage targets
echo ""
echo "=================================================="
echo "  Coverage Target Validation"
echo "=================================================="

# Extract overall coverage percentage
OVERALL_COVERAGE=$(cargo llvm-cov report --summary-only | grep -oP 'TOTAL.*\K[0-9]+\.[0-9]+(?=%)')

echo "Overall Coverage: ${OVERALL_COVERAGE}%"
echo "Target: 80%"

if (( $(echo "$OVERALL_COVERAGE >= 80.0" | bc -l) )); then
    echo -e "${GREEN}✓ Overall coverage target met!${NC}"
else
    echo -e "${YELLOW}⚠ Overall coverage below target${NC}"
fi

# Per-module targets
declare -A MODULE_TARGETS=(
    ["engine/core"]=85
    ["engine/renderer"]=80
    ["engine/assets"]=85
    ["engine/networking"]=80
    ["engine/physics"]=80
)

echo ""
echo "Module-specific targets:"
for module in "${!MODULE_TARGETS[@]}"; do
    target=${MODULE_TARGETS[$module]}
    # Extract module coverage (simplified - would need better parsing)
    echo "  $module: target ${target}%"
done

echo ""
echo "=================================================="
echo "  Coverage Report Generated"
echo "=================================================="
echo ""
echo "Reports available at:"
echo "  - LCOV: coverage.lcov"
echo "  - HTML: coverage-html/index.html"
echo ""
echo "To view HTML report:"
echo "  firefox coverage-html/index.html"
echo "  # or"
echo "  chrome coverage-html/index.html"
echo ""
