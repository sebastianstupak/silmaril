use engine_ops::scene::*;
use serde_json::json;

fn sample_scene() -> Scene {
    let mut scene = Scene::new("test_level");
    scene.add_entity(SceneEntity {
        id: 1,
        name: Some("Player".into()),
        components: vec![
            SceneComponent {
                type_name: "Transform".into(),
                data: json!({"x": 0.0, "y": 1.0, "z": 0.0}),
            },
            SceneComponent {
                type_name: "Health".into(),
                data: json!({"current": 100, "max": 100}),
            },
        ],
    });
    scene.add_entity(SceneEntity {
        id: 2,
        name: Some("Enemy".into()),
        components: vec![SceneComponent {
            type_name: "Transform".into(),
            data: json!({"x": 10.0, "y": 0.0, "z": 5.0}),
        }],
    });
    scene
}

#[test]
fn yaml_round_trip() {
    let scene = sample_scene();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.yaml");

    scene.save_yaml(&path).unwrap();
    let loaded = Scene::load_yaml(&path).unwrap();

    assert_eq!(scene, loaded);
}

#[test]
fn bincode_round_trip() {
    let scene = sample_scene();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.bin");

    scene.save_bincode(&path).unwrap();
    let loaded = Scene::load_bincode(&path).unwrap();

    assert_eq!(scene, loaded);
}

#[test]
fn add_remove_entity() {
    let mut scene = Scene::new("empty");
    assert_eq!(scene.entities.len(), 0);

    scene.add_entity(SceneEntity {
        id: 10,
        name: Some("Foo".into()),
        components: vec![],
    });
    assert_eq!(scene.entities.len(), 1);

    let removed = scene.remove_entity(10);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, 10);
    assert_eq!(scene.entities.len(), 0);

    // Removing non-existent entity returns None
    assert!(scene.remove_entity(999).is_none());
}

#[test]
fn empty_scene() {
    let scene = Scene::new("void");
    assert_eq!(scene.name, "void");
    assert!(scene.entities.is_empty());

    // Round-trip an empty scene through both formats
    let dir = tempfile::tempdir().unwrap();

    let yaml_path = dir.path().join("empty.yaml");
    scene.save_yaml(&yaml_path).unwrap();
    assert_eq!(Scene::load_yaml(&yaml_path).unwrap(), scene);

    let bin_path = dir.path().join("empty.bin");
    scene.save_bincode(&bin_path).unwrap();
    assert_eq!(Scene::load_bincode(&bin_path).unwrap(), scene);
}

#[test]
fn scene_with_multiple_entities_and_components() {
    let mut scene = Scene::new("complex");
    for i in 0..5 {
        scene.add_entity(SceneEntity {
            id: i,
            name: if i % 2 == 0 { Some(format!("Entity_{i}")) } else { None },
            components: vec![
                SceneComponent {
                    type_name: "Transform".into(),
                    data: json!({"x": i as f64}),
                },
                SceneComponent {
                    type_name: "Velocity".into(),
                    data: json!({"vx": 0.0, "vy": -9.8}),
                },
            ],
        });
    }
    assert_eq!(scene.entities.len(), 5);
    assert_eq!(scene.entities[2].name, Some("Entity_2".into()));
    assert_eq!(scene.entities[3].name, None);

    // Verify round-trip preserves everything
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("complex.yaml");
    scene.save_yaml(&path).unwrap();
    let loaded = Scene::load_yaml(&path).unwrap();
    assert_eq!(scene, loaded);
}
