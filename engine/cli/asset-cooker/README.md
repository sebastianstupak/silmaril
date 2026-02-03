# Asset Cooker CLI Tool

Command-line tool for asset pipeline: cooking, bundling, validation, and procedural generation.

## Installation

```bash
cargo build --release
```

The binary will be located at `target/release/asset-cooker` (or `asset-cooker.exe` on Windows).

## Usage

### Cook Assets

Convert raw assets to optimized binary formats:

```bash
# Cook all assets in directory
asset-cooker cook assets/raw/ assets/cooked/

# Cook recursively with mipmaps
asset-cooker cook assets/raw/ assets/cooked/ --recursive --generate-mipmaps

# Cook with mesh optimization
asset-cooker cook assets/raw/ assets/cooked/ --optimize-meshes
```

**Supported formats:**
- Meshes: `.obj`, `.gltf`, `.glb` → `.mesh`
- Textures: `.png`, `.jpg`, `.dds` → `.texture`
- Shaders: `.glsl`, `.vert`, `.frag`, `.spv` → copied as-is
- Audio: `.wav`, `.ogg`, `.mp3` → copied as-is
- Fonts: `.ttf`, `.otf` → copied as-is

### Create Bundles

Package assets into compressed bundles:

```bash
# Create bundle from manifest
asset-cooker bundle assets/manifest.yaml game.bundle

# With compression
asset-cooker bundle assets/manifest.yaml game.bundle --compression zstd
asset-cooker bundle assets/manifest.yaml game.bundle --compression lz4
asset-cooker bundle assets/manifest.yaml game.bundle --compression none
```

**Manifest format (YAML):**
```yaml
version: 1
assets:
  - id: "mesh/cube"
    path: "meshes/cube.mesh"
    asset_type: Mesh
    size_bytes: 1024
    checksum: "abc123..."
    dependencies: []
```

### Validate Assets

Check asset integrity:

```bash
# Validate single asset
asset-cooker validate assets/cube.mesh

# Validate will check:
# - Format magic numbers
# - Version compatibility
# - Data integrity (NaN, bounds)
# - Checksums (for binary formats)
```

### Display Info

Show asset metadata:

```bash
asset-cooker info assets/cube.mesh
# Outputs:
# Asset Information
# Path:      assets/cube.mesh
# Type:      Mesh
# Size:      1024 bytes (1.00 KB)
# Mesh Details:
#   Vertices:  24
#   Indices:   36
#   Triangles: 12
#   Bounding Box: ...
```

### Generate Procedural Assets

Create assets programmatically:

#### Meshes

```bash
# Cube
asset-cooker generate mesh cube 2.0 --output cube.mesh

# Sphere
asset-cooker generate mesh sphere 1.0 16 32 --output sphere.mesh
# Parameters: radius subdivisions_lat subdivisions_lon

# Plane
asset-cooker generate mesh plane 10.0 10.0 5 5 --output plane.mesh
# Parameters: width height subdivisions_x subdivisions_y

# Cylinder
asset-cooker generate mesh cylinder 1.0 2.0 16 --output cylinder.mesh
# Parameters: radius height segments
```

#### Textures

```bash
# Checkerboard
asset-cooker generate texture checkerboard 256 256 32 --output checker.texture
# Parameters: width height tile_size

# Gradient
asset-cooker generate texture gradient 256 256 --output gradient.texture
# Parameters: width height

# Noise
asset-cooker generate texture noise 256 256 0.1 --output noise.texture
# Parameters: width height scale
```

#### Audio

```bash
# Sine wave
asset-cooker generate audio sine 440.0 1.0 --output tone.audio
# Parameters: frequency duration_secs

# White noise
asset-cooker generate audio whitenoise 1.0 --output noise.audio
# Parameters: duration_secs
```

## Examples

### Full Pipeline

```bash
# 1. Generate test assets
asset-cooker generate mesh cube 2.0 --output raw/cube.obj
asset-cooker generate texture checkerboard 256 256 32 --output raw/checker.png

# 2. Cook assets
asset-cooker cook raw/ cooked/ --recursive --generate-mipmaps

# 3. Create manifest (manual step - create YAML file)
# manifest.yaml:
# version: 1
# assets:
#   - id: <computed from content>
#     path: "cooked/cube.mesh"
#     asset_type: Mesh
#     size_bytes: 1024
#     checksum: <blake3 hash>

# 4. Bundle assets
asset-cooker bundle manifest.yaml game.bundle --compression zstd

# 5. Validate
asset-cooker validate cooked/cube.mesh
asset-cooker validate cooked/checker.texture
```

### Batch Validation

```bash
# Validate all cooked assets
find cooked/ -name "*.mesh" -exec asset-cooker validate {} \;
find cooked/ -name "*.texture" -exec asset-cooker validate {} \;
```

## Performance

Typical cooking times (single-threaded):

- **Mesh (1000 vertices):** < 10ms
- **Texture (1024x1024 PNG):** < 50ms
- **Texture with mipmaps:** < 200ms
- **Bundle creation (100 assets):** < 500ms

## Exit Codes

- `0` - Success
- `1` - Generic error
- See stderr for detailed error messages

## Logging

Enable verbose logging:

```bash
asset-cooker --verbose cook assets/raw/ assets/cooked/
# or
RUST_LOG=debug asset-cooker cook assets/raw/ assets/cooked/
```

Log levels:
- `error` - Errors only
- `warn` - Warnings and errors
- `info` - Standard output (default)
- `debug` - Detailed debug info
- `trace` - Very verbose

## Architecture

The asset cooker uses the `engine-assets` crate for all asset operations:

- **Cooking:** Load raw formats → Optimize → Save binary
- **Bundling:** Manifest + assets → Compressed bundle
- **Validation:** AssetValidator trait checks
- **Generation:** ProceduralAssetGenerator trait

All operations are deterministic - same input produces same output.

## See Also

- [Phase 1.7 Asset System Spec](../../../docs/tasks/phase1-7-asset-system.md)
- [Engine Assets Crate](../../assets/)
- [CLAUDE.md](../../../CLAUDE.md) - Development guidelines
