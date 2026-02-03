// Allow dead code for now - this module is part of the codegen API
// and will be used when component/system registry features are implemented
#![allow(dead_code)]

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Component registry for tracking components and systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRegistry {
    pub version: String,
    pub last_updated: String,
    pub components: Vec<ComponentEntry>,
    pub systems: Vec<SystemEntry>,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            components: Vec::new(),
            systems: Vec::new(),
        }
    }
}

impl ComponentRegistry {
    /// Load registry from .silmaril/components.json
    pub fn load() -> Result<Self> {
        let path = PathBuf::from(".silmaril/components.json");

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let registry: Self = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Save registry to .silmaril/components.json
    pub fn save(&self) -> Result<()> {
        let path = PathBuf::from(".silmaril/components.json");

        // Create .silmaril directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a component to the registry
    pub fn add_component(&mut self, entry: ComponentEntry) -> Result<()> {
        // Check for duplicates
        if self
            .components
            .iter()
            .any(|c| c.name == entry.name && c.location == entry.location)
        {
            bail!("Component '{}' already exists in location '{}'", entry.name, entry.location);
        }

        self.components.push(entry);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Add a system to the registry
    pub fn add_system(&mut self, entry: SystemEntry) -> Result<()> {
        // Check for duplicates
        if self
            .systems
            .iter()
            .any(|s| s.name == entry.name && s.location == entry.location)
        {
            bail!("System '{}' already exists in location '{}'", entry.name, entry.location);
        }

        self.systems.push(entry);
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Find a component by name
    pub fn find_component(&self, name: &str) -> Option<&ComponentEntry> {
        self.components.iter().find(|c| c.name == name)
    }

    /// Find a component by name in a specific location
    pub fn find_component_in_location(
        &self,
        name: &str,
        location: &str,
    ) -> Option<&ComponentEntry> {
        self.components.iter().find(|c| c.name == name && c.location == location)
    }

    /// Validate that all query components exist
    pub fn validate_query(&self, components: &[QueryComponent]) -> Result<()> {
        for comp in components {
            if self.find_component(&comp.component).is_none() {
                bail!("Component '{}' not found in registry", comp.component);
            }
        }
        Ok(())
    }

    /// Get all component names
    pub fn component_names(&self) -> Vec<String> {
        self.components.iter().map(|c| c.name.clone()).collect()
    }

    /// Get all system names
    pub fn system_names(&self) -> Vec<String> {
        self.systems.iter().map(|s| s.name.clone()).collect()
    }
}

/// Component entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub name: String,
    pub location: String,
    pub file: PathBuf,
    pub fields: Vec<FieldInfo>,
    pub derives: Vec<String>,
    pub documentation: Option<String>,
    pub created_at: String,
}

/// Field information for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub doc: Option<String>,
}

/// System entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntry {
    pub name: String,
    pub location: String,
    pub file: PathBuf,
    pub query: Vec<QueryComponentInfo>,
    pub phase: String,
    pub documentation: Option<String>,
    pub created_at: String,
}

/// Query component information for a system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryComponentInfo {
    pub component: String,
    pub access: String,
}

/// Query component for validation (not serialized)
#[derive(Debug, Clone, PartialEq)]
pub struct QueryComponent {
    pub component: String,
    pub access: QueryAccess,
}

/// Query access type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryAccess {
    Immutable,
    Mutable,
}

impl QueryAccess {
    pub fn to_string(&self) -> String {
        match self {
            QueryAccess::Immutable => "immutable".to_string(),
            QueryAccess::Mutable => "mutable".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "immutable" => Some(QueryAccess::Immutable),
            "mutable" => Some(QueryAccess::Mutable),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        let registry = ComponentRegistry::default();
        assert_eq!(registry.version, "1.0");
        assert_eq!(registry.components.len(), 0);
        assert_eq!(registry.systems.len(), 0);
    }

    #[test]
    fn test_query_access_conversion() {
        assert_eq!(QueryAccess::Immutable.to_string(), "immutable");
        assert_eq!(QueryAccess::Mutable.to_string(), "mutable");
        assert_eq!(QueryAccess::from_string("immutable"), Some(QueryAccess::Immutable));
        assert_eq!(QueryAccess::from_string("mutable"), Some(QueryAccess::Mutable));
        assert_eq!(QueryAccess::from_string("invalid"), None);
    }
}
