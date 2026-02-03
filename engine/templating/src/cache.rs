//! YAML AST caching layer for template loading.
//!
//! This module provides a two-layer cache:
//! 1. YAML AST cache: Caches parsed `serde_yaml::Value` before Template deserialization
//! 2. Template cache: Caches fully deserialized `Template` instances
//!
//! This avoids re-parsing identical YAML files which is a major performance bottleneck.

use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A cache entry storing both the raw YAML AST and the deserialized template.
#[derive(Clone)]
pub struct CacheEntry {
    /// The parsed YAML AST (cheap to clone, Arc-wrapped)
    pub yaml_ast: Arc<serde_yaml::Value>,

    /// The deserialized template (Arc-wrapped for cheap cloning)
    pub template: Arc<crate::template::Template>,
}

/// Two-layer cache for YAML and Template data.
pub struct TemplateCache {
    /// Maps file paths to cached entries
    entries: FxHashMap<PathBuf, CacheEntry>,
}

impl TemplateCache {
    /// Creates a new empty cache.
    #[must_use] 
    pub fn new() -> Self {
        Self { entries: FxHashMap::default() }
    }

    /// Gets a cached entry by path.
    #[must_use] 
    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        self.entries.get(path)
    }

    /// Inserts a new cache entry.
    pub fn insert(
        &mut self,
        path: PathBuf,
        yaml_ast: serde_yaml::Value,
        template: crate::template::Template,
    ) {
        self.entries.insert(
            path,
            CacheEntry { yaml_ast: Arc::new(yaml_ast), template: Arc::new(template) },
        );
    }

    /// Clears all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns true if the cache is empty.
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of cached templates.
    #[must_use] 
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for TemplateCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{Template, TemplateMetadata};

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = TemplateCache::new();
        let path = PathBuf::from("test.yaml");
        let yaml_ast = serde_yaml::Value::Null;
        let template = Template::new(TemplateMetadata::default());

        cache.insert(path.clone(), yaml_ast, template.clone());

        assert_eq!(cache.len(), 1);
        assert!(cache.get(&path).is_some());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = TemplateCache::new();
        let path = PathBuf::from("test.yaml");
        let yaml_ast = serde_yaml::Value::Null;
        let template = Template::new(TemplateMetadata::default());

        cache.insert(path, yaml_ast, template);
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }
}
