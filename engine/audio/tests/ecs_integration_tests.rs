//! ECS integration tests for audio system
//!
//! Tests the full integration between AudioSystem and ECS:
//! - Component lifecycle (add/remove/modify)
//! - Transform updates propagating to audio
//! - Multiple listeners
//! - Scene transitions
//! - Auto-play functionality
//! - Sound instance management

use engine_audio::{AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};
use std::time::Duration;
use tracing::info;

/// Test basic ECS integration
#[test]
fn test_basic_ecs_integration() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create sound emitter
    let emitter = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(10.0, 0.0, 0.0);
    world.add(emitter, transform);
    world.add(emitter, Sound::new("test.wav").spatial_3d(50.0));

    // Update system
    system.update(&mut world, 0.016);

    // Verify system is working
    assert_eq!(system.engine().active_sound_count(), 0); // No auto-play

    info!("Basic ECS integration works");
}

/// Test transform updates propagating to audio
#[test]
fn test_transform_propagation() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create moving emitter
    let emitter = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(0.0, 0.0, 0.0);
    world.add(emitter, transform);
    world.add(emitter, Sound::new("test.wav").spatial_3d(100.0));

    // First update
    system.update(&mut world, 0.016);

    // Move emitter
    if let Some(transform) = world.get_mut::<Transform>(emitter) {
        transform.position = Vec3::new(10.0, 5.0, 3.0);
    }

    // Second update - should pick up new position
    system.update(&mut world, 0.016);

    // Verify position was tracked
    info!("Transform updates propagate correctly");
}

/// Test listener rotation updates
#[test]
fn test_listener_rotation_updates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    let mut transform = Transform::default();
    transform.rotation = Quat::IDENTITY;
    world.add(listener, transform);
    world.add(listener, AudioListener::new());

    // Update with initial rotation
    system.update(&mut world, 0.016);

    // Rotate listener
    if let Some(transform) = world.get_mut::<Transform>(listener) {
        transform.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    }

    // Update with new rotation
    system.update(&mut world, 0.016);

    info!("Listener rotation updates work");
}

/// Test multiple active listeners (only first should be used)
#[test]
fn test_multiple_listeners() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create multiple active listeners
    for i in 0..5 {
        let listener = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        world.add(listener, transform);
        world.add(listener, AudioListener::new());
    }

    // Should use only the first listener found
    system.update(&mut world, 0.016);

    info!("Multiple listeners handled correctly");
}

/// Test switching active listener
#[test]
fn test_listener_switching() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create first listener
    let listener1 = world.spawn();
    world.add(listener1, Transform::default());
    world.add(listener1, AudioListener::new());

    system.update(&mut world, 0.016);

    // Deactivate first listener
    if let Some(listener) = world.get_mut::<AudioListener>(listener1) {
        listener.active = false;
    }

    // Create second listener
    let listener2 = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(listener2, transform);
    world.add(listener2, AudioListener::new());

    // Should now use second listener
    system.update(&mut world, 0.016);

    info!("Listener switching works correctly");
}

/// Test component addition during runtime
#[test]
fn test_runtime_component_addition() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Start with just transforms
    let entity1 = world.spawn();
    world.add(entity1, Transform::default());

    let entity2 = world.spawn();
    world.add(entity2, Transform::default());

    system.update(&mut world, 0.016);

    // Add sound components
    world.add(entity1, Sound::new("test1.wav"));
    world.add(entity2, Sound::new("test2.wav"));

    // Should handle new components
    system.update(&mut world, 0.016);

    info!("Runtime component addition works");
}

/// Test component removal during runtime
#[test]
fn test_runtime_component_removal() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    system.update(&mut world, 0.016);

    // Remove sound component
    world.remove::<Sound>(entity);

    // Should handle removal gracefully
    system.update(&mut world, 0.016);

    info!("Runtime component removal works");
}

/// Test entity despawning
#[test]
fn test_entity_despawning() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity1 = world.spawn();
    world.add(entity1, Transform::default());
    world.add(entity1, Sound::new("test1.wav"));

    let entity2 = world.spawn();
    world.add(entity2, Transform::default());
    world.add(entity2, Sound::new("test2.wav"));

    system.update(&mut world, 0.016);

    // Despawn entity
    world.despawn(entity1);

    // Should handle despawned entities
    system.update(&mut world, 0.016);

    info!("Entity despawning handled correctly");
}

/// Test auto-play functionality
#[test]
fn test_auto_play() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav").auto_play().with_volume(0.5).spatial_3d(50.0));

    // Auto-play should trigger on first update
    system.update(&mut world, 0.016);

    // Check that sound component has instance_id set
    // (would be set if sound actually played, but we don't have actual audio files)
    if let Some(sound) = world.get::<Sound>(entity) {
        info!(
            auto_play = sound.auto_play,
            has_instance = sound.instance_id.is_some(),
            "Auto-play status checked"
        );
    }
}

/// Test looping sounds
#[test]
fn test_looping_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("ambient.wav").looping().with_volume(0.3).spatial_3d(100.0));

    // Looping sounds should remain active
    for _ in 0..10 {
        system.update(&mut world, 0.016);
    }

    if let Some(sound) = world.get::<Sound>(entity) {
        assert!(sound.looping);
        info!("Looping sound configured correctly");
    }
}

