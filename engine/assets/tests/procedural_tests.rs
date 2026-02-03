//! Integration tests for procedural asset generation

use engine_assets::{
    AssetId, AudioData, AudioFormat, MeshData, ProceduralAssetGenerator, ProceduralAudioGenerator,
    ProceduralAudioParams, ProceduralMeshGenerator, ProceduralMeshParams,
    ProceduralTextureGenerator, ProceduralTextureParams, TextureData, TextureFormat,
};
use glam::Vec3;

// ============================================================================
// Mesh Generation Tests
// ============================================================================

#[test]
fn test_generate_cube_validate_vertices() {
    let generator = ProceduralMeshGenerator::new();
    let params = ProceduralMeshParams::Cube { size: Vec3::new(2.0, 2.0, 2.0) };

    let mesh = generator.generate(0, &params);

    // Validate basic properties
    assert_eq!(mesh.vertex_count(), 24); // 6 faces * 4 vertices
    assert_eq!(mesh.index_count(), 36); // 6 faces * 2 triangles * 3 indices
    assert_eq!(mesh.triangle_count(), 12); // 6 faces * 2 triangles

    // Validate vertices are valid (no NaN/Inf)
    for vertex in &mesh.vertices {
        assert!(vertex.position.is_finite());
        assert!(vertex.normal.is_finite());
        assert!(vertex.uv.is_finite());
    }

    // Validate indices are in bounds
    for &index in &mesh.indices {
        assert!((index as usize) < mesh.vertex_count());
    }
}

#[test]
fn test_generate_sphere_different_subdivisions() {
    let generator = ProceduralMeshGenerator::new();

    // Low detail sphere
    let params_low =
        ProceduralMeshParams::Sphere { radius: 1.0, subdivisions_lat: 8, subdivisions_lon: 16 };
    let mesh_low = generator.generate(0, &params_low);

    // High detail sphere
    let params_high =
        ProceduralMeshParams::Sphere { radius: 1.0, subdivisions_lat: 32, subdivisions_lon: 64 };
    let mesh_high = generator.generate(0, &params_high);

    // Higher subdivisions should have more vertices
    assert!(mesh_high.vertex_count() > mesh_low.vertex_count());
    assert!(mesh_high.index_count() > mesh_low.index_count());
}

#[test]
fn test_generate_plane_subdivisions() {
    let generator = ProceduralMeshGenerator::new();
    let params = ProceduralMeshParams::Plane {
        width: 10.0,
        height: 10.0,
        subdivisions_x: 5,
        subdivisions_y: 5,
    };

    let mesh = generator.generate(0, &params);

    // (subdivisions + 1)^2 vertices
    assert_eq!(mesh.vertex_count(), 6 * 6);

    // subdivisions^2 * 2 triangles * 3 indices
    assert_eq!(mesh.index_count(), 5 * 5 * 6);
}

#[test]
fn test_generate_cylinder_caps() {
    let generator = ProceduralMeshGenerator::new();
    let params = ProceduralMeshParams::Cylinder { radius: 1.0, height: 2.0, segments: 16 };

    let mesh = generator.generate(0, &params);

    // Should have vertices and indices
    assert!(mesh.vertex_count() > 0);
    assert!(mesh.index_count() > 0);

    // All vertices should be valid
    for vertex in &mesh.vertices {
        assert!(vertex.position.is_finite());
        assert!(vertex.normal.is_finite());
    }
}

#[test]
fn test_mesh_determinism_same_seed() {
    let generator = ProceduralMeshGenerator::new();
    let params =
        ProceduralMeshParams::Sphere { radius: 1.0, subdivisions_lat: 16, subdivisions_lon: 32 };

    let mesh1 = generator.generate(42, &params);
    let mesh2 = generator.generate(42, &params);

    // Should produce identical meshes
    assert_eq!(mesh1.vertex_count(), mesh2.vertex_count());
    assert_eq!(mesh1.index_count(), mesh2.index_count());

    for (v1, v2) in mesh1.vertices.iter().zip(mesh2.vertices.iter()) {
        assert_eq!(v1.position, v2.position);
        assert_eq!(v1.normal, v2.normal);
        assert_eq!(v1.uv, v2.uv);
    }

    assert_eq!(mesh1.indices, mesh2.indices);
}

#[test]
fn test_mesh_content_addressable_id_same_params() {
    let params = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };

    let id1 = ProceduralMeshGenerator::compute_id(123, &params);
    let id2 = ProceduralMeshGenerator::compute_id(123, &params);

    assert_eq!(id1, id2);
}

#[test]
fn test_mesh_content_addressable_id_different_seeds() {
    let params = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };

    let id1 = ProceduralMeshGenerator::compute_id(1, &params);
    let id2 = ProceduralMeshGenerator::compute_id(2, &params);

    assert_ne!(id1, id2);
}

