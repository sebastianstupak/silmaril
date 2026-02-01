//! Quaternion type for 3D rotations.
//!
//! Re-exports glam::Quat for performance.

// Re-export glam's Quat directly
pub use glam::Quat;

/// Extension trait for Quat with custom methods.
///
/// These methods provide additional functionality beyond glam's built-in methods,
/// or provide aliases for compatibility with our existing API.
pub trait QuatExt {
    /// Identity rotation (no rotation).
    fn identity() -> Self;

    /// Rotate a vector by this quaternion.
    fn rotate_vec3(self, v: crate::Vec3) -> crate::Vec3;

    /// Multiply two quaternions (concatenate rotations).
    fn mul(self, other: Self) -> Self;
}

impl QuatExt for Quat {
    #[inline]
    fn identity() -> Self {
        Self::IDENTITY
    }

    #[inline(always)]
    fn rotate_vec3(self, v: crate::Vec3) -> crate::Vec3 {
        // glam's Quat * Vec3 performs quaternion rotation
        self * v
    }

    #[inline]
    fn mul(self, other: Self) -> Self {
        self * other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quat_identity() {
        let q = Quat::IDENTITY;
        assert_eq!(q.w, 1.0);
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
    }

    #[test]
    fn test_quat_new() {
        let q = Quat::from_xyzw(0.1, 0.2, 0.3, 0.4);
        assert_eq!(q.x, 0.1);
        assert_eq!(q.y, 0.2);
        assert_eq!(q.z, 0.3);
        assert_eq!(q.w, 0.4);
    }
}
