//! Integration tests for template loader.

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::{Transform, Vec3};
use engine_core::rendering::Camera;
use engine_templating::loader::TemplateLoader;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_template_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write template file");
    path
}

#[test]
fn test_load_simple_template() {
    let temp_dir = TempDir::new().unwrap();

    let yaml = r#"
metadata:
  name: "Simple Template"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
        Health:
          current: 100.0
          max: 100.0
      tags: [player]
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "simple.yaml", yaml);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    let mut loader = TemplateLoader::new();
    let instance = loader.load(&mut world, template_path).unwrap();

    assert_eq!(instance.name, "Simple Template");
    assert_eq!(instance.entities.len(), 1);
    assert_eq!(instance.references.len(), 0);

    let entity = instance.entities[0];
    assert!(world.is_alive(entity));

    let transform = world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.position, Vec3::ZERO);
    assert_eq!(transform.scale, Vec3::ONE);

    let health = world.get::<Health>(entity).unwrap();
    assert_eq!(health.current, 100.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_load_nested_template_with_children() {
    let temp_dir = TempDir::new().unwrap();

    let yaml = r#"
metadata:
  name: "Nested Template"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
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
        children: {}
"#;

    let template_path = create_template_file(&temp_dir, "nested.yaml", yaml);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Camera>();

    let mut loader = TemplateLoader::new();
    let instance = loader.load(&mut world, template_path).unwrap();

    assert_eq!(instance.name, "Nested Template");
    assert_eq!(instance.entities.len(), 1);

    let entity = instance.entities[0];
    assert!(world.is_alive(entity));

    let transform = world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.position, Vec3::ZERO);
}

#[test]
fn test_load_template_with_references() {
    let temp_dir = TempDir::new().unwrap();

    let referenced_yaml = r#"
metadata:
  name: "Guard Tower"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
        Health:
          current: 500.0
          max: 500.0
      tags: [static]
    overrides: {}
    children: {}
"#;

    let referenced_path = create_template_file(&temp_dir, "guard_tower.yaml", referenced_yaml);

    // Convert path to forward slashes for YAML compatibility on Windows
    let referenced_path_str = referenced_path.display().to_string().replace('\\', "/");

    let main_yaml = format!(
        r#"
metadata:
  name: "Battle Arena"

entities:
  Ground:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [100, 1, 100]
      tags: [static]
    overrides: {{}}
    children: {{}}

  Tower1:
    source:
      template: "{}"
    overrides:
      Transform:
        position: [30, 0, 30]
    children: {{}}
"#,
        referenced_path_str
    );

    let main_path = create_template_file(&temp_dir, "arena.yaml", &main_yaml);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    let mut loader = TemplateLoader::new();
    let instance = loader.load(&mut world, main_path).unwrap();

    assert_eq!(instance.name, "Battle Arena");
    assert_eq!(instance.entities.len(), 2);
    assert_eq!(instance.references.len(), 1);

    for entity in &instance.entities {
        assert!(world.is_alive(*entity));
    }

    assert_eq!(instance.references[0].name, "Guard Tower");
    assert_eq!(instance.references[0].entities.len(), 1);
}

#[test]
fn test_apply_overrides() {
    let temp_dir = TempDir::new().unwrap();

    let yaml = r#"
metadata:
  name: "Override Test"

entities:
  Player:
    source:
      components:
        Health:
          current: 50.0
          max: 100.0
      tags: []
    overrides:
      Health:
        current: 75.0
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "override.yaml", yaml);

    let mut world = World::new();
    world.register::<Health>();

    let mut loader = TemplateLoader::new();
    let instance = loader.load(&mut world, template_path).unwrap();

    let entity = instance.entities[0];
    let health = world.get::<Health>(entity).unwrap();

    assert_eq!(health.current, 75.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_cache_prevents_duplicate_loads() {
    let temp_dir = TempDir::new().unwrap();

    let yaml = r#"
metadata:
  name: "Cached Template"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "cached.yaml", yaml);

    let mut world = World::new();
    world.register::<Transform>();

    let mut loader = TemplateLoader::new();

    assert_eq!(loader.cache_size(), 0);
    let instance1 = loader.load(&mut world, &template_path).unwrap();
    assert_eq!(loader.cache_size(), 1);

    let instance2 = loader.load(&mut world, &template_path).unwrap();
    assert_eq!(loader.cache_size(), 1);

    assert_eq!(instance1.entities.len(), 1);
    assert_eq!(instance2.entities.len(), 1);

    assert_ne!(instance1.entities[0], instance2.entities[0]);
}

#[test]
fn test_despawn_instance() {
    let temp_dir = TempDir::new().unwrap();

    let yaml = r#"
metadata:
  name: "Despawn Test"

entities:
  Entity1:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
      tags: []
    overrides: {}
    children: {}

  Entity2:
    source:
      components:
        Transform:
          position: [1, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
      tags: []
    overrides: {}
    children: {}
"#;

    let template_path = create_template_file(&temp_dir, "despawn.yaml", yaml);

    let mut world = World::new();
    world.register::<Transform>();

    let mut loader = TemplateLoader::new();
    let instance = loader.load(&mut world, template_path).unwrap();

    assert_eq!(instance.entities.len(), 2);

    for entity in &instance.entities {
        assert!(world.is_alive(*entity));
    }

    let entities_to_check = instance.entities.clone();
    instance.despawn(&mut world);

    for entity in &entities_to_check {
        assert!(!world.is_alive(*entity));
    }
}
