//! 3D vector type for scalar operations.
//!
//! Re-exports glam::Vec3 for performance, with custom extension methods.

// Re-export glam's Vec3 directly
pub use glam::Vec3;

/// Extension trait for Vec3 with custom methods.
///
/// These methods provide additional functionality beyond glam's built-in methods,
/// or provide aliases for compatibility with our existing API.
pub trait Vec3Ext {
    /// Zero vector (0, 0, 0).
    fn zero() -> Self;

    /// Unit X vector (1, 0, 0).
    fn unit_x() -> Self;

    /// Unit Y vector (0, 1, 0).
    fn unit_y() -> Self;

    /// Unit Z vector (0, 0, 1).
    fn unit_z() -> Self;

    /// Compute the magnitude (length).
    /// Alias for glam's `length()`.
    fn magnitude(self) -> f32;

    /// Compute the squared magnitude (length²).
    /// Alias for glam's `length_squared()`.
    fn magnitude_squared(self) -> f32;

    /// Normalize to unit length, returning None if the vector has zero length.
    fn try_normalize(self) -> Option<Self>
    where
        Self: Sized;
}

impl Vec3Ext for Vec3 {
    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline]
    fn unit_x() -> Self {
        Self::X
    }

    #[inline]
    fn unit_y() -> Self {
        Self::Y
    }

    #[inline]
    fn unit_z() -> Self {
        Self::Z
    }

    #[inline]
    fn magnitude(self) -> f32 {
        self.length()
    }

    #[inline]
    fn magnitude_squared(self) -> f32 {
        self.length_squared()
    }

    #[inline]
    fn try_normalize(self) -> Option<Self> {
        let len_sq = self.length_squared();
        if len_sq > 0.0 {
            Some(self * len_sq.sqrt().recip())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_add() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let c = a + b;
        assert_eq!(c, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn test_vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a.dot(b), 32.0);
    }

    #[test]
    fn test_vec3_normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_zero() {
        let v = Vec3::zero();
        assert_eq!(v, Vec3::ZERO);
    }

    #[test]
    fn test_vec3_unit_vectors() {
        assert_eq!(Vec3::unit_x(), Vec3::X);
        assert_eq!(Vec3::unit_y(), Vec3::Y);
        assert_eq!(Vec3::unit_z(), Vec3::Z);
    }

    #[test]
    fn test_vec3_magnitude() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(v.magnitude(), 5.0);
        assert_eq!(v.magnitude_squared(), 25.0);
    }

    #[test]
    fn test_vec3_try_normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.try_normalize();
        assert!(n.is_some());
        assert!((n.unwrap().magnitude() - 1.0).abs() < 1e-6);

        let zero = Vec3::ZERO;
        assert!(zero.try_normalize().is_none());
    }
}
