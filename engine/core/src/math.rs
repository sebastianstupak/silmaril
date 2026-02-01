//! Math types for 3D transformations and vectors
//!
//! This module re-exports types from `engine-math` and adds ECS Component integration.

use crate::ecs::Component;

// Re-export core math types from engine-math
pub use engine_math::{Quat, Transform as MathTransform, Vec3};

/// Transform component - position, rotation, scale
///
/// This is a type alias to engine_math::Transform with Component trait implemented.
pub type Transform = MathTransform;

// Implement Component for Transform
impl Component for Transform {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_constants() {
        assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(Vec3::ONE, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_quat_identity() {
        let q = Quat::IDENTITY;
        assert_eq!(q.w, 1.0);
        assert_eq!(q.x, 0.0);
    }

    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.position, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }
}
