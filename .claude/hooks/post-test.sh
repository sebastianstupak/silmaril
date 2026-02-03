#!/bin/bash
# Post-test hook for silmaril
# Updates test metrics and generates coverage reports

set -e  # Exit on any error

echo "📊 Running post-test analysis..."
echo ""

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create metrics directory if it doesn't exist
METRICS_DIR=".claude/metrics"
mkdir -p "$METRICS_DIR"

TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")
DATE=$(date +"%Y-%m-%d")

# ============================================================================
# 1. Collect test metrics
# ============================================================================

echo "🧪 Collecting test metrics..."

# Run tests with json output to capture detailed metrics
TEST_OUTPUT=$(cargo test --all-features -- --format json 2>&1 || true)

# Count total tests
TOTAL_TESTS=$(echo "$TEST_OUTPUT" | grep -c '"type":"test"' || echo 0)

# Count passed tests
PASSED_TESTS=$(echo "$TEST_OUTPUT" | grep '"event":"ok"' | wc -l || echo 0)

# Count failed tests
FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep '"event":"failed"' | wc -l || echo 0)

# Count ignored tests
IGNORED_TESTS=$(echo "$TEST_OUTPUT" | grep '"event":"ignored"' | wc -l || echo 0)

# Calculate pass rate
if [ "$TOTAL_TESTS" -gt 0 ]; then
    PASS_RATE=$(awk "BEGIN {printf \"%.2f\", ($PASSED_TESTS / $TOTAL_TESTS) * 100}")
else
    PASS_RATE="0.00"
fi

echo "  Total tests: $TOTAL_TESTS"
echo "  Passed: $PASSED_TESTS"
echo "  Failed: $FAILED_TESTS"
echo "  Ignored: $IGNORED_TESTS"
echo "  Pass rate: ${PASS_RATE}%"
echo ""

# ============================================================================
# 2. Generate coverage report (if tarpaulin is available)
# ============================================================================

echo "📈 Generating coverage report..."

if command -v cargo-tarpaulin &> /dev/null; then
    echo "  Running cargo-tarpaulin..."

    # Generate coverage report
    COVERAGE_OUTPUT=$(cargo tarpaulin \
        --all-features \
        --workspace \
        --timeout 120 \
        --out Xml \
        --out Html \
        --output-dir "$METRICS_DIR/coverage" \
        2>&1 || true)

    # Extract coverage percentage
    COVERAGE=$(echo "$COVERAGE_OUTPUT" | grep -oP '\d+\.\d+%' | head -1 || echo "0.00%")

    echo -e "${GREEN}  Coverage: $COVERAGE${NC}"
    echo "  Report: $METRICS_DIR/coverage/index.html"
else
    echo -e "${YELLOW}  cargo-tarpaulin not installed${NC}"
    echo "  Install with: cargo install cargo-tarpaulin"
    COVERAGE="N/A"
fi
echo ""

# ============================================================================
# 3. Collect benchmark data (if criterion results exist)
# ============================================================================

echo "⚡ Checking for benchmark results..."

if [ -d "target/criterion" ]; then
    BENCH_COUNT=$(find target/criterion -name "report" -type d | wc -l || echo 0)
    echo "  Found $BENCH_COUNT benchmark reports"
    echo "  View at: target/criterion/report/index.html"
else
    echo "  No benchmark results found"
    echo "  Run 'cargo bench' to generate benchmarks"
fi
echo ""

# ============================================================================
# 4. Update metrics file
# ============================================================================

echo "💾 Updating metrics file..."

# Append to daily metrics log
METRICS_FILE="$METRICS_DIR/test-metrics-$DATE.json"

cat >> "$METRICS_FILE" <<EOF
{
  "timestamp": "$TIMESTAMP",
  "tests": {
    "total": $TOTAL_TESTS,
    "passed": $PASSED_TESTS,
    "failed": $FAILED_TESTS,
    "ignored": $IGNORED_TESTS,
    "pass_rate": $PASS_RATE
  },
  "coverage": "$COVERAGE"
}
EOF

echo "  Metrics saved to: $METRICS_FILE"
echo ""

# ============================================================================
# 5. Update historical trends
# ============================================================================

echo "📉 Updating trends..."

TRENDS_FILE="$METRICS_DIR/trends.csv"

# Create header if file doesn't exist
if [ ! -f "$TRENDS_FILE" ]; then
    echo "date,timestamp,total_tests,passed,failed,pass_rate,coverage" > "$TRENDS_FILE"
