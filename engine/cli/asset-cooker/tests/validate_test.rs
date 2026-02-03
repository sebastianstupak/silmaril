//! Integration tests for asset validation

use engine_assets::MeshData;
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
fn test_validate_valid_mesh() {
    let temp_dir = TempDir::new().unwrap();

    // Create valid mesh
    let mesh = MeshData::cube();
    let mesh_data = mesh.to_binary();
    let mesh_path = temp_dir.path().join("cube.mesh");
    fs::write(&mesh_path, mesh_data).unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&mesh_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Validation should succeed for valid mesh");
}

#[test]
fn test_validate_valid_obj() {
    let temp_dir = TempDir::new().unwrap();

    // Create valid OBJ
    let obj_content = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"#;
    let obj_path = temp_dir.path().join("test.obj");
    fs::write(&obj_path, obj_content).unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&obj_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Validation should succeed for valid OBJ");
}

#[test]
fn test_validate_invalid_mesh_corrupted() {
    let temp_dir = TempDir::new().unwrap();

    // Create corrupted mesh file
    let corrupted_data = b"BADM\x00\x00\x00\x01corrupted data";
    let mesh_path = temp_dir.path().join("corrupted.mesh");
    fs::write(&mesh_path, corrupted_data).unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&mesh_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Validation should fail for corrupted mesh");
}

#[test]
fn test_validate_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.mesh");

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&nonexistent_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Validation should fail for nonexistent file");
}

#[test]
fn test_validate_unknown_extension() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with unknown extension
    let unknown_path = temp_dir.path().join("test.unknown");
    fs::write(&unknown_path, b"data").unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&unknown_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Validation should fail for unknown extension");
}

#[test]
fn test_validate_texture_png() {
    let temp_dir = TempDir::new().unwrap();

    // Create valid PNG
    let img = image::RgbaImage::from_fn(16, 16, |x, y| {
        let color = ((x + y) % 256) as u8;
        image::Rgba([color, color, color, 255])
    });
    let png_path = temp_dir.path().join("test.png");
    img.save(&png_path).unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&png_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Validation should succeed for valid PNG");
}

#[test]
fn test_validate_invalid_obj() {
    let temp_dir = TempDir::new().unwrap();

    // Create invalid OBJ (out of bounds index)
    let obj_content = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 999
"#;
    let obj_path = temp_dir.path().join("invalid.obj");
    fs::write(&obj_path, obj_content).unwrap();

    // Run validate command
    let output = Command::new(asset_cooker_bin())
        .arg("validate")
        .arg(&obj_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Validation should fail for invalid OBJ");
}
