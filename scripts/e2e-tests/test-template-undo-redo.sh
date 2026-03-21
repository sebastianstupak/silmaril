#!/usr/bin/env bash
# E2E test suite for silm template subcommands
# Run from repo root: bash scripts/e2e-tests/test-template-undo-redo.sh
# Requires: cargo build --bin silm
set -uo pipefail

SILM="./target/debug/silm"
PASS=0
FAIL=0

# ── Helpers ────────────────────────────────────────────────────────────────

pass() { echo "  PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL+1)); }

# Assert a string appears in a file
assert_in_file() {
    local pattern="$1" file="$2" msg="$3"
    if grep -q "$pattern" "$file"; then
        pass "$msg"
    else
        fail "$msg (pattern '$pattern' not found in $file)"
    fi
}

# Assert a string does NOT appear in a file
assert_not_in_file() {
    local pattern="$1" file="$2" msg="$3"
    if grep -q "$pattern" "$file"; then
        fail "$msg (pattern '$pattern' unexpectedly found in $file)"
    else
        pass "$msg"
    fi
}

# Assert command exits 0
assert_ok() {
    local cmd="$1" msg="$2"
    if eval "$cmd" > /dev/null 2>&1; then
        pass "$msg"
    else
        fail "$msg (command failed: $cmd)"
    fi
}

# Assert command exits non-zero
assert_err() {
    local cmd="$1" msg="$2"
    if eval "$cmd" > /dev/null 2>&1; then
        fail "$msg (expected error but command succeeded)"
    else
        pass "$msg"
    fi
}

# Assert JSON pattern appears in a string
assert_json_contains() {
    local output="$1" pattern="$2" msg="$3"
    if echo "$output" | grep -q "$pattern"; then
        pass "$msg"
    else
        fail "$msg (pattern '$pattern' not in output: $output)"
    fi
}

# Assert JSON pattern does NOT appear in a string
assert_json_not_contains() {
    local output="$1" pattern="$2" msg="$3"
    if echo "$output" | grep -q "$pattern"; then
        fail "$msg (pattern '$pattern' unexpectedly found in output)"
    else
        pass "$msg"
    fi
}

# Create a fresh temp template file and return its path
new_template() {
    local name="${1:-test}"
    local dir
    dir=$(mktemp -d)
    printf 'name: %s\nentities: []\n' "$name" > "$dir/$name.yaml"
    echo "$dir/$name.yaml"
}

# Extract entity ID from JSON output (first id field — the newly created entity)
extract_id() {
    echo "$1" | grep -o '"id": [0-9]*' | tail -1 | grep -o '[0-9]*'
}

# ── entity create ──────────────────────────────────────────────────────────

echo ""
echo "=== entity create ==="
{
    T=$(new_template world)

    OUT=$($SILM template entity --template "$T" create --name "Hero" 2>&1)
    assert_json_contains "$OUT" '"name": "Hero"' "create: JSON output contains entity name"
    assert_json_contains "$OUT" '"id": 1' "create: JSON output contains assigned id"
    assert_json_contains "$OUT" '"components": \[\]' "create: JSON output contains empty components"
    assert_in_file "Hero" "$T" "create: YAML written with entity name"
    assert_in_file "id: 1" "$T" "create: YAML written with entity id"

    OUT2=$($SILM template entity --template "$T" create --name "Sidekick" 2>&1)
    assert_json_contains "$OUT2" '"id": 2' "create second: id increments to 2"
    assert_in_file "Sidekick" "$T" "create second: YAML contains second entity"

    # Create without name — should succeed with null name
    OUT3=$($SILM template entity --template "$T" create 2>&1)
    assert_json_contains "$OUT3" '"name": null' "create without name: JSON has null name"

    rm -rf "$(dirname "$T")"
}

# ── entity delete ──────────────────────────────────────────────────────────

echo ""
echo "=== entity delete ==="
{
    T=$(new_template world)

    OUT=$($SILM template entity --template "$T" create --name "DeleteMe" 2>&1)
    ID=$(extract_id "$OUT")

    DEL_OUT=$($SILM template entity --template "$T" delete "$ID" 2>&1)
    assert_json_not_contains "$DEL_OUT" "DeleteMe" "delete: entity absent from JSON output"
    assert_not_in_file "DeleteMe" "$T" "delete: entity removed from YAML"

    # Verify other entities are unaffected
    $SILM template entity --template "$T" create --name "Keeper" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Sacrifice" > /dev/null 2>&1
    SAC_OUT=$($SILM template entity --template "$T" create --name "Sacrifice2" 2>&1)
    SAC_ID=$(extract_id "$SAC_OUT")
    $SILM template entity --template "$T" delete "$SAC_ID" > /dev/null 2>&1
    assert_in_file "Keeper" "$T" "delete: unrelated entity still in YAML"
    assert_not_in_file "Sacrifice2" "$T" "delete: correct entity removed when multiple exist"

    # Error: delete nonexistent id
    assert_err "$SILM template entity --template '$T' delete 99999" "delete nonexistent id: exits non-zero"

    rm -rf "$(dirname "$T")"
}

# ── entity rename ──────────────────────────────────────────────────────────

echo ""
echo "=== entity rename ==="
{
    T=$(new_template world)

    OUT=$($SILM template entity --template "$T" create --name "OldName" 2>&1)
    ID=$(extract_id "$OUT")

    REN_OUT=$($SILM template entity --template "$T" rename "$ID" "NewName" 2>&1)
    assert_json_contains "$REN_OUT" '"name": "NewName"' "rename: JSON output shows new name"
    assert_json_not_contains "$REN_OUT" '"name": "OldName"' "rename: JSON output does not show old name"
    assert_not_in_file "OldName" "$T" "rename: old name absent from YAML"
    assert_in_file "NewName" "$T" "rename: new name present in YAML"

    # Error: rename nonexistent id
    assert_err "$SILM template entity --template '$T' rename 99999 Whatever" "rename nonexistent id: exits non-zero"

    rm -rf "$(dirname "$T")"
}

# ── entity duplicate ───────────────────────────────────────────────────────

echo ""
echo "=== entity duplicate ==="
{
    T=$(new_template world)

    # Create entity with a component so we can verify deep copy
    $SILM template entity --template "$T" create --name "Original" > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Health '{"max":100}' > /dev/null 2>&1

    DUP_OUT=$($SILM template entity --template "$T" duplicate 1 2>&1)
    assert_json_contains "$DUP_OUT" '"name": "Original (copy)"' "duplicate: copy has '(copy)' suffix"
    assert_json_contains "$DUP_OUT" '"id": 2' "duplicate: copy gets new id"
    # Original should still be present
    assert_json_contains "$DUP_OUT" '"id": 1' "duplicate: original entity still present in output"
    assert_in_file "Original (copy)" "$T" "duplicate: copy present in YAML"
    assert_in_file "Original" "$T" "duplicate: original still in YAML"

    # Count entity entries — should be 2
    COUNT=$(grep -c "name: Original" "$T" || true)
    if [ "$COUNT" -eq 2 ]; then
        pass "duplicate: YAML has exactly 2 entries with 'Original' in name"
    else
        fail "duplicate: expected 2 entries with 'Original' in name, got $COUNT"
    fi

    # Error: duplicate nonexistent id
    assert_err "$SILM template entity --template '$T' duplicate 99999" "duplicate nonexistent id: exits non-zero"

    rm -rf "$(dirname "$T")"
}

# ── component add ──────────────────────────────────────────────────────────

echo ""
echo "=== component add ==="
{
    T=$(new_template world)
    $SILM template entity --template "$T" create --name "Player" > /dev/null 2>&1

    ADD_OUT=$($SILM template component --template "$T" add 1 Health '{"max":100}' 2>&1)
    assert_json_contains "$ADD_OUT" '"type_name": "Health"' "component add: JSON has type_name"
    assert_json_contains "$ADD_OUT" 'max' "component add: JSON contains component data"
    assert_in_file "type_name: Health" "$T" "component add: YAML has component type_name"
    assert_in_file "max" "$T" "component add: YAML has component data"

    # Add second distinct component
    $SILM template component --template "$T" add 1 Transform '{"x":0,"y":0}' > /dev/null 2>&1
    assert_in_file "type_name: Transform" "$T" "component add second: Transform present in YAML"

    # Error: add component to nonexistent entity
    assert_err "$SILM template component --template '$T' add 99999 Health '{\"max\":50}'" "component add to nonexistent entity: exits non-zero"

    # Error: add duplicate component
    assert_err "$SILM template component --template '$T' add 1 Health '{\"max\":50}'" "component add duplicate: exits non-zero"

    rm -rf "$(dirname "$T")"
}

# ── component set ──────────────────────────────────────────────────────────

echo ""
echo "=== component set ==="
{
    T=$(new_template world)
    $SILM template entity --template "$T" create --name "Player" > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Health '{"max":100}' > /dev/null 2>&1

    SET_OUT=$($SILM template component --template "$T" set 1 Health '{"max":250}' 2>&1)
    assert_json_contains "$SET_OUT" '250' "component set: JSON shows updated value"
    assert_in_file "250" "$T" "component set: YAML has updated value"
    assert_not_in_file '"max":100' "$T" "component set: old value absent from YAML"

    # Error: set on nonexistent entity
    assert_err "$SILM template component --template '$T' set 99999 Health '{\"max\":50}'" "component set nonexistent entity: exits non-zero"

    # Note: set on a component type not yet present on the entity acts as upsert (adds it)
    $SILM template component --template "$T" set 1 Transform '{"x":5}' > /dev/null 2>&1
    assert_in_file "type_name: Transform" "$T" "component set upsert: set on new type creates the component"

    rm -rf "$(dirname "$T")"
}

# ── component remove ───────────────────────────────────────────────────────

echo ""
echo "=== component remove ==="
{
    T=$(new_template world)
    $SILM template entity --template "$T" create --name "Player" > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Health '{"max":100}' > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Transform '{"x":0}' > /dev/null 2>&1

    REM_OUT=$($SILM template component --template "$T" remove 1 Health 2>&1)
    assert_json_not_contains "$REM_OUT" '"type_name": "Health"' "component remove: Health absent from JSON"
    assert_json_contains "$REM_OUT" '"type_name": "Transform"' "component remove: Transform still in JSON"
    assert_not_in_file "type_name: Health" "$T" "component remove: Health absent from YAML"
    assert_in_file "type_name: Transform" "$T" "component remove: Transform still in YAML"

    # Error: remove nonexistent component
    assert_err "$SILM template component --template '$T' remove 1 Physics" "component remove nonexistent type: exits non-zero"

    # Error: remove from nonexistent entity
    assert_err "$SILM template component --template '$T' remove 99999 Health" "component remove from nonexistent entity: exits non-zero"

    rm -rf "$(dirname "$T")"
}

# ── undo ───────────────────────────────────────────────────────────────────

echo ""
echo "=== undo ==="
{
    T=$(new_template world)

    # Undo on empty history — exits 0, reports nothing_to_undo
    UNDO_EMPTY=$($SILM template undo --template "$T" 2>&1)
    assert_json_contains "$UNDO_EMPTY" 'nothing_to_undo' "undo empty: reports nothing_to_undo"
    assert_ok "$SILM template undo --template '$T'" "undo empty: exits 0"

    # Create entity, then undo it
    $SILM template entity --template "$T" create --name "Ephemeral" > /dev/null 2>&1
    assert_in_file "Ephemeral" "$T" "undo setup: entity present before undo"

    UNDO_OUT=$($SILM template undo --template "$T" 2>&1)
    assert_json_contains "$UNDO_OUT" '"ok":true' "undo: JSON has ok:true"
    assert_json_contains "$UNDO_OUT" 'undone_action_id' "undo: JSON has undone_action_id field"
    assert_not_in_file "Ephemeral" "$T" "undo: entity removed after undo"

    # Multiple undos
    $SILM template entity --template "$T" create --name "Alpha" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Beta" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Gamma" > /dev/null 2>&1

    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Gamma" "$T" "undo multi-1: third entity removed"
    assert_in_file "Beta" "$T" "undo multi-1: second entity still present"

    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Beta" "$T" "undo multi-2: second entity removed"
    assert_in_file "Alpha" "$T" "undo multi-2: first entity still present"

    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Alpha" "$T" "undo multi-3: first entity removed"

    EMPTY_CHECK=$(grep -c "id:" "$T" || true)
    if [ "$EMPTY_CHECK" -eq 0 ]; then
        pass "undo multi-3: template entities list is empty"
    else
        fail "undo multi-3: expected empty entities list, found $EMPTY_CHECK id entries"
    fi

    rm -rf "$(dirname "$T")"
}

# ── redo ───────────────────────────────────────────────────────────────────

echo ""
echo "=== redo ==="
{
    T=$(new_template world)

    # Redo on empty history — exits 0, reports nothing_to_redo
    REDO_EMPTY=$($SILM template redo --template "$T" 2>&1)
    assert_json_contains "$REDO_EMPTY" 'nothing_to_redo' "redo empty: reports nothing_to_redo"
    assert_ok "$SILM template redo --template '$T'" "redo empty: exits 0"

    # Create, undo, redo
    $SILM template entity --template "$T" create --name "Phoenix" > /dev/null 2>&1
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Phoenix" "$T" "redo setup: entity absent after undo"

    REDO_OUT=$($SILM template redo --template "$T" 2>&1)
    assert_json_contains "$REDO_OUT" '"ok":true' "redo: JSON has ok:true"
    assert_json_contains "$REDO_OUT" 'redone_action_id' "redo: JSON has redone_action_id field"
    assert_in_file "Phoenix" "$T" "redo: entity restored after redo"

    # Multiple undos then multiple redos
    $SILM template entity --template "$T" create --name "One" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Two" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Three" > /dev/null 2>&1

    $SILM template undo --template "$T" > /dev/null 2>&1
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Three" "$T" "redo multi setup: Three removed"
    assert_not_in_file "Two" "$T" "redo multi setup: Two removed"
    assert_in_file "One" "$T" "redo multi setup: One still present"

    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "Two" "$T" "redo multi-1: Two restored"
    assert_not_in_file "Three" "$T" "redo multi-1: Three still absent"

    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "Three" "$T" "redo multi-2: Three restored"

    # New command after undo clears redo stack
    $SILM template undo --template "$T" > /dev/null 2>&1  # undo Three
    $SILM template entity --template "$T" create --name "Interrupter" > /dev/null 2>&1
    REDO_CLEARED=$($SILM template redo --template "$T" 2>&1)
    assert_json_contains "$REDO_CLEARED" 'nothing_to_redo' "redo after new command: redo stack cleared"
    assert_not_in_file "Three" "$T" "redo after new command: Three remains absent (not redo-able)"

    rm -rf "$(dirname "$T")"
}

# ── history ────────────────────────────────────────────────────────────────

echo ""
echo "=== history ==="
{
    T=$(new_template world)

    # Empty history
    HIST_EMPTY=$($SILM template history --template "$T" 2>&1)
    assert_json_contains "$HIST_EMPTY" '\[\]' "history empty: returns empty JSON array"

    # After several commands, history contains entries with action_id and description
    $SILM template entity --template "$T" create --name "Alpha" > /dev/null 2>&1
    $SILM template entity --template "$T" create --name "Beta" > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Health '{"max":100}' > /dev/null 2>&1
    $SILM template entity --template "$T" rename 2 "Beta2" > /dev/null 2>&1

    HIST=$($SILM template history --template "$T" 2>&1)
    assert_json_contains "$HIST" '"action_id"' "history: entries have action_id field"
    assert_json_contains "$HIST" '"description"' "history: entries have description field"
    assert_json_contains "$HIST" 'Alpha' "history: contains Alpha create entry"
    assert_json_contains "$HIST" 'Beta' "history: contains Beta create entry"
    assert_json_contains "$HIST" 'Health' "history: contains Health add entry"
    assert_json_contains "$HIST" 'Beta2' "history: contains rename entry"

    # Action IDs should be numeric
    assert_json_contains "$HIST" '"action_id": 0' "history: first action_id is 0"
    assert_json_contains "$HIST" '"action_id": 1' "history: second action_id is 1"
    assert_json_contains "$HIST" '"action_id": 2' "history: third action_id is 2"
    assert_json_contains "$HIST" '"action_id": 3' "history: fourth action_id is 3"

    # After undo, history reflects the current undo stack (truncates the undone action)
    $SILM template undo --template "$T" > /dev/null 2>&1
    HIST_AFTER_UNDO=$($SILM template history --template "$T" 2>&1)
    assert_json_contains "$HIST_AFTER_UNDO" '"action_id": 2' "history after undo: shows actions up to last committed action"
    assert_json_not_contains "$HIST_AFTER_UNDO" '"action_id": 3' "history after undo: undone action no longer in history"

    rm -rf "$(dirname "$T")"
}

# ── undo/redo sequences with components ───────────────────────────────────

echo ""
echo "=== undo/redo sequences with components ==="
{
    T=$(new_template world)

    # Build up state: entity + 2 components
    $SILM template entity --template "$T" create --name "Tank" > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Health '{"max":500}' > /dev/null 2>&1
    $SILM template component --template "$T" add 1 Armor '{"value":50}' > /dev/null 2>&1

    assert_in_file "type_name: Health" "$T" "seq: Health present after add"
    assert_in_file "type_name: Armor" "$T" "seq: Armor present after add"

    # Undo Armor add
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_in_file "type_name: Health" "$T" "seq undo-1: Health still present"
    assert_not_in_file "type_name: Armor" "$T" "seq undo-1: Armor removed by undo"

    # Undo Health add
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "type_name: Health" "$T" "seq undo-2: Health removed by undo"

    # Undo entity create
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Tank" "$T" "seq undo-3: entity removed by undo"

    # Redo all three
    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "Tank" "$T" "seq redo-1: entity restored"
    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "type_name: Health" "$T" "seq redo-2: Health restored"
    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "type_name: Armor" "$T" "seq redo-3: Armor restored"

    # Component set and undo
    $SILM template component --template "$T" set 1 Health '{"max":1000}' > /dev/null 2>&1
    assert_in_file "1000" "$T" "seq set: new value present"
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "1000" "$T" "seq undo set: new value removed"
    assert_in_file "500" "$T" "seq undo set: old value restored"

    # Component remove and undo
    $SILM template component --template "$T" remove 1 Health > /dev/null 2>&1
    assert_not_in_file "type_name: Health" "$T" "seq remove: Health absent"
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_in_file "type_name: Health" "$T" "seq undo remove: Health restored"

    rm -rf "$(dirname "$T")"
}

# ── undo/redo sequences with entity operations ────────────────────────────

echo ""
echo "=== undo/redo sequences with entity operations ==="
{
    T=$(new_template world)

    # Rename then undo
    $SILM template entity --template "$T" create --name "Original" > /dev/null 2>&1
    $SILM template entity --template "$T" rename 1 "Modified" > /dev/null 2>&1
    assert_in_file "Modified" "$T" "seq entity: rename applied"
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_in_file "Original" "$T" "seq entity undo rename: original name restored"
    assert_not_in_file "Modified" "$T" "seq entity undo rename: modified name absent"

    # Redo rename
    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "Modified" "$T" "seq entity redo rename: renamed name restored"

    # Duplicate then undo
    $SILM template entity --template "$T" duplicate 1 > /dev/null 2>&1
    assert_in_file "Modified (copy)" "$T" "seq entity: duplicate present"
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_not_in_file "Modified (copy)" "$T" "seq entity undo duplicate: copy removed"
    assert_in_file "Modified" "$T" "seq entity undo duplicate: original still present"

    # Redo duplicate
    $SILM template redo --template "$T" > /dev/null 2>&1
    assert_in_file "Modified (copy)" "$T" "seq entity redo duplicate: copy restored"

    # Delete then undo
    $SILM template entity --template "$T" delete 2 > /dev/null 2>&1
    assert_not_in_file "Modified (copy)" "$T" "seq entity delete: copy deleted"
    $SILM template undo --template "$T" > /dev/null 2>&1
    assert_in_file "Modified (copy)" "$T" "seq entity undo delete: copy restored after undo"

    rm -rf "$(dirname "$T")"
}

# ── Summary ────────────────────────────────────────────────────────────────

echo ""
echo "══════════════════════════════════════════"
echo "  Results: $PASS passed, $FAIL failed"
echo "══════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
