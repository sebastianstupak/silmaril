#!/bin/bash
#
# Test Linux Platform Optimizations
#
# This script runs comprehensive tests and benchmarks for the Linux
# platform abstraction layer optimizations.
#
# Requirements:
#   - Linux kernel 2.6.32+ (for vDSO support)
#   - Rust toolchain installed
#   - cargo-criterion (optional, for detailed reports)
#
# Usage:
#   ./scripts/test_linux_optimizations.sh [options]
#
# Options:
#   --quick         Run quick tests only (skip benchmarks)
#   --bench-only    Run benchmarks only (skip tests)
#   --baseline      Save benchmark baseline
#   --compare       Compare with baseline
#   --verbose       Enable verbose output
#   --help          Show this help message

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
QUICK=false
BENCH_ONLY=false
BASELINE=false
COMPARE=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK=true
            shift
            ;;
        --bench-only)
            BENCH_ONLY=true
            shift
            ;;
        --baseline)
            BASELINE=true
            shift
            ;;
        --compare)
            COMPARE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            grep '^#' "$0" | cut -c 3-
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Helper functions
print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Check system requirements
check_requirements() {
    print_header "Checking System Requirements"

    # Check OS
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        print_error "This script is for Linux only"
        print_info "Detected OS: $OSTYPE"
        exit 1
    fi
    print_success "Linux detected"

    # Check kernel version
    KERNEL_VERSION=$(uname -r | cut -d. -f1,2)
    KERNEL_MAJOR=$(echo $KERNEL_VERSION | cut -d. -f1)
    KERNEL_MINOR=$(echo $KERNEL_VERSION | cut -d. -f2)

    if [ "$KERNEL_MAJOR" -lt 2 ] || ([ "$KERNEL_MAJOR" -eq 2 ] && [ "$KERNEL_MINOR" -lt 6 ]); then
        print_error "Kernel version too old (need 2.6.32+)"
        print_info "Current kernel: $(uname -r)"
        exit 1
    fi
    print_success "Kernel version: $(uname -r)"

    # Check for vDSO
    if ldd /bin/ls 2>/dev/null | grep -q "linux-vdso"; then
        print_success "vDSO available (fast clock_gettime)"
    else
        print_warning "vDSO not detected (slower clock_gettime)"
    fi

    # Check CPU info
    CPU_COUNT=$(nproc)
    print_success "CPU cores: $CPU_COUNT"

    CPU_MODEL=$(lscpu | grep "Model name" | cut -d: -f2 | xargs)
    print_info "CPU: $CPU_MODEL"

    # Check CPU governor
    GOVERNOR=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown")
    if [ "$GOVERNOR" == "performance" ]; then
        print_success "CPU governor: $GOVERNOR (optimal for benchmarking)"
    else
        print_warning "CPU governor: $GOVERNOR (recommend 'performance' for benchmarking)"
        print_info "To set: echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"
    fi

    # Check for cargo
    if ! command -v cargo &> /dev/null; then
        print_error "cargo not found"
        print_info "Install Rust from https://rustup.rs"
        exit 1
    fi
    print_success "cargo: $(cargo --version)"

    # Check for cargo-criterion (optional)
    if command -v cargo-criterion &> /dev/null; then
        print_success "cargo-criterion available"
    else
        print_warning "cargo-criterion not found (optional)"
        print_info "Install with: cargo install cargo-criterion"
    fi

    echo ""
}

# Show system configuration
show_config() {
    print_header "System Configuration"

    echo "Distribution: $(lsb_release -d 2>/dev/null | cut -f2 || cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"')"
    echo "Kernel: $(uname -r)"
    echo "Architecture: $(uname -m)"
    echo "CPU: $CPU_MODEL"
    echo "Cores: $CPU_COUNT"
    echo "Governor: $GOVERNOR"

    # Memory
    MEM_TOTAL=$(free -h | grep Mem | awk '{print $2}')
    echo "Memory: $MEM_TOTAL"

    # Check NUMA
    if command -v numactl &> /dev/null; then
        NUMA_NODES=$(numactl --hardware 2>/dev/null | grep "available:" | awk '{print $2}')
        echo "NUMA nodes: $NUMA_NODES"
    fi

    # Check for transparent huge pages
    THP=$(cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null | grep -o '\[.*\]' | tr -d '[]' || echo "unknown")
    echo "Transparent Huge Pages: $THP"

    echo ""
}

