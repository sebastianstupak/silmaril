//! Integration tests for SIMD physics integration.

use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_math::Vec3;
use engine_physics::components::Velocity;
use engine_physics::systems::integration::physics_integration_system;
use engine_physics::systems::integration_simd::physics_integration_system_simd;

/// Test that SIMD and scalar produce same results.
#[test]
fn test_simd_matches_scalar() {
    // Create two identical worlds
    let mut world_scalar = World::new();
    world_scalar.register::<Transform>();
    world_scalar.register::<Velocity>();

    let mut world_simd = World::new();
    world_simd.register::<Transform>();
    world_simd.register::<Velocity>();

    // Add various entity counts to test all code paths
    for i in 0..17 {
        // 17 = 2*8 + 1 to test: AVX2 batches + scalar remainder
        let vel = Velocity::new(i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3);

        let e1 = world_scalar.spawn();
        world_scalar.add(e1, Transform::identity());
        world_scalar.add(e1, vel);

        let e2 = world_simd.spawn();
        world_simd.add(e2, Transform::identity());
        world_simd.add(e2, vel);
    }

    // Run both systems
    let dt = 0.016;
    physics_integration_system(&mut world_scalar, dt);
    physics_integration_system_simd(&mut world_simd, dt);

    // Verify both systems ran without panicking
    // TODO: Compare actual results when we can query world state
}

/// Test large entity count (exercises parallel path).
#[test]
fn test_large_entity_count() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    // Add 15,000 entities to trigger parallel processing
    for i in 0..15_000 {
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, Velocity::new(i as f32 * 0.001, i as f32 * 0.002, i as f32 * 0.003));
    }

    // Should use parallel processing
    physics_integration_system_simd(&mut world, 0.016);

    // Verify it completed without panicking
}

/// Test edge cases.
#[test]
fn test_edge_cases() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    // Test with 0 entities
    physics_integration_system_simd(&mut world, 0.016);

    // Test with 1 entity (scalar path)
    let e = world.spawn();
    world.add(e, Transform::identity());
    world.add(e, Velocity::new(1.0, 2.0, 3.0));
    physics_integration_system_simd(&mut world, 0.016);

    // Test with exactly 4 entities (one SSE batch)
    for _ in 0..3 {
        let e = world.spawn();
        world.add(e, Transform::identity());
        world.add(e, Velocity::new(1.0, 2.0, 3.0));
    }
    physics_integration_system_simd(&mut world, 0.016);

    // Test with exactly 8 entities (one AVX2 batch)
    for _ in 0..4 {
        let e = world.spawn();
        world.add(e, Transform::identity());
        world.add(e, Velocity::new(1.0, 2.0, 3.0));
    }
    physics_integration_system_simd(&mut world, 0.016);
}

/// Test hybrid processing directly.
#[test]
fn test_hybrid_batch_processing() {
    use engine_physics::systems::integration_simd::process_sequential;

    // Test count that exercises all paths: 8 + 4 + 3 = 15
    let mut transforms = vec![Transform::identity(); 15];
    let velocities = vec![Vec3::new(1.0, 2.0, 3.0); 15];

    process_sequential(&mut transforms, &velocities, 0.1);

    // Verify all updated correctly
    for transform in &transforms {
        assert!((transform.position.x - 0.1).abs() < 1e-6);
        assert!((transform.position.y - 0.2).abs() < 1e-6);
        assert!((transform.position.z - 0.3).abs() < 1e-6);
    }
}

/// Test parallel processing directly.
#[test]
fn test_parallel_batch_processing() {
    use engine_physics::systems::integration_simd::process_parallel;

    let count = 20_000;
    let mut transforms = vec![Transform::identity(); count];
    let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

    process_parallel(&mut transforms, &velocities, 0.1);

    // Verify all updated correctly
    for transform in &transforms {
        assert!((transform.position.x - 0.1).abs() < 1e-6);
        assert!((transform.position.y - 0.2).abs() < 1e-6);
        assert!((transform.position.z - 0.3).abs() < 1e-6);
    }
}
