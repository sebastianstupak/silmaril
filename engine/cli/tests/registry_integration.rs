use silm::codegen::registry::*;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to create a test component entry
fn create_test_component(name: &str, location: &str) -> ComponentEntry {
    ComponentEntry {
        name: name.to_string(),
        location: location.to_string(),
        file: PathBuf::from(format!("{}/src/components/{}.rs", location, name.to_lowercase())),
        fields: vec![
            FieldInfo { name: "current".to_string(), type_name: "f32".to_string(), doc: None },
            FieldInfo { name: "max".to_string(), type_name: "f32".to_string(), doc: None },
        ],
        derives: vec!["Debug".to_string(), "Clone".to_string(), "Default".to_string()],
        documentation: Some("Test component for health".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

// Helper to create a test system entry
fn create_test_system(name: &str, location: &str, components: Vec<(&str, &str)>) -> SystemEntry {
    SystemEntry {
        name: name.to_string(),
        location: location.to_string(),
        file: PathBuf::from(format!("{}/src/systems/{}.rs", location, name)),
        query: components
            .into_iter()
            .map(|(comp, access)| QueryComponentInfo {
                component: comp.to_string(),
                access: access.to_string(),
            })
            .collect(),
        phase: "update".to_string(),
        documentation: Some(format!("Test system: {}", name)),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn test_save_and_load_empty_registry() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let registry = ComponentRegistry::default();
    registry.save().unwrap();

    // Verify file exists
    let registry_file = temp_dir.path().join(".silmaril/components.json");
    assert!(registry_file.exists());

    // Load and verify
    let loaded = ComponentRegistry::load().unwrap();
    assert_eq!(loaded.version, "1.0");
    assert!(loaded.components.is_empty());
    assert!(loaded.systems.is_empty());
}

#[test]
fn test_save_and_load_with_components() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mut registry = ComponentRegistry::default();
    registry.add_component(create_test_component("Health", "shared")).unwrap();
    registry.add_component(create_test_component("Position", "shared")).unwrap();
    registry.add_component(create_test_component("Velocity", "shared")).unwrap();

    registry.save().unwrap();

    // Load and verify
    let loaded = ComponentRegistry::load().unwrap();
    assert_eq!(loaded.components.len(), 3);
    assert!(loaded.find_component("Health").is_some());
    assert!(loaded.find_component("Position").is_some());
    assert!(loaded.find_component("Velocity").is_some());
}

#[test]
fn test_save_and_load_with_systems() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mut registry = ComponentRegistry::default();
    registry.add_component(create_test_component("Health", "shared")).unwrap();
    registry
        .add_component(create_test_component("RegenerationRate", "shared"))
        .unwrap();

    registry
        .add_system(create_test_system(
            "health_regen",
            "shared",
            vec![("Health", "mutable"), ("RegenerationRate", "immutable")],
        ))
        .unwrap();

    registry.save().unwrap();

    // Load and verify
    let loaded = ComponentRegistry::load().unwrap();
    assert_eq!(loaded.systems.len(), 1);
    assert_eq!(loaded.systems[0].name, "health_regen");
    assert_eq!(loaded.systems[0].query.len(), 2);
    assert_eq!(loaded.systems[0].query[0].component, "Health");
    assert_eq!(loaded.systems[0].query[0].access, "mutable");
}

#[test]
fn test_load_nonexistent_returns_default() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Don't create the file, just try to load
    let loaded = ComponentRegistry::load().unwrap();
    assert_eq!(loaded.version, "1.0");
    assert!(loaded.components.is_empty());
    assert!(loaded.systems.is_empty());
}

#[test]
fn test_full_workflow_add_component_then_system() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Step 1: Create and save registry with component
    let mut registry = ComponentRegistry::default();
    registry.add_component(create_test_component("Health", "shared")).unwrap();
    registry.save().unwrap();

    // Step 2: Load registry and add system
    let mut loaded = ComponentRegistry::load().unwrap();
    assert_eq!(loaded.components.len(), 1);

    // Validate that component exists before adding system
    let query_components =
        vec![QueryComponent { component: "Health".to_string(), access: QueryAccess::Mutable }];
    assert!(loaded.validate_query(&query_components).is_ok());

    // Add system
    loaded
        .add_system(create_test_system("health_system", "shared", vec![("Health", "mutable")]))
        .unwrap();
    loaded.save().unwrap();

    // Step 3: Load again and verify everything persists
    let final_loaded = ComponentRegistry::load().unwrap();
    assert_eq!(final_loaded.components.len(), 1);
    assert_eq!(final_loaded.systems.len(), 1);
    assert!(final_loaded.find_component("Health").is_some());
}

#[test]
fn test_json_serialization_format() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mut registry = ComponentRegistry::default();
    registry.add_component(create_test_component("Health", "shared")).unwrap();
    registry.save().unwrap();

    // Read raw JSON and verify format
    let json_content = std::fs::read_to_string(".silmaril/components.json").unwrap();

    // Verify it's pretty-printed (contains newlines and indentation)
    assert!(json_content.contains("{\n"));
    assert!(json_content.contains("  \"version\""));
    assert!(json_content.contains("  \"components\""));

    // Verify it contains expected data
    assert!(json_content.contains("\"Health\""));
    assert!(json_content.contains("\"shared\""));
    assert!(json_content.contains("\"current\""));
    assert!(json_content.contains("\"max\""));
}

#[test]
fn test_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Ensure .silmaril directory doesn't exist yet
    let silmaril_dir = temp_dir.path().join(".silmaril");
    assert!(!silmaril_dir.exists());

    // Save registry
    let registry = ComponentRegistry::default();
    registry.save().unwrap();

    // Verify directory was created
    assert!(silmaril_dir.exists());
    assert!(silmaril_dir.is_dir());
}

#[test]
fn test_multiple_saves_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Save first version
    let mut registry = ComponentRegistry::default();
    registry.add_component(create_test_component("Health", "shared")).unwrap();
    registry.save().unwrap();

    // Load, modify, and save again
    let mut loaded = ComponentRegistry::load().unwrap();
    loaded.add_component(create_test_component("Position", "shared")).unwrap();
    loaded.save().unwrap();

    // Load final version and verify
    let final_loaded = ComponentRegistry::load().unwrap();
    assert_eq!(final_loaded.components.len(), 2);
    assert!(final_loaded.find_component("Health").is_some());
    assert!(final_loaded.find_component("Position").is_some());
}
