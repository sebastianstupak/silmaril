//! Integration tests for asset manifest system.

use engine_assets::{AssetEntry, AssetId, AssetManifest, AssetType, ManifestError};
use std::path::PathBuf;

fn create_test_entry(name: &str, asset_type: AssetType) -> AssetEntry {
    let id = AssetId::from_content(name.as_bytes());
    let data = format!("test data for {name}");
    AssetEntry::new(
        id,
        PathBuf::from(format!("{name}.dat")),
        asset_type,
        data.len() as u64,
        *blake3::hash(data.as_bytes()).as_bytes(),
    )
}

#[test]
fn test_manifest_round_trip_yaml() {
    let mut manifest = AssetManifest::new();

    // Add various asset types
    manifest.add_asset(create_test_entry("mesh1", AssetType::Mesh));
    manifest.add_asset(create_test_entry("texture1", AssetType::Texture));
    manifest.add_asset(create_test_entry("shader1", AssetType::Shader));

    // Serialize to YAML
    let yaml = manifest.to_yaml().expect("Should serialize to YAML");

    // Verify YAML contains expected fields
    assert!(yaml.contains("version"));
    assert!(yaml.contains("assets"));
    assert!(yaml.contains("mesh1.dat"));
    assert!(yaml.contains("texture1.dat"));

    // Deserialize back
    let loaded = AssetManifest::from_yaml(&yaml).expect("Should deserialize from YAML");

    assert_eq!(loaded.version, manifest.version);
    assert_eq!(loaded.assets.len(), manifest.assets.len());
}

#[test]
fn test_manifest_round_trip_bincode() {
    let mut manifest = AssetManifest::new();

    manifest.add_asset(create_test_entry("asset1", AssetType::Mesh));
    manifest.add_asset(create_test_entry("asset2", AssetType::Texture));

    // Serialize to bincode
    let bytes = manifest.to_bincode().expect("Should serialize to bincode");

    // Deserialize back
    let loaded = AssetManifest::from_bincode(&bytes).expect("Should deserialize from bincode");

    assert_eq!(loaded.version, manifest.version);
    assert_eq!(loaded.assets.len(), manifest.assets.len());
}

#[test]
fn test_dependency_resolution() {
    let mut manifest = AssetManifest::new();

    // Create dependency chain: C depends on B, B depends on A
    let a = create_test_entry("a", AssetType::Mesh);
    let a_id = a.id;

    let mut b = create_test_entry("b", AssetType::Material);
    let b_id = b.id;
    b.add_dependency(a_id);

    let mut c = create_test_entry("c", AssetType::Texture);
    let c_id = c.id;
    c.add_dependency(b_id);

    manifest.add_asset(a);
    manifest.add_asset(b);
    manifest.add_asset(c);

    // Get dependencies
    let b_deps = manifest.get_dependencies(b_id);
    assert_eq!(b_deps.len(), 1);
    assert!(b_deps.contains(&a_id));

    let c_deps = manifest.get_dependencies(c_id);
    assert_eq!(c_deps.len(), 1);
    assert!(c_deps.contains(&b_id));

    // Get dependents
    let a_dependents = manifest.get_dependents(a_id);
    assert_eq!(a_dependents.len(), 1);
    assert!(a_dependents.contains(&b_id));
}

#[test]
fn test_cyclic_dependency_detection() {
    let mut manifest = AssetManifest::new();

    // Create cycle: A -> B -> C -> A
    let a_id = AssetId::from_content(b"a");
    let b_id = AssetId::from_content(b"b");
    let c_id = AssetId::from_content(b"c");

    let mut a = AssetEntry::new(
        a_id,
        PathBuf::from("a.dat"),
        AssetType::Mesh,
        100,
        *blake3::hash(b"a").as_bytes(),
    );
    a.add_dependency(b_id);

    let mut b = AssetEntry::new(
        b_id,
        PathBuf::from("b.dat"),
        AssetType::Mesh,
        100,
        *blake3::hash(b"b").as_bytes(),
    );
    b.add_dependency(c_id);

    let mut c = AssetEntry::new(
        c_id,
        PathBuf::from("c.dat"),
        AssetType::Mesh,
        100,
        *blake3::hash(b"c").as_bytes(),
    );
    c.add_dependency(a_id); // Create cycle

    manifest.add_asset(a);
    manifest.add_asset(b);
    manifest.add_asset(c);

    // Validation should fail
    let result = manifest.validate();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ManifestError::CyclicDependency { .. }));
}

#[test]
fn test_missing_dependency_detection() {
    let mut manifest = AssetManifest::new();

    // Create asset that depends on non-existent asset
    let a_id = AssetId::from_content(b"a");
    let missing_id = AssetId::from_content(b"missing");

    let mut a = AssetEntry::new(
        a_id,
        PathBuf::from("a.dat"),
        AssetType::Mesh,
        100,
        *blake3::hash(b"a").as_bytes(),
    );
    a.add_dependency(missing_id);

    manifest.add_asset(a);

    // Validation should fail
    let result = manifest.validate();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ManifestError::MissingDependency { .. }));
}

#[test]
fn test_topological_sort_simple() {
    let mut manifest = AssetManifest::new();

    // A -> B -> C (linear dependency chain)
    let a = create_test_entry("a", AssetType::Mesh);
    let a_id = a.id;

    let mut b = create_test_entry("b", AssetType::Mesh);
    let b_id = b.id;
    b.add_dependency(a_id);

    let mut c = create_test_entry("c", AssetType::Mesh);
    let c_id = c.id;
    c.add_dependency(b_id);

    // Add in reverse order
    manifest.add_asset(c);
    manifest.add_asset(b);
    manifest.add_asset(a);

    let sorted = manifest.topological_sort().expect("Should sort successfully");

    // Verify order: A before B before C
    let a_pos = sorted.iter().position(|&id| id == a_id).unwrap();
    let b_pos = sorted.iter().position(|&id| id == b_id).unwrap();
    let c_pos = sorted.iter().position(|&id| id == c_id).unwrap();

    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}

