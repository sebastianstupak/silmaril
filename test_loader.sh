#!/bin/bash
set -e

echo "Building engine-assets with async feature..."
cd "D:\dev\agent-game-engine"

cargo build -p engine-assets --features async --quiet

echo "Running loader unit tests..."
cargo test -p engine-assets --lib loader --features async --quiet

echo "Running loader integration tests..."
cargo test -p engine-assets --test loader_tests --features async --quiet

echo "All tests passed!"
