#!/bin/bash
# Build script for all platform-specific tiers
# Creates optimized binaries for baseline, modern, and high-end CPUs
#
# Usage:
#   ./scripts/build_all_tiers.sh [--release] [--client] [--server] [--both]
#
# Examples:
#   ./scripts/build_all_tiers.sh --release --both    # Build all tiers for client and server
#   ./scripts/build_all_tiers.sh --client            # Build client only (debug mode)
#   ./scripts/build_all_tiers.sh --release --server  # Build server only (release mode)
#
# Output:
#   target/x86_64-baseline/[debug|release]/client
#   target/x86_64-v3-modern/[debug|release]/client
#   target/x86_64-v4-highend/[debug|release]/client
#   (and corresponding server binaries)

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
MODE="debug"
BUILD_CLIENT=false
BUILD_SERVER=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            MODE="release"
            shift
            ;;
        --client)
            BUILD_CLIENT=true
            shift
            ;;
        --server)
            BUILD_SERVER=true
            shift
            ;;
        --both)
            BUILD_CLIENT=true
            BUILD_SERVER=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--release] [--client] [--server] [--both]"
            echo ""
            echo "Options:"
            echo "  --release    Build in release mode (default: debug)"
            echo "  --client     Build client binary"
            echo "  --server     Build server binary"
            echo "  --both       Build both client and server"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

# Default to building both if neither specified
if [[ "$BUILD_CLIENT" == false && "$BUILD_SERVER" == false ]]; then
    BUILD_CLIENT=true
    BUILD_SERVER=true
fi

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Building Multi-Tier Binaries${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "Mode: ${YELLOW}$MODE${NC}"
echo -e "Client: ${YELLOW}$BUILD_CLIENT${NC}"
echo -e "Server: ${YELLOW}$BUILD_SERVER${NC}"
echo ""

# Determine OS and select appropriate target triple suffix
case "$(uname -s)" in
    Linux*)
        TARGET_SUFFIX="unknown-linux-gnu"
        ;;
    Darwin*)
        TARGET_SUFFIX="apple-darwin"
        ;;
    CYGWIN*|MINGW*|MSYS*)
        TARGET_SUFFIX="pc-windows-msvc"
        ;;
    *)
        echo -e "${RED}Unsupported OS: $(uname -s)${NC}"
        exit 1
        ;;
esac

# Build flags
RELEASE_FLAG=""
if [[ "$MODE" == "release" ]]; then
    RELEASE_FLAG="--release"
fi

# Tier definitions
declare -A TIERS=(
    ["baseline"]="x86_64-${TARGET_SUFFIX}"
    ["modern"]="x86_64-v3-${TARGET_SUFFIX}"
    ["highend"]="x86_64-v4-${TARGET_SUFFIX}"
)

# Feature descriptions for each tier
declare -A TIER_FEATURES=(
    ["baseline"]="SSE2 (100% compatible)"
    ["modern"]="AVX2+FMA+SSE4.2 (95% compatible, x86-64-v3)"
    ["highend"]="AVX512+AVX2 (70% compatible, x86-64-v4)"
)

# Expected performance gains (vs baseline)
declare -A TIER_PERF=(
    ["baseline"]="1.0x (baseline)"
    ["modern"]="1.15-1.30x (15-30% faster)"
    ["highend"]="1.20-1.50x (20-50% faster)"
)

# Function to build a binary for a specific tier
build_tier() {
    local tier=$1
    local target=$2
    local binary=$3
    local features=$4

    echo -e "${GREEN}[Building]${NC} $binary (${tier})"
    echo -e "  Target:   ${target}"
    echo -e "  Features: ${TIER_FEATURES[$tier]}"
    echo -e "  Expected: ${TIER_PERF[$tier]}"

    # Create custom target directory for this tier
    local tier_target_dir="target/${tier}"

    # Build with tier-specific rustflags
    case $tier in
        baseline)
            # No special flags - default x86-64 with SSE2
            RUSTFLAGS="" cargo build \
                --bin "$binary" \
                --target-dir "$tier_target_dir" \
                $RELEASE_FLAG
            ;;
        modern)
            # x86-64-v3: AVX2, FMA, SSE4.2
            RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build \
                --bin "$binary" \
                --target-dir "$tier_target_dir" \
                $RELEASE_FLAG
            ;;
        highend)
            # x86-64-v4: AVX512, AVX2, FMA
            RUSTFLAGS="-C target-cpu=x86-64-v4" cargo build \
                --bin "$binary" \
                --target-dir "$tier_target_dir" \
                $RELEASE_FLAG
            ;;
    esac

    # Get binary path
    local binary_path="${tier_target_dir}/${MODE}/${binary}"

    if [[ -f "$binary_path" ]]; then
        local size=$(du -h "$binary_path" | cut -f1)
        echo -e "  ${GREEN}✓${NC} Built successfully (size: $size)"
    else
        echo -e "  ${RED}✗${NC} Build failed"
        return 1
    fi

    echo ""
}

# Function to display CPU feature detection code
show_runtime_detection() {
    cat << 'EOF'

Runtime CPU Detection
=====================

Add this to your binary to automatically select the best tier:

```rust
#[cfg(target_arch = "x86_64")]
fn select_binary_tier() -> &'static str {
    use std::arch::is_x86_feature_detected;

    // Check for x86-64-v4 features (AVX512)
    if is_x86_feature_detected!("avx512f") &&
       is_x86_feature_detected!("avx512dq") &&
       is_x86_feature_detected!("avx512cd") &&
       is_x86_feature_detected!("avx512bw") &&
       is_x86_feature_detected!("avx512vl") {
        return "highend";
    }

    // Check for x86-64-v3 features (AVX2, FMA)
    if is_x86_feature_detected!("avx2") &&
       is_x86_feature_detected!("fma") {
        return "modern";
    }

    // Fallback to baseline (SSE2)
    "baseline"
}
```

EOF
}

# Build all tiers
echo -e "${BLUE}Building tiers...${NC}"
echo ""

BUILD_FAILED=false

# Build client binaries
if [[ "$BUILD_CLIENT" == true ]]; then
    for tier in baseline modern highend; do
        if ! build_tier "$tier" "${TIERS[$tier]}" "client" "client"; then
            BUILD_FAILED=true
        fi
    done
fi

# Build server binaries
if [[ "$BUILD_SERVER" == true ]]; then
    for tier in baseline modern highend; do
        if ! build_tier "$tier" "${TIERS[$tier]}" "server" "server"; then
            BUILD_FAILED=true
        fi
    done
fi

# Summary
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Build Summary${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

if [[ "$BUILD_FAILED" == true ]]; then
    echo -e "${RED}Some builds failed. Check output above.${NC}"
    exit 1
fi

echo -e "${GREEN}All builds completed successfully!${NC}"
echo ""

# Show output locations
echo -e "${YELLOW}Binary locations:${NC}"
for tier in baseline modern highend; do
    if [[ "$BUILD_CLIENT" == true ]]; then
        echo -e "  target/${tier}/${MODE}/client  (${TIER_FEATURES[$tier]})"
    fi
    if [[ "$BUILD_SERVER" == true ]]; then
        echo -e "  target/${tier}/${MODE}/server  (${TIER_FEATURES[$tier]})"
    fi
done
echo ""

# Show runtime detection example
show_runtime_detection

echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Run benchmarks: ./scripts/benchmark_tiers.sh"
echo -e "  2. Test on different CPUs to verify compatibility"
echo -e "  3. Implement runtime binary selection in launcher"
echo ""

echo -e "${GREEN}Done!${NC}"