#[test]
fn test_topological_sort_complex() {
    let mut manifest = AssetManifest::new();

    // Complex dependency graph:
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D

    let a = create_test_entry("a", AssetType::Mesh);
    let a_id = a.id;

    let mut b = create_test_entry("b", AssetType::Mesh);
    let b_id = b.id;
    b.add_dependency(a_id);

    let mut c = create_test_entry("c", AssetType::Mesh);
    let c_id = c.id;
    c.add_dependency(a_id);

    let mut d = create_test_entry("d", AssetType::Mesh);
    let d_id = d.id;
    d.add_dependency(b_id);
    d.add_dependency(c_id);

    manifest.add_asset(d);
    manifest.add_asset(c);
    manifest.add_asset(b);
    manifest.add_asset(a);

    let sorted = manifest.topological_sort().expect("Should sort successfully");

    // Verify A comes before B and C
    let a_pos = sorted.iter().position(|&id| id == a_id).unwrap();
    let b_pos = sorted.iter().position(|&id| id == b_id).unwrap();
    let c_pos = sorted.iter().position(|&id| id == c_id).unwrap();
    let d_pos = sorted.iter().position(|&id| id == d_id).unwrap();

    assert!(a_pos < b_pos);
    assert!(a_pos < c_pos);
    assert!(b_pos < d_pos);
    assert!(c_pos < d_pos);
}

#[test]
fn test_manifest_statistics() {
    let mut manifest = AssetManifest::new();

    let mut mesh1 = create_test_entry("mesh1", AssetType::Mesh);
    mesh1.size_bytes = 1000;

    let mut mesh2 = create_test_entry("mesh2", AssetType::Mesh);
    mesh2.size_bytes = 2000;

    let mut texture1 = create_test_entry("texture1", AssetType::Texture);
    texture1.size_bytes = 5000;

    manifest.add_asset(mesh1);
    manifest.add_asset(mesh2);
    manifest.add_asset(texture1);

    // Test total size
    assert_eq!(manifest.total_size(), 8000);

    // Test count by type
    let counts = manifest.count_by_type();
    assert_eq!(*counts.get(&AssetType::Mesh).unwrap(), 2);
    assert_eq!(*counts.get(&AssetType::Texture).unwrap(), 1);
}

#[test]
fn test_manifest_merge() {
    let mut manifest1 = AssetManifest::new();
    let mut manifest2 = AssetManifest::new();

    manifest1.add_asset(create_test_entry("asset1", AssetType::Mesh));
    manifest1.add_asset(create_test_entry("asset2", AssetType::Texture));

    manifest2.add_asset(create_test_entry("asset3", AssetType::Shader));
    manifest2.add_asset(create_test_entry("asset4", AssetType::Material));

    manifest1.merge(&manifest2);

    assert_eq!(manifest1.assets.len(), 4);
}

#[test]
fn test_manifest_merge_overwrites_duplicates() {
    let mut manifest1 = AssetManifest::new();
    let mut manifest2 = AssetManifest::new();

    let asset1 = create_test_entry("asset1", AssetType::Mesh);
    let asset1_id = asset1.id;

    manifest1.add_asset(asset1.clone());

    // Add same asset with different data
    let mut asset1_v2 = asset1;
    asset1_v2.size_bytes = 9999;
    manifest2.add_asset(asset1_v2);

    manifest1.merge(&manifest2);

    // Should have only one asset with updated size
    assert_eq!(manifest1.assets.len(), 1);
    let entry = manifest1.get_asset(asset1_id).unwrap();
    assert_eq!(entry.size_bytes, 9999);
}

#[test]
fn test_checksum_verification() {
    let data = b"test asset data";
    let checksum = *blake3::hash(data).as_bytes();

    let entry = AssetEntry::new(
        AssetId::from_content(b"test"),
        PathBuf::from("test.dat"),
        AssetType::Mesh,
        data.len() as u64,
        checksum,
    );

    // Correct data should verify
    assert!(entry.verify_checksum(data));

    // Wrong data should fail
    assert!(!entry.verify_checksum(b"wrong data"));
}

#[test]
fn test_manifest_incremental_update() {
    let mut manifest = AssetManifest::new();

    // Add initial assets
    let asset1 = create_test_entry("asset1", AssetType::Mesh);
    let asset1_id = asset1.id;
    manifest.add_asset(asset1);

    assert_eq!(manifest.assets.len(), 1);

    // Remove asset
    let removed = manifest.remove_asset(asset1_id);
    assert!(removed.is_some());
    assert_eq!(manifest.assets.len(), 0);

    // Add new asset
    manifest.add_asset(create_test_entry("asset2", AssetType::Texture));
    assert_eq!(manifest.assets.len(), 1);
}

#[test]
fn test_empty_manifest_validation() {
    let manifest = AssetManifest::new();
    assert!(manifest.validate().is_ok());
}

#[test]
fn test_manifest_with_no_dependencies_validation() {
    let mut manifest = AssetManifest::new();

    manifest.add_asset(create_test_entry("asset1", AssetType::Mesh));
    manifest.add_asset(create_test_entry("asset2", AssetType::Texture));
    manifest.add_asset(create_test_entry("asset3", AssetType::Shader));

    assert!(manifest.validate().is_ok());
}
