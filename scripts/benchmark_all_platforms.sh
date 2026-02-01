#!/usr/bin/env bash
# benchmark_all_platforms.sh - Cross-platform benchmark automation (Linux/macOS)
#
# Usage:
#   ./scripts/benchmark_all_platforms.sh [OPTIONS]
#
# Options:
#   --baseline NAME     Save results as baseline with given name (default: current timestamp)
#   --compare NAME      Compare with named baseline
#   --output DIR        Output directory for results (default: benchmarks/results)
#   --quick            Run subset of benchmarks (faster)
#   --no-platform      Skip platform-specific benchmarks
#   --no-ecs           Skip ECS benchmarks
#   --verbose          Enable verbose output
#   --help             Show this help message

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BASELINE_NAME=""
COMPARE_NAME=""
OUTPUT_DIR="benchmarks/results"
RUN_PLATFORM=true
RUN_ECS=true
QUICK_MODE=false
VERBOSE=false

# Platform detection
OS_NAME=$(uname -s)
case "$OS_NAME" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)       echo -e "${RED}Unsupported platform: $OS_NAME${NC}"; exit 1 ;;
esac

echo -e "${BLUE}=== Cross-Platform Benchmark Suite ===${NC}"
echo -e "Platform: ${GREEN}$PLATFORM${NC}"
echo -e "Timestamp: ${GREEN}$TIMESTAMP${NC}"
echo ""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline)
            BASELINE_NAME="$2"
            shift 2
            ;;
        --compare)
            COMPARE_NAME="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --no-platform)
            RUN_PLATFORM=false
            shift
            ;;
        --no-ecs)
            RUN_ECS=false
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            head -n 14 "$0" | tail -n +2 | sed 's/^# //'
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"
RESULT_DIR="$OUTPUT_DIR/${PLATFORM}_${TIMESTAMP}"
mkdir -p "$RESULT_DIR"

echo -e "${BLUE}Results will be saved to: ${GREEN}$RESULT_DIR${NC}\n"

# Verbose mode helper
log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}[VERBOSE]${NC} $1"
    fi
}

# Run a benchmark suite
run_benchmark() {
    local name=$1
    local package=$2
    local bench=$3
    local output_file="$RESULT_DIR/${name}.json"

    echo -e "${YELLOW}Running benchmark: ${name}${NC}"
    log_verbose "Package: $package, Bench: $bench"

    if [ "$QUICK_MODE" = true ]; then
        # Quick mode: fewer samples
        cargo bench --package "$package" --bench "$bench" -- \
            --warm-up-time 1 --measurement-time 3 --sample-size 20 \
            --save-baseline "$name" 2>&1 | tee "$RESULT_DIR/${name}.log"
    else
        # Full mode: default Criterion settings
        cargo bench --package "$package" --bench "$bench" -- \
            --save-baseline "$name" 2>&1 | tee "$RESULT_DIR/${name}.log"
    fi

    echo -e "${GREEN}✓ Completed: ${name}${NC}\n"
}

# Platform-specific benchmarks
if [ "$RUN_PLATFORM" = true ]; then
    echo -e "${BLUE}=== Platform Abstraction Benchmarks ===${NC}\n"

    # Time backend benchmarks
    run_benchmark "platform_time" "engine-core" "platform_benches"

    # Threading benchmarks (if available)
    if cargo bench --package engine-core --bench platform_benches -- --list 2>/dev/null | grep -q "threading"; then
        log_verbose "Found threading benchmarks"
    fi
fi

# ECS benchmarks
if [ "$RUN_ECS" = true ]; then
    echo -e "${BLUE}=== ECS Benchmarks ===${NC}\n"

    # Core ECS operations
    run_benchmark "ecs_world" "engine-core" "world_benches"
    run_benchmark "ecs_query" "engine-core" "query_benches"
    run_benchmark "ecs_entity" "engine-core" "entity_benches"

    # Storage benchmarks
    run_benchmark "ecs_sparse_set" "engine-core" "sparse_set_benches"

    # Comprehensive ECS benchmarks
    if [ "$QUICK_MODE" = false ]; then
        run_benchmark "ecs_comprehensive" "engine-core" "ecs_comprehensive_benches"
    fi
fi

# Physics benchmarks
echo -e "${BLUE}=== Physics Benchmarks ===${NC}\n"
run_benchmark "physics_integration" "engine-physics" "integration_bench"

# Math/SIMD benchmarks
echo -e "${BLUE}=== Math/SIMD Benchmarks ===${NC}\n"
run_benchmark "math_simd" "engine-math" "simd_benches"
run_benchmark "math_transform" "engine-math" "transform_benches"

# Serialization benchmarks
echo -e "${BLUE}=== Serialization Benchmarks ===${NC}\n"
run_benchmark "serialization" "engine-core" "serialization_benches"

# Profiling overhead benchmarks
echo -e "${BLUE}=== Profiling Overhead Benchmarks ===${NC}\n"
run_benchmark "profiling_overhead" "engine-profiling" "profiling_overhead"

# Generate summary report
echo -e "\n${BLUE}=== Generating Summary Report ===${NC}\n"

