//! Type-erased component data for serialization

use crate::gameplay::Health;
use crate::math::Transform;
use crate::physics_components::Velocity;
use crate::rendering::MeshRenderer;
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
    /// Velocity component
    Velocity(Velocity),
    /// Health component
    Health(Health),
    /// Mesh renderer component
    MeshRenderer(MeshRenderer),
}

impl ComponentData {
    /// Get the TypeId of this component
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Transform(_) => TypeId::of::<Transform>(),
            Self::Velocity(_) => TypeId::of::<Velocity>(),
            Self::Health(_) => TypeId::of::<Health>(),
            Self::MeshRenderer(_) => TypeId::of::<MeshRenderer>(),
        }
    }

    /// Get the type name of this component
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Transform(_) => "Transform",
            Self::Velocity(_) => "Velocity",
            Self::Health(_) => "Health",
            Self::MeshRenderer(_) => "MeshRenderer",
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
}
