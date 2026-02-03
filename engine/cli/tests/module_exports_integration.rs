use anyhow::Result;
use silm::codegen::module_exports::update_module_exports;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a project structure
fn create_project_structure(base_path: &std::path::Path) -> Result<()> {
    let dirs = vec![
        "shared/src/components",
        "shared/src/systems",
        "client/src/components",
        "client/src/systems",
        "server/src/components",
        "server/src/systems",
    ];

    for dir in dirs {
        std::fs::create_dir_all(base_path.join(dir))?;
    }

    Ok(())
}

/// Helper function to read file contents
fn read_file(path: &PathBuf) -> String {
    std::fs::read_to_string(path).expect("Failed to read file")
}

#[test]
fn test_full_component_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_project_structure(temp_dir.path())?;

    let shared_components = temp_dir.path().join("shared/src/components");

    // Simulate adding multiple components
    update_module_exports(&shared_components, "Health", "component")?;
    update_module_exports(&shared_components, "Transform", "component")?;
    update_module_exports(&shared_components, "Velocity", "component")?;
    update_module_exports(&shared_components, "Armor", "component")?;

    // Verify mod.rs was created and has all components
    let mod_file = shared_components.join("mod.rs");
    assert!(mod_file.exists());

    let content = read_file(&mod_file);

    // Verify all components are present
    assert!(content.contains("pub mod armor;"));
    assert!(content.contains("pub mod health;"));
    assert!(content.contains("pub mod transform;"));
    assert!(content.contains("pub mod velocity;"));

    // Verify all re-exports are present
    assert!(content.contains("pub use armor::Armor;"));
    assert!(content.contains("pub use health::Health;"));
    assert!(content.contains("pub use transform::Transform;"));
    assert!(content.contains("pub use velocity::Velocity;"));

    // Verify alphabetical order
    let armor_pos = content.find("pub mod armor;").unwrap();
    let health_pos = content.find("pub mod health;").unwrap();
    let transform_pos = content.find("pub mod transform;").unwrap();
    let velocity_pos = content.find("pub mod velocity;").unwrap();

    assert!(armor_pos < health_pos);
    assert!(health_pos < transform_pos);
    assert!(transform_pos < velocity_pos);

    Ok(())
}

#[test]
fn test_full_system_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_project_structure(temp_dir.path())?;

    let shared_systems = temp_dir.path().join("shared/src/systems");

    // Simulate adding multiple systems
    update_module_exports(&shared_systems, "health_regen", "system")?;
    update_module_exports(&shared_systems, "physics_update", "system")?;
    update_module_exports(&shared_systems, "collision_check", "system")?;

    // Verify mod.rs was created
    let mod_file = shared_systems.join("mod.rs");
    assert!(mod_file.exists());

    let content = read_file(&mod_file);

    // Verify header mentions systems
    assert!(content.contains("silm add system"));

    // Verify all systems are present
    assert!(content.contains("pub mod collision_check;"));
    assert!(content.contains("pub mod health_regen;"));
    assert!(content.contains("pub mod physics_update;"));

    // Verify re-exports
    assert!(content.contains("pub use collision_check::collision_check;"));
    assert!(content.contains("pub use health_regen::health_regen;"));
    assert!(content.contains("pub use physics_update::physics_update;"));

    Ok(())
}

#[test]
fn test_multiple_locations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_project_structure(temp_dir.path())?;

    // Add components to different locations
    let shared_components = temp_dir.path().join("shared/src/components");
    let client_components = temp_dir.path().join("client/src/components");
    let server_components = temp_dir.path().join("server/src/components");

    update_module_exports(&shared_components, "Health", "component")?;
    update_module_exports(&client_components, "CameraState", "component")?;
    update_module_exports(&server_components, "AIState", "component")?;

    // Verify each location has its own mod.rs
    assert!(shared_components.join("mod.rs").exists());
    assert!(client_components.join("mod.rs").exists());
    assert!(server_components.join("mod.rs").exists());

    // Verify contents are independent
    let shared_content = read_file(&shared_components.join("mod.rs"));
    let client_content = read_file(&client_components.join("mod.rs"));
    let server_content = read_file(&server_components.join("mod.rs"));

    assert!(shared_content.contains("pub mod health;"));
    assert!(!shared_content.contains("pub mod camera_state;"));
    assert!(!shared_content.contains("pub mod a_i_state;"));

    assert!(client_content.contains("pub mod camera_state;"));
    assert!(!client_content.contains("pub mod health;"));

    assert!(server_content.contains("pub mod a_i_state;"));
    assert!(!server_content.contains("pub mod health;"));

    Ok(())
}

