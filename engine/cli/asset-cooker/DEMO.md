# Asset Cooker Demo

This demo shows all commands of the asset-cooker CLI tool.

## Setup

```bash
# Build the tool
cargo build -p asset-cooker

# The binary will be at:
# target/debug/asset-cooker (or asset-cooker.exe on Windows)
```

## Demo 1: Generate Procedural Assets

```bash
# Generate a cube mesh
asset-cooker generate mesh cube 2.0 --output demo/cube.mesh

# Generate a sphere
asset-cooker generate mesh sphere 1.0 16 32 --output demo/sphere.mesh

# Generate a plane
asset-cooker generate mesh plane 10.0 10.0 5 5 --output demo/plane.mesh

# Generate a cylinder
asset-cooker generate mesh cylinder 1.0 2.0 16 --output demo/cylinder.mesh

# Generate textures
asset-cooker generate texture checkerboard 256 256 32 --output demo/checker.texture
asset-cooker generate texture gradient 256 256 --output demo/gradient.texture
asset-cooker generate texture noise 256 256 0.1 --output demo/noise.texture

# Generate audio
asset-cooker generate audio sine 440.0 1.0 --output demo/tone.audio
asset-cooker generate audio whitenoise 0.5 --output demo/noise.audio
```

## Demo 2: Validate Assets

```bash
# Validate generated assets
asset-cooker validate demo/cube.mesh
asset-cooker validate demo/checker.texture

# Expected output:
# ✓ Asset validation PASSED: demo/cube.mesh
```

## Demo 3: Display Asset Info

```bash
# Show mesh information
asset-cooker info demo/cube.mesh

# Expected output:
# ═══════════════════════════════════════
# Asset Information
# ═══════════════════════════════════════
# Path:      demo/cube.mesh
# Type:      Mesh
# Size:      XXX bytes
# Mesh Details:
#   Vertices:  24
#   Indices:   36
#   Triangles: 12
#   Bounding Box: ...
# ═══════════════════════════════════════

# Show texture information
asset-cooker info demo/checker.texture

# Expected output:
# Texture Details:
#   Dimensions: 256x256
#   Format:     RGBA8Unorm
#   Mip Levels: 1
#   Memory:     XXX bytes
```

## Demo 4: Cook Raw Assets

```bash
# Create test OBJ file
mkdir -p demo/raw
cat > demo/raw/test.obj <<EOF
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
EOF

# Cook the assets
asset-cooker cook demo/raw demo/cooked

# Cook with mipmaps for textures
asset-cooker cook demo/raw demo/cooked --generate-mipmaps

# Cook recursively
asset-cooker cook demo/raw demo/cooked --recursive
```

## Demo 5: Create Asset Bundles

```bash
# Create manifest file
cat > demo/manifest.yaml <<EOF
version: 1
assets:
  - id: "cube_mesh_001"
    path: "cooked/cube.mesh"
    asset_type: Mesh
    size_bytes: 1024
    checksum: "0000000000000000000000000000000000000000000000000000000000000000"
    dependencies: []
EOF

# Create bundle
asset-cooker bundle demo/manifest.yaml demo/game.bundle --compression none
asset-cooker bundle demo/manifest.yaml demo/game.bundle --compression zstd
```

## Performance Benchmarks

Expected performance (development build):

- **Mesh generation (cube):** < 1ms
- **Mesh generation (sphere 32x64):** < 10ms
- **Texture generation (256x256):** < 5ms
- **Audio generation (1s sine):** < 10ms
- **Validation (mesh):** < 1ms
- **Info display:** < 5ms
- **Cooking (simple asset):** < 50ms
- **Bundle creation (10 assets):** < 100ms

Release builds are significantly faster.

## Integration Example

```bash
#!/bin/bash
# Full asset pipeline

# 1. Generate test assets
asset-cooker generate mesh cube 1.0 --output raw/cube.obj
asset-cooker generate texture checkerboard 512 512 64 --output raw/checker.png

# 2. Cook assets
asset-cooker cook raw/ cooked/ --recursive --generate-mipmaps

# 3. Validate cooked assets
for file in cooked/*.mesh; do
    asset-cooker validate "$file"
done

for file in cooked/*.texture; do
    asset-cooker validate "$file"
done

# 4. Create manifest (manual or scripted)
# ... create manifest.yaml ...

# 5. Bundle assets
asset-cooker bundle manifest.yaml game.bundle --compression zstd

echo "Asset pipeline complete: game.bundle"
```

## Error Handling

The tool provides clear error messages:

```bash
# Nonexistent file
asset-cooker validate nonexistent.mesh
# Error: Asset file not found: nonexistent.mesh

# Invalid format
asset-cooker validate corrupted.mesh
# Error: ✗ Asset validation FAILED: corrupted.mesh
#   Error: Invalid magic number

# Missing parameters
asset-cooker generate mesh
# Error: Missing mesh type. Usage: generate mesh <type> [params...]
```

## Verbose Mode

```bash
# Enable detailed logging
asset-cooker --verbose cook raw/ cooked/

# Shows:
# - Each file being processed
# - Timing information
# - Detailed error traces
```

## Help

```bash
# Show all commands
asset-cooker --help

# Show help for specific command
asset-cooker cook --help
asset-cooker generate --help
```
