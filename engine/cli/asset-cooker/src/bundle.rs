//! Asset bundle creation implementation

use anyhow::{Context, Result};
use engine_assets::{AssetBundle, AssetManifest, CompressionFormat};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Create an asset bundle from a manifest file
pub fn run(manifest_path: PathBuf, output_path: PathBuf, compression: &str) -> Result<()> {
    info!(
        manifest = ?manifest_path,
        output = ?output_path,
        compression = compression,
        "Creating asset bundle"
    );

    // Parse compression format
    let compression_format = parse_compression(compression)?;

    // Load manifest
    let manifest_yaml = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest: {}", manifest_path.display()))?;

    let manifest = AssetManifest::from_yaml(&manifest_yaml)
        .with_context(|| format!("Failed to parse manifest: {}", manifest_path.display()))?;

    info!(asset_count = manifest.assets.len(), "Loaded manifest");

    // Validate manifest
    manifest.validate().context("Manifest validation failed")?;

    // Create bundle
    let mut bundle = AssetBundle::from_manifest(manifest.clone(), compression_format);

    // Load asset data and add to bundle
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Manifest has no parent directory"))?;

    let mut loaded_count = 0;
    let mut missing_count = 0;

    for entry in &manifest.assets {
        let asset_path = manifest_dir.join(&entry.path);

        if !asset_path.exists() {
            warn!(
                id = ?entry.id,
                path = ?asset_path,
                "Asset file not found, skipping"
            );
            missing_count += 1;
            continue;
        }

        let asset_data = fs::read(&asset_path).with_context(|| {
            format!("Failed to read asset file: {}", asset_path.display())
        })?;

        bundle.add_asset(entry.id, asset_data).with_context(|| {
            format!("Failed to add asset to bundle: {}", entry.id)
        })?;

        loaded_count += 1;
    }

    info!(
        loaded = loaded_count,
        missing = missing_count,
        "Assets loaded into bundle"
    );

    if missing_count > 0 {
        warn!("{} assets were missing and not included in bundle", missing_count);
    }

    // Pack bundle
    let packed_data = bundle.pack().context("Failed to pack bundle")?;

    // Write to file
    fs::write(&output_path, packed_data)
        .with_context(|| format!("Failed to write bundle: {}", output_path.display()))?;

    let stats = bundle.stats();
    info!(
        asset_count = stats.asset_count,
        total_size = stats.total_size,
        output = ?output_path,
        "Bundle created successfully"
    );

    Ok(())
}

/// Parse compression format string
fn parse_compression(s: &str) -> Result<CompressionFormat> {
    match s.to_lowercase().as_str() {
        "none" => Ok(CompressionFormat::None),
        "lz4" => Ok(CompressionFormat::Lz4),
        "zstd" => Ok(CompressionFormat::Zstd),
        _ => anyhow::bail!("Unknown compression format: {}", s),
    }
}