# Run tests
run_tests() {
    if [ "$BENCH_ONLY" = true ]; then
        return
    fi

    print_header "Running Platform Tests"

    if [ "$VERBOSE" = true ]; then
        cargo test -p engine-core --lib platform -- --nocapture
    else
        cargo test -p engine-core --lib platform
    fi

    print_success "All tests passed"
    echo ""
}

# Run benchmarks
run_benchmarks() {
    if [ "$QUICK" = true ]; then
        print_warning "Skipping benchmarks (--quick mode)"
        return
    fi

    print_header "Running Platform Benchmarks"

    BENCH_ARGS=""
    if [ "$BASELINE" = true ]; then
        BENCH_ARGS="-- --save-baseline linux-main"
        print_info "Saving baseline as 'linux-main'"
    elif [ "$COMPARE" = true ]; then
        BENCH_ARGS="-- --baseline linux-main"
        print_info "Comparing with baseline 'linux-main'"
    fi

    # Run full benchmark suite
    cargo bench -p engine-core --bench platform_benches $BENCH_ARGS

    print_success "Benchmarks complete"
    echo ""
}

# Analyze results
analyze_results() {
    print_header "Performance Analysis"

    # Check if criterion output exists
    CRITERION_DIR="target/criterion"
    if [ ! -d "$CRITERION_DIR" ]; then
        print_warning "No benchmark results found"
        return
    fi

    echo "Benchmark results are in: $CRITERION_DIR"
    echo ""

    # Key metrics to check
    print_info "Key Metrics (check manually in criterion output):"
    echo "  Time Backend:"
    echo "    - time/monotonic_nanos/single: Target <30ns, Acceptable <50ns"
    echo "    - time/monotonic_nanos/batch_1000: Target <30us, Acceptable <50us"
    echo ""
    echo "  Filesystem Backend:"
    echo "    - filesystem/normalize_path/simple: Target <200ns, Acceptable <500ns"
    echo "    - filesystem/normalize_path/complex: Target <1us, Acceptable <2us"
    echo ""
    echo "  Threading Backend:"
    echo "    - threading/set_priority/normal: Target <2us, Acceptable <5us"
    echo "    - threading/set_affinity/1_core: Target <5us, Acceptable <10us"
    echo "    - threading/num_cpus: Target <100ns, Acceptable <1us"
    echo ""

    # Open HTML report if available
    INDEX_HTML="$CRITERION_DIR/report/index.html"
    if [ -f "$INDEX_HTML" ]; then
        print_info "HTML report available: $INDEX_HTML"
        if command -v xdg-open &> /dev/null; then
            print_info "Opening in browser..."
            xdg-open "$INDEX_HTML" 2>/dev/null &
        fi
    fi
}

# Run profiling (optional advanced analysis)
run_profiling() {
    if [ "$QUICK" = true ]; then
        return
    fi

    print_header "Advanced Profiling (Optional)"

    # Check for perf
    if ! command -v perf &> /dev/null; then
        print_warning "perf not found (optional)"
        print_info "Install with: sudo apt-get install linux-tools-common linux-tools-generic"
        return
    fi

    print_info "To run perf profiling:"
    echo "  1. Build release binary:"
    echo "     cargo build -p engine-core --release"
    echo ""
    echo "  2. Run perf:"
    echo "     perf record -F 999 -g -- cargo bench -p engine-core --bench platform_benches"
    echo ""
    echo "  3. View report:"
    echo "     perf report"
    echo ""
}

# Generate summary
generate_summary() {
    print_header "Test Summary"

    print_success "Platform optimizations tested on Linux"
    echo ""
    echo "System: $(lsb_release -d 2>/dev/null | cut -f2 || cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"')"
    echo "Kernel: $(uname -r)"
    echo "CPU: $CPU_MODEL ($CPU_COUNT cores)"
    echo ""

    print_info "Next Steps:"
    echo "  1. Review benchmark results in target/criterion/"
    echo "  2. Compare with performance targets in LINUX_OPTIMIZATION_RESULTS.md"
    echo "  3. Update documentation with actual results"
    echo "  4. Run on different distributions for compatibility testing"
    echo ""

    print_info "Documentation:"
    echo "  - Optimization details: LINUX_OPTIMIZATION_RESULTS.md"
    echo "  - Profiling guide: docs/profiling.md"
    echo "  - Platform docs: docs/platform-abstraction.md"
    echo ""
}

# Main execution
main() {
    print_header "Linux Platform Optimization Test Suite"
    echo ""

    check_requirements
    show_config
    run_tests
    run_benchmarks
    analyze_results
    run_profiling
    generate_summary

    print_success "Test suite complete!"
}

# Run main
main
