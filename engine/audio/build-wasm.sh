#!/bin/bash
# Build script for Web Audio backend (WASM)

set -e

echo "=== Building engine-audio for WASM ==="

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack not found. Install with:"
    echo "  cargo install wasm-pack"
    exit 1
fi

# Build for web target
echo "Building for web target..."
wasm-pack build --target web engine/audio

echo ""
echo "=== Running WASM Tests ==="

# Run tests in headless browsers
echo "Testing in Firefox..."
wasm-pack test --headless --firefox

echo ""
echo "Testing in Chrome..."
wasm-pack test --headless --chrome

echo ""
echo "=== Build Complete ==="
echo "Output directory: engine/audio/pkg/"
echo ""
echo "To use in a web page:"
echo "  <script type=\"module\">"
echo "    import init, { AudioEngine } from './pkg/engine_audio.js';"
echo "    await init();"
echo "    const audio = AudioEngine.new();"
echo "  </script>"
