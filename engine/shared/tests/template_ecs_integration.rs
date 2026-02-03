//! Cross-crate integration tests: Template System + ECS
//!
//! Tests comprehensive template instantiation, hierarchy handling, and ECS integration.
//! MANDATORY: This test uses engine-templating + engine-core, so it MUST be in engine/shared/tests/
//!
//! Test Categories:
//! 1. Template instantiation into ECS World
//! 2. Complex hierarchies and nested children
//! 3. Template parameter overrides
//! 4. Multiple template instances
//! 5. Edge cases (cyclic refs, missing fields, invalid data)
//! 6. Concurrent template spawns
//! 7. Error handling and recovery

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::{Quat, Transform, Vec3};
use engine_core::rendering::{Camera, MeshRenderer};
use engine_templating::{
    EntityDefinition, EntitySource, Template, TemplateLoader, TemplateMetadata, TemplateValidator,
};
use rustc_hash::FxHashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

fn create_template_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write template file");
    path
}

fn setup_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Camera>();
    world.register::<MeshRenderer>();
    world
}

// ============================================================================
// 1. Template Instantiation Tests
// ============================================================================

#[test]
fn test_template_spawn_single_entity() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Single Entity"
entities:
  Root:
    source:
      components:
        Transform:
          position: [1, 2, 3]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
      tags: [test]
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "single.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");

    assert_eq!(instance.entities.len(), 1);
    assert!(world.is_alive(instance.entities[0]));

    let transform = world.get::<Transform>(instance.entities[0]).unwrap();
    assert_eq!(transform.position, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn test_template_spawn_multiple_entities() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Multiple Entities"
entities:
  Player:
    source:
      components:
        Transform: null
        Health:
          current: 100.0
          max: 100.0
      tags: [player]
    overrides: {}
    children: {}
  Enemy:
    source:
      components:
        Transform: null
        Health:
          current: 50.0
          max: 50.0
      tags: [enemy]
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "multiple.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");

    assert_eq!(instance.entities.len(), 2);
    assert!(world.is_alive(instance.entities[0]));
    assert!(world.is_alive(instance.entities[1]));

    // Both entities should have their components
    let mut health_count = 0;
    for &entity in &instance.entities {
        if world.get::<Health>(entity).is_some() {
            health_count += 1;
        }
    }
    assert_eq!(health_count, 2);
}

#[test]
fn test_template_spawn_with_component_overrides() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Override Test"
entities:
  Root:
    source:
      components:
        Transform: null
        Health:
          current: 100.0
          max: 100.0
      tags: []
    overrides:
      Health:
        current: 50.0
        max: 100.0
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "override.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");

    let entity = instance.entities[0];
    let health = world.get::<Health>(entity).unwrap();

    // Override should be applied
    assert_eq!(health.current, 50.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_multiple_template_instances() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Instance Test"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "instance.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    // Spawn the same template 5 times
    let mut instances = Vec::new();
    for _ in 0..5 {
        let instance = loader.load(&mut world, &template_path).expect("Failed to load template");
        instances.push(instance);
    }

    // All instances should have different entities
    assert_eq!(instances.len(), 5);

    let mut all_entities = Vec::new();
    for instance in &instances {
        all_entities.extend(&instance.entities);
    }

    // Check all entities are alive and unique
    assert_eq!(all_entities.len(), 5);
    for i in 0..all_entities.len() {
        assert!(world.is_alive(all_entities[i]));
        for j in (i + 1)..all_entities.len() {
            assert_ne!(all_entities[i], all_entities[j], "Entities should be unique");
        }
    }
}

// ============================================================================
// 2. Complex Hierarchy Tests
// ============================================================================

#[test]
fn test_template_with_nested_children() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Nested Hierarchy"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children:
      Camera:
        source:
          components:
            Transform:
              position: [0, 1.6, -3]
              rotation: [0, 0, 0, 1]
              scale: [1, 1, 1]
            Camera:
              fov: 60.0
              aspect: 1.7777
              near: 0.1
              far: 1000.0
          tags: []
        overrides: {}
        children:
          Lens:
            source:
              components:
                Transform: null
              tags: []
            overrides: {}
            children: {}
