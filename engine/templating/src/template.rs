//! Template data structures for entity definitions.
//!
//! This module provides the core data structures for the template system.
//! Templates are YAML files that define entities without IDs, used for levels,
//! characters, props, UI, and game state.
//!
//! # Examples
//!
//! ```rust
//! use engine_templating::template::{Template, TemplateMetadata, EntityDefinition, EntitySource};
//! use rustc_hash::FxHashMap;
//!
//! // Create a simple template
//! let metadata = TemplateMetadata {
//!     name: Some("Player Character".to_string()),
//!     description: Some("Main player character".to_string()),
//!     author: Some("GameDev Team".to_string()),
//!     version: Some("1.0".to_string()),
//! };
//!
//! let mut entities = FxHashMap::default();
//! let mut components = FxHashMap::default();
//! components.insert(
//!     "Transform".to_string(),
//!     serde_yaml::Value::Null,
//! );
//!
//! entities.insert(
//!     "Root".to_string(),
//!     EntityDefinition {
//!         source: EntitySource::Inline {
//!             components,
//!             tags: vec!["player".to_string(), "replicate".to_string()],
//!         },
//!         overrides: FxHashMap::default(),
//!         children: FxHashMap::default(),
//!     },
//! );
//!
//! let template = Template { metadata, entities };
//! ```

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// A template defines a collection of named entities that can be spawned into a world.
///
/// Templates are the foundation of the content authoring system. They define
/// entity hierarchies with components and relationships, without specific entity IDs.
///
/// # Examples
///
/// ```rust
/// use engine_templating::template::{Template, TemplateMetadata};
/// use rustc_hash::FxHashMap;
///
/// let template = Template {
///     metadata: TemplateMetadata {
///         name: Some("Test Template".to_string()),
///         description: None,
///         author: None,
///         version: None,
///     },
///     entities: FxHashMap::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Template {
    /// Metadata about the template (name, description, author, version)
    pub metadata: TemplateMetadata,

    /// Named entity definitions in this template
    ///
    /// Keys are entity names (e.g., "Root", "Player", "Camera")
    /// Values are the entity definitions with components and children
    pub entities: FxHashMap<String, EntityDefinition>,
}

impl Template {
    /// Creates a new empty template with the given metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata};
    ///
    /// let metadata = TemplateMetadata {
    ///     name: Some("My Template".to_string()),
    ///     description: Some("A test template".to_string()),
    ///     author: Some("Developer".to_string()),
    ///     version: Some("1.0".to_string()),
    /// };
    ///
    /// let template = Template::new(metadata);
    /// assert_eq!(template.entities.len(), 0);
    /// ```
    #[must_use]
    pub fn new(metadata: TemplateMetadata) -> Self {
        Self { metadata, entities: FxHashMap::default() }
    }

    /// Adds an entity definition to the template.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata, EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut template = Template::new(TemplateMetadata::default());
    ///
    /// let entity_def = EntityDefinition {
    ///     source: EntitySource::Inline {
    ///         components: FxHashMap::default(),
    ///         tags: vec![],
    ///     },
    ///     overrides: FxHashMap::default(),
    ///     children: FxHashMap::default(),
    /// };
    ///
    /// template.add_entity("Root".to_string(), entity_def);
    /// assert_eq!(template.entities.len(), 1);
    /// ```
    pub fn add_entity(&mut self, name: String, definition: EntityDefinition) {
        self.entities.insert(name, definition);
    }

    /// Removes an entity definition from the template.
    ///
    /// Returns the removed entity definition if it existed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata, EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut template = Template::new(TemplateMetadata::default());
    ///
    /// let entity_def = EntityDefinition {
    ///     source: EntitySource::Inline {
    ///         components: FxHashMap::default(),
    ///         tags: vec![],
    ///     },
    ///     overrides: FxHashMap::default(),
    ///     children: FxHashMap::default(),
    /// };
    ///
    /// template.add_entity("Root".to_string(), entity_def);
    /// let removed = template.remove_entity("Root");
    /// assert!(removed.is_some());
    /// assert_eq!(template.entities.len(), 0);
    /// ```
    pub fn remove_entity(&mut self, name: &str) -> Option<EntityDefinition> {
        self.entities.remove(name)
    }

    /// Gets a reference to an entity definition by name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata, EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut template = Template::new(TemplateMetadata::default());
    ///
    /// let entity_def = EntityDefinition {
    ///     source: EntitySource::Inline {
    ///         components: FxHashMap::default(),
    ///         tags: vec![],
    ///     },
    ///     overrides: FxHashMap::default(),
    ///     children: FxHashMap::default(),
    /// };
    ///
    /// template.add_entity("Root".to_string(), entity_def);
    /// assert!(template.get_entity("Root").is_some());
    /// assert!(template.get_entity("NonExistent").is_none());
    /// ```
    #[must_use]
    pub fn get_entity(&self, name: &str) -> Option<&EntityDefinition> {
        self.entities.get(name)
    }

