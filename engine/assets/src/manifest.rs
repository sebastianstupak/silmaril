//! Asset manifest system for tracking asset metadata and dependencies.
//!
//! Manifests provide a declarative way to define asset bundles, track dependencies,
//! and verify asset integrity using checksums.

use crate::{AssetId, AssetType};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, info};

define_error! {
    pub enum ManifestError {
        InvalidFormat { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        CyclicDependency { asset_id: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        MissingDependency { asset_id: String, dependency: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        IoError { path: String, error: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        SerializationError { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

/// Entry in an asset manifest describing a single asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetEntry {
    /// Unique asset identifier.
    pub id: AssetId,
    /// Path to the asset file (relative to manifest).
    pub path: PathBuf,
    /// Type of asset.
    pub asset_type: AssetType,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Blake3 checksum for integrity verification.
    pub checksum: [u8; 32],
    /// List of asset IDs this asset depends on.
    #[serde(default)]
    pub dependencies: Vec<AssetId>,
}

impl AssetEntry {
    /// Create a new asset entry.
    #[must_use]
    pub fn new(
        id: AssetId,
        path: PathBuf,
        asset_type: AssetType,
        size_bytes: u64,
        checksum: [u8; 32],
    ) -> Self {
        Self { id, path, asset_type, size_bytes, checksum, dependencies: Vec::new() }
    }

    /// Add a dependency to this asset.
    pub fn add_dependency(&mut self, dep: AssetId) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    /// Verify the checksum matches expected value.
    #[must_use]
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        let computed = blake3::hash(data);
        computed.as_bytes() == &self.checksum
    }
}

/// Asset manifest describing a collection of assets and their dependencies.
///
/// # Example
///
/// ```yaml
/// version: 1
/// assets:
///   - id: "mesh/cube.obj"
///     path: "meshes/cube.obj"
///     asset_type: Mesh
///     size_bytes: 1024
///     checksum: "abc123..."
///     dependencies: []
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManifest {
    /// Manifest format version.
    pub version: u32,
    /// All assets in this manifest.
    pub assets: Vec<AssetEntry>,
    /// Dependency graph (asset -> dependencies).
    #[serde(skip)]
    dependencies: HashMap<AssetId, Vec<AssetId>>,
}

impl AssetManifest {
    /// Current manifest format version.
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty manifest.
    #[must_use]
    pub fn new() -> Self {
        Self { version: Self::CURRENT_VERSION, assets: Vec::new(), dependencies: HashMap::new() }
    }

    /// Create a manifest with a specific version.
    #[must_use]
    pub fn with_version(version: u32) -> Self {
        Self { version, assets: Vec::new(), dependencies: HashMap::new() }
    }

    /// Add an asset to the manifest.
    pub fn add_asset(&mut self, entry: AssetEntry) {
        // Update dependency graph
        if !entry.dependencies.is_empty() {
            self.dependencies.insert(entry.id, entry.dependencies.clone());
        }

        self.assets.push(entry);
    }

    /// Remove an asset from the manifest.
    pub fn remove_asset(&mut self, id: AssetId) -> Option<AssetEntry> {
        if let Some(pos) = self.assets.iter().position(|e| e.id == id) {
            let entry = self.assets.remove(pos);
            self.dependencies.remove(&id);
            Some(entry)
        } else {
            None
        }
    }

    /// Get an asset entry by ID.
    #[must_use]
    pub fn get_asset(&self, id: AssetId) -> Option<&AssetEntry> {
        self.assets.iter().find(|e| e.id == id)
    }

    /// Get a mutable reference to an asset entry by ID.
    #[must_use]
    pub fn get_asset_mut(&mut self, id: AssetId) -> Option<&mut AssetEntry> {
        self.assets.iter_mut().find(|e| e.id == id)
    }

    /// Get all dependencies for an asset.
    #[must_use]
    pub fn get_dependencies(&self, id: AssetId) -> Vec<AssetId> {
        self.dependencies.get(&id).cloned().unwrap_or_default()
    }

    /// Get all assets that depend on the given asset.
    #[must_use]
    pub fn get_dependents(&self, id: AssetId) -> Vec<AssetId> {
        self.dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(&id))
            .map(|(asset_id, _)| *asset_id)
            .collect()
    }

    /// Validate the manifest for integrity.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Version is unsupported
    /// - Cyclic dependencies exist
    /// - Missing dependencies
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Check version
        if self.version > Self::CURRENT_VERSION {
            return Err(ManifestError::invalidformat(format!(
                "Unsupported manifest version: {}",
                self.version
            )));
        }

        // Build dependency graph
        let mut graph: HashMap<AssetId, Vec<AssetId>> = HashMap::new();
        for entry in &self.assets {
            graph.insert(entry.id, entry.dependencies.clone());
        }

        // Check for cyclic dependencies
        for asset_id in graph.keys() {
            if self.has_cycle(*asset_id, &graph)? {
                return Err(ManifestError::cyclicdependency(format!("{asset_id}")));
            }
        }

        // Check for missing dependencies
        let asset_ids: HashSet<AssetId> = self.assets.iter().map(|e| e.id).collect();
        for entry in &self.assets {
            for dep in &entry.dependencies {
                if !asset_ids.contains(dep) {
                    return Err(ManifestError::missingdependency(
                        format!("{}", entry.id),
                        format!("{dep}"),
                    ));
                }
            }
        }

        info!(asset_count = self.assets.len(), "Manifest validation passed");
        Ok(())
    }

    /// Check if there's a cyclic dependency starting from the given asset.
    fn has_cycle(
        &self,
        start: AssetId,
        graph: &HashMap<AssetId, Vec<AssetId>>,
    ) -> Result<bool, ManifestError> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        self.dfs_cycle_check(start, graph, &mut visited, &mut stack)
    }

    /// Depth-first search to detect cycles.
    fn dfs_cycle_check(
        &self,
        node: AssetId,
        graph: &HashMap<AssetId, Vec<AssetId>>,
        visited: &mut HashSet<AssetId>,
        stack: &mut HashSet<AssetId>,
    ) -> Result<bool, ManifestError> {
        if stack.contains(&node) {
            return Ok(true); // Cycle detected
        }

        if visited.contains(&node) {
            return Ok(false); // Already processed
        }

        visited.insert(node);
        stack.insert(node);

        if let Some(deps) = graph.get(&node) {
            for &dep in deps {
                if self.dfs_cycle_check(dep, graph, visited, stack)? {
                    return Ok(true);
                }
            }
        }

        stack.remove(&node);
        Ok(false)
    }

    /// Get assets in topological order (dependencies first).
    ///
    /// # Errors
    ///
    /// Returns an error if cyclic dependencies exist.
    pub fn topological_sort(&self) -> Result<Vec<AssetId>, ManifestError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();

        let graph: HashMap<AssetId, Vec<AssetId>> =
            self.assets.iter().map(|e| (e.id, e.dependencies.clone())).collect();

        for entry in &self.assets {
            if !visited.contains(&entry.id) {
                self.topo_visit(entry.id, &graph, &mut visited, &mut temp, &mut result)?;
            }
        }

        // No need to reverse - DFS post-order gives us dependencies first
        Ok(result)
    }

    /// Recursive topological sort visit.
    fn topo_visit(
        &self,
        node: AssetId,
        graph: &HashMap<AssetId, Vec<AssetId>>,
        visited: &mut HashSet<AssetId>,
        temp: &mut HashSet<AssetId>,
        result: &mut Vec<AssetId>,
    ) -> Result<(), ManifestError> {
        if temp.contains(&node) {
            return Err(ManifestError::cyclicdependency(format!("{node}")));
        }

        if visited.contains(&node) {
            return Ok(());
        }

        temp.insert(node);

        if let Some(deps) = graph.get(&node) {
            for &dep in deps {
                self.topo_visit(dep, graph, visited, temp, result)?;
            }
        }

        temp.remove(&node);
        visited.insert(node);
        result.push(node);

        Ok(())
    }

    /// Serialize to YAML (human-readable).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        serde_yaml::to_string(self)
            .map_err(|e| ManifestError::serializationerror(format!("YAML: {e}")))
    }

    /// Deserialize from YAML.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_yaml(yaml: &str) -> Result<Self, ManifestError> {
        let mut manifest: Self = serde_yaml::from_str(yaml)
            .map_err(|e| ManifestError::serializationerror(format!("YAML: {e}")))?;

        // Rebuild dependency graph
        for entry in &manifest.assets {
            if !entry.dependencies.is_empty() {
                manifest.dependencies.insert(entry.id, entry.dependencies.clone());
            }
        }

        Ok(manifest)
    }

    /// Serialize to Bincode (fast, compact).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_bincode(&self) -> Result<Vec<u8>, ManifestError> {
        bincode::serialize(self)
            .map_err(|e| ManifestError::serializationerror(format!("Bincode: {e}")))
    }

    /// Deserialize from Bincode.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_bincode(data: &[u8]) -> Result<Self, ManifestError> {
        let mut manifest: Self = bincode::deserialize(data)
            .map_err(|e| ManifestError::serializationerror(format!("Bincode: {e}")))?;

        // Rebuild dependency graph
        for entry in &manifest.assets {
            if !entry.dependencies.is_empty() {
                manifest.dependencies.insert(entry.id, entry.dependencies.clone());
            }
        }

        Ok(manifest)
    }

    /// Get total size of all assets in bytes.
    #[must_use]
    pub fn total_size(&self) -> u64 {
        self.assets.iter().map(|e| e.size_bytes).sum()
    }

    /// Get count of assets by type.
    #[must_use]
    pub fn count_by_type(&self) -> HashMap<AssetType, usize> {
        let mut counts = HashMap::new();
        for entry in &self.assets {
            *counts.entry(entry.asset_type).or_insert(0) += 1;
        }
        counts
    }

    /// Merge another manifest into this one.
    ///
    /// Assets with duplicate IDs will be replaced.
    pub fn merge(&mut self, other: &Self) {
        for entry in &other.assets {
            // Remove existing entry with same ID
            self.remove_asset(entry.id);
            // Add new entry
            self.add_asset(entry.clone());
        }

        debug!(added_assets = other.assets.len(), "Merged manifests");
    }
}