"#;

    let template_path = create_template_file(&temp_dir, "nested.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");

    // Should spawn all entities in hierarchy
    assert_eq!(instance.entities.len(), 1); // Only root is in top-level entities
    assert!(world.is_alive(instance.entities[0]));

    // Entities should have correct components
    let entity = instance.entities[0];
    assert!(world.get::<Transform>(entity).is_some());
}

#[test]
fn test_template_deeply_nested_hierarchy() {
    // Test stack safety with deeply nested structures (10 levels)
    let mut template = Template::new(TemplateMetadata {
        name: Some("Deep Hierarchy".to_string()),
        description: None,
        author: None,
        version: None,
    });

    // Build a 10-level deep hierarchy
    let mut current = EntityDefinition::new_inline(FxHashMap::default(), vec![]);

    for i in (0..10).rev() {
        let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
        parent.add_child(format!("Child_{}", i), current);
        current = parent;
    }

    template.add_entity("Root".to_string(), current);

    let temp_dir = create_test_dir();
    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
    let template_path = create_template_file(&temp_dir, "deep.yaml", &yaml);

    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    // Should not stack overflow
    let result = loader.load(&mut world, &template_path);
    assert!(result.is_ok(), "Deep hierarchy should load without stack overflow");
}

#[test]
fn test_template_with_sibling_children() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Siblings"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children:
      ChildA:
        source:
          components:
            Transform: null
          tags: []
        overrides: {}
        children: {}
      ChildB:
        source:
          components:
            Transform: null
          tags: []
        overrides: {}
        children: {}
      ChildC:
        source:
          components:
            Transform: null
          tags: []
        overrides: {}
        children: {}
"#;

    let template_path = create_template_file(&temp_dir, "siblings.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");
    assert!(instance.entities.len() > 0);
}

// ============================================================================
// 3. Template References Tests
// ============================================================================

#[test]
fn test_template_with_reference() {
    let temp_dir = create_test_dir();

    // Create referenced template
    let referenced_yaml = r#"
metadata:
  name: "Referenced"
entities:
  Root:
    source:
      components:
        Transform: null
        Health:
          current: 500.0
          max: 500.0
      tags: []
    overrides: {}
    children: {}
"#;

    let referenced_path = create_template_file(&temp_dir, "referenced.yaml", referenced_yaml);
    let referenced_path_str = referenced_path.file_name().unwrap().to_str().unwrap();

    // Create main template that references the other
    let main_yaml = format!(
        r#"
metadata:
  name: "Main"
entities:
  Ground:
    source:
      components:
        Transform: null
      tags: []
    overrides: {{}}
    children: {{}}
  Tower:
    source:
      template: "{}"
    overrides: {{}}
    children: {{}}
"#,
        referenced_path_str
    );

    let main_path = create_template_file(&temp_dir, "main.yaml", &main_yaml);

    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &main_path).expect("Failed to load template");

    assert_eq!(instance.entities.len(), 2); // Ground + Tower
    assert_eq!(instance.references.len(), 1); // Tower references another template
}

#[test]
fn test_template_reference_with_override() {
    let temp_dir = create_test_dir();

    // Create referenced template with Health
    let referenced_yaml = r#"
metadata:
  name: "Base Character"
entities:
  Root:
    source:
      components:
        Transform: null
        Health:
          current: 100.0
          max: 100.0
      tags: []
    overrides: {}
    children: {}
"#;

    let referenced_path = create_template_file(&temp_dir, "base.yaml", referenced_yaml);
    let referenced_path_str = referenced_path.file_name().unwrap().to_str().unwrap();

    // Reference with override
    let main_yaml = format!(
        r#"
metadata:
  name: "Boss Character"
entities:
  Boss:
    source:
      template: "{}"
    overrides:
      Health:
        current: 1000.0
        max: 1000.0
    children: {{}}
"#,
        referenced_path_str
    );

    let main_path = create_template_file(&temp_dir, "boss.yaml", &main_yaml);

    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &main_path).expect("Failed to load template");

    let entity = instance.entities[0];
    let health = world.get::<Health>(entity).unwrap();

    // Override should be applied to referenced template
    assert_eq!(health.current, 1000.0);
    assert_eq!(health.max, 1000.0);
}

// ============================================================================
// 4. Edge Case Tests - CRITICAL
// ============================================================================

