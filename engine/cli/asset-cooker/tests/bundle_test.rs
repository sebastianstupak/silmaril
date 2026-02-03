//! Integration tests for asset bundling

use engine_assets::{AssetBundle, AssetEntry, AssetId, AssetManifest, AssetType};
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
fn test_bundle_create_from_manifest() {
    let temp_dir = TempDir::new().unwrap();

    // Create test assets
    let asset1_data = b"test asset 1";
    let asset1_id = AssetId::from_content(asset1_data);
    let asset1_path = temp_dir.path().join("asset1.dat");
    fs::write(&asset1_path, asset1_data).unwrap();

    let asset2_data = b"test asset 2";
    let asset2_id = AssetId::from_content(asset2_data);
    let asset2_path = temp_dir.path().join("asset2.dat");
    fs::write(&asset2_path, asset2_data).unwrap();

    // Create manifest
    let mut manifest = AssetManifest::new();

    let entry1 = AssetEntry::new(
        asset1_id,
        PathBuf::from("asset1.dat"),
        AssetType::Mesh,
        asset1_data.len() as u64,
        *blake3::hash(asset1_data).as_bytes(),
    );
    manifest.add_asset(entry1);

    let entry2 = AssetEntry::new(
        asset2_id,
        PathBuf::from("asset2.dat"),
        AssetType::Texture,
        asset2_data.len() as u64,
        *blake3::hash(asset2_data).as_bytes(),
    );
    manifest.add_asset(entry2);

    // Write manifest to file
    let manifest_path = temp_dir.path().join("manifest.yaml");
    let manifest_yaml = manifest.to_yaml().unwrap();
    fs::write(&manifest_path, manifest_yaml).unwrap();

    // Run bundle command
    let bundle_path = temp_dir.path().join("test.bundle");
    let output = Command::new(asset_cooker_bin())
        .arg("bundle")
        .arg(&manifest_path)
        .arg(&bundle_path)
        .arg("--compression")
        .arg("none")
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success(), "Bundle command failed: {:?}", output);

    // Verify bundle file exists
    assert!(bundle_path.exists(), "Bundle file not created");

    // Verify bundle can be unpacked
    let bundle_data = fs::read(&bundle_path).unwrap();
    let bundle = AssetBundle::unpack(&bundle_data).unwrap();

    assert_eq!(bundle.manifest().assets.len(), 2);
    assert!(bundle.get_asset(asset1_id).is_some());
    assert!(bundle.get_asset(asset2_id).is_some());
}

#[test]
fn test_bundle_with_missing_assets() {
    let temp_dir = TempDir::new().unwrap();

    // Create manifest with assets that don't exist
    let mut manifest = AssetManifest::new();

    let asset_id = AssetId::from_content(b"missing");
    let entry = AssetEntry::new(
        asset_id,
        PathBuf::from("missing.dat"),
        AssetType::Mesh,
        100,
        *blake3::hash(b"missing").as_bytes(),
    );
    manifest.add_asset(entry);

    // Write manifest to file
    let manifest_path = temp_dir.path().join("manifest.yaml");
    let manifest_yaml = manifest.to_yaml().unwrap();
    fs::write(&manifest_path, manifest_yaml).unwrap();

    // Run bundle command (should succeed but warn about missing assets)
    let bundle_path = temp_dir.path().join("test.bundle");
    let output = Command::new(asset_cooker_bin())
        .arg("bundle")
        .arg(&manifest_path)
        .arg(&bundle_path)
        .arg("--compression")
        .arg("none")
        .output()
        .expect("Failed to execute asset-cooker");

    // Should still succeed (warnings are logged, not errors)
    assert!(
        output.status.success(),
        "Bundle command should succeed even with missing assets"
    );
}

#[test]
fn test_bundle_compression_none() {
    let temp_dir = TempDir::new().unwrap();

    // Create simple manifest with one asset
    let asset_data = b"test data for compression";
    let asset_id = AssetId::from_content(asset_data);
    let asset_path = temp_dir.path().join("asset.dat");
    fs::write(&asset_path, asset_data).unwrap();

    let mut manifest = AssetManifest::new();
    let entry = AssetEntry::new(
        asset_id,
        PathBuf::from("asset.dat"),
        AssetType::Mesh,
        asset_data.len() as u64,
        *blake3::hash(asset_data).as_bytes(),
    );
    manifest.add_asset(entry);

    let manifest_path = temp_dir.path().join("manifest.yaml");
    fs::write(&manifest_path, manifest.to_yaml().unwrap()).unwrap();

    // Bundle with no compression
    let bundle_path = temp_dir.path().join("test.bundle");
    let output = Command::new(asset_cooker_bin())
        .arg("bundle")
        .arg(&manifest_path)
        .arg(&bundle_path)
        .arg("--compression")
        .arg("none")
        .output()
        .expect("Failed to execute asset-cooker");

    assert!(output.status.success());
    assert!(bundle_path.exists());

    // Verify bundle can be unpacked
    let bundle_data = fs::read(&bundle_path).unwrap();
    let bundle = AssetBundle::unpack(&bundle_data).unwrap();
    assert_eq!(bundle.get_asset(asset_id).unwrap(), asset_data);
}

#[test]
fn test_bundle_invalid_compression() {
    let temp_dir = TempDir::new().unwrap();

    // Create minimal manifest
    let manifest = AssetManifest::new();
    let manifest_path = temp_dir.path().join("manifest.yaml");
    fs::write(&manifest_path, manifest.to_yaml().unwrap()).unwrap();

    // Try to bundle with invalid compression format
    let bundle_path = temp_dir.path().join("test.bundle");
    let output = Command::new(asset_cooker_bin())
        .arg("bundle")
        .arg(&manifest_path)
        .arg(&bundle_path)
        .arg("--compression")
        .arg("invalid")
        .output()
        .expect("Failed to execute asset-cooker");

    // Should fail
    assert!(!output.status.success(), "Bundle should fail with invalid compression format");
}

#[test]
fn test_bundle_nonexistent_manifest() {
    let temp_dir = TempDir::new().unwrap();

    let manifest_path = temp_dir.path().join("nonexistent.yaml");
    let bundle_path = temp_dir.path().join("test.bundle");

    let output = Command::new(asset_cooker_bin())
        .arg("bundle")
        .arg(&manifest_path)
        .arg(&bundle_path)
        .output()
        .expect("Failed to execute asset-cooker");

    // Should fail
    assert!(!output.status.success(), "Bundle should fail with nonexistent manifest");
}
