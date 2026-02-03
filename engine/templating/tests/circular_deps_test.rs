//! Tests for circular dependency detection in template references.
//!
//! Tests three types of circular dependencies:
//! - Direct: A → A
//! - Indirect: A → B → A
//! - Deep: A → B → C → A

use engine_templating::template::{EntityDefinition, Template, TemplateMetadata};
use engine_templating::validator::TemplateValidator;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a test template directory.
fn create_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[test]
fn test_direct_circular_dependency() {
    // A → A (template references itself)
    let validator = TemplateValidator::new();
    let temp_dir = create_test_dir();

    // Create template that references itself
    let mut template = Template::new(TemplateMetadata {
        name: Some("Self-Referencing".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity = EntityDefinition::new_reference("self_ref.yaml".to_string());
    template.add_entity("Root".to_string(), entity);

    let path = temp_dir.path().join("self_ref.yaml");
    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&path, yaml).expect("Failed to write file");

    // Validate
    let report = validator.validate(&path).expect("Validation should return report");

    assert!(!report.is_valid, "Expected template to be invalid due to circular reference");
    assert!(!report.errors.is_empty(), "Expected at least one error");
    assert!(
        report.errors.iter().any(|e| e.contains("self_ref.yaml")),
        "Expected error to mention the circular reference path"
    );
}

#[test]
fn test_indirect_circular_dependency() {
    // A → B → A
    let validator = TemplateValidator::new();
    let temp_dir = create_test_dir();

    // Create template B that references A
    let mut template_b = Template::new(TemplateMetadata {
        name: Some("Template B".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_b = EntityDefinition::new_reference("a.yaml".to_string());
    template_b.add_entity("Root".to_string(), entity_b);

    let path_b = temp_dir.path().join("b.yaml");
    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    fs::write(&path_b, yaml_b).expect("Failed to write file");

    // Create template A that references B
    let mut template_a = Template::new(TemplateMetadata {
        name: Some("Template A".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_a = EntityDefinition::new_reference("b.yaml".to_string());
    template_a.add_entity("Root".to_string(), entity_a);

    let path_a = temp_dir.path().join("a.yaml");
    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    fs::write(&path_a, yaml_a).expect("Failed to write file");

    // Validate template A (should detect cycle A → B → A)
    let report = validator.validate(&path_a).expect("Validation should return report");

    assert!(!report.is_valid, "Expected template to be invalid due to circular reference");
    assert!(!report.errors.is_empty(), "Expected at least one error");

    // Check that error mentions the cycle
    let has_cycle_error =
        report.errors.iter().any(|e| e.contains("a.yaml") && e.contains("b.yaml"));
    assert!(has_cycle_error, "Expected error to mention both a.yaml and b.yaml in cycle");
}

#[test]
fn test_deep_circular_dependency() {
    // A → B → C → A
    let validator = TemplateValidator::new();
    let temp_dir = create_test_dir();

    // Create template C that references A
    let mut template_c = Template::new(TemplateMetadata {
        name: Some("Template C".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_c = EntityDefinition::new_reference("a.yaml".to_string());
    template_c.add_entity("Root".to_string(), entity_c);

    let path_c = temp_dir.path().join("c.yaml");
    let yaml_c = serde_yaml::to_string(&template_c).expect("Failed to serialize");
    fs::write(&path_c, yaml_c).expect("Failed to write file");

    // Create template B that references C
    let mut template_b = Template::new(TemplateMetadata {
        name: Some("Template B".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_b = EntityDefinition::new_reference("c.yaml".to_string());
    template_b.add_entity("Root".to_string(), entity_b);

    let path_b = temp_dir.path().join("b.yaml");
    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    fs::write(&path_b, yaml_b).expect("Failed to write file");

    // Create template A that references B
    let mut template_a = Template::new(TemplateMetadata {
        name: Some("Template A".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_a = EntityDefinition::new_reference("b.yaml".to_string());
    template_a.add_entity("Root".to_string(), entity_a);

    let path_a = temp_dir.path().join("a.yaml");
    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    fs::write(&path_a, yaml_a).expect("Failed to write file");

    // Validate template A (should detect cycle A → B → C → A)
    let report = validator.validate(&path_a).expect("Validation should return report");

    assert!(!report.is_valid, "Expected template to be invalid due to circular reference");
    assert!(!report.errors.is_empty(), "Expected at least one error");

    // Check that error mentions the cycle
    let has_cycle_error = report
        .errors
        .iter()
        .any(|e| e.contains("a.yaml") && e.contains("b.yaml") && e.contains("c.yaml"));
    assert!(has_cycle_error, "Expected error to mention a.yaml, b.yaml, and c.yaml in cycle");
}

#[test]
fn test_no_circular_dependency_with_diamond() {
    // Diamond pattern (NOT a cycle):
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    // Both B and C reference D, but no cycle exists

    let validator = TemplateValidator::new();
    let temp_dir = create_test_dir();

    // Create template D (leaf)
    let template_d = Template::new(TemplateMetadata {
        name: Some("Template D".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let path_d = temp_dir.path().join("d.yaml");
    let yaml_d = serde_yaml::to_string(&template_d).expect("Failed to serialize");
    fs::write(&path_d, yaml_d).expect("Failed to write file");

    // Create template C that references D
    let mut template_c = Template::new(TemplateMetadata {
        name: Some("Template C".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_c = EntityDefinition::new_reference("d.yaml".to_string());
    template_c.add_entity("Root".to_string(), entity_c);

    let path_c = temp_dir.path().join("c.yaml");
    let yaml_c = serde_yaml::to_string(&template_c).expect("Failed to serialize");
    fs::write(&path_c, yaml_c).expect("Failed to write file");

    // Create template B that references D
    let mut template_b = Template::new(TemplateMetadata {
        name: Some("Template B".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_b = EntityDefinition::new_reference("d.yaml".to_string());
    template_b.add_entity("Root".to_string(), entity_b);

    let path_b = temp_dir.path().join("b.yaml");
    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    fs::write(&path_b, yaml_b).expect("Failed to write file");

    // Create template A that references B and C
    let mut template_a = Template::new(TemplateMetadata {
        name: Some("Template A".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let mut parent = EntityDefinition::new_reference("b.yaml".to_string());
    let child = EntityDefinition::new_reference("c.yaml".to_string());
    parent.add_child("ChildC".to_string(), child);

    template_a.add_entity("Root".to_string(), parent);

    let path_a = temp_dir.path().join("a.yaml");
    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    fs::write(&path_a, yaml_a).expect("Failed to write file");

    // Validate template A (should be valid - diamond is NOT a cycle)
    let report = validator.validate(&path_a).expect("Validation failed");

    assert!(report.is_valid, "Expected diamond pattern to be valid (not a cycle)");
    assert_eq!(report.errors.len(), 0, "Expected no errors for diamond pattern");
}

#[test]
fn test_multiple_independent_references_no_cycle() {
    // A → B, A → C (no cycle, just multiple independent references)
    let validator = TemplateValidator::new();
    let temp_dir = create_test_dir();

    // Create template B
    let template_b = Template::new(TemplateMetadata {
        name: Some("Template B".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let path_b = temp_dir.path().join("b.yaml");
    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    fs::write(&path_b, yaml_b).expect("Failed to write file");

    // Create template C
    let template_c = Template::new(TemplateMetadata {
        name: Some("Template C".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let path_c = temp_dir.path().join("c.yaml");
    let yaml_c = serde_yaml::to_string(&template_c).expect("Failed to serialize");
    fs::write(&path_c, yaml_c).expect("Failed to write file");

    // Create template A that references both B and C
    let mut template_a = Template::new(TemplateMetadata {
        name: Some("Template A".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity_b_ref = EntityDefinition::new_reference("b.yaml".to_string());
    let entity_c_ref = EntityDefinition::new_reference("c.yaml".to_string());

    template_a.add_entity("EntityB".to_string(), entity_b_ref);
    template_a.add_entity("EntityC".to_string(), entity_c_ref);

    let path_a = temp_dir.path().join("a.yaml");
    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    fs::write(&path_a, yaml_a).expect("Failed to write file");

    // Validate template A (should be valid)
    let report = validator.validate(&path_a).expect("Validation failed");

    assert!(report.is_valid, "Expected multiple independent references to be valid");
    assert_eq!(report.errors.len(), 0, "Expected no errors");
    assert_eq!(report.template_references.len(), 2, "Expected 2 template references");
}