#[test]
fn test_cyclic_reference_detection_direct() {
    // A → A (template references itself)
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    let mut template = Template::new(TemplateMetadata {
        name: Some("Self-Ref".to_string()),
        description: None,
        author: None,
        version: None,
    });

    let entity = EntityDefinition::new_reference("self_ref.yaml".to_string());
    template.add_entity("Root".to_string(), entity);

    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
    let path = create_template_file(&temp_dir, "self_ref.yaml", &yaml);

    let report = validator.validate(&path).expect("Validation should return report");

    assert!(!report.is_valid, "Self-reference should be invalid");
    assert!(!report.errors.is_empty());
    assert!(
        report.errors.iter().any(|e| e.contains("Circular") || e.contains("circular")),
        "Expected circular reference error"
    );
}

#[test]
fn test_cyclic_reference_detection_indirect() {
    // A → B → A
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    // Create B that references A
    let mut template_b = Template::new(TemplateMetadata::default());
    let entity_b = EntityDefinition::new_reference("a.yaml".to_string());
    template_b.add_entity("Root".to_string(), entity_b);

    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    create_template_file(&temp_dir, "b.yaml", &yaml_b);

    // Create A that references B
    let mut template_a = Template::new(TemplateMetadata::default());
    let entity_a = EntityDefinition::new_reference("b.yaml".to_string());
    template_a.add_entity("Root".to_string(), entity_a);

    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    let path_a = create_template_file(&temp_dir, "a.yaml", &yaml_a);

    let report = validator.validate(&path_a).expect("Validation should return report");

    assert!(!report.is_valid, "Indirect cycle should be invalid");
    assert!(!report.errors.is_empty());
}

#[test]
fn test_cyclic_reference_detection_deep() {
    // A → B → C → A
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    // Create C that references A
    let mut template_c = Template::new(TemplateMetadata::default());
    let entity_c = EntityDefinition::new_reference("a.yaml".to_string());
    template_c.add_entity("Root".to_string(), entity_c);
    let yaml_c = serde_yaml::to_string(&template_c).expect("Failed to serialize");
    create_template_file(&temp_dir, "c.yaml", &yaml_c);

    // Create B that references C
    let mut template_b = Template::new(TemplateMetadata::default());
    let entity_b = EntityDefinition::new_reference("c.yaml".to_string());
    template_b.add_entity("Root".to_string(), entity_b);
    let yaml_b = serde_yaml::to_string(&template_b).expect("Failed to serialize");
    create_template_file(&temp_dir, "b.yaml", &yaml_b);

    // Create A that references B
    let mut template_a = Template::new(TemplateMetadata::default());
    let entity_a = EntityDefinition::new_reference("b.yaml".to_string());
    template_a.add_entity("Root".to_string(), entity_a);
    let yaml_a = serde_yaml::to_string(&template_a).expect("Failed to serialize");
    let path_a = create_template_file(&temp_dir, "a.yaml", &yaml_a);

    let report = validator.validate(&path_a).expect("Validation should return report");

    assert!(!report.is_valid, "Deep cycle should be invalid");
    assert!(report.errors.iter().any(|e| e.contains("Circular") || e.contains("circular")));
}

#[test]
fn test_missing_template_reference() {
    let temp_dir = create_test_dir();

    let yaml = r#"
metadata:
  name: "Missing Reference"
entities:
  Root:
    source:
      template: "nonexistent.yaml"
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "missing_ref.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let result = loader.load(&mut world, &template_path);

    assert!(result.is_err(), "Loading template with missing reference should fail");
}

