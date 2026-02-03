use silm::codegen::registry::*;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to create a test component entry
fn create_test_component(name: &str, location: &str) -> ComponentEntry {
    ComponentEntry {
        name: name.to_string(),
        location: location.to_string(),
        file: PathBuf::from(format!("{}/src/components/{}.rs", location, name.to_lowercase())),
        fields: vec![FieldInfo {
            name: "value".to_string(),
            type_name: "f32".to_string(),
            doc: None,
        }],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        documentation: Some("Test component".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

// Helper to create a test system entry
fn create_test_system(name: &str, location: &str) -> SystemEntry {
    SystemEntry {
        name: name.to_string(),
        location: location.to_string(),
        file: PathBuf::from(format!("{}/src/systems/{}.rs", location, name)),
        query: vec![QueryComponentInfo {
            component: "TestComponent".to_string(),
            access: "immutable".to_string(),
        }],
        phase: "update".to_string(),
        documentation: Some("Test system".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn test_registry_default() {
    let registry = ComponentRegistry::default();
    assert_eq!(registry.version, "1.0");
    assert!(registry.components.is_empty());
    assert!(registry.systems.is_empty());
    assert!(!registry.last_updated.is_empty());
}

#[test]
fn test_add_component_success() {
    let mut registry = ComponentRegistry::default();
    let entry = create_test_component("Health", "shared");

    let result = registry.add_component(entry);
    assert!(result.is_ok());
    assert_eq!(registry.components.len(), 1);
    assert_eq!(registry.components[0].name, "Health");
}

#[test]
fn test_add_component_duplicate_error() {
    let mut registry = ComponentRegistry::default();
    let entry = create_test_component("Health", "shared");

    registry.add_component(entry.clone()).unwrap();
    let result = registry.add_component(entry);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already exists in location"));
}

#[test]
fn test_add_component_same_name_different_location() {
    let mut registry = ComponentRegistry::default();
    let entry1 = create_test_component("Health", "shared");
    let entry2 = create_test_component("Health", "client");

    registry.add_component(entry1).unwrap();
    let result = registry.add_component(entry2);

    assert!(result.is_ok());
    assert_eq!(registry.components.len(), 2);
}

#[test]
fn test_add_component_updates_timestamp() {
    let mut registry = ComponentRegistry::default();
    let initial_timestamp = registry.last_updated.clone();

    // Sleep a tiny bit to ensure timestamp changes
    std::thread::sleep(std::time::Duration::from_millis(10));

    let entry = create_test_component("Health", "shared");
    registry.add_component(entry).unwrap();

    assert_ne!(registry.last_updated, initial_timestamp);
}

#[test]
fn test_add_system_success() {
    let mut registry = ComponentRegistry::default();
    let entry = create_test_system("health_regen", "shared");

    let result = registry.add_system(entry);
    assert!(result.is_ok());
    assert_eq!(registry.systems.len(), 1);
    assert_eq!(registry.systems[0].name, "health_regen");
}

#[test]
fn test_add_system_duplicate_error() {
    let mut registry = ComponentRegistry::default();
    let entry = create_test_system("health_regen", "shared");

    registry.add_system(entry.clone()).unwrap();
    let result = registry.add_system(entry);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already exists in location"));
}

#[test]
fn test_add_system_same_name_different_location() {
    let mut registry = ComponentRegistry::default();
    let entry1 = create_test_system("render_system", "client");
    let entry2 = create_test_system("render_system", "server");

    registry.add_system(entry1).unwrap();
    let result = registry.add_system(entry2);

    assert!(result.is_ok());
    assert_eq!(registry.systems.len(), 2);
}

#[test]
fn test_find_component_exists() {
    let mut registry = ComponentRegistry::default();
    let entry = create_test_component("Health", "shared");
    registry.add_component(entry).unwrap();

    let found = registry.find_component("Health");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Health");
}

#[test]
fn test_find_component_not_exists() {
    let registry = ComponentRegistry::default();
    let found = registry.find_component("NonExistent");
    assert!(found.is_none());
}

#[test]
fn test_find_component_in_location() {
    let mut registry = ComponentRegistry::default();
    registry
        .add_component(create_test_component("Health", "shared"))
        .unwrap();
    registry
        .add_component(create_test_component("Health", "client"))
        .unwrap();

    let found_shared = registry.find_component_in_location("Health", "shared");
    let found_client = registry.find_component_in_location("Health", "client");
    let found_server = registry.find_component_in_location("Health", "server");

    assert!(found_shared.is_some());
    assert_eq!(found_shared.unwrap().location, "shared");
    assert!(found_client.is_some());
    assert_eq!(found_client.unwrap().location, "client");
    assert!(found_server.is_none());
}

#[test]
fn test_validate_query_all_exist() {
    let mut registry = ComponentRegistry::default();
    registry
        .add_component(create_test_component("Health", "shared"))
        .unwrap();
    registry
        .add_component(create_test_component("Position", "shared"))
        .unwrap();

    let query_components = vec![
        QueryComponent {
            component: "Health".to_string(),
            access: QueryAccess::Mutable,
        },
        QueryComponent {
            component: "Position".to_string(),
            access: QueryAccess::Immutable,
        },
    ];

    let result = registry.validate_query(&query_components);
    assert!(result.is_ok());
}

#[test]
fn test_validate_query_missing_component() {
    let mut registry = ComponentRegistry::default();
    registry
        .add_component(create_test_component("Health", "shared"))
        .unwrap();

    let query_components = vec![
        QueryComponent {
            component: "Health".to_string(),
            access: QueryAccess::Mutable,
        },
        QueryComponent {
            component: "NonExistent".to_string(),
            access: QueryAccess::Immutable,
        },
    ];

    let result = registry.validate_query(&query_components);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found in registry"));
}

#[test]
fn test_component_names() {
    let mut registry = ComponentRegistry::default();
    registry
        .add_component(create_test_component("Health", "shared"))
        .unwrap();
    registry
        .add_component(create_test_component("Position", "shared"))
        .unwrap();
    registry
        .add_component(create_test_component("Velocity", "shared"))
        .unwrap();

    let names = registry.component_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"Health".to_string()));
    assert!(names.contains(&"Position".to_string()));
    assert!(names.contains(&"Velocity".to_string()));
}

#[test]
fn test_system_names() {
    let mut registry = ComponentRegistry::default();
    registry
        .add_system(create_test_system("health_regen", "shared"))
        .unwrap();
    registry
        .add_system(create_test_system("movement", "shared"))
        .unwrap();

    let names = registry.system_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"health_regen".to_string()));
    assert!(names.contains(&"movement".to_string()));
}

#[test]
fn test_field_info_structure() {
    let field = FieldInfo {
        name: "current".to_string(),
        type_name: "f32".to_string(),
        doc: Some("Current value".to_string()),
    };

    assert_eq!(field.name, "current");
    assert_eq!(field.type_name, "f32");
    assert_eq!(field.doc, Some("Current value".to_string()));
}

#[test]
fn test_query_component_info_structure() {
    let query_info = QueryComponentInfo {
        component: "Health".to_string(),
        access: "mutable".to_string(),
    };

    assert_eq!(query_info.component, "Health");
    assert_eq!(query_info.access, "mutable");
}

#[test]
fn test_query_access_to_string() {
    assert_eq!(QueryAccess::Immutable.to_string(), "immutable");
    assert_eq!(QueryAccess::Mutable.to_string(), "mutable");
}

#[test]
fn test_query_access_from_string() {
    assert_eq!(
        QueryAccess::from_string("immutable"),
        Some(QueryAccess::Immutable)
    );
    assert_eq!(
        QueryAccess::from_string("mutable"),
        Some(QueryAccess::Mutable)
    );
    assert_eq!(QueryAccess::from_string("invalid"), None);
    assert_eq!(QueryAccess::from_string(""), None);
}