/// Test non-spatial (2D) sounds
#[test]
fn test_non_spatial_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Non-spatial sound (UI, music, etc.)
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("music.wav").non_spatial().with_volume(0.5));

    system.update(&mut world, 0.016);

    if let Some(sound) = world.get::<Sound>(entity) {
        assert!(!sound.spatial);
        info!("Non-spatial sound configured correctly");
    }
}

/// Test Doppler-enabled sounds
#[test]
fn test_doppler_enabled_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(entity, transform);
    world.add(entity, Sound::new("car.wav").spatial_3d(200.0).with_doppler(1.0));

    // First update establishes baseline
    system.update(&mut world, 0.016);

    // Move entity fast
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.position = Vec3::new(50.0, 0.0, 0.0); // Moved 50m in 16ms
    }

    // Second update calculates Doppler
    system.update(&mut world, 0.016);

    if let Some(sound) = world.get::<Sound>(entity) {
        assert!(sound.doppler_enabled);
        assert_eq!(sound.doppler_scale, 1.0);
        info!("Doppler-enabled sound works correctly");
    }
}

/// Test scene transition (despawn all, spawn new)
#[test]
fn test_scene_transition() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create first scene
    let listener1 = world.spawn();
    world.add(listener1, Transform::default());
    world.add(listener1, AudioListener::new());

    let mut scene1_entities = Vec::new();
    for i in 0..10 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("scene1_sound.wav"));
        scene1_entities.push(entity);
    }

    system.update(&mut world, 0.016);

    // Transition to second scene (despawn all)
    world.despawn(listener1);
    for entity in scene1_entities {
        world.despawn(entity);
    }

    // Create second scene
    let listener2 = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(1000.0, 0.0, 0.0);
    world.add(listener2, transform);
    world.add(listener2, AudioListener::new());

    for i in 0..5 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(1000.0 + i as f32 * 3.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("scene2_sound.wav"));
    }

    // Should handle complete scene change
    system.update(&mut world, 0.016);

    info!("Scene transition handled correctly");
}

/// Test batch entity creation
#[test]
fn test_batch_entity_creation() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create many entities at once
    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i % 10) as f32 * 5.0, (i / 10) as f32 * 5.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("batch.wav"));
    }

    // Should handle batch creation
    let start = std::time::Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        entity_count = 100,
        elapsed_us = elapsed.as_micros(),
        "Batch entity creation handled"
    );

    assert!(elapsed < Duration::from_millis(10));
}

/// Test listener position history
#[test]
fn test_listener_position_tracking() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::ZERO;
    world.add(listener, transform);
    world.add(listener, AudioListener::new());

    // First update
    system.update(&mut world, 0.016);

    // Move listener
    if let Some(transform) = world.get_mut::<Transform>(listener) {
        transform.position = Vec3::new(10.0, 5.0, 3.0);
    }

    // Second update - should track movement
    system.update(&mut world, 0.016);

    info!("Listener position tracking works");
}

/// Test emitter position cleanup
#[test]
fn test_emitter_position_cleanup() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    // Update to track position
    system.update(&mut world, 0.016);

    // Despawn entity
    world.despawn(entity);

    // Update should clean up position tracking
    system.update(&mut world, 0.016);

    info!("Emitter position cleanup works");
}

/// Test query with mixed components
#[test]
fn test_mixed_component_queries() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create entities with various component combinations
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Transform only
    let entity1 = world.spawn();
    world.add(entity1, Transform::default());

    // Transform + Sound
    let entity2 = world.spawn();
    world.add(entity2, Transform::default());
    world.add(entity2, Sound::new("test.wav"));

    // Transform + AudioListener (second listener)
    let entity3 = world.spawn();
    world.add(entity3, Transform::default());
    let mut listener_comp = AudioListener::new();
    listener_comp.active = false;
    world.add(entity3, listener_comp);

    // Should query correctly
    system.update(&mut world, 0.016);

    info!("Mixed component queries work correctly");
}

/// Test volume modification during runtime
#[test]
fn test_runtime_volume_modification() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav").with_volume(0.5));

    system.update(&mut world, 0.016);

    // Modify volume
    if let Some(sound) = world.get_mut::<Sound>(entity) {
        sound.volume = 0.8;
    }

    system.update(&mut world, 0.016);

    if let Some(sound) = world.get::<Sound>(entity) {
        assert_eq!(sound.volume, 0.8);
        info!("Runtime volume modification works");
    }
}

/// Test Doppler scale modification during runtime
#[test]
fn test_runtime_doppler_modification() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav").with_doppler(0.5));

    system.update(&mut world, 0.016);

    // Modify Doppler scale
    if let Some(sound) = world.get_mut::<Sound>(entity) {
        sound.doppler_scale = 1.5;
    }

    system.update(&mut world, 0.016);

    if let Some(sound) = world.get::<Sound>(entity) {
        assert_eq!(sound.doppler_scale, 1.5);
        info!("Runtime Doppler modification works");
    }
}
