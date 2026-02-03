//! Integration tests for asset info display

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
fn test_info_mesh() {
    let temp_dir = TempDir::new().unwrap();

    // Create mesh
    let mesh = MeshData::cube();
    let mesh_data = mesh.to_binary();
    let mesh_path = temp_dir.path().join("cube.mesh");
    fs::write(&mesh_path, mesh_data).unwrap();

    // Run info command
    let output = Command::new(asset_cooker_bin())
        .arg("info")
        .arg(&mesh_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Info command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Asset Information"));
    assert!(stdout.contains("Type:      Mesh"));
    assert!(stdout.contains("Mesh Details:"));
    assert!(stdout.contains("Vertices:"));
    assert!(stdout.contains("Indices:"));
}

#[test]
fn test_info_texture() {
    let temp_dir = TempDir::new().unwrap();

    // Create texture
    let img = image::RgbaImage::from_fn(64, 64, |x, y| {
        let color = ((x + y) % 256) as u8;
        image::Rgba([color, color, color, 255])
    });
    let png_path = temp_dir.path().join("texture.png");
    img.save(&png_path).unwrap();

    // Run info command
    let output = Command::new(asset_cooker_bin())
        .arg("info")
        .arg(&png_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Info command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Asset Information"));
    assert!(stdout.contains("Type:      Texture"));
    assert!(stdout.contains("Texture Details:"));
    assert!(stdout.contains("Dimensions:"));
}

#[test]
fn test_info_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.mesh");

    // Run info command
    let output = Command::new(asset_cooker_bin())
        .arg("info")
        .arg(&nonexistent_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(!output.status.success(), "Info should fail for nonexistent file");
}

#[test]
fn test_info_obj() {
    let temp_dir = TempDir::new().unwrap();

    // Create OBJ file
    let obj_content = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
v 1.0 1.0 0.0
f 1 2 3
f 2 4 3
"#;
    let obj_path = temp_dir.path().join("test.obj");
    fs::write(&obj_path, obj_content).unwrap();

    // Run info command
    let output = Command::new(asset_cooker_bin())
        .arg("info")
        .arg(&obj_path)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Info command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Mesh Details:"));
    assert!(stdout.contains("Vertices:  4"));
    assert!(stdout.contains("Indices:   6"));
    assert!(stdout.contains("Triangles: 2"));
}
