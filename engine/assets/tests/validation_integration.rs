//! Integration tests for asset validation system
//!
//! Tests validation across multiple asset types and edge cases

use engine_assets::{
    AssetValidator, AudioData, AudioFormat, FontData, FontMetrics, FontStyle, FontWeight,
    MaterialData, MeshData, ShaderData, ShaderStage, TextureData, TextureFormat, Vertex,
};
use glam::{Vec2, Vec3};

// ============================================================================
// Cross-Asset Validation Tests
// ============================================================================

#[test]
fn test_all_asset_types_support_validation() {
    // Mesh
    let mesh = MeshData::cube();
    assert!(mesh.validate_all().is_valid());

    // Texture
    let texture_data = vec![128u8; 64];
    let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, texture_data).unwrap();
    assert!(texture.validate_all().is_valid());

    // Material
    let material = MaterialData::new("test");
    assert!(material.validate_all().is_valid());

    // Audio
    let audio_data = vec![0u8; 1000];
    let audio = AudioData::new(44100, 2, AudioFormat::PCM16, audio_data);
    assert!(audio.validate_all().is_valid());

    // Shader
    let shader_source = "#version 450\nvoid main() {}".to_string();
    let shader = ShaderData::from_glsl(ShaderStage::Vertex, shader_source, None).unwrap();
    assert!(shader.validate_all().is_valid());

    // Font (will fail parse but that's expected with dummy data)
    let font_data = vec![1, 2, 3, 4];
    let font = FontData::new(
        "Test".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        font_data,
        FontMetrics::new(800, -200, 100, 1000),
    );
    // Font validation will fail because it's not real font data
    assert!(!font.validate_all().is_valid());
}

#[test]
fn test_validation_catches_corrupted_data() {
    // Mesh with NaN
    let mut mesh = MeshData::triangle();
    mesh.vertices[0].position.x = f32::NAN;
    assert!(!mesh.validate_all().is_valid());

    // Texture with invalid dimensions
    let mut texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
    texture.width = 0;
    assert!(!texture.validate_all().is_valid());

    // Material with out-of-range factor
    let mut material = MaterialData::new("test");
    material.metallic_factor = 2.0;
    assert!(!material.validate_all().is_valid());

    // Audio with zero sample rate
    let audio = AudioData::new(0, 2, AudioFormat::PCM16, vec![0u8; 100]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_checksum_validation_across_types() {
    // Mesh
    let mesh = MeshData::cube();
    let mesh_checksum = mesh.compute_checksum();
    assert!(mesh.validate_checksum(&mesh_checksum).is_ok());

    // Texture
    let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, vec![128u8; 64]).unwrap();
    let texture_checksum = texture.compute_checksum();
    assert!(texture.validate_checksum(&texture_checksum).is_ok());

    // Material
    let material = MaterialData::new("test");
    let material_checksum = material.compute_checksum();
    assert!(material.validate_checksum(&material_checksum).is_ok());

    // Audio
    let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 1000]);
    let audio_checksum = audio.compute_checksum();
    assert!(audio.validate_checksum(&audio_checksum).is_ok());

    // Shader
    let shader_source = "#version 450\nvoid main() {}".to_string();
    let shader = ShaderData::from_glsl(ShaderStage::Vertex, shader_source, None).unwrap();
    let shader_checksum = shader.compute_checksum();
    assert!(shader.validate_checksum(&shader_checksum).is_ok());

    // Font
    let font = FontData::new(
        "Test".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        vec![1, 2, 3, 4],
        FontMetrics::new(800, -200, 100, 1000),
    );
    let font_checksum = font.compute_checksum();
    assert!(font.validate_checksum(&font_checksum).is_ok());
}

// ============================================================================
// Mesh Validation Edge Cases
// ============================================================================

#[test]
fn test_mesh_empty_vertices_fails() {
    let mesh = MeshData::new();
    assert!(!mesh.validate_all().is_valid());
}

#[test]
fn test_mesh_index_out_of_bounds_fails() {
    let mut mesh = MeshData::triangle();
    mesh.indices.push(999); // Way out of bounds
    assert!(!mesh.validate_all().is_valid());
}

#[test]
fn test_mesh_infinity_in_vertices_fails() {
    let mut mesh = MeshData::triangle();
    mesh.vertices[0].normal.z = f32::INFINITY;
    assert!(!mesh.validate_all().is_valid());
}

