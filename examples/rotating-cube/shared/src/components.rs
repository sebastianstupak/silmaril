//! Game components (data structures attached to entities)
//!
//! Components for the rotating cube demo: Transform, MeshRenderer, and RotationSpeed

use serde::{Deserialize, Serialize};

/// Transform component (position, rotation, scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // Quaternion (x, y, z, w)
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// MeshRenderer component - references a mesh asset by ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRenderer {
    pub mesh_id: u64,
    pub visible: bool,
}

impl MeshRenderer {
    pub fn new(mesh_id: u64) -> Self {
        Self { mesh_id, visible: true }
    }
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self { mesh_id: 0, visible: true }
    }
}

/// RotationSpeed component - rotation rate in radians per second
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RotationSpeed {
    pub radians_per_second: f32,
}

impl RotationSpeed {
    pub fn new(radians_per_second: f32) -> Self {
        Self { radians_per_second }
    }
}

impl Default for RotationSpeed {
    fn default() -> Self {
        Self { radians_per_second: 1.0 }
    }
}
