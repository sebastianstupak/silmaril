//! Template save/load — YAML for development, Bincode for release.

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::undo::EntityId;
use crate::error::OpsError;

mod json_as_string {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let s = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Value, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        serde_json::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateComponent {
    pub type_name: String,
    #[serde(with = "json_as_string")]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateEntity {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateState {
    pub name: String,
    pub entities: Vec<TemplateEntity>,
}

impl TemplateState {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), entities: Vec::new() }
    }

    pub fn save_yaml(&self, path: &Path) -> Result<(), OpsError> {
        let yaml = serde_yaml::to_string(self).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(path, yaml).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })
    }

    pub fn load_yaml(path: &Path) -> Result<Self, OpsError> {
        let data = std::fs::read_to_string(path).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
        let mut state: Self = serde_yaml::from_str(&data).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        state.name = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        Ok(state)
    }

    pub fn save_bincode(&self, path: &Path) -> Result<(), OpsError> {
        let bytes = bincode::serialize(self).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(path, bytes).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })
    }

    pub fn load_bincode(path: &Path) -> Result<Self, OpsError> {
        let bytes = std::fs::read(path).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
        let mut state: Self = bincode::deserialize(&bytes).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        state.name = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        Ok(state)
    }

    pub fn add_entity(&mut self, entity: TemplateEntity) {
        self.entities.push(entity);
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Option<TemplateEntity> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }

    pub fn find_entity(&self, id: EntityId) -> Option<&TemplateEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn find_entity_mut(&mut self, id: EntityId) -> Option<&mut TemplateEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }
}