fi

# Append current metrics
echo "$DATE,$TIMESTAMP,$TOTAL_TESTS,$PASSED_TESTS,$FAILED_TESTS,$PASS_RATE,$COVERAGE" >> "$TRENDS_FILE"

echo "  Trends updated: $TRENDS_FILE"
echo ""

# ============================================================================
# 6. Generate summary
# ============================================================================

echo "📋 Test Summary"
echo "========================================"
echo -e "Timestamp:     ${BLUE}$TIMESTAMP${NC}"
echo -e "Total Tests:   ${BLUE}$TOTAL_TESTS${NC}"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"

if [ "$FAILED_TESTS" -gt 0 ]; then
    echo -e "Failed:        ${RED}$FAILED_TESTS${NC}"
else
    echo -e "Failed:        ${GREEN}$FAILED_TESTS${NC}"
fi

echo -e "Pass Rate:     ${GREEN}${PASS_RATE}%${NC}"
echo -e "Coverage:      ${GREEN}$COVERAGE${NC}"
echo "========================================"
echo ""

# ============================================================================
# 7. Check against targets
# ============================================================================

echo "🎯 Checking against targets..."

# Target: > 80% coverage
TARGET_COVERAGE=80.0
COVERAGE_NUM=$(echo "$COVERAGE" | grep -oP '\d+\.\d+' || echo "0")

if command -v bc &> /dev/null && [ "$COVERAGE" != "N/A" ]; then
    if (( $(echo "$COVERAGE_NUM >= $TARGET_COVERAGE" | bc -l) )); then
        echo -e "${GREEN}✓ Coverage target met (${TARGET_COVERAGE}%)${NC}"
    else
        echo -e "${YELLOW}⚠ Coverage below target (${TARGET_COVERAGE}%)${NC}"
        echo "  Current: $COVERAGE"
        echo "  Target: ${TARGET_COVERAGE}%"
    fi
else
    echo "  Coverage target: ${TARGET_COVERAGE}% (current: $COVERAGE)"
fi

# Target: 100% pass rate
if [ "$FAILED_TESTS" -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passing${NC}"
else
    echo -e "${YELLOW}⚠ Some tests failing${NC}"
    echo "  Failed: $FAILED_TESTS"
fi
echo ""

# ============================================================================
# 8. Generate recommendations
# ============================================================================

echo "💡 Recommendations"
echo "========================================"

RECOMMENDATIONS=0

# Check test count
if [ "$TOTAL_TESTS" -lt 50 ]; then
    echo "• Add more tests (current: $TOTAL_TESTS, recommended: 50+)"
    RECOMMENDATIONS=$((RECOMMENDATIONS + 1))
fi

# Check coverage
if [ "$COVERAGE" != "N/A" ] && (( $(echo "$COVERAGE_NUM < $TARGET_COVERAGE" | bc -l) )); then
    echo "• Increase test coverage (current: $COVERAGE, target: ${TARGET_COVERAGE}%)"
    RECOMMENDATIONS=$((RECOMMENDATIONS + 1))
fi

# Check for failed tests
if [ "$FAILED_TESTS" -gt 0 ]; then
    echo "• Fix failing tests before committing"
    RECOMMENDATIONS=$((RECOMMENDATIONS + 1))
fi

# Check for ignored tests
if [ "$IGNORED_TESTS" -gt 5 ]; then
    echo "• Review ignored tests (current: $IGNORED_TESTS)"
    RECOMMENDATIONS=$((RECOMMENDATIONS + 1))
fi

if [ "$RECOMMENDATIONS" -eq 0 ]; then
    echo -e "${GREEN}No recommendations - looking good!${NC}"
fi

echo "========================================"
echo ""

# ============================================================================
# 9. Export for CI integration
# ============================================================================

# Export metrics for potential CI integration
export TEST_TOTAL="$TOTAL_TESTS"
export TEST_PASSED="$PASSED_TESTS"
export TEST_FAILED="$FAILED_TESTS"
export TEST_PASS_RATE="$PASS_RATE"
export TEST_COVERAGE="$COVERAGE"

echo -e "${GREEN}✓ Post-test analysis complete${NC}"
echo ""
echo "View detailed reports:"
echo "  Metrics: $METRICS_FILE"
echo "  Coverage: $METRICS_DIR/coverage/index.html (if available)"
echo "  Trends: $TRENDS_FILE"
echo ""

exit 0