impl Default for AssetManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetType {
    /// Get a string representation of the asset type for serialization.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mesh => "Mesh",
            Self::Texture => "Texture",
            Self::Shader => "Shader",
            Self::Material => "Material",
            Self::Audio => "Audio",
            Self::Font => "Font",
        }
    }

    /// Parse asset type from string.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Mesh" => Some(Self::Mesh),
            "Texture" => Some(Self::Texture),
            "Shader" => Some(Self::Shader),
            "Material" => Some(Self::Material),
            "Audio" => Some(Self::Audio),
            "Font" => Some(Self::Font),
            _ => None,
        }
    }
}

// Implement Serialize/Deserialize for AssetType
impl Serialize for AssetType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AssetType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown asset type: {s}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id_str: &str, deps: Vec<&str>) -> AssetEntry {
        let id = AssetId::from_content(id_str.as_bytes());
        let mut entry = AssetEntry::new(
            id,
            PathBuf::from(format!("{id_str}.obj")),
            AssetType::Mesh,
            1024,
            *blake3::hash(id_str.as_bytes()).as_bytes(),
        );

        for dep in deps {
            entry.add_dependency(AssetId::from_content(dep.as_bytes()));
        }

        entry
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = AssetManifest::new();
        assert_eq!(manifest.version, AssetManifest::CURRENT_VERSION);
        assert!(manifest.assets.is_empty());
    }

    #[test]
    fn test_add_remove_asset() {
        let mut manifest = AssetManifest::new();
        let entry = create_test_entry("asset1", vec![]);

        manifest.add_asset(entry.clone());
        assert_eq!(manifest.assets.len(), 1);

        let removed = manifest.remove_asset(entry.id);
        assert!(removed.is_some());
        assert_eq!(manifest.assets.len(), 0);
    }

    #[test]
    fn test_get_asset() {
        let mut manifest = AssetManifest::new();
        let entry = create_test_entry("asset1", vec![]);
        let id = entry.id;

        manifest.add_asset(entry);

        let found = manifest.get_asset(id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn test_dependencies() {
        let mut manifest = AssetManifest::new();

        // A depends on nothing
        let a = create_test_entry("a", vec![]);
        let a_id = a.id;

        // B depends on A
        let b = create_test_entry("b", vec!["a"]);
        let b_id = b.id;

        manifest.add_asset(a);
        manifest.add_asset(b);

        let b_deps = manifest.get_dependencies(b_id);
        assert_eq!(b_deps.len(), 1);
        assert_eq!(b_deps[0], a_id);

        let a_dependents = manifest.get_dependents(a_id);
        assert_eq!(a_dependents.len(), 1);
        assert_eq!(a_dependents[0], b_id);
    }

    #[test]
    fn test_validation_success() {
        let mut manifest = AssetManifest::new();

        let a = create_test_entry("a", vec![]);
        let b = create_test_entry("b", vec!["a"]);

        manifest.add_asset(a);
        manifest.add_asset(b);

        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_validation_missing_dependency() {
        let mut manifest = AssetManifest::new();

        // B depends on A, but A is not in manifest
        let b = create_test_entry("b", vec!["a"]);
        manifest.add_asset(b);

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ManifestError::MissingDependency { .. }));
    }

    #[test]
    fn test_validation_cyclic_dependency() {
        let mut manifest = AssetManifest::new();

        // Create cycle: A -> B -> A
        let a_id = AssetId::from_content(b"a");
        let b_id = AssetId::from_content(b"b");

        let mut a = AssetEntry::new(
            a_id,
            PathBuf::from("a.obj"),
            AssetType::Mesh,
            1024,
            *blake3::hash(b"a").as_bytes(),
        );
        a.add_dependency(b_id);

        let mut b = AssetEntry::new(
            b_id,
            PathBuf::from("b.obj"),
            AssetType::Mesh,
            1024,
            *blake3::hash(b"b").as_bytes(),
        );
        b.add_dependency(a_id);

        manifest.add_asset(a);
        manifest.add_asset(b);

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ManifestError::CyclicDependency { .. }));
    }

    #[test]
    fn test_topological_sort() {
        let mut manifest = AssetManifest::new();

        // C depends on B, B depends on A
        let a = create_test_entry("a", vec![]);
        let b = create_test_entry("b", vec!["a"]);
        let c = create_test_entry("c", vec!["b"]);

        let a_id = a.id;
        let b_id = b.id;
        let c_id = c.id;

        manifest.add_asset(c);
        manifest.add_asset(a);
        manifest.add_asset(b);

        let sorted = manifest.topological_sort().unwrap();

        // A should come before B, B should come before C
        let a_pos = sorted.iter().position(|&id| id == a_id).unwrap();
        let b_pos = sorted.iter().position(|&id| id == b_id).unwrap();
        let c_pos = sorted.iter().position(|&id| id == c_id).unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_yaml_serialization() {
        let mut manifest = AssetManifest::new();
        let entry = create_test_entry("asset1", vec![]);
        manifest.add_asset(entry);

        let yaml = manifest.to_yaml().unwrap();
        assert!(yaml.contains("version"));
        assert!(yaml.contains("assets"));

        let deserialized = AssetManifest::from_yaml(&yaml).unwrap();
        assert_eq!(deserialized.version, manifest.version);
        assert_eq!(deserialized.assets.len(), manifest.assets.len());
    }

    #[test]
    fn test_bincode_serialization() {
        let mut manifest = AssetManifest::new();
        let entry = create_test_entry("asset1", vec![]);
        manifest.add_asset(entry);

        let bytes = manifest.to_bincode().unwrap();
        let deserialized = AssetManifest::from_bincode(&bytes).unwrap();

        assert_eq!(deserialized.version, manifest.version);
        assert_eq!(deserialized.assets.len(), manifest.assets.len());
    }

    #[test]
    fn test_total_size() {
        let mut manifest = AssetManifest::new();

        let mut entry1 = create_test_entry("asset1", vec![]);
        entry1.size_bytes = 1000;

        let mut entry2 = create_test_entry("asset2", vec![]);
        entry2.size_bytes = 2000;

        manifest.add_asset(entry1);
        manifest.add_asset(entry2);

        assert_eq!(manifest.total_size(), 3000);
    }

    #[test]
    fn test_count_by_type() {
        let mut manifest = AssetManifest::new();

        let mut mesh1 = create_test_entry("mesh1", vec![]);
        mesh1.asset_type = AssetType::Mesh;

        let mut mesh2 = create_test_entry("mesh2", vec![]);
        mesh2.asset_type = AssetType::Mesh;

        let mut texture1 = create_test_entry("texture1", vec![]);
        texture1.asset_type = AssetType::Texture;

        manifest.add_asset(mesh1);
        manifest.add_asset(mesh2);
        manifest.add_asset(texture1);

        let counts = manifest.count_by_type();
        assert_eq!(*counts.get(&AssetType::Mesh).unwrap(), 2);
        assert_eq!(*counts.get(&AssetType::Texture).unwrap(), 1);
    }

    #[test]
    fn test_merge_manifests() {
        let mut manifest1 = AssetManifest::new();
        let mut manifest2 = AssetManifest::new();

        let entry1 = create_test_entry("asset1", vec![]);
        let entry2 = create_test_entry("asset2", vec![]);

        manifest1.add_asset(entry1);
        manifest2.add_asset(entry2);

        manifest1.merge(&manifest2);
        assert_eq!(manifest1.assets.len(), 2);
    }

    #[test]
    fn test_checksum_verification() {
        let data = b"test data";
        let checksum = *blake3::hash(data).as_bytes();

        let entry = AssetEntry::new(
            AssetId::from_content(b"test"),
            PathBuf::from("test.obj"),
            AssetType::Mesh,
            data.len() as u64,
            checksum,
        );

        assert!(entry.verify_checksum(data));
        assert!(!entry.verify_checksum(b"wrong data"));
    }
}
