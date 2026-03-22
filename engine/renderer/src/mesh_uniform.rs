//! Per-mesh GPU data following Bevy's MeshUniform pattern.

/// Per-mesh GPU data: model matrix + pre-computed normal matrix.
///
/// Following Bevy's pattern: `world_from_local` is the model matrix,
/// `local_from_world_transpose` is its inverse-transpose (for correct normal
/// transformation under non-uniform scale).
///
/// Both matrices are 64 bytes each = 128 bytes total per entity.
/// Uploaded once per frame to a storage buffer (one entry per visible entity).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MeshUniform {
    /// Model matrix: object → world space (applied to vertex positions)
    pub world_from_local: glam::Mat4,
    /// Pre-computed inverse-transpose of model matrix (correct normal transform)
    /// Equivalent to Bevy's `local_from_world_transpose`.
    pub local_from_world_transpose: glam::Mat4,
}

impl MeshUniform {
    /// Build a MeshUniform from a Transform component.
    ///
    /// Pre-computes the normal matrix on CPU — same approach as Bevy.
    pub fn from_transform(transform: &engine_core::Transform) -> Self {
        let model = transform.matrix();
        let normal_mat = model.inverse().transpose();
        Self {
            world_from_local: model,
            local_from_world_transpose: normal_mat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::Transform;

    #[test]
    fn test_mesh_uniform_identity() {
        let t = Transform::default();
        let u = MeshUniform::from_transform(&t);
        assert!(
            (u.world_from_local - glam::Mat4::IDENTITY).abs_diff_eq(glam::Mat4::ZERO, 1e-5),
            "identity transform → identity model matrix"
        );
        assert!(
            (u.local_from_world_transpose - glam::Mat4::IDENTITY).abs_diff_eq(glam::Mat4::ZERO, 1e-5),
            "identity transform → identity normal matrix"
        );
    }

    #[test]
    fn test_mesh_uniform_uniform_scale() {
        // Use Transform::new so the internal affine matrix is built correctly
        let t = Transform::new(
            glam::Vec3::ZERO,
            glam::Quat::IDENTITY,
            glam::Vec3::splat(2.0),
        );
        let u = MeshUniform::from_transform(&t);
        // After transformation, normalizing should recover original direction
        let normal_in = glam::Vec3::Y;
        let transformed =
            (glam::Mat3::from_mat4(u.local_from_world_transpose) * normal_in).normalize();
        assert!(
            (transformed - normal_in).length() < 1e-4,
            "uniform scale: normal direction preserved after normalize, got {:?}",
            transformed
        );
    }

    #[test]
    fn test_mesh_uniform_rotation() {
        // 90° rotation around Z — Y normal becomes -X normal
        // Use Transform::new so the internal affine matrix is built correctly
        let t = Transform::new(
            glam::Vec3::ZERO,
            glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            glam::Vec3::ONE,
        );
        let u = MeshUniform::from_transform(&t);
        // For a rotation (orthogonal transform), the normal matrix = model matrix
        let normal_in = glam::Vec3::Y;
        let via_normal_mat =
            (glam::Mat3::from_mat4(u.local_from_world_transpose) * normal_in).normalize();
        let via_model =
            (glam::Mat3::from_mat4(u.world_from_local) * normal_in).normalize();
        assert!(
            (via_normal_mat - via_model).length() < 1e-3,
            "rotation: normal matrix and model matrix agree, got {:?} vs {:?}",
            via_normal_mat,
            via_model
        );
    }
}
