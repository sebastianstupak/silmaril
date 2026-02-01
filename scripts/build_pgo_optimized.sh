#!/usr/bin/env bash
# Build optimized binary using Profile-Guided Optimization (PGO) data
#
# Usage:
#   ./scripts/build_pgo_optimized.sh [profile_dir]
#
# This script builds the final optimized release binary using profile data
# collected from running the instrumented binary.
#
# Prerequisites:
#   1. Run build_pgo_instrumented.sh
#   2. Run run_pgo_workload.sh to generate profile data

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
echo -e "${BLUE}Step 3/3: Build Optimized Binary${NC}"
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
    echo -e "${YELLOW}Run build_pgo_instrumented.sh and run_pgo_workload.sh first${NC}"
    exit 1
fi

# Check if any .profraw files exist
PROFRAW_COUNT=$(find "$PROFILE_DIR" -name "*.profraw" 2>/dev/null | wc -l)
if [ "$PROFRAW_COUNT" -eq 0 ]; then
    echo -e "${RED}Error: No .profraw files found in $PROFILE_DIR${NC}"
    echo -e "${YELLOW}Run run_pgo_workload.sh to generate profile data first${NC}"
    exit 1
fi

echo -e "${GREEN}Found $PROFRAW_COUNT profile data files${NC}"
echo ""

# Merge profile data
echo -e "${BLUE}Merging profile data...${NC}"
MERGED_PROFILE="$PROFILE_DIR/merged.profdata"

# Use llvm-profdata to merge all .profraw files
if command -v llvm-profdata >/dev/null 2>&1; then
    llvm-profdata merge -o "$MERGED_PROFILE" "$PROFILE_DIR"/*.profraw
    echo -e "${GREEN}Profile data merged successfully${NC}"
elif command -v rustup >/dev/null 2>&1; then
    # Try using rustup's llvm-tools
    LLVM_PROFDATA=$(rustup which --toolchain stable llvm-profdata 2>/dev/null || echo "")
    if [ -n "$LLVM_PROFDATA" ]; then
        "$LLVM_PROFDATA" merge -o "$MERGED_PROFILE" "$PROFILE_DIR"/*.profraw
        echo -e "${GREEN}Profile data merged successfully${NC}"
    else
        echo -e "${YELLOW}Warning: llvm-profdata not found${NC}"
        echo -e "${YELLOW}Install llvm-tools: rustup component add llvm-tools-preview${NC}"
        echo -e "${YELLOW}Attempting to use .profraw files directly (may not work)${NC}"
        MERGED_PROFILE="$PROFILE_DIR"
    fi
else
    echo -e "${YELLOW}Warning: Neither llvm-profdata nor rustup found${NC}"
    echo -e "${YELLOW}Attempting to use .profraw files directly (may not work)${NC}"
    MERGED_PROFILE="$PROFILE_DIR"
fi

echo ""

# Build with profile data
echo -e "${BLUE}Building PGO-optimized binaries...${NC}"
echo -e "${YELLOW}This uses profile data to optimize hot paths${NC}"
echo ""

# Export RUSTFLAGS for profile-use
export RUSTFLAGS="-C profile-use=$MERGED_PROFILE -C llvm-args=-pgo-warn-missing-function"

# Build all workspace members in release mode with PGO
cargo build --release --all-targets

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}PGO-Optimized Build Complete!${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${BLUE}Performance Gains:${NC}"
echo -e "  - Expected: ${GREEN}5-15% faster${NC} on typical workloads"
echo -e "  - Hot paths are optimized based on actual usage"
echo -e "  - Better branch prediction and code layout"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo -e "  1. Run benchmarks to measure improvement:"
echo -e "     ${YELLOW}cargo bench${NC}"
echo ""
echo -e "  2. Compare with non-PGO build:"
echo -e "     ${YELLOW}./scripts/compare_pgo_performance.sh${NC}"
echo ""
echo -e "${YELLOW}Note:${NC} PGO-optimized binaries are tuned for the workload"
echo -e "      used during profiling. For best results, use a"
echo -e "      representative workload that matches production usage."
echo ""