    /// Gets a mutable reference to an entity definition by name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata, EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut template = Template::new(TemplateMetadata::default());
    ///
    /// let entity_def = EntityDefinition {
    ///     source: EntitySource::Inline {
    ///         components: FxHashMap::default(),
    ///         tags: vec![],
    ///     },
    ///     overrides: FxHashMap::default(),
    ///     children: FxHashMap::default(),
    /// };
    ///
    /// template.add_entity("Root".to_string(), entity_def);
    ///
    /// if let Some(entity) = template.get_entity_mut("Root") {
    ///     entity.overrides.insert("Health".to_string(), serde_yaml::Value::Null);
    /// }
    /// ```
    pub fn get_entity_mut(&mut self, name: &str) -> Option<&mut EntityDefinition> {
        self.entities.get_mut(name)
    }

    /// Returns the number of top-level entities in the template.
    ///
    /// Note: This does not count nested children.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{Template, TemplateMetadata};
    ///
    /// let template = Template::new(TemplateMetadata::default());
    /// assert_eq!(template.entity_count(), 0);
    /// ```
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

/// Metadata about a template.
///
/// All fields are optional to allow flexibility in template authoring.
///
/// # Examples
///
/// ```rust
/// use engine_templating::template::TemplateMetadata;
///
/// let metadata = TemplateMetadata {
///     name: Some("Battle Arena".to_string()),
///     description: Some("5v5 competitive map".to_string()),
///     author: Some("Level Designer".to_string()),
///     version: Some("1.2.0".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TemplateMetadata {
    /// Human-readable name of the template
    pub name: Option<String>,

    /// Description of what this template represents
    pub description: Option<String>,

    /// Author or creator of the template
    pub author: Option<String>,

    /// Version string (e.g., "1.0", "2.1.3")
    pub version: Option<String>,
}

/// Defines a single entity within a template.
///
/// An entity can either be defined inline with components and tags,
/// or reference another template file.
///
/// # Examples
///
/// Inline entity:
/// ```rust
/// use engine_templating::template::{EntityDefinition, EntitySource};
/// use rustc_hash::FxHashMap;
///
/// let mut components = FxHashMap::default();
/// components.insert("Transform".to_string(), serde_yaml::Value::Null);
///
/// let entity = EntityDefinition {
///     source: EntitySource::Inline {
///         components,
///         tags: vec!["player".to_string()],
///     },
///     overrides: FxHashMap::default(),
///     children: FxHashMap::default(),
/// };
/// ```
///
/// Referenced entity:
/// ```rust
/// use engine_templating::template::{EntityDefinition, EntitySource};
/// use rustc_hash::FxHashMap;
///
/// let entity = EntityDefinition {
///     source: EntitySource::Reference {
///         template: "templates/characters/player.yaml".to_string(),
///     },
///     overrides: FxHashMap::default(),
///     children: FxHashMap::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityDefinition {
    /// The source of this entity's definition (inline or reference)
    pub source: EntitySource,

    /// Component overrides to apply on top of the source definition
    ///
    /// Keys are component names (e.g., "Transform", "Health")
    /// Values are component field overrides in YAML format
    pub overrides: FxHashMap<String, serde_yaml::Value>,

    /// Named child entities that should be spawned as children of this entity
    ///
    /// Keys are child entity names
    /// Values are the child entity definitions
    pub children: FxHashMap<String, EntityDefinition>,
}

impl EntityDefinition {
    /// Creates a new inline entity definition with the given components and tags.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::EntityDefinition;
    /// use rustc_hash::FxHashMap;
    ///
    /// let entity = EntityDefinition::new_inline(
    ///     FxHashMap::default(),
    ///     vec!["player".to_string()],
    /// );
    /// ```
    #[must_use]
    pub fn new_inline(components: FxHashMap<String, serde_yaml::Value>, tags: Vec<String>) -> Self {
        Self {
            source: EntitySource::Inline { components, tags },
            overrides: FxHashMap::default(),
            children: FxHashMap::default(),
        }
    }

    /// Creates a new reference entity definition pointing to another template.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::EntityDefinition;
    ///
    /// let entity = EntityDefinition::new_reference(
    ///     "templates/props/guard_tower.yaml".to_string()
    /// );
    /// ```
    #[must_use]
    pub fn new_reference(template: String) -> Self {
        Self {
            source: EntitySource::Reference { template },
            overrides: FxHashMap::default(),
            children: FxHashMap::default(),
        }
    }