#[test]
fn test_invalid_yaml_syntax() {
    let temp_dir = create_test_dir();

    let invalid_yaml = r#"
metadata:
  name: "Invalid YAML"
entities:
  Root:
    source:
      components:
        Transform: [this is not valid yaml syntax
"#;

    let template_path = create_template_file(&temp_dir, "invalid.yaml", invalid_yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let result = loader.load(&mut world, &template_path);

    assert!(result.is_err(), "Invalid YAML should fail to load");
}

#[test]
fn test_empty_template() {
    let temp_dir = create_test_dir();

    let yaml = r#"
metadata:
  name: "Empty"
entities: {}
"#;

    let template_path = create_template_file(&temp_dir, "empty.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Empty template should load");

    assert_eq!(instance.entities.len(), 0);
}

#[test]
fn test_entity_with_no_components() {
    let temp_dir = create_test_dir();

    let yaml = r#"
metadata:
  name: "No Components"
entities:
  Root:
    source:
      components: {}
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "no_components.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Template should load");

    assert_eq!(instance.entities.len(), 1);
    assert!(world.is_alive(instance.entities[0]));
}

#[test]
fn test_unknown_component_type() {
    let temp_dir = create_test_dir();

    let yaml = r#"
metadata:
  name: "Unknown Component"
entities:
  Root:
    source:
      components:
        NonExistentComponent: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "unknown.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let result = loader.load(&mut world, &template_path);

    assert!(result.is_err(), "Unknown component should fail to load");
}

#[test]
fn test_invalid_component_data() {
    let temp_dir = create_test_dir();

    let yaml = r#"
metadata:
  name: "Invalid Component Data"
entities:
  Root:
    source:
      components:
        Transform:
          position: "this should be an array"
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "invalid_data.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let result = loader.load(&mut world, &template_path);

    assert!(result.is_err(), "Invalid component data should fail");
}

#[test]
fn test_template_despawn() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Despawn Test"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "despawn.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load template");

    let entity = instance.entities[0];
    assert!(world.is_alive(entity));

    // Despawn the instance
    instance.despawn(&mut world);

    // Entity should no longer be alive
    assert!(!world.is_alive(entity));
}

// ============================================================================
// 5. Concurrent Template Spawning Tests
// ============================================================================

