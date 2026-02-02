use engine_assets::MeshData;
use glam::{Vec2, Vec3};

// ============================================================================
// glTF Loader Tests (TDD - write tests first!)
// ============================================================================

#[test]
fn test_gltf_load_simple_triangle() {
    // Simple glTF with a single triangle
    // This test will fail until we implement from_gltf
    let gltf_data = include_bytes!("../test_data/triangle.gltf");
    let gltf_bin = include_bytes!("../test_data/triangle.bin");

    let result = MeshData::from_gltf(gltf_data, Some(gltf_bin.as_slice()));
    assert!(result.is_ok(), "Failed to load simple glTF triangle");

    let mesh = result.unwrap();
    assert_eq!(mesh.vertex_count(), 3, "Triangle should have 3 vertices");
    assert_eq!(mesh.index_count(), 3, "Triangle should have 3 indices");
    assert_eq!(mesh.triangle_count(), 1, "Should have 1 triangle");
}

#[test]
fn test_gltf_load_cube() {
    // More complex glTF with a cube
    let gltf_data = include_bytes!("../test_data/cube.gltf");
    let gltf_bin = include_bytes!("../test_data/cube.bin");

    let result = MeshData::from_gltf(gltf_data, Some(gltf_bin.as_slice()));
    assert!(result.is_ok(), "Failed to load glTF cube");

    let mesh = result.unwrap();
    // Cube should have 24 vertices (6 faces * 4 vertices, no sharing due to normals)
    assert!(mesh.vertex_count() >= 8, "Cube should have at least 8 vertices");
    assert!(mesh.index_count() >= 36, "Cube should have at least 36 indices");
    assert_eq!(mesh.triangle_count(), mesh.index_count() / 3);
}

#[test]
fn test_gltf_vertex_data_integrity() {
    // Ensure vertex data is correctly parsed
    let gltf_data = include_bytes!("../test_data/triangle.gltf");
    let gltf_bin = include_bytes!("../test_data/triangle.bin");

    let mesh = MeshData::from_gltf(gltf_data, Some(gltf_bin.as_slice())).unwrap();

    // Check that vertices have valid data (not all zeros)
    let has_valid_positions = mesh.vertices.iter().any(|v| v.position != Vec3::ZERO);
    assert!(has_valid_positions, "Should have non-zero positions");
}

#[test]
fn test_gltf_invalid_data() {
    // Test error handling with invalid data
    let invalid_data = b"not a valid gltf file";
    let result = MeshData::from_gltf(invalid_data, None);
    assert!(result.is_err(), "Should fail on invalid glTF data");
}

#[test]
fn test_gltf_embedded_buffers() {
    // Test glTF with embedded (base64) buffers
    let gltf_data = include_bytes!("../test_data/triangle_embedded.gltf");

    let result = MeshData::from_gltf(gltf_data, None);
    assert!(result.is_ok(), "Should load glTF with embedded buffers");

    let mesh = result.unwrap();
    assert_eq!(mesh.vertex_count(), 3);
}

// ============================================================================
// Binary Format Tests (TDD - write tests first!)
// ============================================================================

#[test]
fn test_binary_serialize_triangle() {
    let original = MeshData::triangle();

    let binary = original.to_binary();
    assert!(!binary.is_empty(), "Binary data should not be empty");

    // Check magic number
    assert_eq!(&binary[0..4], b"MESH", "Should start with MESH magic");
}

#[test]
fn test_binary_deserialize_triangle() {
    let original = MeshData::triangle();
    let binary = original.to_binary();

    let result = MeshData::from_binary(&binary);
    assert!(result.is_ok(), "Should deserialize valid binary");

    let deserialized = result.unwrap();
    assert_eq!(deserialized.vertex_count(), original.vertex_count());
    assert_eq!(deserialized.index_count(), original.index_count());
}

#[test]
fn test_binary_roundtrip_cube() {
    let original = MeshData::cube();

    // Serialize
    let binary = original.to_binary();

    // Deserialize
    let deserialized = MeshData::from_binary(&binary).unwrap();

    // Verify roundtrip
    assert_eq!(deserialized.vertex_count(), original.vertex_count());
    assert_eq!(deserialized.index_count(), original.index_count());

    // Check vertex data integrity
    for (i, (orig, deser)) in original.vertices.iter().zip(deserialized.vertices.iter()).enumerate()
    {
        assert_eq!(orig.position, deser.position, "Position mismatch at vertex {}", i);
        assert_eq!(orig.normal, deser.normal, "Normal mismatch at vertex {}", i);
        assert_eq!(orig.uv, deser.uv, "UV mismatch at vertex {}", i);
    }

    // Check index data integrity
    assert_eq!(original.indices, deserialized.indices);
}

