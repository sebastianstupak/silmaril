//! Asset cooking implementation

use anyhow::{Context, Result};
use engine_assets::{AssetType, MeshData, TextureData, TextureFormat};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Cook raw assets from source directory to output directory
pub fn run(
    source_dir: PathBuf,
    output_dir: PathBuf,
    generate_mipmaps: bool,
    optimize_meshes: bool,
    recursive: bool,
) -> Result<()> {
    info!(
        source = ?source_dir,
        output = ?output_dir,
        mipmaps = generate_mipmaps,
        optimize = optimize_meshes,
        recursive = recursive,
        "Starting asset cooking"
    );

    // Validate source directory exists
    if !source_dir.exists() {
        anyhow::bail!("Source directory does not exist: {}", source_dir.display());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    // Collect all asset files
    let files = collect_asset_files(&source_dir, recursive)?;
    info!(file_count = files.len(), "Found asset files to cook");

    if files.is_empty() {
        warn!("No asset files found in source directory");
        return Ok(());
    }

    // Create progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Failed to create progress style")
            .progress_chars("=>-"),
    );

    let mut cooked_count = 0;
    let mut error_count = 0;

    // Process each file
    for file_path in files {
        let relative_path = file_path.strip_prefix(&source_dir)?;
        pb.set_message(format!("Cooking {}", relative_path.display()));

        match cook_asset(&file_path, &output_dir, &source_dir, generate_mipmaps, optimize_meshes) {
            Ok(output_path) => {
                debug!(
                    source = ?file_path,
                    output = ?output_path,
                    "Asset cooked successfully"
                );
                cooked_count += 1;
            }
            Err(e) => {
                error!(
                    source = ?file_path,
                    error = ?e,
                    "Failed to cook asset"
                );
                error_count += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Cooking complete");

    info!(cooked = cooked_count, errors = error_count, "Asset cooking finished");

    if error_count > 0 {
        anyhow::bail!("{} assets failed to cook", error_count);
    }

    Ok(())
}

/// Collect all asset files from a directory
fn collect_asset_files(source_dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if recursive {
        for entry in WalkDir::new(source_dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if AssetType::from_extension(ext.to_str().unwrap_or("")).is_some() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    } else {
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if AssetType::from_extension(ext.to_str().unwrap_or("")).is_some() {
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    Ok(files)
}

/// Cook a single asset file
fn cook_asset(
    source_path: &Path,
    output_dir: &Path,
    source_dir: &Path,
    generate_mipmaps: bool,
    _optimize_meshes: bool,
) -> Result<PathBuf> {
    // Determine asset type from extension
    let ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

    let asset_type = AssetType::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("Unsupported asset type: {}", ext))?;

    // Calculate relative path to preserve directory structure
    let relative_path = source_path.strip_prefix(source_dir)?;
    let output_path = output_dir.join(relative_path);

    // Create output directory
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    match asset_type {
        AssetType::Mesh => cook_mesh(source_path, &output_path),
        AssetType::Texture => cook_texture(source_path, &output_path, generate_mipmaps),
        AssetType::Shader => cook_shader(source_path, &output_path),
        AssetType::Material => cook_material(source_path, &output_path),
        AssetType::Audio => cook_audio(source_path, &output_path),
        AssetType::Font => cook_font(source_path, &output_path),
    }
}

/// Cook a mesh asset
fn cook_mesh(source_path: &Path, output_path: &Path) -> Result<PathBuf> {
    let ext = source_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let mesh = match ext.to_lowercase().as_str() {
        "obj" => {
            let obj_data = fs::read_to_string(source_path)?;
            MeshData::from_obj(&obj_data)?
        }
        "gltf" | "glb" => {
            let gltf_data = fs::read(source_path)?;
            MeshData::from_gltf(&gltf_data, None)?
        }
        _ => anyhow::bail!("Unsupported mesh format: {}", ext),
    };

    // Convert to binary format for fast loading
    let binary_data = mesh.to_binary();
    let output_path = output_path.with_extension("mesh");
    fs::write(&output_path, binary_data)?;

    Ok(output_path)
}

/// Cook a texture asset
fn cook_texture(source_path: &Path, output_path: &Path, generate_mipmaps: bool) -> Result<PathBuf> {
    let ext = source_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let texture = match ext.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" => {
            let image_data = fs::read(source_path)?;
            TextureData::from_image_bytes(&image_data)?
        }
        "dds" => {
            let dds_data = fs::read(source_path)?;
            TextureData::from_dds_bytes(&dds_data)?
        }
        _ => anyhow::bail!("Unsupported texture format: {}", ext),
    };

    // Generate mipmaps if requested and texture is power-of-2
    let texture = if generate_mipmaps
        && (texture.format == TextureFormat::RGBA8Unorm
            || texture.format == TextureFormat::RGBA8Srgb)
        && texture.width.is_power_of_two()
        && texture.height.is_power_of_two()
    {
        texture.generate_mipmaps()?
    } else {
        texture
    };

    // Serialize to bincode for fast loading
    let binary_data = bincode::serialize(&texture)?;
    let output_path = output_path.with_extension("texture");
    fs::write(&output_path, binary_data)?;

    Ok(output_path)
}

/// Cook a shader asset
fn cook_shader(source_path: &Path, output_path: &Path) -> Result<PathBuf> {
    // For shaders, just copy them as-is for now
    // In a real implementation, you'd compile GLSL -> SPIR-V here
    fs::copy(source_path, output_path)?;
    Ok(output_path.to_path_buf())
}

/// Cook a material asset
fn cook_material(source_path: &Path, output_path: &Path) -> Result<PathBuf> {
    // For materials, just copy them as-is for now
    fs::copy(source_path, output_path)?;
    Ok(output_path.to_path_buf())
}

/// Cook an audio asset
fn cook_audio(source_path: &Path, output_path: &Path) -> Result<PathBuf> {
    // For audio, just copy them as-is for now
    // In a real implementation, you'd convert to PCM16, normalize, etc.
    fs::copy(source_path, output_path)?;
    Ok(output_path.to_path_buf())
}

/// Cook a font asset
fn cook_font(source_path: &Path, output_path: &Path) -> Result<PathBuf> {
    // For fonts, just copy them as-is
    fs::copy(source_path, output_path)?;
    Ok(output_path.to_path_buf())
}