#[test]
fn test_mesh_content_addressable_id_different_params() {
    let params1 = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };
    let params2 = ProceduralMeshParams::Cube { size: Vec3::new(2.0, 2.0, 2.0) };

    let id1 = ProceduralMeshGenerator::compute_id(0, &params1);
    let id2 = ProceduralMeshGenerator::compute_id(0, &params2);

    assert_ne!(id1, id2);
}

// ============================================================================
// Texture Generation Tests
// ============================================================================

#[test]
fn test_generate_perlin_noise_validate_pixels() {
    let generator = ProceduralTextureGenerator::new();
    let params = ProceduralTextureParams::PerlinNoise {
        width: 128,
        height: 128,
        frequency: 0.1,
        octaves: 4,
    };

    let texture = generator.generate(0, &params);

    assert_eq!(texture.width, 128);
    assert_eq!(texture.height, 128);
    assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
    assert_eq!(texture.data.len(), 128 * 128 * 4);

    // All pixels should have valid alpha
    for chunk in texture.data.chunks(4) {
        assert_eq!(chunk[3], 255); // Alpha channel should be 255
    }
}

#[test]
fn test_generate_checkerboard_pattern_validation() {
    let generator = ProceduralTextureGenerator::new();
    let params = ProceduralTextureParams::Checkerboard {
        width: 64,
        height: 64,
        square_size: 8,
        color1: [255, 0, 0, 255], // Red
        color2: [0, 0, 255, 255], // Blue
    };

    let texture = generator.generate(0, &params);

    assert_eq!(texture.width, 64);
    assert_eq!(texture.height, 64);

    // Check that we have both colors in the texture
    let mut has_red = false;
    let mut has_blue = false;

    for chunk in texture.data.chunks(4) {
        if chunk == &[255, 0, 0, 255] {
            has_red = true;
        }
        if chunk == &[0, 0, 255, 255] {
            has_blue = true;
        }
    }

    assert!(has_red, "Checkerboard should contain red squares");
    assert!(has_blue, "Checkerboard should contain blue squares");
}

#[test]
fn test_generate_solid_color_all_pixels_same() {
    let generator = ProceduralTextureGenerator::new();
    let params =
        ProceduralTextureParams::SolidColor { width: 256, height: 256, color: [200, 100, 50, 255] };

    let texture = generator.generate(0, &params);

    // All pixels should be the exact same color
    for chunk in texture.data.chunks(4) {
        assert_eq!(chunk, &[200, 100, 50, 255]);
    }
}

#[test]
fn test_texture_determinism_same_seed() {
    let generator = ProceduralTextureGenerator::new();
    let params =
        ProceduralTextureParams::PerlinNoise { width: 64, height: 64, frequency: 0.05, octaves: 3 };

    let texture1 = generator.generate(999, &params);
    let texture2 = generator.generate(999, &params);

    // Should produce bit-identical textures
    assert_eq!(texture1.data, texture2.data);
}

#[test]
fn test_texture_different_seeds_different_output() {
    let generator = ProceduralTextureGenerator::new();
    let params =
        ProceduralTextureParams::PerlinNoise { width: 64, height: 64, frequency: 0.05, octaves: 3 };

    let texture1 = generator.generate(1, &params);
    let texture2 = generator.generate(2, &params);

    // Different seeds should produce different textures
    assert_ne!(texture1.data, texture2.data);
}

#[test]
fn test_texture_content_addressable_id() {
    let params = ProceduralTextureParams::SolidColor {
        width: 128,
        height: 128,
        color: [128, 128, 128, 255],
    };

    let id1 = ProceduralTextureGenerator::compute_id(0, &params);
    let id2 = ProceduralTextureGenerator::compute_id(0, &params);

    assert_eq!(id1, id2);
}

// ============================================================================
// Audio Generation Tests
// ============================================================================

#[test]
fn test_generate_sine_wave_validate_samples() {
    let generator = ProceduralAudioGenerator::new();
    let params =
        ProceduralAudioParams::SineWave { frequency: 440.0, duration: 1.0, sample_rate: 44100 };

    let audio = generator.generate(0, &params);

    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.format, AudioFormat::PCM16);

    // 1 second at 44100 Hz = 44100 samples * 2 bytes per sample
    assert_eq!(audio.data.len(), 44100 * 2);
}

#[test]
fn test_generate_square_wave_duration() {
    let generator = ProceduralAudioGenerator::new();
    let params =
        ProceduralAudioParams::SquareWave { frequency: 440.0, duration: 0.5, sample_rate: 48000 };

    let audio = generator.generate(0, &params);

    assert_eq!(audio.sample_rate, 48000);
    let expected_samples = (0.5 * 48000.0) as usize;
    assert_eq!(audio.data.len(), expected_samples * 2);
}

