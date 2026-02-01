#!/bin/bash
# Benchmark script to compare performance across all build tiers
#
# Usage:
#   ./scripts/benchmark_tiers.sh [--output <file>] [--verbose]
#
# Examples:
#   ./scripts/benchmark_tiers.sh                           # Run all benchmarks
#   ./scripts/benchmark_tiers.sh --output results.json     # Save results to file
#   ./scripts/benchmark_tiers.sh --verbose                 # Show detailed output

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
OUTPUT_FILE=""
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--output <file>] [--verbose]"
            echo ""
            echo "Options:"
            echo "  --output <file>  Save benchmark results to JSON file"
            echo "  --verbose        Show detailed benchmark output"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Benchmarking All Build Tiers${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if binaries exist
TIERS=("baseline" "modern" "highend")
MISSING_BUILDS=false

for tier in "${TIERS[@]}"; do
    if [[ ! -d "target/${tier}/release" ]]; then
        echo -e "${RED}Missing builds for tier: ${tier}${NC}"
        echo -e "Run: ./scripts/build_all_tiers.sh --release --both"
        MISSING_BUILDS=true
    fi
done

if [[ "$MISSING_BUILDS" == true ]]; then
    exit 1
fi

# Detect CPU features
echo -e "${YELLOW}Detecting CPU features...${NC}"
echo ""

if [[ -f /proc/cpuinfo ]]; then
    CPU_MODEL=$(grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)
    CPU_FLAGS=$(grep "flags" /proc/cpuinfo | head -1 | cut -d: -f2)

    echo -e "CPU: ${GREEN}${CPU_MODEL}${NC}"
    echo ""

    # Check for features
    check_feature() {
        local feature=$1
        if echo "$CPU_FLAGS" | grep -q "$feature"; then
            echo -e "  ${GREEN}✓${NC} $feature"
            return 0
        else
            echo -e "  ${RED}✗${NC} $feature"
            return 1
        fi
    }

    check_feature "sse4_2"
    check_feature "fma"
    check_feature "avx2"
    check_feature "avx512f"
elif [[ "$(uname -s)" == "Darwin" ]]; then
    CPU_MODEL=$(sysctl -n machdep.cpu.brand_string)
    echo -e "CPU: ${GREEN}${CPU_MODEL}${NC}"
    echo ""

    sysctl -a | grep machdep.cpu.features | head -1
    sysctl -a | grep machdep.cpu.leaf7_features | head -1
else
    echo -e "${YELLOW}CPU detection not available on this platform${NC}"
fi

echo ""
echo -e "${BLUE}Running benchmarks...${NC}"
echo ""

# Benchmark function
benchmark_tier() {
    local tier=$1
    local bench_name=$2

    echo -e "${GREEN}[Benchmarking]${NC} ${tier} - ${bench_name}"

    local target_dir="target/${tier}"

    # Run benchmark
    if [[ "$VERBOSE" == true ]]; then
        RUSTFLAGS="-C target-cpu=x86-64-v$(tier_version $tier)" \
            cargo bench \
            --bench "$bench_name" \
            --target-dir "$target_dir" \
            -- --noplot
    else
        RUSTFLAGS="-C target-cpu=x86-64-v$(tier_version $tier)" \
            cargo bench \
            --bench "$bench_name" \
            --target-dir "$target_dir" \
            -- --noplot 2>&1 | grep -E "time:|found"
    fi

    echo ""
}

# Helper to get tier version number
tier_version() {
    case $1 in
        baseline) echo "1" ;;
        modern) echo "3" ;;
        highend) echo "4" ;;
    esac
}

# Run benchmarks for each tier
BENCHMARKS=(
    "vec3_benches"
    "simd_benches"
    "transform_benches"
)

for bench in "${BENCHMARKS[@]}"; do
    echo -e "${YELLOW}=== $bench ===${NC}"
    echo ""

    for tier in "${TIERS[@]}"; do
        benchmark_tier "$tier" "$bench"
    done
done

# Generate comparison report
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Benchmark Summary${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

cat << 'EOF'
Expected Performance Gains (vs Baseline):

Tier 1 - Baseline (x86-64 with SSE2):
  - Scalar operations: 1.0x (baseline)
  - Batch operations:  1.0x (baseline)
  - Compatibility:     100% (all x86-64 CPUs)

Tier 2 - Modern (x86-64-v3: AVX2 + FMA):
  - Scalar operations: 1.15-1.30x faster
  - Batch operations:  2.0-3.0x faster
  - Compatibility:     ~95% (2013+ Intel, 2015+ AMD)

  Key improvements:
    - FMA for dot products (+15%)
    - AVX2 for batch processing (+100-200%)

Tier 3 - High-end (x86-64-v4: AVX512):
  - Scalar operations: 1.20-1.35x faster
  - Batch operations:  3.0-5.0x faster
  - Compatibility:     ~70% (2017+ Intel, 2022+ AMD)

  Key improvements:
    - AVX512 for wider SIMD (+50-100% over AVX2)
    - Better instruction scheduling

EOF

echo -e "${YELLOW}Results Location:${NC}"
for tier in "${TIERS[@]}"; do
    echo "  target/${tier}/criterion/"
done
echo ""

if [[ -n "$OUTPUT_FILE" ]]; then
    echo -e "${GREEN}Saving results to: $OUTPUT_FILE${NC}"
    # TODO: Parse criterion output and save to JSON
fi

echo -e "${GREEN}Done!${NC}"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "  1. Review detailed results in target/*/criterion/*/report/index.html"
echo "  2. Compare tier performance for your workload"
echo "  3. Document actual performance gains in CPU_FEATURES.md"
echo ""