    /// Adds a component override to this entity definition.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::EntityDefinition;
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    /// entity.add_override("Health".to_string(), serde_yaml::Value::Null);
    /// ```
    pub fn add_override(&mut self, component: String, value: serde_yaml::Value) {
        self.overrides.insert(component, value);
    }

    /// Adds a child entity to this entity definition.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::EntityDefinition;
    /// use rustc_hash::FxHashMap;
    ///
    /// let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    /// let child = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    ///
    /// parent.add_child("Camera".to_string(), child);
    /// ```
    pub fn add_child(&mut self, name: String, child: EntityDefinition) {
        self.children.insert(name, child);
    }

    /// Returns true if this entity is defined inline (not a reference).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let inline = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    /// assert!(inline.is_inline());
    ///
    /// let reference = EntityDefinition::new_reference("template.yaml".to_string());
    /// assert!(!reference.is_inline());
    /// ```
    #[must_use]
    pub fn is_inline(&self) -> bool {
        matches!(self.source, EntitySource::Inline { .. })
    }

    /// Returns true if this entity is a template reference.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::template::{EntityDefinition, EntitySource};
    /// use rustc_hash::FxHashMap;
    ///
    /// let reference = EntityDefinition::new_reference("template.yaml".to_string());
    /// assert!(reference.is_reference());
    ///
    /// let inline = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    /// assert!(!inline.is_reference());
    /// ```
    #[must_use]
    pub fn is_reference(&self) -> bool {
        matches!(self.source, EntitySource::Reference { .. })
    }
}

/// The source of an entity's definition.
///
/// Entities can be defined either inline with components and tags,
/// or by referencing another template file.
///
/// # Examples
///
/// ```rust
/// use engine_templating::template::EntitySource;
/// use rustc_hash::FxHashMap;
///
/// // Inline definition
/// let inline = EntitySource::Inline {
///     components: FxHashMap::default(),
///     tags: vec!["player".to_string()],
/// };
///
/// // Template reference
/// let reference = EntitySource::Reference {
///     template: "templates/characters/player.yaml".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EntitySource {
    /// Entity defined inline with components and tags
    Inline {
        /// Component definitions for this entity
        ///
        /// Keys are component names (e.g., "Transform", "Health")
        /// Values are component data in YAML format
        components: FxHashMap<String, serde_yaml::Value>,

        /// Tags for this entity (e.g., "player", "replicate", "static")
        tags: Vec<String>,
    },

    /// Entity defined by referencing another template file
    Reference {
        /// Path to the template file to reference
        ///
        /// Should be relative to the templates directory
        /// (e.g., "templates/characters/player.yaml")
        template: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let metadata = TemplateMetadata {
            name: Some("Test".to_string()),
            description: None,
            author: None,
            version: None,
        };

        let template = Template::new(metadata.clone());

        assert_eq!(template.metadata, metadata);
        assert_eq!(template.entity_count(), 0);
    }

    #[test]
    fn test_add_entity() {
        let mut template = Template::new(TemplateMetadata::default());
        let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

        template.add_entity("Root".to_string(), entity);

        assert_eq!(template.entity_count(), 1);
        assert!(template.get_entity("Root").is_some());
    }

    #[test]
    fn test_remove_entity() {
        let mut template = Template::new(TemplateMetadata::default());
        let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

        template.add_entity("Root".to_string(), entity);
        assert_eq!(template.entity_count(), 1);

        let removed = template.remove_entity("Root");
        assert!(removed.is_some());
        assert_eq!(template.entity_count(), 0);
    }

    #[test]
    fn test_entity_definition_inline() {
        let mut components = FxHashMap::default();
        components.insert("Transform".to_string(), serde_yaml::Value::Null);

        let entity = EntityDefinition::new_inline(components, vec!["player".to_string()]);

        assert!(entity.is_inline());
        assert!(!entity.is_reference());
    }

    #[test]
    fn test_entity_definition_reference() {
        let entity = EntityDefinition::new_reference("templates/player.yaml".to_string());

        assert!(entity.is_reference());
        assert!(!entity.is_inline());
    }

    #[test]
    fn test_add_override() {
        let mut entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

        entity.add_override("Health".to_string(), serde_yaml::Value::Null);

        assert_eq!(entity.overrides.len(), 1);
        assert!(entity.overrides.contains_key("Health"));
    }

    #[test]
    fn test_add_child() {
        let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
        let child = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

        parent.add_child("Camera".to_string(), child);

        assert_eq!(parent.children.len(), 1);
        assert!(parent.children.contains_key("Camera"));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = TemplateMetadata::default();

        assert!(metadata.name.is_none());
        assert!(metadata.description.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.version.is_none());
    }
}
