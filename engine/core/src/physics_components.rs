//! Physics-related components
//!
//! NOTE: This module provides basic physics components for engine-core.
//! Advanced physics functionality is available in `engine-physics`.

use crate::math::Vec3;
use engine_macros::Component;
use serde::{Deserialize, Serialize};

/// Velocity component (linear velocity in units/second)
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    /// X velocity
    pub x: f32,
    /// Y velocity
    pub y: f32,
    /// Z velocity
    pub z: f32,
}

impl Velocity {
    /// Create a new velocity
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Create a zero velocity
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    /// Convert to Vec3
    pub fn to_vec3(&self) -> Vec3 {
        Vec3 { x: self.x, y: self.y, z: self.z }
    }

    /// Create from Vec3
    pub fn from_vec3(v: Vec3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_new() {
        let vel = Velocity::new(1.0, 2.0, 3.0);
        assert_eq!(vel.x, 1.0);
        assert_eq!(vel.y, 2.0);
        assert_eq!(vel.z, 3.0);
    }

    #[test]
    fn test_velocity_zero() {
        let vel = Velocity::zero();
        assert_eq!(vel.x, 0.0);
        assert_eq!(vel.y, 0.0);
        assert_eq!(vel.z, 0.0);
    }
}
