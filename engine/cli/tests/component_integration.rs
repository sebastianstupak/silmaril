use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use silm::commands::add::add_component;

/// Helper to create a minimal project structure for testing
fn create_project_structure(root: &PathBuf) {
    // Create shared/src/components directory
    let components_dir = root.join("shared/src/components");
    fs::create_dir_all(&components_dir).unwrap();

    // Create client/src/components directory
    let client_components = root.join("client/src/components");
    fs::create_dir_all(&client_components).unwrap();

    // Create server/src/components directory
    let server_components = root.join("server/src/components");
    fs::create_dir_all(&server_components).unwrap();
}

#[test]
fn test_add_component_basic() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Generate component
    let result = add_component(
        "Health",
        "current:f32,max:f32",
        "shared",
        Some("Default".to_string()),
        Some("Player health".to_string()),
    );

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    // Verify file created
    let component_file = root.join("shared/src/components/health.rs");
    assert!(
        component_file.exists(),
        "Component file not created: {}",
        component_file.display()
    );

    // Verify content
    let content = fs::read_to_string(&component_file).unwrap();
    assert!(content.contains("pub struct Health"));
    assert!(content.contains("pub current: f32"));
    assert!(content.contains("pub max: f32"));
    assert!(content.contains("impl Default for Health"));
    assert!(content.contains("/// Player health"));
}

#[test]
fn test_add_component_client_location() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component("CameraState", "fov:f32,zoom:f32", "client", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok());

    // Verify file in client location
    let component_file = root.join("client/src/components/camera_state.rs");
    assert!(component_file.exists());

    let content = fs::read_to_string(&component_file).unwrap();
    assert!(content.contains("pub struct CameraState"));
}

#[test]
fn test_add_component_server_location() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component("ServerState", "tick:u64", "server", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok());

    // Verify file in server location
    let component_file = root.join("server/src/components/server_state.rs");
    assert!(component_file.exists());
}

#[test]
fn test_add_component_with_complex_types() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component(
        "Inventory",
        "items:Vec<Item>,capacity:usize",
        "shared",
        Some("Default".to_string()),
        None,
    );

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok());

    let component_file = root.join("shared/src/components/inventory.rs");
    let content = fs::read_to_string(&component_file).unwrap();
    assert!(content.contains("pub items: Vec<Item>"));
    assert!(content.contains("pub capacity: usize"));
    assert!(content.contains("items: Vec::new()"));
}

#[test]
fn test_add_component_with_array_type() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component(
        "Transform",
        "position:[f32;3],rotation:[f32;4],scale:[f32;3]",
        "shared",
        Some("Default".to_string()),
        None,
    );

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok());

    let component_file = root.join("shared/src/components/transform.rs");
    let content = fs::read_to_string(&component_file).unwrap();
    assert!(content.contains("pub position: [f32;3]"));
    assert!(content.contains("position: [0.0; 3]"));
}

#[test]
fn test_add_component_duplicate_error() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Create component first time
    let result1 = add_component("Health", "hp:f32", "shared", None, None);
    assert!(result1.is_ok());

    // Try to create again
    let result2 = add_component("Health", "hp:f32", "shared", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result2.is_err(), "Should fail when file already exists");
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_add_component_invalid_name() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Try with lowercase name (invalid PascalCase)
    let result = add_component("health", "hp:f32", "shared", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must start with uppercase"));
}

#[test]
fn test_add_component_invalid_location() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component("Health", "hp:f32", "invalid_location", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid location"));
}

#[test]
fn test_add_component_missing_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    // Don't create project structure

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component("Health", "hp:f32", "shared", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn test_add_component_empty_fields() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = add_component("Empty", "", "shared", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
}

#[test]
fn test_add_component_invalid_field_format() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    create_project_structure(&root);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Invalid format: no colon
    let result = add_component("Health", "hp", "shared", None, None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
}
