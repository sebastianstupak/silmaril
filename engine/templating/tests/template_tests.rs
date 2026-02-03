//! Unit tests for template system core data structures.

use engine_templating::template::{EntityDefinition, EntitySource, Template, TemplateMetadata};
use rustc_hash::FxHashMap;

#[test]
fn test_template_creation() {
    let metadata = TemplateMetadata {
        name: Some("Test Template".to_string()),
        description: Some("A test template".to_string()),
        author: Some("Test Author".to_string()),
        version: Some("1.0.0".to_string()),
    };

    let template = Template::new(metadata.clone());

    assert_eq!(template.metadata.name, Some("Test Template".to_string()));
    assert_eq!(template.metadata.description, Some("A test template".to_string()));
    assert_eq!(template.metadata.author, Some("Test Author".to_string()));
    assert_eq!(template.metadata.version, Some("1.0.0".to_string()));
    assert_eq!(template.entity_count(), 0);
}

#[test]
fn test_template_add_entity() {
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    template.add_entity("Root".to_string(), entity);

    assert_eq!(template.entity_count(), 1);
    assert!(template.get_entity("Root").is_some());
}

#[test]
fn test_template_remove_entity() {
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    template.add_entity("Root".to_string(), entity);

    assert_eq!(template.entity_count(), 1);

    let removed = template.remove_entity("Root");
    assert!(removed.is_some());
    assert_eq!(template.entity_count(), 0);

    // Removing non-existent entity should return None
    let removed = template.remove_entity("NonExistent");
    assert!(removed.is_none());
}

#[test]
fn test_template_get_entity() {
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    template.add_entity("Root".to_string(), entity);

    assert!(template.get_entity("Root").is_some());
    assert!(template.get_entity("NonExistent").is_none());
}

#[test]
fn test_template_get_entity_mut() {
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    template.add_entity("Root".to_string(), entity);

    if let Some(entity) = template.get_entity_mut("Root") {
        entity.add_override("Health".to_string(), serde_yaml::Value::Null);
    }

    let entity = template.get_entity("Root").unwrap();
    assert_eq!(entity.overrides.len(), 1);
}

#[test]
fn test_entity_definition_inline() {
    let mut components = FxHashMap::default();
    components.insert("Transform".to_string(), serde_yaml::Value::Null);
    components.insert("Health".to_string(), serde_yaml::Value::Null);

    let tags = vec!["player".to_string(), "replicate".to_string()];

    let entity = EntityDefinition::new_inline(components.clone(), tags.clone());

    assert!(entity.is_inline());
    assert!(!entity.is_reference());

    match &entity.source {
        EntitySource::Inline { components: c, tags: t } => {
            assert_eq!(c.len(), 2);
            assert_eq!(t.len(), 2);
            assert!(c.contains_key("Transform"));
            assert!(c.contains_key("Health"));
            assert!(t.contains(&"player".to_string()));
            assert!(t.contains(&"replicate".to_string()));
        }
        _ => panic!("Expected Inline source"),
    }
}

#[test]
fn test_entity_definition_reference() {
    let template_path = "templates/characters/player.yaml".to_string();
    let entity = EntityDefinition::new_reference(template_path.clone());

    assert!(entity.is_reference());
    assert!(!entity.is_inline());

    match &entity.source {
        EntitySource::Reference { template } => {
            assert_eq!(template, &template_path);
        }
        _ => panic!("Expected Reference source"),
    }
}

#[test]
fn test_entity_add_override() {
    let mut entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    assert_eq!(entity.overrides.len(), 0);

    entity.add_override("Health".to_string(), serde_yaml::Value::Null);
    entity.add_override("Speed".to_string(), serde_yaml::Value::Null);

    assert_eq!(entity.overrides.len(), 2);
    assert!(entity.overrides.contains_key("Health"));
    assert!(entity.overrides.contains_key("Speed"));
}

#[test]
fn test_entity_add_child() {
    let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    let child1 = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    let child2 = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    assert_eq!(parent.children.len(), 0);

    parent.add_child("Camera".to_string(), child1);
    parent.add_child("Weapon".to_string(), child2);

    assert_eq!(parent.children.len(), 2);
    assert!(parent.children.contains_key("Camera"));
    assert!(parent.children.contains_key("Weapon"));
}

#[test]
fn test_metadata_default() {
    let metadata = TemplateMetadata::default();

    assert!(metadata.name.is_none());
    assert!(metadata.description.is_none());
    assert!(metadata.author.is_none());
    assert!(metadata.version.is_none());
}

