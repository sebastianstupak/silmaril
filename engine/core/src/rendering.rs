//! Rendering-related components

use crate::ecs::Component;
use serde::{Deserialize, Serialize};

/// Mesh renderer component - references to mesh and material assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRenderer {
    /// Mesh asset ID
    pub mesh_id: u64,
    /// Material asset ID
    pub material_id: u64,
}

impl Component for MeshRenderer {}

impl MeshRenderer {
    /// Create a new mesh renderer
    pub const fn new(mesh_id: u64, material_id: u64) -> Self {
        Self { mesh_id, material_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_renderer_new() {
        let mr = MeshRenderer::new(123, 456);
        assert_eq!(mr.mesh_id, 123);
        assert_eq!(mr.material_id, 456);
    }
}
