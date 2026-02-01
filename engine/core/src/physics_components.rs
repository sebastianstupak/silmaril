//! Physics-related components

use crate::ecs::Component;
use serde::{Deserialize, Serialize};

/// Velocity component - movement in 3D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    /// X velocity
    pub x: f32,
    /// Y velocity
    pub y: f32,
    /// Z velocity
    pub z: f32,
}

impl Component for Velocity {}

impl Velocity {
    /// Zero velocity
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    /// Create a new velocity
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_zero() {
        let v = Velocity::ZERO;
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
    }

    #[test]
    fn test_velocity_new() {
        let v = Velocity::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }
}
