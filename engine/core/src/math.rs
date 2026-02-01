//! Math types for 3D transformations and vectors

use crate::ecs::Component;
use serde::{Deserialize, Serialize};

/// 3D vector
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Z coordinate
    pub z: f32,
}

impl Vec3 {
    /// Zero vector (0, 0, 0)
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    /// Unit vector (1, 1, 1)
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    /// Unit X vector (1, 0, 0)
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    /// Unit Y vector (0, 1, 0)
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    /// Unit Z vector (0, 0, 1)
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    /// Create a new vector
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Quaternion for rotations
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quat {
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
    /// Z component
    pub z: f32,
    /// W component
    pub w: f32,
}

impl Quat {
    /// Identity rotation (no rotation)
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Create a new quaternion
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

/// Transform component - position, rotation, scale
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Position in 3D space
    pub position: Vec3,
    /// Rotation as quaternion
    pub rotation: Quat,
    /// Scale factors
    pub scale: Vec3,
}

impl Component for Transform {}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

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