#[test]
fn test_generate_sawtooth_wave() {
    let generator = ProceduralAudioGenerator::new();
    let params = ProceduralAudioParams::SawtoothWave {
        frequency: 220.0,
        duration: 0.25,
        sample_rate: 44100,
    };

    let audio = generator.generate(0, &params);

    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channels, 1);
    assert!(audio.data.len() > 0);
}

#[test]
fn test_generate_white_noise_determinism() {
    let generator = ProceduralAudioGenerator::new();
    let params = ProceduralAudioParams::WhiteNoise { duration: 0.1, sample_rate: 44100 };

    let audio1 = generator.generate(12345, &params);
    let audio2 = generator.generate(12345, &params);

    // Same seed should produce bit-identical audio
    assert_eq!(audio1.data, audio2.data);
}

#[test]
fn test_generate_white_noise_different_seeds() {
    let generator = ProceduralAudioGenerator::new();
    let params = ProceduralAudioParams::WhiteNoise { duration: 0.1, sample_rate: 44100 };

    let audio1 = generator.generate(1, &params);
    let audio2 = generator.generate(2, &params);

    // Different seeds should produce different noise
    assert_ne!(audio1.data, audio2.data);
}

#[test]
fn test_audio_content_addressable_id() {
    let params =
        ProceduralAudioParams::SineWave { frequency: 440.0, duration: 1.0, sample_rate: 44100 };

    let id1 = ProceduralAudioGenerator::compute_id(0, &params);
    let id2 = ProceduralAudioGenerator::compute_id(0, &params);

    assert_eq!(id1, id2);
}

// ============================================================================
// End-to-End Tests
// ============================================================================

#[test]
fn test_e2e_generate_cache_retrieve_mesh() {
    let generator = ProceduralMeshGenerator::new();
    let params =
        ProceduralMeshParams::Sphere { radius: 1.0, subdivisions_lat: 16, subdivisions_lon: 32 };

    // Generate mesh
    let mesh = generator.generate(42, &params);

    // Compute content-addressable ID
    let id = ProceduralMeshGenerator::compute_id(42, &params);

    // Verify mesh is valid
    assert!(mesh.vertex_count() > 0);
    assert!(mesh.index_count() > 0);

    // Same ID for same params
    let id2 = ProceduralMeshGenerator::compute_id(42, &params);
    assert_eq!(id, id2);
}

#[test]
fn test_e2e_generate_cache_retrieve_texture() {
    let generator = ProceduralTextureGenerator::new();
    let params = ProceduralTextureParams::PerlinNoise {
        width: 256,
        height: 256,
        frequency: 0.1,
        octaves: 4,
    };

    // Generate texture
    let texture = generator.generate(123, &params);

    // Compute content-addressable ID
    let id = ProceduralTextureGenerator::compute_id(123, &params);

    // Verify texture is valid
    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);
    assert!(texture.data.len() > 0);

    // Same ID for same params
    let id2 = ProceduralTextureGenerator::compute_id(123, &params);
    assert_eq!(id, id2);
}

#[test]
fn test_e2e_generate_cache_retrieve_audio() {
    let generator = ProceduralAudioGenerator::new();
    let params = ProceduralAudioParams::WhiteNoise { duration: 1.0, sample_rate: 44100 };

    // Generate audio
    let audio = generator.generate(999, &params);

    // Compute content-addressable ID
    let id = ProceduralAudioGenerator::compute_id(999, &params);

    // Verify audio is valid
    assert_eq!(audio.sample_rate, 44100);
    assert!(audio.data.len() > 0);

    // Same ID for same params
    let id2 = ProceduralAudioGenerator::compute_id(999, &params);
    assert_eq!(id, id2);
}

#[test]
fn test_procedural_params_serialization_roundtrip() {
    // Mesh params
    let mesh_params =
        ProceduralMeshParams::Sphere { radius: 1.5, subdivisions_lat: 24, subdivisions_lon: 48 };
    let serialized = bincode::serialize(&mesh_params).unwrap();
    let deserialized: ProceduralMeshParams = bincode::deserialize(&serialized).unwrap();
    assert_eq!(mesh_params, deserialized);

    // Texture params
    let texture_params = ProceduralTextureParams::Checkerboard {
        width: 512,
        height: 512,
        square_size: 32,
        color1: [255, 0, 0, 255],
        color2: [0, 255, 0, 255],
    };
    let serialized = bincode::serialize(&texture_params).unwrap();
    let deserialized: ProceduralTextureParams = bincode::deserialize(&serialized).unwrap();
    assert_eq!(texture_params, deserialized);

    // Audio params
    let audio_params =
        ProceduralAudioParams::SineWave { frequency: 880.0, duration: 2.0, sample_rate: 48000 };
    let serialized = bincode::serialize(&audio_params).unwrap();
    let deserialized: ProceduralAudioParams = bincode::deserialize(&serialized).unwrap();
    assert_eq!(audio_params, deserialized);
}
