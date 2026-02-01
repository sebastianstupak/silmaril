#!/usr/bin/env bash
# Run representative workload for Profile-Guided Optimization (PGO)
#
# Usage:
#   ./scripts/run_pgo_workload.sh [profile_dir]
#
# This script runs the instrumented binary through a representative workload
# to collect profiling data. The workload includes various scenarios:
# - Physics simulation with 1K, 10K, 100K entities
# - ECS queries with different access patterns
# - Rendering workloads
# - SIMD math operations
#
# Prerequisites:
#   Run build_pgo_instrumented.sh first

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default profile directory (handle Windows and Unix)
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    DEFAULT_PROFILE_DIR="$TEMP/pgo-data"
else
    DEFAULT_PROFILE_DIR="/tmp/pgo-data"
fi
PROFILE_DIR="${1:-$DEFAULT_PROFILE_DIR}"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Profile-Guided Optimization (PGO)${NC}"
echo -e "${BLUE}Step 2/3: Run Representative Workload${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from repository root${NC}"
    exit 1
fi

# Check if profile directory exists
if [ ! -d "$PROFILE_DIR" ]; then
    echo -e "${RED}Error: Profile directory not found: $PROFILE_DIR${NC}"
    echo -e "${YELLOW}Run build_pgo_instrumented.sh first${NC}"
    exit 1
fi

# Check if instrumented binary exists
if [ ! -f "target/release/cargo" ]; then
    echo -e "${YELLOW}Warning: Instrumented binaries may not exist${NC}"
    echo -e "${YELLOW}Make sure you ran build_pgo_instrumented.sh${NC}"
fi

echo -e "${BLUE}Running representative workload...${NC}"
echo -e "${YELLOW}This will take several minutes${NC}"
echo ""

# Set profile directory for runtime
export LLVM_PROFILE_FILE="$PROFILE_DIR/pgo-%p-%m.profraw"

# Track profiling status
WORKLOAD_COUNT=0
TOTAL_WORKLOADS=8

run_workload() {
    local name="$1"
    local command="$2"

    WORKLOAD_COUNT=$((WORKLOAD_COUNT + 1))
    echo -e "${BLUE}[$WORKLOAD_COUNT/$TOTAL_WORKLOADS] Running: $name${NC}"

    if eval "$command"; then
        echo -e "${GREEN}✓ Completed: $name${NC}"
    else
        echo -e "${YELLOW}⚠ Warning: $name failed (continuing anyway)${NC}"
    fi
    echo ""
}

# 1. ECS Core Benchmarks
run_workload "ECS World Operations" \
    "cargo bench --package engine-core --bench world_benches -- --sample-size 20"

# 2. ECS Query Benchmarks
run_workload "ECS Query System" \
    "cargo bench --package engine-core --bench query_benches -- --sample-size 20"

# 3. Physics Integration (1K entities)
run_workload "Physics: 1K Entities" \
    "cargo bench --package engine-physics --bench integration_bench -- 'integration.*1000' --sample-size 20"

# 4. Physics Integration (10K entities)
run_workload "Physics: 10K Entities" \
    "cargo bench --package engine-physics --bench integration_bench -- 'integration.*10000' --sample-size 20"

# 5. Physics Integration (100K entities)
run_workload "Physics: 100K Entities" \
    "cargo bench --package engine-physics --bench integration_bench -- 'integration.*100000' --sample-size 20"

# 6. SIMD Math Operations
run_workload "SIMD Math Operations" \
    "cargo bench --package engine-math --bench simd_benches -- --sample-size 20"

# 7. Vector Math Operations
run_workload "Vector Math Operations" \
    "cargo bench --package engine-math --bench vec3_benches -- --sample-size 20"

# 8. Transform Benchmarks
run_workload "Transform Operations" \
    "cargo bench --package engine-math --bench transform_benches -- --sample-size 20"

# Count generated profile files
PROFRAW_COUNT=$(find "$PROFILE_DIR" -name "*.profraw" 2>/dev/null | wc -l)

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}Workload Complete!${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${BLUE}Profile Data Summary:${NC}"
echo -e "  - Profile files generated: ${GREEN}$PROFRAW_COUNT${NC}"
echo -e "  - Profile directory: $PROFILE_DIR"
echo -e "  - Total workloads run: $WORKLOAD_COUNT/$TOTAL_WORKLOADS"
echo ""

if [ "$PROFRAW_COUNT" -eq 0 ]; then
    echo -e "${RED}Error: No profile data was generated!${NC}"
    echo -e "${YELLOW}Possible issues:${NC}"
    echo -e "  - Instrumented binary not built correctly"
    echo -e "  - LLVM_PROFILE_FILE environment variable not set"
    echo -e "  - Benchmarks failed to run"
    exit 1
fi

echo -e "${BLUE}Next Steps:${NC}"
echo ""
echo -e "Build the optimized binary with collected profile data:"
echo -e "  ${YELLOW}./scripts/build_pgo_optimized.sh${NC}"
echo ""
echo -e "${GREEN}Profile data is ready for optimization!${NC}"
echo ""
