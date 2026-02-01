#!/usr/bin/env bash
# Build instrumented binary for Profile-Guided Optimization (PGO)
#
# Usage:
#   ./scripts/build_pgo_instrumented.sh [profile_dir]
#
# This script builds a release binary instrumented to collect profiling data.
# After building, run the binary through representative workloads to generate
# profile data, which can then be used with build_pgo_optimized.sh for a
# final optimized build.

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
echo -e "${BLUE}Step 1/3: Build Instrumented Binary${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from repository root${NC}"
    exit 1
fi

# Clean up old profile data
if [ -d "$PROFILE_DIR" ]; then
    echo -e "${YELLOW}Cleaning old profile data in $PROFILE_DIR${NC}"
    rm -rf "$PROFILE_DIR"
fi

# Create profile directory
mkdir -p "$PROFILE_DIR"
echo -e "${GREEN}Created profile directory: $PROFILE_DIR${NC}"
echo ""

# Build with instrumentation
echo -e "${BLUE}Building instrumented binaries...${NC}"
echo -e "${YELLOW}This will be slower than a regular build${NC}"
echo ""

# Export RUSTFLAGS for profile generation
export RUSTFLAGS="-C profile-generate=$PROFILE_DIR"

# Build all workspace members in release mode with instrumentation
cargo build --release --all-targets

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}Instrumented Build Complete!${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo ""
echo -e "1. Run representative workload to generate profile data:"
echo -e "   ${YELLOW}./scripts/run_pgo_workload.sh${NC}"
echo ""
echo -e "2. Build optimized binary with collected profile:"
echo -e "   ${YELLOW}./scripts/build_pgo_optimized.sh${NC}"
echo ""
echo -e "${BLUE}Profile Directory:${NC} $PROFILE_DIR"
echo ""
echo -e "${YELLOW}Note:${NC} The instrumented binaries in target/release/ are slower than"
echo -e "      normal release builds. Do NOT use them for production!"
echo ""
