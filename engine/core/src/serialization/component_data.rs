//! Type-erased component data for serialization

use crate::gameplay::Health;
use crate::math::Transform;
use crate::physics_components::Velocity;
use crate::rendering::{Camera, MeshRenderer};
use serde::{Deserialize, Serialize};
use std::any::TypeId;

/// Type-erased component data for serialization
///
/// This enum wraps all component types to enable serialization without
/// runtime type information. New components must be added here manually
/// (will be automated with proc macros in Phase 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentData {
    /// Transform component
    Transform(Transform),
    /// Health component
    Health(Health),
    /// Velocity component
    Velocity(Velocity),
    /// Mesh renderer component
    MeshRenderer(MeshRenderer),
    /// Camera component
    Camera(Camera),
}

impl ComponentData {
    /// Get the TypeId of this component
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Transform(_) => TypeId::of::<Transform>(),
            Self::Health(_) => TypeId::of::<Health>(),
            Self::Velocity(_) => TypeId::of::<Velocity>(),
            Self::MeshRenderer(_) => TypeId::of::<MeshRenderer>(),
            Self::Camera(_) => TypeId::of::<Camera>(),
        }
    }

    /// Get the type name of this component
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Transform(_) => "Transform",
            Self::Health(_) => "Health",
            Self::Velocity(_) => "Velocity",
            Self::MeshRenderer(_) => "MeshRenderer",
            Self::Camera(_) => "Camera",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_data_type_id() {
        let transform = ComponentData::Transform(Transform::default());
        assert_eq!(transform.type_id(), TypeId::of::<Transform>());
        assert_eq!(transform.type_name(), "Transform");
    }

    #[test]
    fn test_component_data_serialization() {
        let health = ComponentData::Health(Health::new(75.0, 100.0));

        // Test bincode serialization
        let bytes = bincode::serialize(&health).unwrap();
        let deserialized: ComponentData = bincode::deserialize(&bytes).unwrap();

        assert_eq!(health, deserialized);
    }

    #[test]
    fn test_camera_serialization() {
        use std::f32::consts::PI;

        let camera = ComponentData::Camera(Camera::new(PI / 4.0, 16.0 / 9.0));

        // Test bincode serialization (Camera has #[serde(skip)] fields that should be handled)
        let bytes = bincode::serialize(&camera).unwrap();
        let deserialized: ComponentData = bincode::deserialize(&bytes).unwrap();

        // Camera should roundtrip correctly (skipped fields are reconstructed)
        if let ComponentData::Camera(c) = deserialized {
            if let ComponentData::Camera(orig) = camera {
                assert!((c.fov - orig.fov).abs() < 0.0001);
                assert!((c.aspect - orig.aspect).abs() < 0.0001);
                assert!((c.near - orig.near).abs() < 0.0001);
                assert!((c.far - orig.far).abs() < 0.0001);
            }
        } else {
            panic!("Expected Camera component");
        }
    }

    #[test]
    fn test_all_component_types() {
        // Ensure all 5 component types are in the enum
        let components = vec![
            ComponentData::Transform(Transform::default()),
            ComponentData::Health(Health::new(100.0, 100.0)),
            ComponentData::Velocity(Velocity::default()),
            ComponentData::MeshRenderer(MeshRenderer::new(123)),
            ComponentData::Camera(Camera::new(1.57, 1.78)),
        ];

        // All should serialize/deserialize correctly
        for comp in components {
            let bytes = bincode::serialize(&comp).unwrap();
            let _deserialized: ComponentData = bincode::deserialize(&bytes).unwrap();
            // Just verify no panics
        }
    }
}
