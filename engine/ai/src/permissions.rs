// stub — full implementation in Task 2

/// The level of access granted for a particular command or category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantLevel {
    /// Allowed for this single invocation only.
    Once,
    /// Allowed for the duration of the current editor session.
    Session,
    /// Permanently allowed (persisted across sessions).
    Always,
}

/// Stores permission grants, keyed by category and command id.
///
/// Full persistence and lookup logic is implemented in Task 2.
pub struct PermissionStore;

impl PermissionStore {
    /// Create an in-memory (non-persistent) permission store.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Create a permission store backed by the given file path.
    #[must_use]
    pub fn with_path(_path: &std::path::Path) -> Self {
        Self
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}
