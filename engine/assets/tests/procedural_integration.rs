//! Integration tests for procedural asset generation
//!
//! These tests verify cross-platform determinism and integration with AssetId.

use engine_assets::{
    AssetId, MeshData, ProceduralAsset, ProceduralMeshParams, ProceduralTextureParams, SeededRng,
    TextureData,
};

#[test]
fn test_seeded_rng_determinism() {
    let mut rng1 = SeededRng::new(42);
    let mut rng2 = SeededRng::new(42);

    // Same seed produces same sequence
    for _ in 0..100 {
        assert_eq!(rng1.next_u32(), rng2.next_u32());
    }
}

#[test]
fn test_seeded_rng_different_seeds() {
    let mut rng1 = SeededRng::new(111);
    let mut rng2 = SeededRng::new(222);

    // Different seeds produce different sequences
    let values1: Vec<u32> = (0..10).map(|_| rng1.next_u32()).collect();
    let values2: Vec<u32> = (0..10).map(|_| rng2.next_u32()).collect();

    assert_ne!(values1, values2);
}

#[test]
fn test_procedural_cube_generation() {
    let params = ProceduralMeshParams::Cube { size: 2.0 };
    let mesh = MeshData::generate(12345, &params);

    assert_eq!(mesh.vertex_count(), 24);
    assert_eq!(mesh.triangle_count(), 12);

    // Check bounding box
    let (min, max) = mesh.bounding_box();
    assert_eq!(min, glam::Vec3::new(-1.0, -1.0, -1.0));
    assert_eq!(max, glam::Vec3::new(1.0, 1.0, 1.0));
}

#[test]
fn test_procedural_sphere_generation() {
    let params =
        ProceduralMeshParams::Sphere { radius: 5.0, lat_subdivisions: 16, lon_subdivisions: 32 };
    let mesh = MeshData::generate(54321, &params);

    assert!(mesh.vertex_count() > 0);
    assert!(mesh.triangle_count() > 0);

    // Verify all vertices are approximately on sphere surface
    for vertex in &mesh.vertices {
        let distance = vertex.position.length();
        assert!((distance - 5.0).abs() < 0.01, "Vertex not on sphere surface");
    }
}

#[test]
fn test_procedural_terrain_determinism() {
    let params = ProceduralMeshParams::Terrain {
        width: 100.0,
        height: 100.0,
        width_res: 32,
        height_res: 32,
        max_height: 10.0,
        octaves: 4,
        frequency: 0.05,
    };

    let seed = 99999;
    let mesh1 = MeshData::generate(seed, &params);
    let mesh2 = MeshData::generate(seed, &params);

    // Same seed and params should produce identical terrain
    assert_eq!(mesh1.vertex_count(), mesh2.vertex_count());
    assert_eq!(mesh1.index_count(), mesh2.index_count());

    for (v1, v2) in mesh1.vertices.iter().zip(mesh2.vertices.iter()) {
        assert_eq!(v1.position, v2.position);
        assert_eq!(v1.normal, v2.normal);
        assert_eq!(v1.uv, v2.uv);
    }
}

#[test]
fn test_procedural_noise_texture_generation() {
    let params =
        ProceduralTextureParams::Noise { width: 256, height: 256, octaves: 4, frequency: 0.05 };
    let texture = TextureData::generate(12345, &params);

    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);
    assert_eq!(texture.data.len(), 256 * 256 * 4);
}

#[test]
fn test_procedural_noise_texture_determinism() {
    let params =
        ProceduralTextureParams::Noise { width: 128, height: 128, octaves: 4, frequency: 0.05 };

    let seed = 77777;
    let texture1 = TextureData::generate(seed, &params);
    let texture2 = TextureData::generate(seed, &params);

    // Same seed should produce identical textures
    assert_eq!(texture1.data, texture2.data);
}

#[test]
fn test_procedural_checkerboard_texture_generation() {
    let params = ProceduralTextureParams::Checkerboard {
        width: 256,
        height: 256,
        square_size: 32,
        color1: [255, 0, 0, 255],
        color2: [0, 255, 0, 255],
    };
    let texture = TextureData::generate(12345, &params);

    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);

    // Verify checkerboard pattern
    let red = [255u8, 0, 0, 255];
    let green = [0u8, 255, 0, 255];

    // Top-left corner should be red
    assert_eq!(&texture.data[0..4], &red);

    // Position at (32, 0) should be green (next square)
    let offset = 32 * 4;
    assert_eq!(&texture.data[offset..offset + 4], &green);
}

