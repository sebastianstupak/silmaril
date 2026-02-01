#!/usr/bin/env bash
# Network Benchmark Runner
#
# Quick script to run network integration benchmarks with common options

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}========================================${NC}"
}

print_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse command line arguments
MODE="${1:-quick}"
BASELINE="${2:-}"

case "$MODE" in
    quick)
        print_header "Running Quick Network Benchmarks"
        print_info "Running: end_to_end_latency and simulator_overhead"
        cargo bench --bench integration_benches -- \
            --quick \
            'end_to_end_latency|simulator_overhead'
        ;;

    full)
        print_header "Running Full Network Benchmark Suite"
        print_info "This will take 10-20 minutes..."
        cargo bench --bench integration_benches
        ;;

    scenarios)
        print_header "Running Game Scenario Benchmarks"
        cargo bench --bench integration_benches -- game_scenarios
        ;;

    latency)
        print_header "Running Latency Benchmarks"
        cargo bench --bench integration_benches -- end_to_end_latency
        ;;

    bandwidth)
        print_header "Running Bandwidth Benchmarks"
        cargo bench --bench integration_benches -- bandwidth_usage
        ;;

    scalability)
        print_header "Running Scalability Benchmarks"
        cargo bench --bench integration_benches -- \
            'concurrent_clients|scalability'
        ;;

    resilience)
        print_header "Running Packet Loss Resilience Benchmarks"
        cargo bench --bench integration_benches -- packet_loss_resilience
        ;;

    baseline)
        if [ -z "$BASELINE" ]; then
            BASELINE="main"
        fi
        print_header "Creating Baseline: $BASELINE"
        cargo bench --bench integration_benches -- --save-baseline "$BASELINE"
        print_info "Baseline saved as '$BASELINE'"
        ;;

    compare)
        if [ -z "$BASELINE" ]; then
            print_error "Baseline name required for comparison"
            echo "Usage: $0 compare <baseline-name>"
            exit 1
        fi
        print_header "Comparing Against Baseline: $BASELINE"
        cargo bench --bench integration_benches -- --baseline "$BASELINE"
        ;;

    report)
        print_header "Opening Benchmark Report"
        REPORT_PATH="$PROJECT_ROOT/target/criterion/report/index.html"
        if [ -f "$REPORT_PATH" ]; then
            print_info "Opening $REPORT_PATH"
            if command -v xdg-open > /dev/null; then
                xdg-open "$REPORT_PATH"
            elif command -v open > /dev/null; then
                open "$REPORT_PATH"
            else
                print_info "Please open manually: $REPORT_PATH"
            fi
        else
            print_error "Report not found. Run benchmarks first."
            exit 1
        fi
        ;;

    clean)
        print_header "Cleaning Benchmark Data"
        rm -rf "$PROJECT_ROOT/target/criterion"
        print_info "Benchmark data cleaned"
        ;;

    help|--help|-h)
        cat << EOF
Network Benchmark Runner

Usage: $0 [mode] [baseline-name]

Modes:
    quick           Run quick tests (latency + overhead) [default]
    full            Run complete benchmark suite (10-20 min)
    scenarios       Run game scenario benchmarks (MMORPG, FPS, etc.)
    latency         Run end-to-end latency benchmarks
    bandwidth       Run bandwidth usage benchmarks
    scalability     Run concurrent client and scalability benchmarks
    resilience      Run packet loss resilience benchmarks

    baseline <name> Create a new baseline for comparison
    compare <name>  Compare current performance vs baseline
    report          Open HTML benchmark report in browser
    clean           Remove all benchmark data
    help            Show this help message

Examples:
    $0 quick                    # Quick test
    $0 full                     # Full suite
    $0 baseline main            # Save baseline as 'main'
    $0 compare main             # Compare against 'main'
    $0 latency                  # Only latency tests
    $0 report                   # View results

Output:
    Results saved to: target/criterion/
    HTML report: target/criterion/report/index.html
EOF
        ;;

    *)
        print_error "Unknown mode: $MODE"
        echo "Run '$0 help' for usage information"
        exit 1
        ;;
esac

echo ""
print_info "Benchmark complete!"

if [ "$MODE" != "report" ] && [ "$MODE" != "clean" ] && [ "$MODE" != "help" ]; then
    print_info "View results: $0 report"
    print_info "View HTML report: target/criterion/report/index.html"
fi
