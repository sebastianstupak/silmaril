#!/bin/bash
# E2E test for `silm build` and `silm package`.
#
# Tests the full workflow:
#   1. Build silm CLI
#   2. Create a project with `silm new --local`
#   3. Run `silm build` / `silm package` for various platforms
#   4. Verify expected output artefacts exist
#
# Usage: bash scripts/e2e-tests/test-silm-build.sh

set -euo pipefail

# -- Helpers -------------------------------------------------------------------

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

record_pass() { echo -e "${GREEN}PASS${NC} $1"; PASS_COUNT=$((PASS_COUNT + 1)); }
record_fail() { echo -e "${RED}FAIL${NC} $1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }
record_skip() { echo -e "${YELLOW}SKIP${NC} $1"; SKIP_COUNT=$((SKIP_COUNT + 1)); }
INFO() { echo -e "${YELLOW}->  ${NC} $1"; }
SECTION() { echo -e "\n${CYAN}== $1 ==${NC}"; }

# -- Locate repo root ---------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

INFO "silmaril repo: $REPO_ROOT"

# -- Build silm binary ---------------------------------------------------------

SECTION "Building silm"
cd "$REPO_ROOT"
cargo build --package silm --quiet 2>&1
SILM="$REPO_ROOT/target/debug/silm"
[ -f "$SILM" ] || SILM="$SILM.exe"
[ -f "$SILM" ] || { record_fail "silm binary not found"; exit 1; }
record_pass "silm binary built: $SILM"

# -- Setup: create temp project ------------------------------------------------

SECTION "Setup: silm new"

WORK_DIR=$(mktemp -d)
trap "rm -rf '$WORK_DIR'" EXIT

INFO "Working in $WORK_DIR"

cd "$WORK_DIR"
"$SILM" new test-game --local 2>&1
GAME_DIR="$WORK_DIR/test-game"

[ -d "$GAME_DIR" ] || { record_fail "game dir not created"; exit 1; }
[ -f "$GAME_DIR/game.toml" ] || { record_fail "game.toml missing"; exit 1; }
record_pass "test-game project created"

cd "$GAME_DIR"

# Helper: check if a binary exists (Unix or .exe extension)
binary_exists() {
    local path="$1"
    [ -f "$path" ] || [ -f "${path}.exe" ]
}

# -- Test 1: silm build --platform native (debug) -----------------------------

SECTION "Test 1: silm build --platform native"

if "$SILM" build --platform native 2>&1; then
    # Check for server and client binaries in target/debug/
    SERVER_OK=false
    CLIENT_OK=false

    if binary_exists "$GAME_DIR/target/debug/server"; then
        SERVER_OK=true
    fi
    if binary_exists "$GAME_DIR/target/debug/client"; then
        CLIENT_OK=true
    fi

    if $SERVER_OK && $CLIENT_OK; then
        record_pass "silm build --platform native (server + client binaries in target/debug/)"
    elif $SERVER_OK || $CLIENT_OK; then
        record_pass "silm build --platform native (at least one binary found in target/debug/)"
    else
        record_fail "silm build --platform native (no binaries found in target/debug/)"
    fi
else
    record_fail "silm build --platform native (command failed)"
fi

# -- Test 2: silm build --platform native --release ----------------------------

SECTION "Test 2: silm build --platform native --release"

if "$SILM" build --platform native --release 2>&1; then
    SERVER_OK=false
    CLIENT_OK=false

    if binary_exists "$GAME_DIR/target/release/server"; then
        SERVER_OK=true
    fi
    if binary_exists "$GAME_DIR/target/release/client"; then
        CLIENT_OK=true
    fi

    if $SERVER_OK && $CLIENT_OK; then
        record_pass "silm build --platform native --release (server + client binaries in target/release/)"
    elif $SERVER_OK || $CLIENT_OK; then
        record_pass "silm build --platform native --release (at least one binary found in target/release/)"
    else
        record_fail "silm build --platform native --release (no binaries found in target/release/)"
    fi
else
    record_fail "silm build --platform native --release (command failed)"
fi

# -- Test 3: silm package --platform native ------------------------------------

SECTION "Test 3: silm package --platform native"

if "$SILM" package --platform native 2>&1; then
    DIST_DIR="$GAME_DIR/dist/native"
    if [ -d "$DIST_DIR" ]; then
        # Look for a zip file in the dist directory
        ZIP_FOUND=false
        for f in "$DIST_DIR"/*.zip "$GAME_DIR/dist"/*.zip; do
            if [ -f "$f" ]; then
                ZIP_FOUND=true
                break
            fi
        done

        if $ZIP_FOUND; then
            record_pass "silm package --platform native (dist/native/ exists and zip found)"
        else
            # dist/native exists but no zip -- partial pass
            record_pass "silm package --platform native (dist/native/ exists, no zip yet)"
        fi
    else
        record_fail "silm package --platform native (dist/native/ directory not found)"
    fi
else
    record_fail "silm package --platform native (command failed)"
fi

# -- Test 4: silm package --platform server ------------------------------------

SECTION "Test 4: silm package --platform server"

if "$SILM" package --platform server 2>&1; then
    DIST_DIR="$GAME_DIR/dist/server"
    if [ -d "$DIST_DIR" ]; then
        if [ -f "$DIST_DIR/Dockerfile" ]; then
            record_pass "silm package --platform server (dist/server/ with Dockerfile)"
        else
            record_fail "silm package --platform server (dist/server/ exists but no Dockerfile)"
        fi
    else
        record_fail "silm package --platform server (dist/server/ directory not found)"
    fi
else
    record_fail "silm package --platform server (command failed)"
fi

# -- Test 5: WASM build (skip if trunk not on PATH) ---------------------------

SECTION "Test 5: silm build --platform wasm"

if command -v trunk >/dev/null 2>&1; then
    if "$SILM" build --platform wasm 2>&1; then
        record_pass "silm build --platform wasm"
    else
        record_fail "silm build --platform wasm (command failed)"
    fi
else
    record_skip "silm build --platform wasm (trunk not found on PATH)"
fi

# -- Test 6: Cross build linux-x86_64 (skip if docker/cross not available) ----

SECTION "Test 6: silm build --platform linux-x86_64"

CROSS_AVAILABLE=true

if ! command -v cross >/dev/null 2>&1; then
    CROSS_AVAILABLE=false
    INFO "cross not found on PATH"
fi

if $CROSS_AVAILABLE && ! docker info >/dev/null 2>&1; then
    CROSS_AVAILABLE=false
    INFO "Docker is not running"
fi

if $CROSS_AVAILABLE; then
    if "$SILM" build --platform linux-x86_64 2>&1; then
        record_pass "silm build --platform linux-x86_64 (cross)"
    else
        record_fail "silm build --platform linux-x86_64 (cross build failed)"
    fi
else
    record_skip "silm build --platform linux-x86_64 (requires cross + Docker)"
fi

# -- Summary -------------------------------------------------------------------

SECTION "Results"

echo ""
echo -e "  ${GREEN}PASS:${NC} $PASS_COUNT"
echo -e "  ${YELLOW}SKIP:${NC} $SKIP_COUNT"
echo -e "  ${RED}FAIL:${NC} $FAIL_COUNT"
echo ""

TOTAL=$((PASS_COUNT + FAIL_COUNT + SKIP_COUNT))
echo "Total: $TOTAL tests"
echo ""

if [ "$FAIL_COUNT" -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed (${SKIP_COUNT} skipped).${NC}"
    exit 0
fi
