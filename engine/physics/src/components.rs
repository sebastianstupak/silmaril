//! Physics components

use engine_core::ecs::Component;
use engine_math::Vec3;
use serde::{Deserialize, Serialize};

/// Velocity component - movement in 3D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    /// Linear velocity vector
    pub linear: Vec3,
}

impl Component for Velocity {}

impl Velocity {
    /// Zero velocity
    pub const ZERO: Self = Self { linear: Vec3::ZERO };

    /// Create a new velocity
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { linear: Vec3::new(x, y, z) }
    }

    /// Get X component
    pub fn x(&self) -> f32 {
        self.linear.x
    }

    /// Get Y component
    pub fn y(&self) -> f32 {
        self.linear.y
    }

    /// Get Z component
    pub fn z(&self) -> f32 {
        self.linear.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_zero() {
        let v = Velocity::ZERO;
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
    }

    #[test]
    fn test_velocity_new() {
        let v = Velocity::new(1.0, 2.0, 3.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
    }
}
