//! Asset metadata display implementation

use anyhow::Result;
use engine_assets::{AssetType, MeshData, TextureData};
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Display asset metadata
pub fn run(asset_path: PathBuf) -> Result<()> {
    info!(path = ?asset_path, "Displaying asset info");

    // Check file exists
    if !asset_path.exists() {
        anyhow::bail!("Asset file not found: {}", asset_path.display());
    }

    // Get file size
    let file_size = fs::metadata(&asset_path)?.len();

    // Determine asset type from extension
    let ext = asset_path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

    let asset_type = AssetType::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("Unknown asset type from extension: {}", ext))?;

    println!("\n═══════════════════════════════════════");
    println!("Asset Information");
    println!("═══════════════════════════════════════");
    println!("Path:      {}", asset_path.display());
    println!("Type:      {:?}", asset_type);
    println!("Size:      {} bytes ({:.2} KB)", file_size, file_size as f64 / 1024.0);
    println!("Extension: {}", ext);

    // Display type-specific information
    match asset_type {
        AssetType::Mesh => display_mesh_info(&asset_path)?,
        AssetType::Texture => display_texture_info(&asset_path)?,
        AssetType::Shader => display_shader_info(&asset_path)?,
        AssetType::Material => display_material_info(&asset_path)?,
        AssetType::Audio => display_audio_info(&asset_path)?,
        AssetType::Font => display_font_info(&asset_path)?,
    }

    println!("═══════════════════════════════════════\n");

    Ok(())
}

/// Display mesh-specific information
fn display_mesh_info(path: &PathBuf) -> Result<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let mesh = match ext.to_lowercase().as_str() {
        "mesh" => {
            let data = fs::read(path)?;
            MeshData::from_binary(&data)?
        }
        "obj" => {
            let obj_data = fs::read_to_string(path)?;
            MeshData::from_obj(&obj_data)?
        }
        "gltf" | "glb" => {
            let gltf_data = fs::read(path)?;
            MeshData::from_gltf(&gltf_data, None)?
        }
        _ => anyhow::bail!("Unsupported mesh format: {}", ext),
    };

    println!("\nMesh Details:");
    println!("  Vertices:  {}", mesh.vertex_count());
    println!("  Indices:   {}", mesh.index_count());
    println!("  Triangles: {}", mesh.triangle_count());

    let (min, max) = mesh.bounding_box();
    println!("  Bounding Box:");
    println!("    Min: ({:.2}, {:.2}, {:.2})", min.x, min.y, min.z);
    println!("    Max: ({:.2}, {:.2}, {:.2})", max.x, max.y, max.z);

    let centroid = mesh.centroid();
    println!("  Centroid: ({:.2}, {:.2}, {:.2})", centroid.x, centroid.y, centroid.z);

    Ok(())
}

/// Display texture-specific information
fn display_texture_info(path: &PathBuf) -> Result<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let texture = match ext.to_lowercase().as_str() {
        "texture" => {
            let data = fs::read(path)?;
            bincode::deserialize(&data)?
        }
        "png" | "jpg" | "jpeg" => {
            let image_data = fs::read(path)?;
            TextureData::from_image_bytes(&image_data)?
        }
        "dds" => {
            let dds_data = fs::read(path)?;
            TextureData::from_dds_bytes(&dds_data)?
        }
        _ => anyhow::bail!("Unsupported texture format: {}", ext),
    };

    println!("\nTexture Details:");
    println!("  Dimensions: {}x{}", texture.width, texture.height);
    println!("  Format:     {:?}", texture.format);
    println!("  Mip Levels: {}", texture.mip_count());
    println!(
        "  Memory:     {} bytes ({:.2} KB)",
        texture.memory_size(),
        texture.memory_size() as f64 / 1024.0
    );

    if texture.format.is_compressed() {
        println!("  Compressed: Yes");
        println!("  Block Size: {} bytes", texture.format.block_size());
    } else {
        println!("  Compressed: No");
        if let Some(bpp) = texture.format.bytes_per_pixel() {
            println!("  Bytes/Pixel: {}", bpp);
        }
    }

    Ok(())
}

/// Display shader-specific information
fn display_shader_info(path: &PathBuf) -> Result<()> {
    let data = fs::read(path)?;

    println!("\nShader Details:");
    println!("  Size: {} bytes", data.len());

    // Check if SPIR-V
    if data.len() >= 4 {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic == 0x07230203 {
            println!("  Format: SPIR-V binary");
            println!("  Words: {}", data.len() / 4);
        } else {
            println!("  Format: GLSL text (assumed)");
        }
    }

    Ok(())
}

/// Display material-specific information
fn display_material_info(path: &PathBuf) -> Result<()> {
    let _data = fs::read(path)?;
    println!("\nMaterial Details:");
    println!("  (Material parsing not yet implemented)");
    Ok(())
}

/// Display audio-specific information
fn display_audio_info(path: &PathBuf) -> Result<()> {
    let _data = fs::read(path)?;
    println!("\nAudio Details:");
    println!("  (Audio parsing not yet implemented)");
    Ok(())
}

/// Display font-specific information
fn display_font_info(path: &PathBuf) -> Result<()> {
    let _data = fs::read(path)?;
    println!("\nFont Details:");
    println!("  (Font parsing not yet implemented)");
    Ok(())
}
