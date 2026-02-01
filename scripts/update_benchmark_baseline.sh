#!/bin/bash
# Update benchmark baseline for regression testing
#
# Usage:
#   ./scripts/update_benchmark_baseline.sh [baseline_name] [platform]
#
# Arguments:
#   baseline_name: Name of baseline (default: main)
#   platform: Platform identifier (default: auto-detect)
#
# Examples:
#   ./scripts/update_benchmark_baseline.sh main
#   ./scripts/update_benchmark_baseline.sh develop linux-x64

set -e

# Configuration
BASELINE_NAME=${1:-main}
PLATFORM=${2:-$(uname -s)-$(uname -m)}
BASELINE_DIR="benchmarks/baselines/${PLATFORM}/${BASELINE_NAME}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Updating Benchmark Baseline"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Baseline: ${BASELINE_NAME}"
echo "Platform: ${PLATFORM}"
echo "Target:   ${BASELINE_DIR}"
echo ""

# Ensure we're in the repository root
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must be run from repository root"
    exit 1
fi

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo "⚠️  Warning: You have uncommitted changes"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Step 1: Run benchmarks
echo "Step 1/4: Running benchmarks..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo bench --all-features -- --save-baseline "${BASELINE_NAME}"

# Step 2: Create baseline directory
echo ""
echo "Step 2/4: Creating baseline directory..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
mkdir -p "${BASELINE_DIR}"

# Step 3: Copy benchmark results
echo ""
echo "Step 3/4: Copying benchmark results..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Remove old baseline data
if [ -d "${BASELINE_DIR}/criterion" ]; then
    echo "Removing old baseline data..."
    rm -rf "${BASELINE_DIR}/criterion"
fi

# Copy new baseline data
echo "Copying results from target/criterion..."
cp -r target/criterion "${BASELINE_DIR}/criterion"

# Step 4: Create metadata file
echo ""
echo "Step 4/4: Creating baseline metadata..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

COMMIT_HASH=$(git rev-parse HEAD)
COMMIT_DATE=$(git log -1 --format=%cd --date=iso-strict)
CURRENT_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
RUST_VERSION=$(rustc --version)

cat > "${BASELINE_DIR}/baseline-info.json" << EOF
{
  "baseline_name": "${BASELINE_NAME}",
  "platform": "${PLATFORM}",
  "commit": {
    "hash": "${COMMIT_HASH}",
    "date": "${COMMIT_DATE}",
    "message": "$(git log -1 --format=%s)"
  },
  "created_at": "${CURRENT_DATE}",
  "environment": {
    "rust_version": "${RUST_VERSION}",
    "os": "$(uname -s)",
    "arch": "$(uname -m)",
    "hostname": "$(hostname)"
  },
  "benchmark_count": $(find "${BASELINE_DIR}/criterion" -name "estimates.json" | wc -l)
}
EOF

echo "Created baseline metadata:"
cat "${BASELINE_DIR}/baseline-info.json"

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Baseline Updated Successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Location: ${BASELINE_DIR}"
echo "Benchmarks: $(find "${BASELINE_DIR}/criterion" -name "estimates.json" | wc -l)"
echo ""

# Calculate baseline size
BASELINE_SIZE=$(du -sh "${BASELINE_DIR}" | cut -f1)
echo "Baseline size: ${BASELINE_SIZE}"
echo ""

# Show git status
echo "Git status:"
git status --short "${BASELINE_DIR}"
echo ""

# Commit instructions
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Next steps:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1. Review the baseline data:"
echo "   ls -lah ${BASELINE_DIR}"
echo ""
echo "2. Commit the baseline:"
echo "   git add ${BASELINE_DIR}"
echo "   git commit -m 'chore: Update ${BASELINE_NAME} benchmark baseline (${PLATFORM})'"
echo ""
echo "3. Push to remote:"
echo "   git push origin \$(git branch --show-current)"
echo ""
echo "⚠️  Note: If baseline files are large (>50MB), consider using Git LFS"
echo "   See benchmarks/baselines/README.md for Git LFS setup"
echo ""
