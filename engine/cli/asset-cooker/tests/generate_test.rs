//! Integration tests for procedural asset generation

use engine_assets::{MeshData, TextureData};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the asset-cooker binary
fn asset_cooker_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("asset-cooker");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn test_generate_mesh_cube() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("cube.mesh");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("mesh")
        .arg("cube")
        .arg("2.0")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated mesh file not found");

    // Verify generated mesh
    let mesh_data = fs::read(&output_path).unwrap();
    let mesh = MeshData::from_binary(&mesh_data).unwrap();

    assert_eq!(mesh.vertex_count(), 24);
    assert_eq!(mesh.index_count(), 36);
}

#[test]
fn test_generate_mesh_sphere() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("sphere.mesh");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("mesh")
        .arg("sphere")
        .arg("1.0")
        .arg("16")
        .arg("32")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated mesh file not found");

    // Verify generated mesh
    let mesh_data = fs::read(&output_path).unwrap();
    let mesh = MeshData::from_binary(&mesh_data).unwrap();

    assert!(mesh.vertex_count() > 0);
    assert!(mesh.index_count() > 0);
}

#[test]
fn test_generate_mesh_plane() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("plane.mesh");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("mesh")
        .arg("plane")
        .arg("10.0")
        .arg("10.0")
        .arg("5")
        .arg("5")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated mesh file not found");

    // Verify generated mesh
    let mesh_data = fs::read(&output_path).unwrap();
    let mesh = MeshData::from_binary(&mesh_data).unwrap();

    // (subdivisions + 1)^2 vertices
    assert_eq!(mesh.vertex_count(), 6 * 6);
}

#[test]
fn test_generate_mesh_cylinder() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("cylinder.mesh");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("mesh")
        .arg("cylinder")
        .arg("1.0")
        .arg("2.0")
        .arg("16")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated mesh file not found");

    // Verify generated mesh
    let mesh_data = fs::read(&output_path).unwrap();
    let mesh = MeshData::from_binary(&mesh_data).unwrap();

    assert!(mesh.vertex_count() > 0);
    assert!(mesh.index_count() > 0);
}

#[test]
fn test_generate_texture_checkerboard() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("checkerboard.texture");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("texture")
        .arg("checkerboard")
        .arg("256")
        .arg("256")
        .arg("32")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated texture file not found");

    // Verify generated texture
    let texture_data = fs::read(&output_path).unwrap();
    let texture: TextureData = bincode::deserialize(&texture_data).unwrap();

    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);
}

#[test]
fn test_generate_texture_gradient() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("gradient.texture");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("texture")
        .arg("gradient")
        .arg("128")
        .arg("128")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated texture file not found");

    // Verify generated texture
    let texture_data = fs::read(&output_path).unwrap();
    let texture: TextureData = bincode::deserialize(&texture_data).unwrap();

    assert_eq!(texture.width, 128);
    assert_eq!(texture.height, 128);
}

#[test]
fn test_generate_texture_noise() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("noise.texture");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("texture")
        .arg("noise")
        .arg("256")
        .arg("256")
        .arg("0.1")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated texture file not found");

    // Verify generated texture
    let texture_data = fs::read(&output_path).unwrap();
    let texture: TextureData = bincode::deserialize(&texture_data).unwrap();

    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);
}

#[test]
fn test_generate_audio_sine() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("sine.audio");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("audio")
        .arg("sine")
        .arg("440.0")
        .arg("1.0")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated audio file not found");
}

#[test]
fn test_generate_audio_whitenoise() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("noise.audio");

    // Run generate command
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("audio")
        .arg("whitenoise")
        .arg("0.5")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate command failed: {:?}", output);
    assert!(output_path.exists(), "Generated audio file not found");
}

#[test]
fn test_generate_unknown_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.dat");

    // Run generate command with unknown type
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("unknown")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Generate should fail for unknown type");
}

#[test]
fn test_generate_mesh_with_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("cube_defaults.mesh");

    // Run generate command with defaults (no size specified)
    let output = Command::new(asset_cooker_bin())
        .arg("generate")
        .arg("mesh")
        .arg("cube")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Generate with defaults should succeed");
    assert!(output_path.exists());
}