#[test]
fn test_mesh_large_valid_mesh_passes() {
    let mut mesh = MeshData::with_capacity(10000, 30000);
    for i in 0..10000 {
        mesh.vertices
            .push(Vertex::new(Vec3::new(i as f32, 0.0, 0.0), Vec3::Z, Vec2::ZERO));
    }
    for i in 0..30000 {
        mesh.indices.push((i % 10000) as u32);
    }
    assert!(mesh.validate_all().is_valid());
}

// ============================================================================
// Texture Validation Edge Cases
// ============================================================================

#[test]
fn test_texture_zero_dimensions_fails() {
    let mut texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
    texture.width = 0;
    assert!(!texture.validate_all().is_valid());
}

#[test]
fn test_texture_oversized_dimensions_fails() {
    let mut texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
    texture.width = 20000; // Exceeds MAX_DIMENSION
    assert!(!texture.validate_all().is_valid());
}

#[test]
fn test_texture_invalid_mipmap_chain_fails() {
    let mut texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
    texture.mip_levels.clear();
    assert!(!texture.validate_all().is_valid());
}

#[test]
fn test_texture_mipmap_dimension_mismatch_fails() {
    let mut texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
    texture.mip_levels[0].width = 4; // Wrong!
    assert!(!texture.validate_all().is_valid());
}

// ============================================================================
// Material Validation Edge Cases
// ============================================================================

#[test]
fn test_material_base_color_out_of_range_fails() {
    let mut material = MaterialData::new("test");
    material.base_color_factor[0] = 1.5;
    assert!(!material.validate_all().is_valid());
}

#[test]
fn test_material_negative_metallic_fails() {
    let mut material = MaterialData::new("test");
    material.metallic_factor = -0.1;
    assert!(!material.validate_all().is_valid());
}

#[test]
fn test_material_roughness_above_one_fails() {
    let mut material = MaterialData::new("test");
    material.roughness_factor = 1.5;
    assert!(!material.validate_all().is_valid());
}

#[test]
fn test_material_emissive_nan_fails() {
    let mut material = MaterialData::new("test");
    material.emissive_factor[0] = f32::NAN;
    assert!(!material.validate_all().is_valid());
}

#[test]
fn test_material_emissive_infinity_fails() {
    let mut material = MaterialData::new("test");
    material.emissive_factor[1] = f32::INFINITY;
    assert!(!material.validate_all().is_valid());
}

// ============================================================================
// Audio Validation Edge Cases
// ============================================================================

