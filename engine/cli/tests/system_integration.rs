use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project structure
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let shared_systems = temp_dir.path().join("shared/src/systems");
    fs::create_dir_all(&shared_systems).unwrap();
    temp_dir
}

#[test]
fn test_system_generation_creates_file() {
    let temp_dir = create_test_project();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // Simulate the add_system command
    let result = silm::commands::add::handle_add_command(
        silm::commands::add::AddCommand::System {
            name: "health_regen".to_string(),
            query: "&mut Health,&RegenerationRate".to_string(),
            location: "shared".to_string(),
            phase: "update".to_string(),
            doc: None,
        }
    );
    
    assert!(result.is_ok());
    
    let file_path = temp_dir.path().join("shared/src/systems/health_regen.rs");
    assert!(file_path.exists());
}

#[test]
fn test_generated_system_code_structure() {
    let temp_dir = create_test_project();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    let result = silm::commands::add::handle_add_command(
        silm::commands::add::AddCommand::System {
            name: "health_regen".to_string(),
            query: "&mut Health,&RegenerationRate".to_string(),
            location: "shared".to_string(),
            phase: "update".to_string(),
            doc: Some("Regenerate health over time".to_string()),
        }
    );
    
    assert!(result.is_ok());
    
    let file_path = temp_dir.path().join("shared/src/systems/health_regen.rs");
    let content = fs::read_to_string(&file_path).unwrap();
    
    // Verify imports
    assert!(content.contains("use engine_core::ecs::{Query, World}"));
    assert!(content.contains("use tracing::{debug, instrument}"));
    
    // Verify function signature
    assert!(content.contains("pub fn health_regen(world: &mut World, delta_time: f32)"));
    
    // Verify query
    assert!(content.contains("world.query::<(&mut Health, &RegenerationRate)>()"));
    
    // Verify documentation
    assert!(content.contains("Regenerate health over time"));
    
    // Verify tests
    assert!(content.contains("#[cfg(test)]"));
    assert!(content.contains("fn test_health_regen_basic()"));
}

#[test]
fn test_system_generation_invalid_name() {
    let temp_dir = create_test_project();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    let result = silm::commands::add::handle_add_command(
        silm::commands::add::AddCommand::System {
            name: "HealthRegen".to_string(), // Invalid: PascalCase
            query: "&mut Health".to_string(),
            location: "shared".to_string(),
            phase: "update".to_string(),
            doc: None,
        }
    );
    
    assert!(result.is_err());
}

#[test]
fn test_system_generation_invalid_query() {
    let temp_dir = create_test_project();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    let result = silm::commands::add::handle_add_command(
        silm::commands::add::AddCommand::System {
            name: "health_regen".to_string(),
            query: "Health".to_string(), // Invalid: missing &
            location: "shared".to_string(),
            phase: "update".to_string(),
            doc: None,
        }
    );
    
    assert!(result.is_err());
}

#[test]
fn test_system_generation_different_phases() {
    let temp_dir = create_test_project();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // Test fixed_update phase
    let result = silm::commands::add::handle_add_command(
        silm::commands::add::AddCommand::System {
            name: "physics_step".to_string(),
            query: "&mut Transform,&Velocity".to_string(),
            location: "shared".to_string(),
            phase: "fixed_update".to_string(),
            doc: None,
        }
    );
    
    assert!(result.is_ok());
    
    let file_path = temp_dir.path().join("shared/src/systems/physics_step.rs");
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("fixed_update"));
}
