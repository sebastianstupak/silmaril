//! Standard (scalar) physics integration system.
//!
//! Updates Transform positions based on Velocity.
//! Processes one entity at a time (scalar operations).

use crate::components::Velocity;
use engine_core::ecs::World;
use engine_core::math::Transform;

#[cfg(feature = "profiling")]
use silmaril_profiling::profile_scope;

/// Standard physics integration system (scalar).
///
/// Updates entity positions based on velocity:
/// `position += velocity * dt`
///
/// # Performance
/// Processes entities one at a time. For better performance with many entities,
/// use `physics_integration_system_simd` which processes 4-8 entities at once.
pub fn physics_integration_system(world: &mut World, dt: f32) {
    #[cfg(feature = "profiling")]
    profile_scope!("physics_integration_system");

    // Query for all entities with Transform and Velocity
    for (_entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>() {
        // Scalar operation: process one entity at a time
        transform.position += velocity.linear * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_integration() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Velocity>();

        // Create entity with transform and velocity
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, Velocity::new(1.0, 2.0, 3.0));

        // Run integration for 0.1 seconds
        physics_integration_system(&mut world, 0.1);

        // Check position updated
        let transform = world.get::<Transform>(entity).unwrap();
        assert!((transform.position.x - 0.1).abs() < 1e-6);
        assert!((transform.position.y - 0.2).abs() < 1e-6);
        assert!((transform.position.z - 0.3).abs() < 1e-6);
    }
}
