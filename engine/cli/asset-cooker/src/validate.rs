//! Asset validation implementation

use anyhow::{Context, Result};
use engine_assets::{AssetType, AssetValidator, MeshData, TextureData};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

/// Validate an asset file
pub fn run(asset_path: PathBuf) -> Result<()> {
    info!(path = ?asset_path, "Validating asset");

    // Check file exists
    if !asset_path.exists() {
        anyhow::bail!("Asset file not found: {}", asset_path.display());
    }

    // Determine asset type from extension
    let ext = asset_path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

    let asset_type = AssetType::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("Unknown asset type from extension: {}", ext))?;

    // Validate based on type
    let result = match asset_type {
        AssetType::Mesh => validate_mesh(&asset_path),
        AssetType::Texture => validate_texture(&asset_path),
        AssetType::Shader => validate_shader(&asset_path),
        AssetType::Material => validate_material(&asset_path),
        AssetType::Audio => validate_audio(&asset_path),
        AssetType::Font => validate_font(&asset_path),
    };

    match result {
        Ok(()) => {
            info!("✓ Asset validation PASSED");
            println!("✓ Asset validation PASSED: {}", asset_path.display());
            Ok(())
        }
        Err(e) => {
            error!(error = ?e, "✗ Asset validation FAILED");
            println!("✗ Asset validation FAILED: {}", asset_path.display());
            println!("  Error: {}", e);
            anyhow::bail!("Validation failed");
        }
    }
}

/// Validate mesh asset
fn validate_mesh(path: &PathBuf) -> Result<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "mesh" => {
            // Binary mesh format
            let data = fs::read(path)?;
            MeshData::validate_format(&data).context("Format validation failed")?;

            let mesh = MeshData::from_binary(&data).context("Failed to load mesh")?;

            let report = mesh.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Mesh validation failed:\n{:#?}", report.errors);
            }
        }
        "obj" => {
            let obj_data = fs::read_to_string(path)?;
            let mesh = MeshData::from_obj(&obj_data).context("Failed to parse OBJ")?;

            let report = mesh.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Mesh validation failed:\n{:#?}", report.errors);
            }
        }
        "gltf" | "glb" => {
            let gltf_data = fs::read(path)?;
            let mesh = MeshData::from_gltf(&gltf_data, None).context("Failed to parse glTF")?;

            let report = mesh.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Mesh validation failed:\n{:#?}", report.errors);
            }
        }
        _ => anyhow::bail!("Unsupported mesh format: {}", ext),
    }

    Ok(())
}

/// Validate texture asset
fn validate_texture(path: &PathBuf) -> Result<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "texture" => {
            // Binary texture format
            let data = fs::read(path)?;
            let texture: TextureData =
                bincode::deserialize(&data).context("Failed to deserialize texture")?;

            let report = texture.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Texture validation failed:\n{:#?}", report.errors);
            }
        }
        "png" | "jpg" | "jpeg" => {
            let image_data = fs::read(path)?;
            let texture =
                TextureData::from_image_bytes(&image_data).context("Failed to load image")?;

            let report = texture.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Texture validation failed:\n{:#?}", report.errors);
            }
        }
        "dds" => {
            let dds_data = fs::read(path)?;
            let texture = TextureData::from_dds_bytes(&dds_data).context("Failed to load DDS")?;

            let report = texture.validate_all();
            if !report.is_valid() {
                anyhow::bail!("Texture validation failed:\n{:#?}", report.errors);
            }
        }
        _ => anyhow::bail!("Unsupported texture format: {}", ext),
    }

    Ok(())
}

/// Validate shader asset
fn validate_shader(path: &PathBuf) -> Result<()> {
    // Basic validation - check file is readable
    let _data = fs::read(path)?;
    // TODO: Add GLSL/SPIR-V validation
    Ok(())
}

/// Validate material asset
fn validate_material(path: &PathBuf) -> Result<()> {
    // Basic validation - check file is readable
    let _data = fs::read(path)?;
    // TODO: Add material validation
    Ok(())
}

/// Validate audio asset
fn validate_audio(path: &PathBuf) -> Result<()> {
    // Basic validation - check file is readable
    let _data = fs::read(path)?;
    // TODO: Add audio validation
    Ok(())
}

/// Validate font asset
fn validate_font(path: &PathBuf) -> Result<()> {
    // Basic validation - check file is readable
    let _data = fs::read(path)?;
    // TODO: Add font validation
    Ok(())
}
