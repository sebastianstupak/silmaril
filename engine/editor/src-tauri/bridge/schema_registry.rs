//! Component schema registry — tracks field definitions for all component types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The kind of a component field — drives which widget the inspector renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldType {
    F32 {
        min: Option<f32>,
        max: Option<f32>,
        step: Option<f32>,
    },
    Bool,
    String,
    Vec3,
    Enum {
        options: Vec<String>,
    },
}

/// Schema for a single field within a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    /// Internal field name (key in componentValues on the frontend).
    pub name: String,
    /// Human-readable label shown in the inspector.
    pub label: String,
    /// Field type (serialized as snake_case JSON key `"field_type"`, matching TypeScript FieldSchema).
    pub field_type: FieldType,
}

/// Full schema for one component type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Type name as it appears in EntityInfo.components (e.g. "Transform").
    pub name: String,
    /// Human-readable display name.
    pub label: String,
    /// UI category for grouping (e.g. "Core", "Physics", "Rendering").
    pub category: String,
    pub fields: Vec<FieldSchema>,
}

/// Registry of all known component schemas.
#[derive(Default)]
pub struct ComponentSchemaRegistry {
    schemas: HashMap<String, ComponentSchema>,
}

impl ComponentSchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or overwrites) a component schema.
    pub fn register(&mut self, schema: ComponentSchema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    /// Looks up a component by type name.
    pub fn get(&self, name: &str) -> Option<&ComponentSchema> {
        self.schemas.get(name)
    }

    /// Returns all registered schemas.
    pub fn all(&self) -> Vec<&ComponentSchema> {
        self.schemas.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_f32_field(name: &str) -> FieldSchema {
        FieldSchema {
            name: name.into(),
            label: name.into(),
            field_type: FieldType::F32 { min: None, max: None, step: None },
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema {
            name: "Health".into(),
            label: "Health".into(),
            category: "Core".into(),
            fields: vec![make_f32_field("current")],
        });
        assert!(reg.get("Health").is_some());
        assert!(reg.get("Missing").is_none());
    }

    #[test]
    fn test_all_returns_all_registered() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema { name: "A".into(), label: "A".into(), category: "x".into(), fields: vec![] });
        reg.register(ComponentSchema { name: "B".into(), label: "B".into(), category: "x".into(), fields: vec![] });
        assert_eq!(reg.all().len(), 2);
    }

    #[test]
    fn test_empty_registry_all() {
        let reg = ComponentSchemaRegistry::new();
        assert_eq!(reg.all().len(), 0);
    }

    #[test]
    fn field_type_serializes_with_kind_tag() {
        let v = serde_json::to_value(FieldType::Bool).unwrap();
        assert_eq!(v["kind"], "bool");

        let v = serde_json::to_value(FieldType::Vec3).unwrap();
        assert_eq!(v["kind"], "vec3");

        let f32_val = FieldType::F32 { min: Some(0.0), max: None, step: None };
        let v = serde_json::to_value(f32_val).unwrap();
        assert_eq!(v["kind"], "f32");
        assert_eq!(v["min"], 0.0);
        assert!(v["max"].is_null());

        let enum_val = FieldType::Enum { options: vec!["A".into(), "B".into()] };
        let v = serde_json::to_value(enum_val).unwrap();
        assert_eq!(v["kind"], "enum");
        assert_eq!(v["options"][0], "A");
    }

    #[test]
    fn test_register_overwrites_existing() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema { name: "X".into(), label: "Old".into(), category: "c".into(), fields: vec![] });
        reg.register(ComponentSchema { name: "X".into(), label: "New".into(), category: "c".into(), fields: vec![] });
        assert_eq!(reg.get("X").unwrap().label, "New");
    }
}