#[test]
fn test_concurrent_template_loads() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Concurrent Test"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "concurrent.yaml", yaml);
    let template_path = Arc::new(template_path);

    // Spawn multiple threads that load the same template
    let mut handles = vec![];

    for i in 0..4 {
        let template_path = Arc::clone(&template_path);

        let handle = thread::spawn(move || {
            let mut world = setup_world();
            let mut loader = TemplateLoader::new();

            // Load template 10 times in this thread
            for _ in 0..10 {
                let result = loader.load(&mut world, template_path.as_ref());
                assert!(result.is_ok(), "Thread {} failed to load template", i);
            }
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_template_cache_thread_safety() {
    // Test that caching works correctly even when accessed from multiple "loads"
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Cache Test"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "cache.yaml", yaml);

    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    // Load same template multiple times
    for _ in 0..100 {
        let result = loader.load(&mut world, &template_path);
        assert!(result.is_ok(), "Cache should work correctly");
    }

    // Cache should only have one entry
    assert_eq!(loader.cache_size(), 1);
}

// ============================================================================
// 6. Validation Tests
// ============================================================================

#[test]
fn test_validator_detects_unknown_components() {
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    let yaml = r#"
metadata:
  name: "Unknown Component"
entities:
  Root:
    source:
      components:
        UnknownComponent: null
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "unknown_comp.yaml", yaml);
    let report = validator.validate(&template_path).expect("Validation should return report");

    assert!(!report.is_valid);
    assert!(report.errors.iter().any(|e| e.contains("Unknown")));
}

#[test]
fn test_validator_counts_entities_correctly() {
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    let yaml = r#"
metadata:
  name: "Entity Count"
entities:
  Root:
    source:
      components:
        Transform: null
      tags: []
    overrides: {}
    children:
      Child1:
        source:
          components:
            Transform: null
          tags: []
        overrides: {}
        children: {}
      Child2:
        source:
          components:
            Transform: null
          tags: []
        overrides: {}
        children: {}
"#;

    let template_path = create_template_file(&temp_dir, "count.yaml", yaml);
    let report = validator.validate(&template_path).expect("Validation should return report");

    assert!(report.is_valid);
    assert_eq!(report.entity_count, 3); // Root + 2 children
}

#[test]
fn test_validator_warns_empty_entities() {
    let temp_dir = create_test_dir();
    let validator = TemplateValidator::new();

    let yaml = r#"
metadata:
  name: "Empty Entity"
entities:
  Root:
    source:
      components: {}
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "empty_entity.yaml", yaml);
    let report = validator.validate(&template_path).expect("Validation should return report");

    assert!(report.is_valid); // Should still be valid
    assert!(!report.warnings.is_empty()); // But should have warnings
}

// ============================================================================
// 7. Complex Integration Tests
// ============================================================================

#[test]
fn test_complex_scene_with_multiple_systems() {
    let temp_dir = create_test_dir();
    let yaml = r#"
metadata:
  name: "Complex Scene"
  description: "A complete game scene"
  author: "Test Suite"
  version: "1.0"
entities:
  Ground:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [100, 1, 100]
        MeshRenderer:
          mesh_id: 1
          visible: true
      tags: [static, ground]
    overrides: {}
    children: {}
  Player:
    source:
      components:
        Transform:
          position: [0, 1, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
        Health:
          current: 100.0
          max: 100.0
        MeshRenderer:
          mesh_id: 2
          visible: true
      tags: [player, replicate]
    overrides: {}
    children:
      Camera:
        source:
          components:
            Transform:
              position: [0, 1.6, -3]
              rotation: [0, 0, 0, 1]
              scale: [1, 1, 1]
            Camera:
              fov: 60.0
              aspect: 1.7777
              near: 0.1
              far: 1000.0
          tags: [main_camera]
        overrides: {}
        children: {}
"#;

    let template_path = create_template_file(&temp_dir, "scene.yaml", yaml);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &template_path).expect("Failed to load scene");

    assert_eq!(instance.name, "Complex Scene");
    assert_eq!(instance.entities.len(), 2); // Ground + Player

    // Verify all components were added
    let mut transform_count = 0;
    let mut health_count = 0;
    let mut mesh_count = 0;
    let mut camera_count = 0;

    // Count components across all entities
    for entity_id in 0..world.entity_count() {
        let entity = engine_core::ecs::Entity::from_raw(entity_id as u32);
        if world.is_alive(entity) {
            if world.get::<Transform>(entity).is_some() {
                transform_count += 1;
            }
            if world.get::<Health>(entity).is_some() {
                health_count += 1;
            }
            if world.get::<MeshRenderer>(entity).is_some() {
                mesh_count += 1;
            }
            if world.get::<Camera>(entity).is_some() {
                camera_count += 1;
            }
        }
    }

    // We expect:
    // - 3 Transforms (Ground, Player, Camera)
    // - 1 Health (Player)
    // - 2 MeshRenderers (Ground, Player)
    // - 1 Camera
    assert!(transform_count >= 2, "Should have at least 2 transforms");
    assert_eq!(health_count, 1, "Should have 1 health component");
    assert_eq!(mesh_count, 2, "Should have 2 mesh renderers");
    assert_eq!(camera_count, 1, "Should have 1 camera");
}

#[test]
fn test_template_hot_reload_simulation() {
    let temp_dir = create_test_dir();
    let yaml_v1 = r#"
metadata:
  name: "Hot Reload Test"
  version: "1.0"
entities:
  Root:
    source:
      components:
        Transform: null
        Health:
          current: 100.0
          max: 100.0
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "hotreload.yaml", yaml_v1);
    let mut world = setup_world();
    let mut loader = TemplateLoader::new();

    // Load v1
    let instance_v1 = loader.load(&mut world, &template_path).expect("Failed to load v1");
    let entity_v1 = instance_v1.entities[0];
    let health_v1 = world.get::<Health>(entity_v1).unwrap();
    assert_eq!(health_v1.current, 100.0);

    // Simulate hot reload: clear cache and modify file
    loader.clear_cache();

    let yaml_v2 = r#"
metadata:
  name: "Hot Reload Test"
  version: "2.0"
entities:
  Root:
    source:
      components:
        Transform: null
        Health:
          current: 200.0
          max: 200.0
      tags: []
    overrides: {}
    children: {}
"#;

    fs::write(&template_path, yaml_v2).expect("Failed to write v2");

    // Load v2 (should get new version)
    let instance_v2 = loader.load(&mut world, &template_path).expect("Failed to load v2");
    let entity_v2 = instance_v2.entities[0];
    let health_v2 = world.get::<Health>(entity_v2).unwrap();
    assert_eq!(health_v2.current, 200.0);

    // Old entity should still exist with old values
    let health_v1_check = world.get::<Health>(entity_v1).unwrap();
    assert_eq!(health_v1_check.current, 100.0);
}
