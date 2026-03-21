//! Permission store for the MCP server.
//!
//! Controls which command categories an MCP client may execute.
//! Grants persist to `<project>/.silmaril/ai-permissions.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How long a permission grant lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantLevel {
    /// Allow this single call only.
    Once,
    /// Allow for the lifetime of the editor session.
    Session,
    /// Allow forever (persisted to disk).
    Always,
}

/// Persisted format.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedGrants {
    grants: HashMap<String, GrantLevel>,
}

/// Runtime permission state.
///
/// Manages permission grants with three levels:
/// - `Once`: consumed after a single use
/// - `Session`: in-memory only, not persisted
/// - `Always`: persisted to disk for future sessions
pub struct PermissionStore {
    session_grants: HashMap<String, GrantLevel>,
    persisted_path: Option<PathBuf>,
}

impl PermissionStore {
    /// Create an empty store with no persistence path (for tests).
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_grants: HashMap::new(),
            persisted_path: None,
        }
    }

    /// Create a store that persists `Always` grants to `project_root/.silmaril/ai-permissions.json`.
    ///
    /// Loads any existing persistent grants from disk if the file exists.
    #[must_use]
    pub fn with_path(project_root: &Path) -> Self {
        let path = project_root.join(".silmaril").join("ai-permissions.json");
        let mut store = Self {
            session_grants: HashMap::new(),
            persisted_path: Some(path.clone()),
        };
        if path.exists() {
            store.load_from_disk();
        }
        store
    }

    /// Check if the given category is currently granted.
    ///
    /// Returns `None` if no grant exists (caller must request permission).
    /// Returns the grant level if one exists.
    pub fn check(&self, category: &str) -> Option<GrantLevel> {
        self.session_grants.get(category).copied()
    }

    /// Record a grant.
    ///
    /// If the grant level is `Always`, immediately persists to disk.
    /// For `Once` and `Session` grants, only stored in memory.
    ///
    /// # Disk write failures
    ///
    /// If the `.silmaril/` directory is not writable, the write silently fails.
    /// The grant remains active for the current session but will not survive a restart.
    pub fn grant(&mut self, category: &str, level: GrantLevel) {
        self.session_grants.insert(category.to_string(), level);
        if level == GrantLevel::Always {
            self.save_to_disk();
        }
    }

    /// Remove a `Once` grant after it has been used.
    ///
    /// Only removes the grant if it is exactly at `Once` level.
    /// Other grant levels are left untouched.
    pub fn consume_once(&mut self, category: &str) {
        if self.session_grants.get(category) == Some(&GrantLevel::Once) {
            self.session_grants.remove(category);
        }
    }

    fn save_to_disk(&self) {
        let Some(path) = &self.persisted_path else { return };
        let always_grants: HashMap<String, GrantLevel> = self
            .session_grants
            .iter()
            .filter(|(_, &v)| v == GrantLevel::Always)
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        let persisted = PersistedGrants {
            grants: always_grants,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(path, json);
        }
    }

    fn load_from_disk(&mut self) {
        let Some(path) = &self.persisted_path else { return };
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(persisted) = serde_json::from_str::<PersistedGrants>(&content) {
                for (cat, level) in persisted.grants {
                    // Only restore Always grants from disk
                    if level == GrantLevel::Always {
                        self.session_grants.insert(cat, level);
                    }
                }
            }
        }
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_returns_none_when_no_grant() {
        let store = PermissionStore::new();
        assert!(store.check("scene").is_none());
    }

    #[test]
    fn session_grant_is_visible() {
        let mut store = PermissionStore::new();
        store.grant("scene", GrantLevel::Session);
        assert_eq!(store.check("scene"), Some(GrantLevel::Session));
    }

    #[test]
    fn once_grant_consumed_after_use() {
        let mut store = PermissionStore::new();
        store.grant("viewport", GrantLevel::Once);
        assert_eq!(store.check("viewport"), Some(GrantLevel::Once));
        store.consume_once("viewport");
        assert!(store.check("viewport").is_none());
    }

    #[test]
    fn always_grant_persists_to_disk_and_loads() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        {
            let mut store = PermissionStore::with_path(dir.path());
            store.grant("read", GrantLevel::Always);
        }
        // Load fresh instance
        let store = PermissionStore::with_path(dir.path());
        assert_eq!(store.check("read"), Some(GrantLevel::Always));
    }

    #[test]
    fn session_grant_does_not_persist() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        {
            let mut store = PermissionStore::with_path(dir.path());
            store.grant("scene", GrantLevel::Session);
        }
        let store = PermissionStore::with_path(dir.path());
        // Session grant should not have persisted
        assert!(store.check("scene").is_none());
    }
}
