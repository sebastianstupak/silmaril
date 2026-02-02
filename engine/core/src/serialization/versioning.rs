//! Serialization versioning and migration support
//!
//! Enables backward compatibility by:
//! - Tracking schema versions in serialized data
//! - Detecting version mismatches on load
//! - Providing migration paths from old versions
//! - Validating schema compatibility
//!
//! Target: Support 3+ versions backward

use super::{SerializationError, WorldState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current serialization schema version
///
/// Increment this when making breaking changes to WorldState structure.
/// Always maintain migration paths from previous versions.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Minimum supported schema version for backward compatibility
///
/// Data older than this version cannot be loaded and will error.
pub const MIN_SUPPORTED_VERSION: u32 = 1;

/// Maximum supported schema version (for future-proofing)
///
/// Data from newer versions will error unless migration is available.
pub const MAX_SUPPORTED_VERSION: u32 = 1;

/// Schema version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaVersion {
    /// Schema version number
    pub version: u32,
    /// Optional version name/tag (e.g., "v1.0-alpha", "v2.0-stable")
    pub tag: Option<String>,
    /// Timestamp when this version was introduced (Unix timestamp)
    pub introduced_at: u64,
    /// List of deprecated features in this version
    pub deprecated_features: Vec<String>,
    /// List of new features in this version
    pub new_features: Vec<String>,
}

impl SchemaVersion {
    /// Create a new schema version
    pub fn new(version: u32) -> Self {
        Self {
            version,
            tag: None,
            introduced_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            deprecated_features: Vec::new(),
            new_features: Vec::new(),
        }
    }

    /// Create the current schema version
    pub fn current() -> Self {
        Self::new(CURRENT_SCHEMA_VERSION)
    }

    /// Check if this version is supported
    pub fn is_supported(&self) -> bool {
        self.version >= MIN_SUPPORTED_VERSION && self.version <= MAX_SUPPORTED_VERSION
    }

    /// Check if this version is deprecated
    pub fn is_deprecated(&self) -> bool {
        self.version < CURRENT_SCHEMA_VERSION
    }

    /// Check if migration is needed
    pub fn needs_migration(&self) -> bool {
        self.version != CURRENT_SCHEMA_VERSION && self.is_supported()
    }
}

/// Migration function signature
///
/// Takes old WorldState and returns migrated WorldState or error.
pub type MigrationFn = fn(&WorldState) -> Result<WorldState, SerializationError>;

/// Schema migration registry
///
/// Maintains migration paths from old versions to current version.
pub struct MigrationRegistry {
    /// Registered migrations: (from_version, to_version) -> migration_fn
    migrations: HashMap<(u32, u32), MigrationFn>,
}

impl MigrationRegistry {
    /// Create a new migration registry
    pub fn new() -> Self {
        Self { migrations: HashMap::new() }
    }

    /// Register a migration from one version to another
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::serialization::versioning::MigrationRegistry;
    /// let mut registry = MigrationRegistry::new();
    ///
    /// // Register migration from v1 to v2
    /// registry.register_migration(1, 2, |old_state| {
    ///     // Perform migration logic
    ///     Ok(old_state.clone())
    /// });
    /// ```
    pub fn register_migration(&mut self, from: u32, to: u32, migration: MigrationFn) {
        self.migrations.insert((from, to), migration);
    }

    /// Get migration function for version transition
    pub fn get_migration(&self, from: u32, to: u32) -> Option<&MigrationFn> {
        self.migrations.get(&(from, to))
    }

    /// Find migration path from source to target version
    ///
    /// Returns a sequence of migration functions to apply.
    /// Uses breadth-first search to find shortest path.
    pub fn find_migration_path(&self, from: u32, to: u32) -> Option<Vec<MigrationFn>> {
        if from == to {
            return Some(Vec::new());
        }

        // BFS to find shortest path
        let mut queue = vec![(from, vec![])];
        let mut visited = std::collections::HashSet::new();
        visited.insert(from);

        while let Some((current, path)) = queue.pop() {
            // Check all registered migrations from current version
            for ((from_ver, to_ver), migration) in &self.migrations {
                if *from_ver == current && !visited.contains(to_ver) {
                    let mut new_path = path.clone();
                    new_path.push(*migration);

                    if *to_ver == to {
                        return Some(new_path);
                    }

                    visited.insert(*to_ver);
                    queue.push((*to_ver, new_path));
                }
            }
        }

        None
    }

    /// Apply migration path to WorldState
    ///
    /// Applies each migration in sequence, returning final state or first error.
    pub fn apply_migrations(
        &self,
        mut state: WorldState,
        migrations: &[MigrationFn],
    ) -> Result<WorldState, SerializationError> {
        for migration in migrations {
            state = migration(&state)?;
        }
        Ok(state)
    }

