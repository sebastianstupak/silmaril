use engine_ops::template::*;
use serde_json::json;
use tempfile::tempdir;

fn sample_template() -> TemplateState {
    let mut t = TemplateState::new("test_level");
    t.add_entity(TemplateEntity {
        id: 1,
        name: Some("Player".into()),
        components: vec![TemplateComponent {
            type_name: "Transform".into(),
            data: json!({"x": 0.0, "y": 1.0, "z": 0.0}),
        }],
    });
    t
}

#[test]
fn new_template_is_empty() {
    let t = TemplateState::new("my_level");
    assert_eq!(t.name, "my_level");
    assert!(t.entities.is_empty());
}

#[test]
fn add_and_remove_entity() {
    let mut t = sample_template();
    assert_eq!(t.entities.len(), 1);
    let removed = t.remove_entity(1);
    assert!(removed.is_some());
    assert!(t.entities.is_empty());
}

#[test]
fn yaml_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test_level.yaml");
    let original = sample_template();
    original.save_yaml(&path).unwrap();
    let loaded = TemplateState::load_yaml(&path).unwrap();
    assert_eq!(original, loaded);
}

#[test]
fn bincode_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test_level.bin");
    let original = sample_template();
    original.save_bincode(&path).unwrap();
    let loaded = TemplateState::load_bincode(&path).unwrap();
    assert_eq!(original, loaded);
}

#[test]
fn load_yaml_sets_name_to_file_stem() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("my_level.yaml");
    let original = sample_template();
    original.save_yaml(&path).unwrap();
    let loaded = TemplateState::load_yaml(&path).unwrap();
    assert_eq!(loaded.name, "my_level");
}