#[test]
fn test_binary_roundtrip_large_mesh() {
    // Create a large mesh for testing
    let mut mesh = MeshData::with_capacity(1000, 3000);

    for i in 0..1000 {
        let t = i as f32 / 1000.0;
        mesh.vertices.push(engine_assets::Vertex::new(
            Vec3::new(t, t * 2.0, t * 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(t, 1.0 - t),
        ));
    }

    for i in 0..999 {
        mesh.indices.push(i as u32);
        mesh.indices.push((i + 1) as u32);
        mesh.indices.push(0);
    }

    // Roundtrip test
    let binary = mesh.to_binary();
    let deserialized = MeshData::from_binary(&binary).unwrap();

    assert_eq!(deserialized.vertex_count(), mesh.vertex_count());
    assert_eq!(deserialized.index_count(), mesh.index_count());
}

#[test]
fn test_binary_invalid_magic() {
    let invalid_data = b"INVL\x01\x00\x00\x00\x03\x00\x00\x00\x03\x00\x00\x00";
    let result = MeshData::from_binary(invalid_data);
    assert!(result.is_err(), "Should reject invalid magic number");
}

#[test]
fn test_binary_truncated_data() {
    let original = MeshData::cube();
    let mut binary = original.to_binary();

    // Truncate the data
    binary.truncate(binary.len() / 2);

    let result = MeshData::from_binary(&binary);
    assert!(result.is_err(), "Should reject truncated data");
}

#[test]
fn test_binary_format_version() {
    let mesh = MeshData::triangle();
    let binary = mesh.to_binary();

    // Check version (bytes 4-7)
    let version = u32::from_le_bytes([binary[4], binary[5], binary[6], binary[7]]);
    assert_eq!(version, 1, "Binary format version should be 1");
}

#[test]
fn test_binary_empty_mesh() {
    let empty = MeshData::new();

    let binary = empty.to_binary();
    let deserialized = MeshData::from_binary(&binary).unwrap();

    assert_eq!(deserialized.vertex_count(), 0);
    assert_eq!(deserialized.index_count(), 0);
}

// ============================================================================
// Performance Comparison Tests
// ============================================================================

#[test]
fn test_binary_smaller_than_obj() {
    let mesh = MeshData::cube();

    // Create OBJ representation (approximate)
    let obj_size = estimate_obj_size(&mesh);
    let binary_size = mesh.to_binary().len();

    // Binary should be significantly smaller
    assert!(
        binary_size < obj_size,
        "Binary format ({} bytes) should be smaller than OBJ (~{} bytes)",
        binary_size,
        obj_size
    );
}

fn estimate_obj_size(mesh: &MeshData) -> usize {
    // Rough estimate: each vertex line ~30 bytes, each face line ~20 bytes
    mesh.vertex_count() * 30 + mesh.index_count() / 3 * 20
}

// ============================================================================
// FBX Loader Tests (feature-gated)
// ============================================================================

#[cfg(feature = "fbx-support")]
#[test]
fn test_fbx_load_simple() {
    let fbx_data = include_bytes!("../test_data/cube.fbx");

    let result = MeshData::from_fbx(fbx_data);
    assert!(result.is_ok(), "Should load FBX file");

    let mesh = result.unwrap();
    assert!(mesh.vertex_count() > 0, "FBX mesh should have vertices");
    assert!(mesh.index_count() > 0, "FBX mesh should have indices");
}

#[cfg(feature = "fbx-support")]
#[test]
fn test_fbx_invalid_data() {
    let invalid_data = b"not an fbx file";
    let result = MeshData::from_fbx(invalid_data);
    assert!(result.is_err(), "Should reject invalid FBX data");
}

// ============================================================================
// Integration Tests - Compare All Loaders
// ============================================================================

#[test]
fn test_all_loaders_produce_valid_meshes() {
    // Test that all loaders produce meshes that pass validation

    // OBJ
    let obj = r#"
        v 0.0 -0.5 0.0
        v 0.5 0.5 0.0
        v -0.5 0.5 0.0
        f 1 2 3
    "#;
    let obj_mesh = MeshData::from_obj(obj).unwrap();
    assert!(is_valid_mesh(&obj_mesh));

    // Binary (roundtrip)
    let binary = obj_mesh.to_binary();
    let binary_mesh = MeshData::from_binary(&binary).unwrap();
    assert!(is_valid_mesh(&binary_mesh));

    // glTF would go here but requires test files
}

fn is_valid_mesh(mesh: &MeshData) -> bool {
    // Basic validation
    if mesh.vertices.is_empty() {
        return false;
    }

    if mesh.indices.is_empty() {
        return false;
    }

    // All indices should be valid
    let max_index = mesh.vertex_count() as u32;
    if mesh.indices.iter().any(|&idx| idx >= max_index) {
        return false;
    }

    // Should have multiple of 3 indices (triangles)
    if mesh.indices.len() % 3 != 0 {
        return false;
    }

    true
}