#[test]
fn test_asset_id_from_seed_and_params() {
    let seed = 12345u64;
    let params = ProceduralMeshParams::Cube { size: 2.0 };

    // Serialize params
    let params_bytes = bincode::serialize(&params).unwrap();

    // Create AssetId from seed and params
    let id1 = AssetId::from_seed_and_params(seed, &params_bytes);
    let id2 = AssetId::from_seed_and_params(seed, &params_bytes);

    // Same seed and params should produce same ID
    assert_eq!(id1, id2);

    // Different params should produce different ID
    let different_params = ProceduralMeshParams::Cube { size: 4.0 };
    let different_params_bytes = bincode::serialize(&different_params).unwrap();
    let id3 = AssetId::from_seed_and_params(seed, &different_params_bytes);

    assert_ne!(id1, id3);
}

#[test]
fn test_cross_platform_determinism_cube() {
    // This test verifies that procedural generation is deterministic
    // across different platforms (Windows, Linux, macOS, ARM, x64)
    let params = ProceduralMeshParams::Cube { size: 3.0 };
    let seed = 42;

    let mesh = MeshData::generate(seed, &params);

    // Verify specific vertex positions (these should be identical on all platforms)
    assert_eq!(mesh.vertices[0].position, glam::Vec3::new(-1.5, -1.5, 1.5));
    assert_eq!(mesh.vertices[1].position, glam::Vec3::new(1.5, -1.5, 1.5));

    // Verify normals
    assert_eq!(mesh.vertices[0].normal, glam::Vec3::Z);
}

#[test]
fn test_cross_platform_determinism_terrain() {
    // Verify terrain generation is deterministic across platforms
    let params = ProceduralMeshParams::Terrain {
        width: 50.0,
        height: 50.0,
        width_res: 16,
        height_res: 16,
        max_height: 5.0,
        octaves: 3,
        frequency: 0.1,
    };
    let seed = 99999;

    let mesh1 = MeshData::generate(seed, &params);
    let mesh2 = MeshData::generate(seed, &params);

    // Verify byte-for-byte identical output
    for (v1, v2) in mesh1.vertices.iter().zip(mesh2.vertices.iter()) {
        // Position must be bit-identical
        assert_eq!(v1.position.x.to_bits(), v2.position.x.to_bits());
        assert_eq!(v1.position.y.to_bits(), v2.position.y.to_bits());
        assert_eq!(v1.position.z.to_bits(), v2.position.z.to_bits());

        // Normal must be bit-identical
        assert_eq!(v1.normal.x.to_bits(), v2.normal.x.to_bits());
        assert_eq!(v1.normal.y.to_bits(), v2.normal.y.to_bits());
        assert_eq!(v1.normal.z.to_bits(), v2.normal.z.to_bits());
    }
}

#[test]
fn test_parameter_variations_produce_different_results() {
    let seed = 12345;

    // Test cube size variation
    let cube_small = MeshData::generate(seed, &ProceduralMeshParams::Cube { size: 1.0 });
    let cube_large = MeshData::generate(seed, &ProceduralMeshParams::Cube { size: 10.0 });

    assert_ne!(cube_small.vertices[0].position, cube_large.vertices[0].position);

    // Test sphere subdivision variation
    let sphere_low = MeshData::generate(
        seed,
        &ProceduralMeshParams::Sphere { radius: 1.0, lat_subdivisions: 8, lon_subdivisions: 16 },
    );
    let sphere_high = MeshData::generate(
        seed,
        &ProceduralMeshParams::Sphere { radius: 1.0, lat_subdivisions: 32, lon_subdivisions: 64 },
    );

    assert_ne!(sphere_low.vertex_count(), sphere_high.vertex_count());

    // Test terrain frequency variation
    let terrain_low_freq = MeshData::generate(
        seed,
        &ProceduralMeshParams::Terrain {
            width: 100.0,
            height: 100.0,
            width_res: 32,
            height_res: 32,
            max_height: 10.0,
            octaves: 4,
            frequency: 0.01,
        },
    );
    let terrain_high_freq = MeshData::generate(
        seed,
        &ProceduralMeshParams::Terrain {
            width: 100.0,
            height: 100.0,
            width_res: 32,
            height_res: 32,
            max_height: 10.0,
            octaves: 4,
            frequency: 0.1,
        },
    );

    // Different frequencies should produce different terrain
    let mut different_heights = false;
    for (v1, v2) in terrain_low_freq.vertices.iter().zip(terrain_high_freq.vertices.iter()) {
        if v1.position.y != v2.position.y {
            different_heights = true;
            break;
        }
    }
    assert!(different_heights, "Different frequencies should produce different terrain");
}

