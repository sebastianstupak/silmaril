//! Game systems (logic that operates on components)
//!
//! Systems for the rotating cube demo

use crate::components::*;
use tracing::debug;

/// Rotation system - rotates entities with RotationSpeed component
///
/// Applies rotation around the Y axis (up) using quaternion math
pub fn rotation_system(transform: &mut Transform, speed: &RotationSpeed, dt: f32) {
    // Convert current quaternion to usable form
    let quat = glam::Quat::from_xyzw(
        transform.rotation[0],
        transform.rotation[1],
        transform.rotation[2],
        transform.rotation[3],
    );

    // Calculate rotation delta for this frame
    let angle_delta = speed.radians_per_second * dt;

    // Create rotation quaternion around Y axis
    let rotation_delta = glam::Quat::from_rotation_y(angle_delta);

    // Apply rotation
    let new_quat = rotation_delta * quat;

    // Store back
    transform.rotation[0] = new_quat.x;
    transform.rotation[1] = new_quat.y;
    transform.rotation[2] = new_quat.z;
    transform.rotation[3] = new_quat.w;

    debug!(
        rotation = ?transform.rotation,
        speed = speed.radians_per_second,
        "Entity rotated"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_system() {
        let mut transform = Transform::default();
        let speed = RotationSpeed::new(std::f32::consts::PI); // 180 degrees per second

        // Rotate for 1 second (should rotate 180 degrees)
        rotation_system(&mut transform, &speed, 1.0);

        // After 180 degree rotation around Y, quaternion should be close to (0, 1, 0, 0)
        // (or its equivalent)
        let quat = glam::Quat::from_xyzw(
            transform.rotation[0],
            transform.rotation[1],
            transform.rotation[2],
            transform.rotation[3],
        );

        // Verify it's still normalized
        assert!((quat.length() - 1.0).abs() < 0.001);

        // Verify it represents a rotation (not identity)
        let identity = glam::Quat::IDENTITY;
        assert!((quat.dot(identity) - 1.0).abs() > 0.1);
    }

    #[test]
    fn test_rotation_accumulates() {
        let mut transform = Transform::default();
        let speed = RotationSpeed::new(1.0);

        // Rotate in small steps
        for _ in 0..10 {
            rotation_system(&mut transform, &speed, 0.1);
        }

        // Total rotation should be ~1 radian
        let quat = glam::Quat::from_xyzw(
            transform.rotation[0],
            transform.rotation[1],
            transform.rotation[2],
            transform.rotation[3],
        );

        // Should still be normalized
        assert!((quat.length() - 1.0).abs() < 0.001);
    }
}
