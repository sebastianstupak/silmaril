//! Integration tests for asset bundle system.

use engine_assets::{
    AssetBundle, AssetEntry, AssetId, AssetManifest, AssetType, CompressionFormat,
};
use std::path::PathBuf;

fn create_test_manifest_with_data() -> (AssetManifest, Vec<(AssetId, Vec<u8>)>) {
    let mut manifest = AssetManifest::new();
    let mut asset_data = Vec::new();

    // Create test assets with actual data
    for i in 0..3 {
        let name = format!("asset{i}");
        let data = format!("This is test data for asset {i}").repeat(10); // ~300 bytes each
        let id = AssetId::from_content(name.as_bytes());

        let entry = AssetEntry::new(
            id,
            PathBuf::from(format!("{name}.dat")),
            AssetType::Mesh,
            data.len() as u64,
            *blake3::hash(data.as_bytes()).as_bytes(),
        );

        manifest.add_asset(entry);
        asset_data.push((id, data.into_bytes()));
    }

    (manifest, asset_data)
}

#[test]
fn test_bundle_pack_unpack_no_compression() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    // Add all assets
    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    // Pack bundle
    let packed = bundle.pack().expect("Should pack bundle");
    assert!(!packed.is_empty());

    // Unpack bundle
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack bundle");

    // Verify all assets are present
    assert_eq!(unpacked.manifest().assets.len(), asset_data.len());

    for (id, data) in &asset_data {
        let unpacked_data = unpacked.get_asset(*id).expect("Asset should exist");
        assert_eq!(unpacked_data, data.as_slice());
    }
}

#[cfg(feature = "lz4")]
#[test]
fn test_bundle_pack_unpack_lz4() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Lz4);

    // Add all assets
    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    // Pack bundle
    let packed = bundle.pack().expect("Should pack bundle with LZ4");
    assert!(!packed.is_empty());

    // Unpack bundle
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack LZ4 bundle");

    // Verify compression was used
    assert_eq!(unpacked.compression(), CompressionFormat::Lz4);

    // Verify all assets are present
    for (id, data) in &asset_data {
        let unpacked_data = unpacked.get_asset(*id).expect("Asset should exist");
        assert_eq!(unpacked_data, data.as_slice());
    }
}

#[cfg(feature = "zstd")]
#[test]
fn test_bundle_pack_unpack_zstd() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Zstd);

    // Add all assets
    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    // Pack bundle
    let packed = bundle.pack().expect("Should pack bundle with Zstd");
    assert!(!packed.is_empty());

    // Unpack bundle
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack Zstd bundle");

    // Verify compression was used
    assert_eq!(unpacked.compression(), CompressionFormat::Zstd);

    // Verify all assets are present
    for (id, data) in &asset_data {
        let unpacked_data = unpacked.get_asset(*id).expect("Asset should exist");
        assert_eq!(unpacked_data, data.as_slice());
    }
}

#[test]
fn test_bundle_checksum_verification() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    // Try to add asset with wrong data (checksum mismatch)
    let (id, _) = &asset_data[0];
    let wrong_data = b"wrong data".to_vec();

    let result = bundle.add_asset(*id, wrong_data);
    assert!(result.is_err());
}

#[test]
fn test_bundle_empty() {
    let bundle = AssetBundle::new(CompressionFormat::None);

    // Pack empty bundle
    let packed = bundle.pack().expect("Should pack empty bundle");

    // Unpack
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack empty bundle");
    assert_eq!(unpacked.manifest().assets.len(), 0);
}

#[test]
fn test_bundle_single_asset() {
    let mut manifest = AssetManifest::new();
    let data = b"single asset data";
    let id = AssetId::from_content(b"single");

    let entry = AssetEntry::new(
        id,
        PathBuf::from("single.dat"),
        AssetType::Mesh,
        data.len() as u64,
        *blake3::hash(data).as_bytes(),
    );

    manifest.add_asset(entry);

    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);
    bundle.add_asset(id, data.to_vec()).expect("Should add asset");

    // Round trip
    let packed = bundle.pack().expect("Should pack");
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack");

    assert_eq!(unpacked.get_asset(id).unwrap(), data);
}