#[test]
fn test_template_serialization_yaml() {
    let metadata = TemplateMetadata {
        name: Some("Test".to_string()),
        description: Some("Test template".to_string()),
        author: Some("Author".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec!["test".to_string()]);
    template.add_entity("Root".to_string(), entity);

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize template");
    assert!(!yaml.is_empty());

    // Deserialize from YAML
    let deserialized: Template =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize template");

    assert_eq!(deserialized.metadata.name, Some("Test".to_string()));
    assert_eq!(deserialized.entity_count(), 1);
    assert!(deserialized.get_entity("Root").is_some());
}

#[test]
fn test_template_serialization_bincode() {
    // Note: Direct bincode serialization of Template is not supported because
    // Template contains serde_yaml::Value which doesn't support bincode.
    // Use TemplateCompiler for YAML → Bincode conversion instead.
    // See bincode_integration_test.rs for proper bincode compilation tests.

    let metadata = TemplateMetadata {
        name: Some("Test".to_string()),
        description: Some("Test template".to_string()),
        author: Some("Author".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec!["test".to_string()]);
    template.add_entity("Root".to_string(), entity);

    // Test that metadata can be serialized to bincode (it doesn't contain serde_yaml::Value)
    let metadata_encoded =
        bincode::serialize(&template.metadata).expect("Failed to serialize metadata");
    assert!(!metadata_encoded.is_empty());

    let metadata_deserialized: TemplateMetadata =
        bincode::deserialize(&metadata_encoded).expect("Failed to deserialize metadata");

    assert_eq!(metadata_deserialized.name, Some("Test".to_string()));
    assert_eq!(metadata_deserialized.description, Some("Test template".to_string()));
}

#[test]
fn test_nested_children() {
    let mut template = Template::new(TemplateMetadata::default());

    // Create root entity
    let mut root = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    // Create child entity
    let mut camera = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    // Create grandchild entity
    let lens = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    // Build hierarchy
    camera.add_child("Lens".to_string(), lens);
    root.add_child("Camera".to_string(), camera);
    template.add_entity("Root".to_string(), root);

    // Verify hierarchy
    let root = template.get_entity("Root").unwrap();
    assert_eq!(root.children.len(), 1);

    let camera = root.children.get("Camera").unwrap();
    assert_eq!(camera.children.len(), 1);

    let lens = camera.children.get("Lens").unwrap();
    assert_eq!(lens.children.len(), 0);
}

#[test]
fn test_template_with_references() {
    let mut template = Template::new(TemplateMetadata::default());

    // Add inline entity
    let inline_entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    template.add_entity("Ground".to_string(), inline_entity);

    // Add reference entity
    let reference_entity =
        EntityDefinition::new_reference("templates/props/tower.yaml".to_string());
    template.add_entity("Tower".to_string(), reference_entity);

    assert_eq!(template.entity_count(), 2);

    let ground = template.get_entity("Ground").unwrap();
    assert!(ground.is_inline());

    let tower = template.get_entity("Tower").unwrap();
    assert!(tower.is_reference());
}

#[test]
fn test_entity_overrides_on_reference() {
    let mut entity =
        EntityDefinition::new_reference("templates/characters/player.yaml".to_string());

    assert_eq!(entity.overrides.len(), 0);

    // Add overrides
    entity.add_override("Transform".to_string(), serde_yaml::Value::Null);
    entity.add_override("Health".to_string(), serde_yaml::Value::Null);

    assert_eq!(entity.overrides.len(), 2);
    assert!(entity.is_reference());
}

#[test]
fn test_multiple_entities() {
    let mut template = Template::new(TemplateMetadata::default());

    // Add multiple entities
    template.add_entity(
        "Ground".to_string(),
        EntityDefinition::new_inline(FxHashMap::default(), vec!["static".to_string()]),
    );

    template.add_entity(
        "Player".to_string(),
        EntityDefinition::new_inline(FxHashMap::default(), vec!["player".to_string()]),
    );

    template.add_entity(
        "Enemy".to_string(),
        EntityDefinition::new_inline(FxHashMap::default(), vec!["enemy".to_string()]),
    );

    assert_eq!(template.entity_count(), 3);
    assert!(template.get_entity("Ground").is_some());
    assert!(template.get_entity("Player").is_some());
    assert!(template.get_entity("Enemy").is_some());
}
