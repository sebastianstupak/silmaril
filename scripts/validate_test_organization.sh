#!/usr/bin/env bash
#
# Validate test organization according to TESTING_ARCHITECTURE.md
# Ensures cross-crate tests are in engine/shared/tests/
#

set -euo pipefail

VIOLATIONS=0
WARNINGS=0

echo "========================================"
echo "Test Organization Validation"
echo "========================================"
echo

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Check for cross-crate tests in wrong location
echo "Checking for cross-crate test violations..."
echo

# Define crates to check
CRATES=("audio" "networking" "renderer" "physics" "core" "assets" "interest" "auth" "auto-update" "observability" "profiling")

for crate in "${CRATES[@]}"; do
    TEST_DIR="engine/$crate/tests"
    if [ ! -d "$TEST_DIR" ]; then
        continue
    fi

    echo "Scanning $TEST_DIR..."

    # Find test files that import from other engine crates (excluding engine_math which is allowed)
    while IFS= read -r file; do
        # Skip if file doesn't exist or is a directory
        if [ ! -f "$file" ]; then
            continue
        fi

        # Check for cross-crate imports (excluding self-imports and engine-math)
        CROSS_IMPORTS=$(grep -E "^use engine_(core|renderer|physics|audio|assets|networking|interest)" "$file" | \
                       grep -v "use engine_$crate" || true)

        if [ -n "$CROSS_IMPORTS" ]; then
            echo -e "${RED}VIOLATION${NC}: Cross-crate test in wrong location"
            echo "  File: $file"
            echo "  Imports:"
            echo "$CROSS_IMPORTS" | sed 's/^/    /'
            echo "  Action: Move to engine/shared/tests/"
            echo
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    done < <(find "$TEST_DIR" -name "*.rs" -type f)
done

echo
echo "----------------------------------------"
echo "Checking for cross-crate benchmarks..."
echo

for crate in "${CRATES[@]}"; do
    BENCH_DIR="engine/$crate/benches"
    if [ ! -d "$BENCH_DIR" ]; then
        continue
    fi

    echo "Scanning $BENCH_DIR..."

    while IFS= read -r file; do
        if [ ! -f "$file" ]; then
            continue
        fi

        # Check for cross-crate imports
        CROSS_IMPORTS=$(grep -E "^use engine_(core|renderer|physics|audio|assets|networking|interest)" "$file" | \
                       grep -v "use engine_$crate" || true)

        if [ -n "$CROSS_IMPORTS" ]; then
            echo -e "${RED}VIOLATION${NC}: Cross-crate benchmark in wrong location"
            echo "  File: $file"
            echo "  Imports:"
            echo "$CROSS_IMPORTS" | sed 's/^/    /'
            echo "  Action: Move to engine/shared/benches/"
            echo
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    done < <(find "$BENCH_DIR" -name "*.rs" -type f)
done

echo
echo "----------------------------------------"
echo "Checking shared tests/benches..."
echo

# Verify shared tests exist
if [ ! -d "engine/shared/tests" ]; then
    echo -e "${YELLOW}WARNING${NC}: engine/shared/tests/ directory missing"
    WARNINGS=$((WARNINGS + 1))
fi

if [ ! -d "engine/shared/benches" ]; then
    echo -e "${YELLOW}WARNING${NC}: engine/shared/benches/ directory missing"
    WARNINGS=$((WARNINGS + 1))
fi

# Count shared tests
SHARED_TESTS=$(find engine/shared/tests -name "*.rs" -type f 2>/dev/null | wc -l || echo "0")
SHARED_BENCHES=$(find engine/shared/benches -name "*.rs" -type f 2>/dev/null | wc -l || echo "0")

echo "Shared integration tests: $SHARED_TESTS"
echo "Shared integration benches: $SHARED_BENCHES"
echo

echo "========================================"
echo "Summary"
echo "========================================"
echo

if [ $VIOLATIONS -eq 0 ]; then
    echo -e "${GREEN}✓${NC} All tests/benches correctly organized"
else
    echo -e "${RED}✗${NC} Found $VIOLATIONS test/benchmark organization violations"
fi

if [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠${NC} $WARNINGS warnings"
fi

echo
echo "Total violations: $VIOLATIONS"
echo "Total warnings: $WARNINGS"
echo

# Exit with error if violations found
if [ $VIOLATIONS -gt 0 ]; then
    exit 1
fi

exit 0