#[test]
fn test_bundle_stats() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    let stats = bundle.stats();
    assert_eq!(stats.asset_count, asset_data.len());
    assert_eq!(stats.compression, CompressionFormat::None);

    // Verify total size
    let expected_size: u64 = asset_data.iter().map(|(_, d)| d.len() as u64).sum();
    assert_eq!(stats.total_size, expected_size);
}

#[test]
fn test_bundle_get_nonexistent_asset() {
    let bundle = AssetBundle::new(CompressionFormat::None);
    let id = AssetId::from_content(b"nonexistent");

    assert!(bundle.get_asset(id).is_none());
}

#[test]
fn test_bundle_add_asset_not_in_manifest() {
    let manifest = AssetManifest::new(); // Empty manifest
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    let id = AssetId::from_content(b"unknown");
    let data = b"some data".to_vec();

    let result = bundle.add_asset(id, data);
    assert!(result.is_err());
}

#[test]
fn test_bundle_invalid_data() {
    let invalid_data = b"not a valid bundle";
    let result = AssetBundle::unpack(invalid_data);

    assert!(result.is_err());
}

#[test]
fn test_bundle_truncated_data() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    let packed = bundle.pack().expect("Should pack");

    // Truncate the data
    let truncated = &packed[..packed.len() / 2];
    let result = AssetBundle::unpack(truncated);

    assert!(result.is_err());
}

#[cfg(all(feature = "lz4", feature = "zstd"))]
#[test]
fn test_compression_ratio_comparison() {
    // Create assets with repetitive data (good for compression)
    let mut manifest = AssetManifest::new();
    let data = b"AAAAAAAAAA".repeat(100); // 1KB of repeating data
    let id = AssetId::from_content(b"compressible");

    let entry = AssetEntry::new(
        id,
        PathBuf::from("compressible.dat"),
        AssetType::Mesh,
        data.len() as u64,
        *blake3::hash(&data).as_bytes(),
    );

    manifest.add_asset(entry);

    // Pack with no compression
    let mut bundle_none = AssetBundle::from_manifest(manifest.clone(), CompressionFormat::None);
    bundle_none.add_asset(id, data.clone()).unwrap();
    let packed_none = bundle_none.pack().unwrap();

    // Pack with LZ4
    let mut bundle_lz4 = AssetBundle::from_manifest(manifest.clone(), CompressionFormat::Lz4);
    bundle_lz4.add_asset(id, data.clone()).unwrap();
    let packed_lz4 = bundle_lz4.pack().unwrap();

    // Pack with Zstd
    let mut bundle_zstd = AssetBundle::from_manifest(manifest, CompressionFormat::Zstd);
    bundle_zstd.add_asset(id, data.clone()).unwrap();
    let packed_zstd = bundle_zstd.pack().unwrap();

    // Compression should reduce size for repetitive data
    assert!(packed_lz4.len() < packed_none.len());
    assert!(packed_zstd.len() < packed_none.len());

    // All should unpack to same data
    assert_eq!(
        AssetBundle::unpack(&packed_none).unwrap().get_asset(id).unwrap(),
        data.as_slice()
    );
    assert_eq!(
        AssetBundle::unpack(&packed_lz4).unwrap().get_asset(id).unwrap(),
        data.as_slice()
    );
    assert_eq!(
        AssetBundle::unpack(&packed_zstd).unwrap().get_asset(id).unwrap(),
        data.as_slice()
    );
}

#[test]
fn test_bundle_manifest_preservation() {
    let (manifest, asset_data) = create_test_manifest_with_data();
    let original_version = manifest.version;
    let original_asset_count = manifest.assets.len();

    let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

    for (id, data) in &asset_data {
        bundle.add_asset(*id, data.clone()).expect("Should add asset");
    }

    let packed = bundle.pack().expect("Should pack");
    let unpacked = AssetBundle::unpack(&packed).expect("Should unpack");

    // Verify manifest was preserved
    assert_eq!(unpacked.manifest().version, original_version);
    assert_eq!(unpacked.manifest().assets.len(), original_asset_count);
}

#[test]
fn test_compression_format_extensions() {
    assert_eq!(CompressionFormat::None.extension(), "bundle");
    assert_eq!(CompressionFormat::Lz4.extension(), "bundle.lz4");
    assert_eq!(CompressionFormat::Zstd.extension(), "bundle.zst");
}
