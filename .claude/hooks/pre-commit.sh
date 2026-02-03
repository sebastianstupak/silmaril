#!/bin/bash
# Pre-commit hook for silmaril
# Ensures code quality before commit

set -e  # Exit on any error

echo "🔍 Running pre-commit checks..."
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall status
CHECKS_PASSED=true

# ============================================================================
# 1. Check for forbidden patterns
# ============================================================================

echo "📋 Checking for forbidden patterns..."

FORBIDDEN_PATTERNS=(
    "println!"
    "eprintln!"
    "print!"
    "eprint!"
    "dbg!"
)

STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

if [ -n "$STAGED_FILES" ]; then
    for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
        echo "  Checking for ${pattern}..."

        # Search in staged files only
        MATCHES=$(echo "$STAGED_FILES" | xargs grep -n "$pattern" 2>/dev/null || true)

        if [ -n "$MATCHES" ]; then
            echo -e "${RED}✗ Found forbidden pattern: ${pattern}${NC}"
            echo "$MATCHES"
            echo ""
            echo "  Fix: Use 'tracing' macros instead (info!, warn!, error!, debug!)"
            echo "  Reference: docs/rules/coding-standards.md"
            CHECKS_PASSED=false
        fi
    done
fi

if [ "$CHECKS_PASSED" = true ]; then
    echo -e "${GREEN}✓ No forbidden patterns found${NC}"
fi
echo ""

# ============================================================================
# 2. Run cargo fmt --check
# ============================================================================

echo "🎨 Checking code formatting (cargo fmt)..."

if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Code is formatted correctly${NC}"
else
    echo -e "${RED}✗ Code formatting issues found${NC}"
    echo ""
    echo "  Fix: Run 'cargo fmt' to auto-format your code"
    echo "  Reference: docs/rules/coding-standards.md"
    CHECKS_PASSED=false
fi
echo ""

# ============================================================================
# 3. Run cargo clippy
# ============================================================================

echo "🔧 Running cargo clippy..."

# Run clippy with all important lints
if cargo clippy --all-targets --all-features -- \
    -D warnings \
    -D clippy::print_stdout \
    -D clippy::print_stderr \
    -D clippy::dbg_macro \
    -D clippy::unwrap_used \
    -D clippy::expect_used \
    -W clippy::panic \
    > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Clippy checks passed${NC}"
else
    echo -e "${RED}✗ Clippy found issues${NC}"
    echo ""
    echo "  Run 'cargo clippy --all-targets --all-features -- -D warnings' to see details"
    echo "  Reference: docs/rules/coding-standards.md"
    CHECKS_PASSED=false
fi
echo ""

# ============================================================================
# 4. Check for TODO/FIXME in committed code
# ============================================================================

echo "📝 Checking for TODO/FIXME markers..."

if [ -n "$STAGED_FILES" ]; then
    TODO_COUNT=$(echo "$STAGED_FILES" | xargs grep -n -E "(TODO|FIXME|XXX|HACK)" 2>/dev/null | wc -l || echo 0)

    if [ "$TODO_COUNT" -gt 0 ]; then
        echo -e "${YELLOW}⚠ Found $TODO_COUNT TODO/FIXME markers${NC}"
        echo "$STAGED_FILES" | xargs grep -n -E "(TODO|FIXME|XXX|HACK)" || true
        echo ""
        echo "  Note: Consider resolving these before committing"
        echo "  (This is a warning, not blocking)"
    else
        echo -e "${GREEN}✓ No TODO/FIXME markers found${NC}"
    fi
fi
echo ""

# ============================================================================
# 5. Check for large files
# ============================================================================

echo "📦 Checking for large files..."

LARGE_FILES=$(git diff --cached --name-only --diff-filter=ACM | while read file; do
    if [ -f "$file" ]; then
        SIZE=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
        if [ "$SIZE" -gt 1048576 ]; then  # > 1MB
            echo "$file ($SIZE bytes)"
        fi
    fi
done)

if [ -n "$LARGE_FILES" ]; then
    echo -e "${YELLOW}⚠ Found large files:${NC}"
    echo "$LARGE_FILES"
    echo ""
    echo "  Consider using Git LFS for binary files"
    echo "  (This is a warning, not blocking)"
else
    echo -e "${GREEN}✓ No large files found${NC}"
fi
echo ""

# ============================================================================
# 6. Verify no sensitive data
# ============================================================================

echo "🔒 Checking for sensitive data..."

SENSITIVE_PATTERNS=(
    "password"
    "api_key"
    "apiKey"
    "secret"
    "token"
    "private_key"
    "privateKey"
)

SENSITIVE_FOUND=false

if [ -n "$STAGED_FILES" ]; then
    for pattern in "${SENSITIVE_PATTERNS[@]}"; do
        MATCHES=$(echo "$STAGED_FILES" | xargs grep -in "$pattern" 2>/dev/null | grep -v "// test" | grep -v "// example" || true)

        if [ -n "$MATCHES" ]; then
            echo -e "${YELLOW}⚠ Potentially sensitive data found: ${pattern}${NC}"
            echo "$MATCHES"
            SENSITIVE_FOUND=true
        fi
    done
fi

if [ "$SENSITIVE_FOUND" = false ]; then
    echo -e "${GREEN}✓ No obvious sensitive data found${NC}"
else
    echo ""
    echo "  Review these matches to ensure no secrets are committed"
    echo "  (This is a warning, not blocking)"
fi
echo ""

# ============================================================================
# Final verdict
# ============================================================================

echo "========================================"

if [ "$CHECKS_PASSED" = true ]; then
    echo -e "${GREEN}✓ All pre-commit checks passed!${NC}"
    echo ""
    echo "Proceeding with commit..."
    exit 0
else
    echo -e "${RED}✗ Pre-commit checks failed${NC}"
    echo ""
    echo "Fix the issues above before committing."
    echo "Reference: docs/rules/coding-standards.md"
    echo ""
    echo "To bypass this hook (NOT RECOMMENDED):"
    echo "  git commit --no-verify"
    exit 1
fi
