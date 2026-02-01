#!/bin/bash
# Setup git hooks for agent-game-engine development

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "Setting up development environment..."
echo "========================================="
echo ""

# Find git root directory
GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$GIT_ROOT" ]; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
fi

HOOKS_DIR="$GIT_ROOT/.git/hooks"
SOURCE_HOOK="$GIT_ROOT/scripts/hooks/pre-commit"

echo -e "${BLUE}Git root:${NC} $GIT_ROOT"
echo -e "${BLUE}Hooks directory:${NC} $HOOKS_DIR"
echo ""

# Check if source hook exists
if [ ! -f "$SOURCE_HOOK" ]; then
    echo -e "${RED}Error: Pre-commit hook not found at $SOURCE_HOOK${NC}"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Copy pre-commit hook
echo "Installing pre-commit hook..."
cp "$SOURCE_HOOK" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"
echo -e "${GREEN}✓${NC} Pre-commit hook installed"
echo ""

# Check for optional tools
echo "Checking for optional development tools..."
echo ""

# Check for cargo-deny
if command -v cargo-deny &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-deny installed"
else
    echo -e "${YELLOW}○${NC} cargo-deny not installed (optional)"
    echo "  Install with: cargo install cargo-deny"
fi

# Check for cargo-watch (for hot-reload)
if command -v cargo-watch &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-watch installed"
else
    echo -e "${YELLOW}○${NC} cargo-watch not installed (optional)"
    echo "  Install with: cargo install cargo-watch"
fi

# Check for cargo-flamegraph
if command -v cargo-flamegraph &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-flamegraph installed"
else
    echo -e "${YELLOW}○${NC} cargo-flamegraph not installed (optional)"
    echo "  Install with: cargo install flamegraph"
fi

# Check for criterion (installed as dev dependency)
echo -e "${GREEN}✓${NC} criterion (installed as dev dependency)"

echo ""
echo "========================================="
echo -e "${GREEN}Setup complete!${NC}"
echo "========================================="
echo ""
echo "Pre-commit hooks will now run automatically before each commit."
echo ""
echo "The following checks will run:"
echo "  • Code formatting (cargo fmt)"
echo "  • Linting (cargo clippy)"
echo "  • Unit tests (cargo test --lib)"
echo "  • Dependency checks (cargo deny, if installed)"
echo "  • Common issue detection (println!, anyhow, etc.)"
echo ""
echo "To manually run these checks:"
echo "  $HOOKS_DIR/pre-commit"
echo ""
echo "To bypass hooks (not recommended):"
echo "  git commit --no-verify"
echo ""
echo "Happy coding!"
echo ""