    /// Migrate WorldState from source to target version
    ///
    /// Finds and applies the migration path automatically.
    pub fn migrate(
        &self,
        state: WorldState,
        from: u32,
        to: u32,
    ) -> Result<WorldState, SerializationError> {
        if from == to {
            return Ok(state);
        }

        let path = self.find_migration_path(from, to).ok_or_else(|| {
            SerializationError::bincodedeserialize(format!(
                "No migration path found from version {} to {}",
                from, to
            ))
        })?;

        self.apply_migrations(state, &path)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global migration registry
///
/// Use this to register and access migrations throughout the application.
static GLOBAL_REGISTRY: std::sync::Mutex<Option<MigrationRegistry>> = std::sync::Mutex::new(None);

/// Get or initialize the global migration registry
pub fn global_registry() -> std::sync::MutexGuard<'static, Option<MigrationRegistry>> {
    GLOBAL_REGISTRY.lock().unwrap()
}

/// Initialize the global registry with default migrations
pub fn initialize_global_registry() {
    let mut guard = global_registry();
    if guard.is_none() {
        let registry = MigrationRegistry::new();

        // Register default migrations here
        // Example: registry.register_migration(1, 2, migrate_v1_to_v2);
        // When adding migrations in the future, make registry mutable again

        *guard = Some(registry);
    }
}

/// Versioned WorldState wrapper
///
/// Wraps WorldState with version information for safe serialization/deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedWorldState {
    /// Schema version
    pub schema_version: SchemaVersion,
    /// Actual world state data
    pub state: WorldState,
}

impl VersionedWorldState {
    /// Create a new versioned world state with current schema
    pub fn new(state: WorldState) -> Self {
        Self { schema_version: SchemaVersion::current(), state }
    }

    /// Load and migrate if necessary
    ///
    /// Automatically detects version and applies migrations.
    pub fn load_with_migration(data: &[u8]) -> Result<Self, SerializationError> {
        // Deserialize with version info
        let versioned: Self = bincode::deserialize(data)
            .map_err(|e| SerializationError::bincodedeserialize(e.to_string()))?;

        // Check if version is supported
        if !versioned.schema_version.is_supported() {
            return Err(SerializationError::bincodedeserialize(format!(
                "Unsupported schema version {}. Supported range: {}-{}",
                versioned.schema_version.version, MIN_SUPPORTED_VERSION, MAX_SUPPORTED_VERSION
            )));
        }

        // Check if migration is needed
        if versioned.schema_version.needs_migration() {
            let registry_guard = global_registry();
            let registry = registry_guard.as_ref().ok_or_else(|| {
                SerializationError::bincodedeserialize(
                    "Migration registry not initialized".to_string(),
                )
            })?;

            let migrated_state = registry.migrate(
                versioned.state,
                versioned.schema_version.version,
                CURRENT_SCHEMA_VERSION,
            )?;

            Ok(Self::new(migrated_state))
        } else {
            Ok(versioned)
        }
    }

    /// Save with version information
    pub fn save(&self) -> Result<Vec<u8>, SerializationError> {
        bincode::serialize(self).map_err(|e| SerializationError::bincodeserialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_current() {
        let version = SchemaVersion::current();
        assert_eq!(version.version, CURRENT_SCHEMA_VERSION);
        assert!(version.is_supported());
        assert!(!version.needs_migration());
    }

    #[test]
    fn test_schema_version_supported() {
        let version = SchemaVersion::new(1);
        assert!(version.is_supported());

        let too_old = SchemaVersion::new(0);
        assert!(!too_old.is_supported());

        let too_new = SchemaVersion::new(999);
        assert!(!too_new.is_supported());
    }

    #[test]
    fn test_migration_registry() {
        let mut registry = MigrationRegistry::new();

        // Register dummy migration
        fn migrate_v1_to_v2(state: &WorldState) -> Result<WorldState, SerializationError> {
            Ok(state.clone())
        }

        registry.register_migration(1, 2, migrate_v1_to_v2);

        assert!(registry.get_migration(1, 2).is_some());
        assert!(registry.get_migration(2, 3).is_none());
    }

    #[test]
    fn test_migration_path() {
        let mut registry = MigrationRegistry::new();

        fn migrate_v1_to_v2(state: &WorldState) -> Result<WorldState, SerializationError> {
            Ok(state.clone())
        }
        fn migrate_v2_to_v3(state: &WorldState) -> Result<WorldState, SerializationError> {
            Ok(state.clone())
        }

        registry.register_migration(1, 2, migrate_v1_to_v2);
        registry.register_migration(2, 3, migrate_v2_to_v3);

        // Direct path
        let path = registry.find_migration_path(1, 2);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);

        // Multi-step path
        let path = registry.find_migration_path(1, 3);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 2);

        // No path
        let path = registry.find_migration_path(3, 1);
        assert!(path.is_none());
    }

    #[test]
    fn test_versioned_world_state() {
        let state = WorldState::new();
        let versioned = VersionedWorldState::new(state);

        assert_eq!(versioned.schema_version.version, CURRENT_SCHEMA_VERSION);

        // Test serialization
        let bytes = versioned.save().unwrap();
        assert!(!bytes.is_empty());

        // Test deserialization
        let loaded = VersionedWorldState::load_with_migration(&bytes).unwrap();
        assert_eq!(loaded.schema_version.version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_versioned_roundtrip() {
        let mut state = WorldState::new();
        state.metadata.entity_count = 42;

        let versioned = VersionedWorldState::new(state);
        let bytes = versioned.save().unwrap();

        let loaded = VersionedWorldState::load_with_migration(&bytes).unwrap();
        assert_eq!(loaded.state.metadata.entity_count, 42);
    }
}
