//! Built-in component schema registrations for core engine components.
//!
//! New modules register their own schemas via the EditorPlugin trait (future).
//! These cover the components that ship with the engine itself.

use super::schema_registry::{ComponentSchema, ComponentSchemaRegistry, FieldSchema, FieldType};

fn f32_field(name: &str, label: &str, min: Option<f32>, max: Option<f32>) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::F32 { min, max, step: None },
    }
}

fn vec3_field(name: &str, label: &str) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::Vec3,
    }
}

fn bool_field(name: &str, label: &str) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::Bool,
    }
}

/// Registers all built-in engine component schemas.
pub fn register_builtin_schemas(registry: &mut ComponentSchemaRegistry) {
    registry.register(ComponentSchema {
        name: "Transform".into(),
        label: "Transform".into(),
        category: "Core".into(),
        fields: vec![
            vec3_field("position", "Position"),
            vec3_field("rotation", "Rotation"),
            vec3_field("scale", "Scale"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Health".into(),
        label: "Health".into(),
        category: "Core".into(),
        fields: vec![
            f32_field("current", "Current HP", Some(0.0), Some(10000.0)),
            f32_field("max", "Max HP", Some(1.0), Some(10000.0)),
        ],
    });

    registry.register(ComponentSchema {
        name: "Velocity".into(),
        label: "Velocity".into(),
        category: "Physics".into(),
        fields: vec![
            vec3_field("linear", "Linear"),
            vec3_field("angular", "Angular"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Camera".into(),
        label: "Camera".into(),
        category: "Rendering".into(),
        fields: vec![
            f32_field("fov", "Field of View", Some(1.0), Some(180.0)),
            f32_field("near", "Near Clip", Some(0.001), Some(10.0)),
            f32_field("far", "Far Clip", Some(1.0), Some(100_000.0)),
        ],
    });

    registry.register(ComponentSchema {
        name: "MeshRenderer".into(),
        label: "Mesh Renderer".into(),
        category: "Rendering".into(),
        fields: vec![
            bool_field("visible", "Visible"),
            bool_field("cast_shadows", "Cast Shadows"),
            bool_field("receive_shadows", "Receive Shadows"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Collider".into(),
        label: "Collider".into(),
        category: "Physics".into(),
        fields: vec![
            bool_field("is_trigger", "Is Trigger"),
            f32_field("friction", "Friction", Some(0.0), Some(1.0)),
            f32_field("restitution", "Restitution", Some(0.0), Some(1.0)),
        ],
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::schema_registry::ComponentSchemaRegistry;

    fn registry_with_builtins() -> ComponentSchemaRegistry {
        let mut reg = ComponentSchemaRegistry::new();
        register_builtin_schemas(&mut reg);
        reg
    }

    #[test]
    fn test_transform_has_three_vec3_fields() {
        let reg = registry_with_builtins();
        let t = reg.get("Transform").expect("Transform not registered");
        assert_eq!(t.fields.len(), 3);
        assert!(t.fields.iter().any(|f| f.name == "position"));
        assert!(t.fields.iter().any(|f| f.name == "rotation"));
        assert!(t.fields.iter().any(|f| f.name == "scale"));
    }

    #[test]
    fn test_health_current_has_range() {
        use crate::bridge::schema_registry::FieldType;
        let reg = registry_with_builtins();
        let h = reg.get("Health").expect("Health not registered");
        let current = h.fields.iter().find(|f| f.name == "current").unwrap();
        if let FieldType::F32 { min, max, .. } = &current.field_type {
            assert_eq!(*min, Some(0.0));
            assert_eq!(*max, Some(10000.0));
        } else {
            panic!("expected F32");
        }
    }

    #[test]
    fn test_all_builtins_have_nonempty_category() {
        let reg = registry_with_builtins();
        for schema in reg.all() {
            assert!(!schema.category.is_empty(), "{} missing category", schema.name);
        }
    }

    #[test]
    fn test_six_builtins_registered() {
        let reg = registry_with_builtins();
        assert_eq!(reg.all().len(), 6);
    }
}
