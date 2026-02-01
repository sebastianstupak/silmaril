//! Transform type for 3D position, rotation, and scale.

use crate::quat::QuatExt;
use crate::vec3::Vec3Ext;
use crate::{Quat, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 3D transformation (position, rotation, scale).
///
/// Commonly used in game engines for entity positioning.
/// Aligned to 16 bytes for better cache performance.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(C, align(16))]
pub struct Transform {
    /// Position in 3D space
    pub position: Vec3,
    /// Rotation as quaternion
    pub rotation: Quat,
    /// Scale factor
    pub scale: Vec3,
}

impl Transform {
    /// Create a new transform.
    #[inline]
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self { position, rotation, scale }
    }

    /// Identity transform (zero position/rotation, unit scale).
    #[inline]
    pub fn identity() -> Self {
        Self { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    /// Translate by a vector.
    #[inline]
    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
    }

    /// Transform a point (applies rotation, scale, and translation).
    ///
    /// This is the most common transform operation, heavily optimized with inline(always).
    /// Formula: position + rotation * (scale * point)
    #[inline(always)]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        // Scale first
        let scaled =
            Vec3::new(point.x * self.scale.x, point.y * self.scale.y, point.z * self.scale.z);

        // Then rotate using efficient quaternion rotation
        let rotated = self.rotation.rotate_vec3(scaled);

        // Finally translate
        self.position + rotated
    }

    /// Transform a vector (applies rotation and scale only, no translation).
    ///
    /// Use this for directions, velocities, or any vector that shouldn't be affected by position.
    #[inline(always)]
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        // Scale
        let scaled =
            Vec3::new(vector.x * self.scale.x, vector.y * self.scale.y, vector.z * self.scale.z);

        // Rotate
        self.rotation.rotate_vec3(scaled)
    }

    /// Inverse transform a point.
    ///
    /// Given a point in world space, returns the point in local space.
    /// Useful for raycasting, collision detection, etc.
    #[inline]
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        // Inverse translation
        let translated = point - self.position;

        // Inverse rotation (conjugate for unit quaternions)
        let inv_rot = self.rotation.conjugate();
        let rotated = inv_rot.rotate_vec3(translated);

        // Inverse scale
        Vec3::new(rotated.x / self.scale.x, rotated.y / self.scale.y, rotated.z / self.scale.z)
    }

    /// Inverse transform a vector (inverse rotation and scale only).
    #[inline]
    pub fn inverse_transform_vector(&self, vector: Vec3) -> Vec3 {
        // Inverse rotation
        let inv_rot = self.rotation.conjugate();
        let rotated = inv_rot.rotate_vec3(vector);

        // Inverse scale
        Vec3::new(rotated.x / self.scale.x, rotated.y / self.scale.y, rotated.z / self.scale.z)
    }

    /// Compose two transforms (multiply transforms).
    ///
    /// Combines this transform with another, efficiently computing the resulting transform.
    /// Result represents applying `self` first, then `other`.
    #[inline]
    pub fn compose(&self, other: &Transform) -> Transform {
        Transform {
            // Resulting position: other.position + other.rotation * (other.scale * self.position)
            position: other.transform_point(self.position),

            // Resulting rotation: other.rotation * self.rotation
            rotation: other.rotation * self.rotation,

            // Resulting scale: component-wise multiply
            scale: Vec3::new(
                self.scale.x * other.scale.x,
                self.scale.y * other.scale.y,
                self.scale.z * other.scale.z,
            ),
        }
    }

    /// Linear interpolation between two transforms.
    ///
    /// For rotation, uses spherical linear interpolation (slerp) for smooth rotation.
    /// For position and scale, uses standard linear interpolation.
    ///
    /// # Arguments
    /// * `other` - Target transform to interpolate towards
    /// * `t` - Interpolation factor (0.0 = self, 1.0 = other)
    #[inline]
    pub fn lerp(&self, other: &Transform, t: f32) -> Transform {
        Transform {
            position: self.position.lerp(other.position, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }

    /// Set rotation from axis and angle (in radians).
    #[inline]
    pub fn set_rotation_axis_angle(&mut self, axis: Vec3, angle: f32) {
        self.rotation = Quat::from_axis_angle(axis, angle);
    }

    /// Rotate by a quaternion.
    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    /// Scale uniformly by a factor.
    #[inline]
    pub fn scale_uniform(&mut self, factor: f32) {
        self.scale *= factor;
    }

    /// Look at a target position (rotate to face target).
    ///
    /// The forward direction will point towards the target.
    /// Up vector is assumed to be (0, 1, 0).
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.position).normalize();
        if forward.magnitude_squared() < 1e-6 {
            return; // Can't look at self
        }

        let right = forward.cross(up).normalize();
        if right.magnitude_squared() < 1e-6 {
            return; // Forward and up are parallel
        }

        let up = right.cross(forward);

        // Build rotation matrix and convert to quaternion
        // This is a simplified version; for production, use proper matrix->quat conversion
        self.rotation = quat_from_forward_up(forward, up);
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

/// Helper function to create a quaternion from forward and up vectors.
/// Used internally by look_at().
#[inline]
fn quat_from_forward_up(forward: Vec3, up: Vec3) -> Quat {
    // Build orthonormal basis
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);

    // Convert rotation matrix to quaternion
    // Matrix columns: right, up, -forward (right-handed, -Z forward convention)
    let trace = right.x + up.y - forward.z;

    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        Quat::from_xyzw(
            (up.z - (-forward.y)) / s,
            ((-forward.x) - right.z) / s,
            (right.y - up.x) / s,
            0.25 * s,
        )
    } else if right.x > up.y && right.x > -forward.z {
        let s = (1.0 + right.x - up.y - (-forward.z)).sqrt() * 2.0;
        Quat::from_xyzw(
            0.25 * s,
            (right.y + up.x) / s,
            ((-forward.x) + right.z) / s,
            (up.z - (-forward.y)) / s,
        )
    } else if up.y > -forward.z {
        let s = (1.0 + up.y - right.x - (-forward.z)).sqrt() * 2.0;
        Quat::from_xyzw(
            (right.y + up.x) / s,
            0.25 * s,
            (up.z + (-forward.y)) / s,
            ((-forward.x) - right.z) / s,
        )
    } else {
        let s = (1.0 + (-forward.z) - right.x - up.y).sqrt() * 2.0;
        Quat::from_xyzw(
            ((-forward.x) + right.z) / s,
            (up.z + (-forward.y)) / s,
            0.25 * s,
            (right.y - up.x) / s,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let t = Transform::identity();
        assert_eq!(t.position, Vec3::zero());
        assert_eq!(t.scale, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_transform_translate() {
        let mut t = Transform::identity();
        t.translate(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_transform_point() {
        let mut t = Transform::identity();
        t.position = Vec3::new(10.0, 0.0, 0.0);

        let point = Vec3::new(1.0, 0.0, 0.0);
        let result = t.transform_point(point);

        assert_eq!(result, Vec3::new(11.0, 0.0, 0.0));
    }

    #[test]
    fn test_transform_point_with_scale() {
        let mut t = Transform::identity();
        t.scale = Vec3::new(2.0, 2.0, 2.0);

        let point = Vec3::new(1.0, 1.0, 1.0);
        let result = t.transform_point(point);

        assert_eq!(result, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_transform_vector() {
        let mut t = Transform::identity();
        t.position = Vec3::new(10.0, 0.0, 0.0); // Should not affect vector
        t.scale = Vec3::new(2.0, 2.0, 2.0);

        let vector = Vec3::new(1.0, 0.0, 0.0);
        let result = t.transform_vector(vector);

        // Only scaled, not translated
        assert_eq!(result, Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn test_inverse_transform_point() {
        let mut t = Transform::identity();
        t.position = Vec3::new(10.0, 5.0, 0.0);
        t.scale = Vec3::new(2.0, 2.0, 2.0);

        let point = Vec3::new(1.0, 1.0, 1.0);
        let transformed = t.transform_point(point);
        let inverse = t.inverse_transform_point(transformed);

        // Should get back original point
        assert!((inverse.x - point.x).abs() < 1e-5);
        assert!((inverse.y - point.y).abs() < 1e-5);
        assert!((inverse.z - point.z).abs() < 1e-5);
    }

    #[test]
    fn test_compose() {
        let mut t1 = Transform::identity();
        t1.position = Vec3::new(1.0, 0.0, 0.0);

        let mut t2 = Transform::identity();
        t2.position = Vec3::new(0.0, 2.0, 0.0);

        let composed = t1.compose(&t2);

        // Composed position should be t2.position + t1.position
        assert_eq!(composed.position, Vec3::new(1.0, 2.0, 0.0));
    }

    #[test]
    fn test_lerp() {
        let t1 = Transform::identity();

        let mut t2 = Transform::identity();
        t2.position = Vec3::new(10.0, 0.0, 0.0);
        t2.scale = Vec3::new(2.0, 2.0, 2.0);

        let mid = t1.lerp(&t2, 0.5);

        assert_eq!(mid.position, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(mid.scale, Vec3::new(1.5, 1.5, 1.5));
    }

    #[test]
    fn test_transform_alignment() {
        // Ensure Transform is properly aligned to 16 bytes
        assert_eq!(std::mem::align_of::<Transform>(), 16);
    }

    #[test]
    fn test_rotation_composition() {
        use std::f32::consts::PI;

        let mut t1 = Transform::identity();
        t1.rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 2.0);

        let mut t2 = Transform::identity();
        t2.rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 2.0);

        let composed = t1.compose(&t2);

        // Two 90-degree rotations should equal 180-degree rotation
        let point = Vec3::new(1.0, 0.0, 0.0);
        let result = composed.transform_point(point);

        // After 180-degree rotation around Y, (1,0,0) becomes (-1,0,0)
        assert!((result.x - (-1.0)).abs() < 1e-5);
        assert!(result.y.abs() < 1e-5);
        assert!(result.z.abs() < 1e-5);
    }
}