#[test]
fn test_incremental_additions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let components_dir = temp_dir.path().join("src/components");

    // Add components incrementally
    update_module_exports(&components_dir, "First", "component")?;
    let content1 = read_file(&components_dir.join("mod.rs"));
    assert_eq!(content1.matches("pub mod").count(), 1);

    update_module_exports(&components_dir, "Second", "component")?;
    let content2 = read_file(&components_dir.join("mod.rs"));
    assert_eq!(content2.matches("pub mod").count(), 2);

    update_module_exports(&components_dir, "Third", "component")?;
    let content3 = read_file(&components_dir.join("mod.rs"));
    assert_eq!(content3.matches("pub mod").count(), 3);

    // Verify all three are present and sorted
    let first_pos = content3.find("pub mod first;").unwrap();
    let second_pos = content3.find("pub mod second;").unwrap();
    let third_pos = content3.find("pub mod third;").unwrap();

    assert!(first_pos < second_pos);
    assert!(second_pos < third_pos);

    Ok(())
}

#[test]
fn test_mixed_components_and_systems() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_project_structure(temp_dir.path())?;

    let components_dir = temp_dir.path().join("shared/src/components");
    let systems_dir = temp_dir.path().join("shared/src/systems");

    // Add components
    update_module_exports(&components_dir, "Health", "component")?;
    update_module_exports(&components_dir, "Velocity", "component")?;

    // Add systems
    update_module_exports(&systems_dir, "health_regen", "system")?;
    update_module_exports(&systems_dir, "movement", "system")?;

    // Verify both mod.rs files exist independently
    let components_mod = components_dir.join("mod.rs");
    let systems_mod = systems_dir.join("mod.rs");

    assert!(components_mod.exists());
    assert!(systems_mod.exists());

    // Verify component mod.rs has correct content
    let components_content = read_file(&components_mod);
    assert!(components_content.contains("silm add component"));
    assert!(components_content.contains("pub mod health;"));
    assert!(components_content.contains("pub mod velocity;"));
    assert!(!components_content.contains("health_regen"));

    // Verify system mod.rs has correct content
    let systems_content = read_file(&systems_mod);
    assert!(systems_content.contains("silm add system"));
    assert!(systems_content.contains("pub mod health_regen;"));
    assert!(systems_content.contains("pub mod movement;"));
    assert!(!systems_content.contains("Health"));

    Ok(())
}

#[test]
fn test_resilience_to_manual_edits() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let components_dir = temp_dir.path().join("src/components");
    std::fs::create_dir_all(&components_dir)?;

    // Create a manually edited mod.rs
    let mod_file = components_dir.join("mod.rs");
    std::fs::write(
        &mod_file,
        "// Custom header comment\n\
         // Additional info\n\n\
         pub mod health;\n\
         // Some random comment\n\
         pub mod velocity;\n\n\
         pub use health::Health;\n\
         pub use velocity::Velocity;\n",
    )?;

    // Add a new component
    update_module_exports(&components_dir, "Armor", "component")?;

    // Verify the new component was added and sorting is maintained
    let content = read_file(&mod_file);

    assert!(content.contains("pub mod armor;"));
    assert!(content.contains("pub mod health;"));
    assert!(content.contains("pub mod velocity;"));

    // Verify sorting
    let armor_pos = content.find("pub mod armor;").unwrap();
    let health_pos = content.find("pub mod health;").unwrap();
    let velocity_pos = content.find("pub mod velocity;").unwrap();

    assert!(armor_pos < health_pos);
    assert!(health_pos < velocity_pos);

    Ok(())
}
