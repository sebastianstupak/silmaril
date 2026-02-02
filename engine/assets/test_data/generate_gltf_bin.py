#!/usr/bin/env python3
"""Generate binary data for glTF test files."""

import struct

def generate_triangle_bin():
    """Generate triangle.bin for triangle.gltf"""
    data = bytearray()

    # POSITION (3 vertices * 3 floats = 36 bytes)
    positions = [
        (0.0, -0.5, 0.0),
        (0.5, 0.5, 0.0),
        (-0.5, 0.5, 0.0),
    ]
    for pos in positions:
        data.extend(struct.pack('fff', *pos))

    # NORMAL (3 vertices * 3 floats = 36 bytes)
    normals = [
        (0.0, 0.0, 1.0),
        (0.0, 0.0, 1.0),
        (0.0, 0.0, 1.0),
    ]
    for normal in normals:
        data.extend(struct.pack('fff', *normal))

    # TEXCOORD_0 (3 vertices * 2 floats = 24 bytes)
    uvs = [
        (0.5, 1.0),
        (1.0, 0.0),
        (0.0, 0.0),
    ]
    for uv in uvs:
        data.extend(struct.pack('ff', *uv))

    # INDICES (3 indices * 4 bytes = 12 bytes)
    indices = [0, 1, 2]
    for idx in indices:
        data.extend(struct.pack('I', idx))

    assert len(data) == 108, f"Expected 108 bytes, got {len(data)}"

    with open('triangle.bin', 'wb') as f:
        f.write(data)
    print(f"Generated triangle.bin ({len(data)} bytes)")

def generate_cube_bin():
    """Generate cube.bin for cube.gltf"""
    data = bytearray()

    # Cube with 24 vertices (6 faces, no sharing)
    positions = [
        # Front face (Z+)
        (-1.0, -1.0, 1.0), (1.0, -1.0, 1.0), (1.0, 1.0, 1.0), (-1.0, 1.0, 1.0),
        # Back face (Z-)
        (1.0, -1.0, -1.0), (-1.0, -1.0, -1.0), (-1.0, 1.0, -1.0), (1.0, 1.0, -1.0),
        # Top face (Y+)
        (-1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, -1.0), (-1.0, 1.0, -1.0),
        # Bottom face (Y-)
        (-1.0, -1.0, -1.0), (1.0, -1.0, -1.0), (1.0, -1.0, 1.0), (-1.0, -1.0, 1.0),
        # Right face (X+)
        (1.0, -1.0, 1.0), (1.0, -1.0, -1.0), (1.0, 1.0, -1.0), (1.0, 1.0, 1.0),
        # Left face (X-)
        (-1.0, -1.0, -1.0), (-1.0, -1.0, 1.0), (-1.0, 1.0, 1.0), (-1.0, 1.0, -1.0),
    ]

    normals = [
        # Front
        (0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0),
        # Back
        (0.0, 0.0, -1.0), (0.0, 0.0, -1.0), (0.0, 0.0, -1.0), (0.0, 0.0, -1.0),
        # Top
        (0.0, 1.0, 0.0), (0.0, 1.0, 0.0), (0.0, 1.0, 0.0), (0.0, 1.0, 0.0),
        # Bottom
        (0.0, -1.0, 0.0), (0.0, -1.0, 0.0), (0.0, -1.0, 0.0), (0.0, -1.0, 0.0),
        # Right
        (1.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 0.0, 0.0),
        # Left
        (-1.0, 0.0, 0.0), (-1.0, 0.0, 0.0), (-1.0, 0.0, 0.0), (-1.0, 0.0, 0.0),
    ]

    uvs = [
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
        (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0),
    ]

    # Write positions
    for pos in positions:
        data.extend(struct.pack('fff', *pos))

    # Write normals
    for normal in normals:
        data.extend(struct.pack('fff', *normal))

    # Write UVs
    for uv in uvs:
        data.extend(struct.pack('ff', *uv))

    # Indices (36 indices for 12 triangles)
    indices = []
    for face in range(6):
        base = face * 4
        indices.extend([base, base+1, base+2, base+2, base+3, base])

    for idx in indices:
        data.extend(struct.pack('I', idx))

    expected = 24 * 3 * 4 + 24 * 3 * 4 + 24 * 2 * 4 + 36 * 4
    assert len(data) == expected, f"Expected {expected} bytes, got {len(data)}"

    with open('cube.bin', 'wb') as f:
        f.write(data)
    print(f"Generated cube.bin ({len(data)} bytes)")

if __name__ == '__main__':
    generate_triangle_bin()
    generate_cube_bin()
