//! Scene save/load — YAML for development, Bincode for release.
//!
//! Scenes are the primary unit of persistence for the editor and game runtime.
//! YAML format is used during development for human-readable diffs; Bincode
//! is used for release builds where load speed matters.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serde helper that encodes `serde_json::Value` as a JSON string so that
/// binary formats like Bincode (which lack `deserialize_any`) can round-trip it.
mod json_as_string {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        serde_json::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// A single component attached to a scene entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneComponent {
    pub type_name: String,
    #[serde(with = "json_as_string")]
    pub data: serde_json::Value,
}

/// An entity within a scene, carrying an optional name and a list of components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneEntity {
    pub id: u64,
    pub name: Option<String>,
    pub components: Vec<SceneComponent>,
}

/// A complete scene — a named collection of entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub name: String,
    pub entities: Vec<SceneEntity>,
}

impl Scene {
    /// Create an empty scene with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entities: Vec::new(),
        }
    }

    /// Serialize to YAML and write to `path`.
    pub fn save_yaml(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Read a YAML file and deserialize into a [`Scene`].
    pub fn load_yaml(path: &Path) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let scene: Scene = serde_yaml::from_str(&data)?;
        Ok(scene)
    }

    /// Serialize to Bincode and write to `path`.
    pub fn save_bincode(&self, path: &Path) -> Result<()> {
        let bytes = bincode::serialize(self)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Read a Bincode file and deserialize into a [`Scene`].
    pub fn load_bincode(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let scene: Scene = bincode::deserialize(&bytes)?;
        Ok(scene)
    }

    /// Append an entity to the scene.
    pub fn add_entity(&mut self, entity: SceneEntity) {
        self.entities.push(entity);
    }

    /// Remove and return the entity with the given `id`, if present.
    pub fn remove_entity(&mut self, id: u64) -> Option<SceneEntity> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }
}
