#!/usr/bin/env bash
# E2E test: silm template entity undo/redo round-trip
# Requires: pre-built silm at target/debug/silm (run cargo build --bin silm first)
set -euo pipefail

SILM="./target/debug/silm"
TMPDIR=$(mktemp -d)
TEMPLATE="$TMPDIR/world.yaml"

printf 'name: world\nentities: []\n' > "$TEMPLATE"

echo "=== Creating entity ==="
$SILM template entity --template "$TEMPLATE" create --name "Hero"

grep -q "Hero" "$TEMPLATE" || { echo "FAIL: Hero not in YAML after create"; exit 1; }
echo "PASS: entity present after create"

echo "=== Undoing create ==="
$SILM template undo --template "$TEMPLATE"

grep -q "Hero" "$TEMPLATE" && { echo "FAIL: Hero still in YAML after undo"; exit 1; } || true
echo "PASS: entity absent after undo"

echo "=== Redoing create ==="
$SILM template redo --template "$TEMPLATE"

grep -q "Hero" "$TEMPLATE" || { echo "FAIL: Hero not in YAML after redo"; exit 1; }
echo "PASS: entity present after redo"

echo "=== All E2E tests passed ==="
rm -rf "$TMPDIR"
