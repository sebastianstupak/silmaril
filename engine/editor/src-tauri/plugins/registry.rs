//! Panel registry — tracks all registered panels and their configurations.

use std::collections::HashMap;

/// Source of a panel's UI definition.
#[derive(Debug, Clone)]
pub enum PanelSource {
    /// Schema-driven panel (auto-rendered from metadata).
    Schema(SchemaDefinition),
    /// JavaScript bundle loaded in an iframe.
    JsBundle(String),
}

/// A schema definition describing a panel's sections and fields.
#[derive(Debug, Clone)]
pub struct SchemaDefinition {
    /// Display title of the panel.
    pub title: String,
    /// Sections within the panel.
    pub sections: Vec<SchemaSection>,
}

/// A named section within a schema panel.
#[derive(Debug, Clone)]
pub struct SchemaSection {
    /// Section heading.
    pub name: String,
    /// Fields within this section.
    pub fields: Vec<SchemaField>,
}

/// A single editable field within a schema section.
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field label.
    pub name: String,
    /// Type hint (e.g. "f32", "bool", "enum", "color").
    pub field_type: String,
    /// Default value as JSON.
    pub default: Option<serde_json::Value>,
    /// Optional numeric range (min, max).
    pub range: Option<(f64, f64)>,
    /// Optional enum variants.
    pub options: Option<Vec<String>>,
}

/// Registry of all available editor panels.
#[derive(Default)]
pub struct PanelRegistry {
    panels: HashMap<String, PanelSource>,
}

impl PanelRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a panel by name.
    pub fn register(&mut self, name: String, source: PanelSource) {
        self.panels.insert(name, source);
    }

    /// Looks up a panel by name.
    pub fn get(&self, name: &str) -> Option<&PanelSource> {
        self.panels.get(name)
    }

    /// Lists all registered panel names.
    pub fn list(&self) -> Vec<&String> {
        self.panels.keys().collect()
    }
}