SUMMARY_FILE="$RESULT_DIR/SUMMARY.md"
cat > "$SUMMARY_FILE" << EOF
# Benchmark Results Summary

**Platform:** $PLATFORM
**Date:** $(date)
**Mode:** $([ "$QUICK_MODE" = true ] && echo "Quick" || echo "Full")

---

## Benchmark Suites Run

EOF

# List all benchmarks run
if [ "$RUN_PLATFORM" = true ]; then
    echo "### Platform Abstraction" >> "$SUMMARY_FILE"
    echo "- Time Backend" >> "$SUMMARY_FILE"
    echo "" >> "$SUMMARY_FILE"
fi

if [ "$RUN_ECS" = true ]; then
    echo "### ECS" >> "$SUMMARY_FILE"
    echo "- World Operations" >> "$SUMMARY_FILE"
    echo "- Query System" >> "$SUMMARY_FILE"
    echo "- Entity Management" >> "$SUMMARY_FILE"
    echo "- Sparse Set Storage" >> "$SUMMARY_FILE"
    [ "$QUICK_MODE" = false ] && echo "- Comprehensive ECS" >> "$SUMMARY_FILE"
    echo "" >> "$SUMMARY_FILE"
fi

echo "### Physics" >> "$SUMMARY_FILE"
echo "- Integration System" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

echo "### Math/SIMD" >> "$SUMMARY_FILE"
echo "- SIMD Operations" >> "$SUMMARY_FILE"
echo "- Transform Operations" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

echo "### Serialization" >> "$SUMMARY_FILE"
echo "- Component Serialization" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

echo "### Profiling" >> "$SUMMARY_FILE"
echo "- Profiling Overhead" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

cat >> "$SUMMARY_FILE" << EOF
---

## Files Generated

- Benchmark logs: \`*.log\`
- Criterion output: \`target/criterion/\`
- Summary: \`SUMMARY.md\`

---

## Next Steps

1. **View detailed results:**
   \`\`\`bash
   open target/criterion/report/index.html
   \`\`\`

2. **Compare with baseline:**
   \`\`\`bash
   ./scripts/benchmark_all_platforms.sh --compare baseline_name
   \`\`\`

3. **Check for regressions:**
   \`\`\`bash
   python scripts/compare_with_industry.py --results $RESULT_DIR
   \`\`\`

EOF

echo -e "${GREEN}Summary report saved to: ${SUMMARY_FILE}${NC}"

# Save as baseline if requested
if [ -n "$BASELINE_NAME" ]; then
    echo -e "\n${BLUE}=== Saving Baseline ===${NC}\n"
    BASELINE_DIR="benchmarks/baselines/${PLATFORM}_${BASELINE_NAME}"
    mkdir -p "$BASELINE_DIR"

    # Copy Criterion baselines
    cp -r target/criterion "$BASELINE_DIR/"

    # Copy our results
    cp -r "$RESULT_DIR" "$BASELINE_DIR/results"

    # Save metadata
    cat > "$BASELINE_DIR/metadata.json" << EOF
{
  "platform": "$PLATFORM",
  "baseline_name": "$BASELINE_NAME",
  "timestamp": "$TIMESTAMP",
  "quick_mode": $QUICK_MODE,
  "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"
}
EOF

    echo -e "${GREEN}✓ Baseline saved to: ${BASELINE_DIR}${NC}"
fi

# Compare with baseline if requested
if [ -n "$COMPARE_NAME" ]; then
    echo -e "\n${BLUE}=== Comparing with Baseline ===${NC}\n"
    BASELINE_DIR="benchmarks/baselines/${PLATFORM}_${COMPARE_NAME}"

    if [ ! -d "$BASELINE_DIR" ]; then
        echo -e "${RED}Error: Baseline '${COMPARE_NAME}' not found for platform '${PLATFORM}'${NC}"
        echo -e "Available baselines:"
        ls -1 benchmarks/baselines/ | grep "^${PLATFORM}_" | sed "s/^${PLATFORM}_/  - /"
        exit 1
    fi

    echo -e "Baseline: ${GREEN}${COMPARE_NAME}${NC}"
    echo -e "Using Python comparison script...\n"

    python3 scripts/benchmark_regression_check.py \
        --baseline "$BASELINE_DIR/criterion" \
        --current "target/criterion" \
        --threshold 20 \
        --format criterion \
        --output "$RESULT_DIR/comparison.md"

    echo -e "\n${GREEN}✓ Comparison report saved to: ${RESULT_DIR}/comparison.md${NC}"
fi

# Final summary
echo -e "\n${GREEN}=== Benchmark Suite Complete ===${NC}\n"
echo -e "Results directory: ${BLUE}${RESULT_DIR}${NC}"
echo -e "HTML report: ${BLUE}target/criterion/report/index.html${NC}"
echo -e ""
echo -e "To view HTML report:"
if [ "$PLATFORM" = "macos" ]; then
    echo -e "  ${YELLOW}open target/criterion/report/index.html${NC}"
else
    echo -e "  ${YELLOW}xdg-open target/criterion/report/index.html${NC}"
fi
echo -e ""

exit 0