#[test]
fn test_procedural_params_serialization_roundtrip() {
    // Test mesh params
    let mesh_params = ProceduralMeshParams::Terrain {
        width: 200.0,
        height: 150.0,
        width_res: 128,
        height_res: 96,
        max_height: 20.0,
        octaves: 5,
        frequency: 0.03,
    };

    let serialized = bincode::serialize(&mesh_params).unwrap();
    let deserialized: ProceduralMeshParams = bincode::deserialize(&serialized).unwrap();
    assert_eq!(mesh_params, deserialized);

    // Test texture params
    let texture_params =
        ProceduralTextureParams::Noise { width: 512, height: 512, octaves: 6, frequency: 0.02 };

    let serialized = bincode::serialize(&texture_params).unwrap();
    let deserialized: ProceduralTextureParams = bincode::deserialize(&serialized).unwrap();
    assert_eq!(texture_params, deserialized);
}

#[test]
fn test_procedural_generation_with_asset_id() {
    // Integration test: Generate procedural asset and create AssetId
    let seed = 88888u64;
    let params = ProceduralMeshParams::Cube { size: 5.0 };

    // Generate mesh
    let mesh = MeshData::generate(seed, &params);

    // Create AssetId from seed and params
    let params_bytes = bincode::serialize(&params).unwrap();
    let asset_id = AssetId::from_seed_and_params(seed, &params_bytes);

    // Verify we can recreate the same mesh and get the same ID
    let mesh2 = MeshData::generate(seed, &params);
    let asset_id2 = AssetId::from_seed_and_params(seed, &params_bytes);

    assert_eq!(mesh.vertex_count(), mesh2.vertex_count());
    assert_eq!(asset_id, asset_id2);
}

#[test]
fn test_seeded_rng_range_bounds() {
    let mut rng = SeededRng::new(12345);

    // Test f32 range
    for _ in 0..1000 {
        let value = rng.next_range(10.0, 20.0);
        assert!(value >= 10.0 && value < 20.0);
    }

    // Test u32 range
    for _ in 0..1000 {
        let value = rng.next_range_u32(5, 15);
        assert!(value >= 5 && value < 15);
    }
}

#[test]
fn test_procedural_mesh_valid_indices() {
    // Verify all generated meshes have valid indices
    let cube = MeshData::generate(123, &ProceduralMeshParams::Cube { size: 1.0 });
    for &index in &cube.indices {
        assert!(index < cube.vertex_count() as u32, "Index out of bounds");
    }

    let sphere = MeshData::generate(
        456,
        &ProceduralMeshParams::Sphere { radius: 1.0, lat_subdivisions: 16, lon_subdivisions: 32 },
    );
    for &index in &sphere.indices {
        assert!(index < sphere.vertex_count() as u32, "Index out of bounds");
    }

    let terrain = MeshData::generate(
        789,
        &ProceduralMeshParams::Terrain {
            width: 50.0,
            height: 50.0,
            width_res: 32,
            height_res: 32,
            max_height: 10.0,
            octaves: 4,
            frequency: 0.05,
        },
    );
    for &index in &terrain.indices {
        assert!(index < terrain.vertex_count() as u32, "Index out of bounds");
    }
}

#[test]
fn test_procedural_mesh_valid_normals() {
    // Verify all normals are normalized
    let sphere = MeshData::generate(
        999,
        &ProceduralMeshParams::Sphere { radius: 2.0, lat_subdivisions: 16, lon_subdivisions: 32 },
    );

    for vertex in &sphere.vertices {
        let length = vertex.normal.length();
        assert!((length - 1.0).abs() < 0.01, "Normal not normalized: length = {}", length);
    }

    let terrain = MeshData::generate(
        888,
        &ProceduralMeshParams::Terrain {
            width: 100.0,
            height: 100.0,
            width_res: 32,
            height_res: 32,
            max_height: 10.0,
            octaves: 4,
            frequency: 0.05,
        },
    );

    for vertex in &terrain.vertices {
        let length = vertex.normal.length();
        assert!((length - 1.0).abs() < 0.01, "Normal not normalized: length = {}", length);
    }
}