#[test]
fn test_audio_zero_sample_rate_fails() {
    let audio = AudioData::new(0, 2, AudioFormat::PCM16, vec![0u8; 100]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_audio_excessive_sample_rate_fails() {
    let audio = AudioData::new(999999, 2, AudioFormat::PCM16, vec![0u8; 100]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_audio_zero_channels_fails() {
    let audio = AudioData::new(44100, 0, AudioFormat::PCM16, vec![0u8; 100]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_audio_too_many_channels_fails() {
    let audio = AudioData::new(44100, 10, AudioFormat::PCM16, vec![0u8; 100]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_audio_empty_data_fails() {
    let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![]);
    assert!(!audio.validate_all().is_valid());
}

#[test]
fn test_audio_pcm16_odd_size_fails() {
    let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 99]); // Odd
    assert!(!audio.validate_all().is_valid());
}

// ============================================================================
// Shader Validation Edge Cases
// ============================================================================

#[test]
fn test_shader_glsl_empty_fails() {
    let result = ShaderData::from_glsl(ShaderStage::Vertex, "".to_string(), None);
    assert!(result.is_err());
}

#[test]
fn test_shader_spirv_invalid_magic_fails() {
    let result = ShaderData::from_spirv(ShaderStage::Vertex, vec![0xDEADBEEF], None);
    assert!(result.is_err());
}

#[test]
fn test_shader_spirv_empty_fails() {
    let result = ShaderData::from_spirv(ShaderStage::Vertex, vec![], None);
    assert!(result.is_err());
}

#[test]
fn test_shader_valid_glsl_passes() {
    let source = "#version 450\nvoid main() { gl_Position = vec4(0.0); }".to_string();
    let shader = ShaderData::from_glsl(ShaderStage::Vertex, source, None).unwrap();
    assert!(shader.validate_all().is_valid());
}

#[test]
fn test_shader_valid_spirv_passes() {
    let spirv = vec![0x07230203, 0x00010000, 0x00000000, 0x00000001, 0x00000000];
    let shader = ShaderData::from_spirv(ShaderStage::Fragment, spirv, None).unwrap();
    assert!(shader.validate_all().is_valid());
}

// ============================================================================
// Font Validation Edge Cases
// ============================================================================

#[test]
fn test_font_empty_family_fails() {
    let font = FontData::new(
        "".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        vec![1, 2, 3, 4],
        FontMetrics::new(800, -200, 100, 1000),
    );
    assert!(!font.validate_all().is_valid());
}

#[test]
fn test_font_empty_data_fails() {
    let font = FontData::new(
        "Test".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        vec![],
        FontMetrics::new(800, -200, 100, 1000),
    );
    assert!(!font.validate_all().is_valid());
}

#[test]
fn test_font_zero_units_per_em_fails() {
    let font = FontData::new(
        "Test".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        vec![1, 2, 3, 4],
        FontMetrics::new(800, -200, 100, 0),
    );
    assert!(!font.validate_all().is_valid());
}

#[test]
fn test_font_excessive_units_per_em_fails() {
    let font = FontData::new(
        "Test".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        vec![1, 2, 3, 4],
        FontMetrics::new(800, -200, 100, 20000),
    );
    assert!(!font.validate_all().is_valid());
}

// ============================================================================
// Format Validation Tests
// ============================================================================

#[test]
fn test_mesh_format_validation() {
    let mesh = MeshData::cube();
    let binary = mesh.to_binary();
    assert!(MeshData::validate_format(&binary).is_ok());
}

#[test]
fn test_mesh_format_invalid_magic() {
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(b"BADM");
    assert!(MeshData::validate_format(&data).is_err());
}

#[test]
fn test_material_format_validation() {
    let material = MaterialData::new("test");
    let yaml = material.to_yaml().unwrap();
    assert!(MaterialData::validate_format(yaml.as_bytes()).is_ok());
}

#[test]
fn test_material_format_invalid_yaml() {
    let invalid = b"this is not valid yaml: {{[";
    assert!(MaterialData::validate_format(invalid).is_err());
}

#[test]
fn test_shader_format_glsl() {
    let glsl = b"#version 450\nvoid main() {}";
    assert!(ShaderData::validate_format(glsl).is_ok());
}

#[test]
fn test_shader_format_spirv() {
    let spirv = vec![0x03, 0x02, 0x23, 0x07, 0x00, 0x00, 0x01, 0x00];
    assert!(ShaderData::validate_format(&spirv).is_ok());
}

#[test]
fn test_audio_format_wav() {
    use hound::WavWriter;
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..100 {
            writer.write_sample(i as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    let wav_data = cursor.into_inner();
    assert!(AudioData::validate_format(&wav_data).is_ok());
}

// ============================================================================
// Performance Tests (not benchmarks, just sanity checks)
// ============================================================================

#[test]
fn test_validation_performance_is_reasonable() {
    use std::time::Instant;

    // Large mesh (10k vertices)
    let mut mesh = MeshData::with_capacity(10000, 30000);
    for i in 0..10000 {
        mesh.vertices
            .push(Vertex::new(Vec3::new(i as f32, 0.0, 0.0), Vec3::Z, Vec2::ZERO));
    }
    for i in 0..30000 {
        mesh.indices.push((i % 10000) as u32);
    }

    let start = Instant::now();
    mesh.validate_data().unwrap();
    let elapsed = start.elapsed();

    // Should complete in under 10ms (very generous)
    assert!(elapsed.as_millis() < 10, "Validation took {:?}", elapsed);
}

#[test]
fn test_checksum_performance_is_reasonable() {
    use std::time::Instant;

    // 1MB texture
    let data = vec![128u8; 1024 * 1024];
    let texture = TextureData::new(512, 512, TextureFormat::RGBA8Unorm, data).unwrap();

    let start = Instant::now();
    texture.compute_checksum();
    let elapsed = start.elapsed();

    // Blake3 is very fast, should complete in under 5ms
    assert!(elapsed.as_millis() < 5, "Checksum took {:?}", elapsed);
}
