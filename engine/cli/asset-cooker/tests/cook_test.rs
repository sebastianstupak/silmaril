//! Integration tests for asset cooking

use engine_assets::{MeshData, TextureData};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the asset-cooker binary
fn asset_cooker_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove "deps" directory
    path.push("asset-cooker");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn test_cook_mesh_obj_to_binary() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&source_dir).unwrap();

    // Create test OBJ file
    let obj_content = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"#;
    let obj_path = source_dir.join("test.obj");
    fs::write(&obj_path, obj_content).unwrap();

    // Run cook command
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Cook command failed: {:?}", output);

    // Verify output file exists
    let cooked_path = output_dir.join("test.mesh");
    assert!(cooked_path.exists(), "Cooked mesh file not found");

    // Verify cooked mesh can be loaded
    let mesh_data = fs::read(&cooked_path).unwrap();
    let mesh = MeshData::from_binary(&mesh_data).unwrap();

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.index_count(), 3);
}

#[test]
fn test_cook_texture_png() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&source_dir).unwrap();

    // Create test PNG file (4x4 solid red)
    let img = image::RgbaImage::from_fn(4, 4, |_, _| image::Rgba([255, 0, 0, 255]));
    let png_path = source_dir.join("test.png");
    img.save(&png_path).unwrap();

    // Run cook command
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Cook command failed: {:?}", output);

    // Verify output file exists
    let cooked_path = output_dir.join("test.texture");
    assert!(cooked_path.exists(), "Cooked texture file not found");

    // Verify cooked texture can be loaded
    let texture_data = fs::read(&cooked_path).unwrap();
    let texture: TextureData = bincode::deserialize(&texture_data).unwrap();

    assert_eq!(texture.width, 4);
    assert_eq!(texture.height, 4);
}

#[test]
fn test_cook_with_mipmaps() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&source_dir).unwrap();

    // Create test PNG file (power of 2 dimensions for mipmaps)
    let img = image::RgbaImage::from_fn(256, 256, |x, y| {
        let color = ((x + y) % 256) as u8;
        image::Rgba([color, color, color, 255])
    });
    let png_path = source_dir.join("test.png");
    img.save(&png_path).unwrap();

    // Run cook command with mipmap generation
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .arg("--generate-mipmaps")
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Cook command failed: {:?}", output);

    // Verify output file exists
    let cooked_path = output_dir.join("test.texture");
    assert!(cooked_path.exists(), "Cooked texture file not found");

    // Verify texture has mipmaps
    let texture_data = fs::read(&cooked_path).unwrap();
    let texture: TextureData = bincode::deserialize(&texture_data).unwrap();

    assert_eq!(texture.width, 256);
    assert_eq!(texture.height, 256);
    assert!(texture.mip_count() > 1, "Mipmaps not generated");
}

#[test]
fn test_cook_recursive() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let subdir = source_dir.join("models");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&subdir).unwrap();

    // Create test files in subdirectory
    let obj_content = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3";
    fs::write(subdir.join("mesh1.obj"), obj_content).unwrap();
    fs::write(subdir.join("mesh2.obj"), obj_content).unwrap();

    // Run cook command with recursive flag
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .arg("--recursive")
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Cook command failed: {:?}", output);

    // Verify output files exist in subdirectory
    let output_subdir = output_dir.join("models");
    assert!(output_subdir.join("mesh1.mesh").exists());
    assert!(output_subdir.join("mesh2.mesh").exists());
}

#[test]
fn test_cook_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&source_dir).unwrap();

    // Run cook command on empty directory
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .output()
        .expect("Failed to execute asset-cooker");

    // Should succeed but warn about no files
    assert!(output.status.success(), "Cook command should succeed on empty directory");
}

#[test]
fn test_cook_nonexistent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("nonexistent");
    let output_dir = temp_dir.path().join("output");

    // Run cook command on nonexistent directory
    let output = Command::new(asset_cooker_bin())
        .arg("cook")
        .arg(&source_dir)
        .arg(&output_dir)
        .output()
        .expect("Failed to execute asset-cooker");

    // Should fail
    assert!(!output.status.success(), "Cook command should fail on nonexistent directory");
}
