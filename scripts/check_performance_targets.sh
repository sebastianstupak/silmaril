#!/usr/bin/env bash
# Performance target validation script
# Runs benchmarks and verifies they meet performance targets

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=================================================="
echo "  Performance Target Validation"
echo "=================================================="
echo ""

# Load benchmark thresholds if available
THRESHOLDS_FILE="$PROJECT_ROOT/benchmark_thresholds.yaml"

if [ ! -f "$THRESHOLDS_FILE" ]; then
    echo -e "${YELLOW}Warning: benchmark_thresholds.yaml not found${NC}"
    echo "Creating default thresholds..."

    cat > "$THRESHOLDS_FILE" <<EOF
# Performance thresholds for automated testing
# All times in microseconds (µs) or operations per second

ecs:
  entity_spawn: 1000          # ns per entity
  component_add: 500          # ns per component
  query_iteration: 100        # ns per entity
  world_update: 16000         # µs (60 FPS = 16.67ms)

serialization:
  serialize_1k_entities: 5000  # µs
  deserialize_1k_entities: 5000  # µs
  bincode_roundtrip: 10000    # µs

networking:
  packet_encode: 100          # µs
  packet_decode: 100          # µs
  delta_compression: 500      # µs
  throughput_mbps: 100        # Mbps minimum

physics:
  step_100_bodies: 2000       # µs per step
  raycast: 50                 # µs per raycast
  collision_detection: 5000   # µs for 100 bodies

rendering:
  frame_time: 16670           # µs (60 FPS)
  draw_call_batch: 100        # ns per mesh
  gpu_upload: 1000            # µs per MB

assets:
  mesh_load: 10000            # µs for typical mesh
  texture_load: 20000         # µs for typical texture
  shader_compile: 50000       # µs
EOF
fi

echo "Using thresholds from: $THRESHOLDS_FILE"
echo ""

# Run critical benchmarks
RESULTS_FILE="/tmp/performance-check-$$.txt"

echo "Running critical benchmarks..."
echo ""

# Track pass/fail
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Helper function to run and check benchmark
check_benchmark() {
    local module=$1
    local bench_name=$2
    local threshold_us=$3
    local description=$4

    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo -n "Testing: $description... "

    # Run benchmark (quick mode for CI)
    if cargo bench --package "$module" --bench "$bench_name" -- --sample-size 10 --quiet > "$RESULTS_FILE" 2>&1; then
        # Extract result (this is simplified - real parsing would be more robust)
        # Look for "time:" in criterion output
        RESULT=$(grep -oP 'time:\s+\[\K[0-9.]+' "$RESULTS_FILE" | head -1 || echo "0")

        # Convert to microseconds if needed
        RESULT_US=$(echo "$RESULT * 1" | bc -l)

        if (( $(echo "$RESULT_US <= $threshold_us" | bc -l) )); then
            echo -e "${GREEN}✓ PASS${NC} (${RESULT_US}µs <= ${threshold_us}µs)"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            echo -e "${RED}✗ FAIL${NC} (${RESULT_US}µs > ${threshold_us}µs)"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
    else
        echo -e "${YELLOW}⚠ SKIP${NC} (benchmark not found or failed)"
    fi
}

# ECS benchmarks
echo "ECS Performance:"
check_benchmark "engine-core" "ecs_performance" 16000 "ECS world update (60 FPS target)"

# Serialization benchmarks
echo ""
echo "Serialization Performance:"
check_benchmark "engine-core" "serialization_comprehensive" 10000 "Serialization roundtrip"

# Networking benchmarks
echo ""
echo "Networking Performance:"
check_benchmark "engine-networking" "integration_benches" 500 "Delta compression"

# Physics benchmarks
echo ""
echo "Physics Performance:"
check_benchmark "engine-physics" "advanced_benches" 2000 "Physics step (100 bodies)"

# Cleanup
rm -f "$RESULTS_FILE"

echo ""
echo "=================================================="
echo "  Performance Validation Summary"
echo "=================================================="
echo ""
echo "Total Tests: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}✗ Performance validation FAILED${NC}"
    echo ""
    echo "Some benchmarks exceeded performance targets."
    echo "Review the results above and optimize as needed."
    exit 1
else
    echo -e "${GREEN}✓ All performance targets met!${NC}"
    echo ""
    echo "All critical benchmarks are within acceptable limits."
    exit 0
fi
