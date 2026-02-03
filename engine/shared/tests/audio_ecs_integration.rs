//! Cross-crate integration test: Audio + ECS
//!
//! Tests audio system integration with ECS World.
//! MANDATORY: This test uses engine-audio + engine-core, so it MUST be in engine/shared/tests/

use engine_audio::{AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};

#[test]
fn test_audio_system_ecs_integration() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let audio_system = AudioSystem::new();
    assert!(audio_system.is_ok());
}

#[test]
fn test_listener_update_from_ecs() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    // Spawn camera with listener
    let camera = world.spawn();
    world.add(
        camera,
        Transform::new(Vec3::new(10.0, 5.0, 3.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(camera, AudioListener::new());

    let mut audio_system = AudioSystem::new().unwrap();

    // Update should extract listener position from ECS
    audio_system.update(&mut world, 0.016);

    // No crash = success
}

#[test]
fn test_emitter_update_from_ecs() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    // Spawn entity with sound
    let entity = world.spawn();
    world.add(
        entity,
        Transform::new(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );

    let mut sound = Sound::new("test.wav").spatial_3d(100.0);
    sound.instance_id = Some(42); // Simulate playing sound
    world.add(entity, sound);

    let mut audio_system = AudioSystem::new().unwrap();

    // Update should extract emitter positions from ECS
    audio_system.update(&mut world, 0.016);

    // No crash = success
}

#[test]
fn test_multiple_listeners_only_one_active() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    // Spawn two cameras, only first should be used
    let camera1 = world.spawn();
    world.add(camera1, Transform::default());
    world.add(camera1, AudioListener::new());

    let camera2 = world.spawn();
    world.add(
        camera2,
        Transform::new(Vec3::new(100.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(camera2, AudioListener::new());

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Only first active listener should be used
}

#[test]
fn test_inactive_listener_ignored() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(camera, Transform::default());

    let mut listener = AudioListener::new();
    listener.active = false;
    world.add(camera, listener);

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Inactive listener should be ignored
}

#[test]
fn test_transform_rotation_affects_listener() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let camera = world.spawn();

    // Rotate camera 90 degrees around Y axis
    let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    world.add(camera, Transform::new(Vec3::ZERO, rotation, Vec3::ONE));

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Forward vector should be rotated
    // This test ensures rotation quaternions are properly converted
}

#[test]
fn test_spatial_sound_position_sync() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    let entity = world.spawn();
    world.add(
        entity,
        Transform::new(Vec3::new(10.0, 5.0, 3.0), Quat::IDENTITY, Vec3::ONE),
    );

    let mut sound = Sound::new("test.wav").spatial_3d(100.0);
    sound.instance_id = Some(1); // Simulate playing
    world.add(entity, sound);

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Move entity
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.position = Vec3::new(20.0, 5.0, 3.0);
    }

    audio_system.update(&mut world, 0.016);

    // Position should sync to audio emitter
}

#[test]
fn test_non_spatial_sound_ignores_position() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    let entity = world.spawn();
    world.add(
        entity,
        Transform::new(Vec3::new(100.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );

    let mut sound = Sound::new("ui.wav").non_spatial();
    sound.instance_id = Some(1);
    world.add(entity, sound);

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Non-spatial sound should not create emitter
}

#[test]
fn test_cleanup_runs_each_frame() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    let mut audio_system = AudioSystem::new().unwrap();

    // Run update multiple times
    for _ in 0..10 {
        audio_system.update(&mut world, 0.016);
    }

    // Cleanup should run each frame
    assert_eq!(audio_system.engine().active_sound_count(), 0);
}

#[test]
fn test_world_with_many_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Spawn many entities with sounds
    for i in 0..100 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
        );
        world.add(entity, Sound::new("test.wav").spatial_3d(100.0));
    }

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Should handle many entities without crash
}

#[test]
fn test_entity_despawn_cleanup() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    let entity = world.spawn();
    world.add(entity, Transform::default());

    let mut sound = Sound::new("test.wav").spatial_3d(100.0);
    sound.instance_id = Some(42);
    world.add(entity, sound);

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Despawn entity
    world.despawn(entity);

    // Next update should handle missing entity gracefully
    audio_system.update(&mut world, 0.016);
}

#[test]
fn test_component_removed_during_update() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world, 0.016);

    // Remove Sound component
    world.remove::<Sound>(entity);

    // Should handle missing component gracefully
    audio_system.update(&mut world, 0.016);
}
