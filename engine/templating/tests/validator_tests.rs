//! Unit tests for template validator.
//!
//! Tests validation of YAML syntax, component types, template references,
//! and generation of validation reports.

use engine_templating::template::{EntityDefinition, Template, TemplateMetadata};
use engine_templating::validator::TemplateValidator;
use rustc_hash::FxHashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a test template directory.
fn create_test_template_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Helper function to write a template to a file.
fn write_template(dir: &TempDir, filename: &str, template: &Template) -> PathBuf {
    let path = dir.path().join(filename);
    let yaml = serde_yaml::to_string(template).expect("Failed to serialize template");
    fs::write(&path, yaml).expect("Failed to write template file");
    path
}

#[test]
fn test_valid_template_passes() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create a valid template
    let metadata = TemplateMetadata {
        name: Some("Test Template".to_string()),
        description: Some("A valid test template".to_string()),
        author: Some("Test Author".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    // Add entity with valid components
    let mut components = FxHashMap::default();
    components.insert("Transform".to_string(), serde_yaml::Value::Null);
    components.insert("Health".to_string(), serde_yaml::Value::Null);

    let entity = EntityDefinition::new_inline(components, vec!["player".to_string()]);
    template.add_entity("Root".to_string(), entity);

    let path = write_template(&temp_dir, "valid.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(report.is_valid, "Expected template to be valid");
    assert_eq!(report.errors.len(), 0, "Expected no errors");
    assert_eq!(report.entity_count, 1, "Expected 1 entity");
    assert_eq!(report.template_references.len(), 0, "Expected no references");
}

#[test]
fn test_invalid_yaml_fails() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create a file with invalid YAML (tabs are not allowed in YAML)
    let path = temp_dir.path().join("invalid.yaml");
    fs::write(
        &path,
        "metadata:\n\tname: \"Test\"\nentities:\n  Root:\n\tcomponents:\n\t  Transform: value",
    )
    .expect("Failed to write file");

    // Validate
    let report = validator.validate(&path).expect("Validation should return report");

    assert!(!report.is_valid, "Expected template to be invalid");
    assert!(!report.errors.is_empty(), "Expected at least one YAML error");
    assert!(
        report.errors[0].contains("Invalid YAML syntax"),
        "Expected YAML syntax error message"
    );
}

#[test]
fn test_unknown_component_fails() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create template with unknown component
    let mut template = Template::new(TemplateMetadata::default());

    let mut components = FxHashMap::default();
    components.insert("NonExistentComponent".to_string(), serde_yaml::Value::Null);

    let entity = EntityDefinition::new_inline(components, vec![]);
    template.add_entity("Root".to_string(), entity);

    let path = write_template(&temp_dir, "unknown_component.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(!report.is_valid, "Expected template to be invalid");
    assert!(!report.errors.is_empty(), "Expected at least one error");
    assert!(
        report.errors[0].contains("NonExistentComponent"),
        "Expected error about unknown component"
    );
}

#[test]
fn test_missing_template_reference_fails() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create template referencing non-existent template
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_reference("non_existent.yaml".to_string());
    template.add_entity("Player".to_string(), entity);

    let path = write_template(&temp_dir, "missing_ref.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(!report.is_valid, "Expected template to be invalid");
    assert!(!report.errors.is_empty(), "Expected at least one error");
    assert!(
        report.errors[0].contains("does not exist"),
        "Expected error about missing template reference"
    );
}

#[test]
fn test_warnings_for_unused_entities() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create template with empty entity (no components, tags, or children)
    let mut template = Template::new(TemplateMetadata::default());

    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
    template.add_entity("EmptyEntity".to_string(), entity);

    let path = write_template(&temp_dir, "unused.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(report.is_valid, "Expected template to be valid (warnings only)");
    assert_eq!(report.errors.len(), 0, "Expected no errors");
    assert!(!report.warnings.is_empty(), "Expected at least one warning");
    assert!(
        report.warnings[0].contains("unused entity"),
        "Expected warning about unused entity"
    );
}

#[test]
fn test_custom_component_registration() {
    let mut validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Register custom component
    validator.register_component("CustomWeapon".to_string());

    // Create template with custom component
    let mut template = Template::new(TemplateMetadata::default());

    let mut components = FxHashMap::default();
    components.insert("CustomWeapon".to_string(), serde_yaml::Value::Null);

    let entity = EntityDefinition::new_inline(components, vec![]);
    template.add_entity("Root".to_string(), entity);

    let path = write_template(&temp_dir, "custom.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(report.is_valid, "Expected template to be valid");
    assert_eq!(report.errors.len(), 0, "Expected no errors");
}

#[test]
fn test_template_references_collected() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create referenced template first
    let referenced_template = Template::new(TemplateMetadata::default());
    write_template(&temp_dir, "referenced.yaml", &referenced_template);

    // Create template with reference
    let mut template = Template::new(TemplateMetadata::default());
    let entity = EntityDefinition::new_reference("referenced.yaml".to_string());
    template.add_entity("Player".to_string(), entity);

    let path = write_template(&temp_dir, "main.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(report.is_valid, "Expected template to be valid");
    assert_eq!(report.template_references.len(), 1, "Expected 1 reference");
    assert_eq!(report.template_references[0], "referenced.yaml");
}

#[test]
fn test_entity_count_with_children() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create template with nested children
    let mut template = Template::new(TemplateMetadata::default());

    let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec!["parent".to_string()]);
    let mut child1 = EntityDefinition::new_inline(FxHashMap::default(), vec!["child1".to_string()]);
    let child2 = EntityDefinition::new_inline(FxHashMap::default(), vec!["child2".to_string()]);

    child1.add_child("GrandChild".to_string(), child2);
    parent.add_child("Child".to_string(), child1);

    template.add_entity("Root".to_string(), parent);

    let path = write_template(&temp_dir, "nested.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(report.is_valid, "Expected template to be valid");
    assert_eq!(report.entity_count, 3, "Expected 3 entities (parent + child + grandchild)");
}

#[test]
fn test_unknown_component_in_overrides() {
    let validator = TemplateValidator::new();
    let temp_dir = create_test_template_dir();

    // Create referenced template
    let referenced_template = Template::new(TemplateMetadata::default());
    write_template(&temp_dir, "base.yaml", &referenced_template);

    // Create template with reference and invalid override
    let mut template = Template::new(TemplateMetadata::default());
    let mut entity = EntityDefinition::new_reference("base.yaml".to_string());
    entity.add_override("InvalidComponent".to_string(), serde_yaml::Value::Null);

    template.add_entity("Root".to_string(), entity);

    let path = write_template(&temp_dir, "override.yaml", &template);

    // Validate
    let report = validator.validate(&path).expect("Validation failed");

    assert!(!report.is_valid, "Expected template to be invalid");
    assert!(
        report.errors.iter().any(|e| e.contains("InvalidComponent")),
        "Expected error about invalid override component"
    );
}
