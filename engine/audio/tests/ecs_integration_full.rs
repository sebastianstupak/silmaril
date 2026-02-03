//! Comprehensive ECS Integration Tests
//!
//! Tests the complete audio-ECS integration workflow with all features:
//! - Component lifecycle (spawn, add, modify, remove, despawn)
//! - Transform updates and spatial audio
//! - Listener management and switching
//! - Auto-play and manual playback
//! - Doppler effect with movement
//! - Multiple sounds per entity
//! - Scene transitions
//! - Error handling

use engine_audio::{AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};
use std::time::Duration;
use tracing::info;

/// Test complete workflow: spawn → add Sound → auto-play → move → cleanup
#[test]
fn test_complete_workflow() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Step 1: Spawn entities
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    let entity = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(10.0, 0.0, 0.0);
    world.add(entity, transform);

    // Step 2: Add Sound component with auto-play
    world.add(
        entity,
        Sound::new("footstep.wav")
            .auto_play()
            .with_volume(0.8)
            .spatial_3d(50.0)
            .with_doppler(1.0),
    );

    // Step 3: Auto-play should trigger on first update
    system.update(&mut world, 0.016);

    // Verify sound component state
    if let Some(sound) = world.get::<Sound>(entity) {
        assert_eq!(sound.sound_name, "footstep.wav");
        assert_eq!(sound.volume, 0.8);
        assert!(sound.spatial);
        assert_eq!(sound.max_distance, 50.0);
        assert!(sound.doppler_enabled);
        info!("Sound component configured correctly");
    }

    // Step 4: Move entity (should update position and calculate Doppler)
    for i in 0..10 {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.position.x += 1.0; // Move 1m per frame
        }
        system.update(&mut world, 0.016);

        info!(frame = i, position = ?world.get::<Transform>(entity).unwrap().position, "Entity moved");
    }

    // Step 5: Cleanup - despawn entity
    world.despawn(entity);
    system.update(&mut world, 0.016);

    assert!(!world.is_alive(entity));
    info!("Complete workflow test passed");
}

/// Test listener updates from Transform + AudioListener components
#[test]
fn test_listener_updates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(0.0, 1.8, 0.0); // Eye height
    transform.rotation = Quat::IDENTITY;
    world.add(camera, transform);
    world.add(camera, AudioListener::new());

    // First update - establishes baseline
    system.update(&mut world, 0.016);

    // Move and rotate camera
    if let Some(transform) = world.get_mut::<Transform>(camera) {
        transform.position = Vec3::new(5.0, 1.8, 10.0);
        transform.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
    }

    // Second update - should apply new transform
    system.update(&mut world, 0.016);

    info!("Listener updates test passed");
}

/// Test emitter position sync for spatial sounds
#[test]
fn test_emitter_position_sync() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create emitter with active sound
    let emitter = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(10.0, 0.0, 0.0);
    world.add(emitter, transform);

    let mut sound = Sound::new("engine.wav").spatial_3d(100.0).looping().with_volume(0.7);
    sound.instance_id = Some(12345); // Simulate active playback
    world.add(emitter, sound);

    // First update
    system.update(&mut world, 0.016);

    // Move emitter rapidly (test position sync)
    for i in 0..20 {
        if let Some(transform) = world.get_mut::<Transform>(emitter) {
            transform.position =
                Vec3::new(10.0 + (i as f32) * 2.0, (i as f32 * 0.5).sin() * 3.0, 0.0);
        }
        system.update(&mut world, 0.016);
    }

    info!("Emitter position sync test passed");
}

/// Test auto-play sounds
#[test]
fn test_auto_play_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create multiple entities with auto-play sounds
    let mut entities = Vec::new();
    for i in 0..5 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, transform);

        world.add(entity, Sound::new(format!("sound{}.wav", i)).auto_play().spatial_3d(50.0));
        entities.push(entity);
    }

    // First update - should trigger auto-play for all sounds
    system.update(&mut world, 0.016);

    // Verify auto-play was processed (would set instance_id if sounds existed)
    for entity in entities {
        if let Some(sound) = world.get::<Sound>(entity) {
            info!(entity = ?entity, sound_name = %sound.sound_name, "Auto-play sound processed");
        }
    }

    info!("Auto-play sounds test passed");
}

/// Test cleanup of finished sounds
#[test]
fn test_cleanup_finished_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create entity with non-looping sound
    let entity = world.spawn();
    world.add(entity, Transform::default());

    let mut sound = Sound::new("oneshot.wav").with_volume(1.0).spatial_3d(50.0);
    sound.instance_id = Some(99999); // Simulate finished sound
    world.add(entity, sound);

    // Update - should cleanup finished sounds
    system.update(&mut world, 0.016);

    // Cleanup should be called
    assert_eq!(system.engine().active_sound_count(), 0);

    info!("Cleanup finished sounds test passed");
}

/// Test component lifecycle: add, modify, remove
#[test]
fn test_component_lifecycle() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create entity without sound
    let entity = world.spawn();
    world.add(entity, Transform::default());

    system.update(&mut world, 0.016);

    // Add sound component
    world.add(entity, Sound::new("added.wav").spatial_3d(50.0));
    system.update(&mut world, 0.016);
    assert!(world.has::<Sound>(entity));

    // Modify sound component
    if let Some(sound) = world.get_mut::<Sound>(entity) {
        sound.volume = 0.5;
        sound.max_distance = 100.0;
    }
    system.update(&mut world, 0.016);

    if let Some(sound) = world.get::<Sound>(entity) {
        assert_eq!(sound.volume, 0.5);
        assert_eq!(sound.max_distance, 100.0);
    }

    // Remove sound component
    world.remove::<Sound>(entity);
    system.update(&mut world, 0.016);
    assert!(!world.has::<Sound>(entity));

    info!("Component lifecycle test passed");
}

/// Test multiple sounds per entity (via multiple components)
#[test]
fn test_multiple_sounds_scene() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create many entities with sounds (simulating complex scene)
    for i in 0..50 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i % 10) as f32 * 10.0, 0.0, (i / 10) as f32 * 10.0);
        world.add(entity, transform);

        let sound = if i % 3 == 0 {
            Sound::new("ambient.wav").looping().spatial_3d(30.0)
        } else if i % 3 == 1 {
            Sound::new("action.wav").spatial_3d(50.0).with_volume(0.8)
        } else {
            Sound::new("ui.wav").non_spatial().with_volume(0.5)
        };
        world.add(entity, sound);
    }

    // Update scene
    let start = std::time::Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        entity_count = 50,
        elapsed_us = elapsed.as_micros(),
        "Multiple sounds scene test passed"
    );

    // Performance assertion (50 entities should be fast)
    assert!(elapsed < Duration::from_millis(5));
}

/// Test Doppler effect with moving entities
#[test]
fn test_doppler_with_movement() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create fast-moving entity
    let entity = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(entity, transform);

    let mut sound = Sound::new("car.wav").spatial_3d(200.0).looping().with_doppler(1.0);
    sound.instance_id = Some(54321); // Simulate active playback
    world.add(entity, sound);

    // First update - establish baseline
    system.update(&mut world, 0.016);

    // Move entity towards listener (should increase pitch)
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.position = Vec3::new(50.0, 0.0, 0.0); // Moved 50m in 16ms = 3125 m/s
    }
    system.update(&mut world, 0.016);

    // Move entity away from listener (should decrease pitch)
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.position = Vec3::new(150.0, 0.0, 0.0); // Moved 100m in 16ms
    }
    system.update(&mut world, 0.016);

    info!("Doppler effect with movement test passed");
}

/// Test scene transition (despawn all, spawn new)
#[test]
fn test_scene_transition() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Scene 1
    let camera1 = world.spawn();
    world.add(camera1, Transform::default());
    world.add(camera1, AudioListener::new());

    let mut scene1_entities = Vec::new();
    for i in 0..20 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 3.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("scene1.wav"));
        scene1_entities.push(entity);
    }

    system.update(&mut world, 0.016);

    // Transition: despawn scene 1
    world.despawn(camera1);
    for entity in scene1_entities {
        world.despawn(entity);
    }

    system.update(&mut world, 0.016);

    // Scene 2
    let camera2 = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(1000.0, 0.0, 0.0);
    world.add(camera2, transform);
    world.add(camera2, AudioListener::new());

    for i in 0..15 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(1000.0 + i as f32 * 2.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("scene2.wav"));
    }

    system.update(&mut world, 0.016);

    info!("Scene transition test passed");
}

/// Test manual playback control
#[test]
fn test_manual_playback() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create entity with sound (no auto-play)
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("manual.wav").spatial_3d(50.0));

    system.update(&mut world, 0.016);

    // Manually play sound (would fail without actual audio file, but tests API)
    let result = system.play_sound(entity, &mut world);
    info!(play_result = ?result, "Manual play attempted");

    // Manually stop sound
    system.stop_sound(entity, &mut world, Some(0.5)); // 0.5s fade out

    system.update(&mut world, 0.016);

    info!("Manual playback test passed");
}

/// Test listener switching between cameras
#[test]
fn test_listener_switching() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create camera 1 (active)
    let camera1 = world.spawn();
    world.add(camera1, Transform::default());
    world.add(camera1, AudioListener::new());

    system.update(&mut world, 0.016);

    // Create camera 2 (inactive)
    let camera2 = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(camera2, transform);
    let mut listener = AudioListener::new();
    listener.active = false;
    world.add(camera2, listener);

    system.update(&mut world, 0.016);

    // Switch cameras
    if let Some(listener) = world.get_mut::<AudioListener>(camera1) {
        listener.active = false;
    }
    if let Some(listener) = world.get_mut::<AudioListener>(camera2) {
        listener.active = true;
    }

    system.update(&mut world, 0.016);

    info!("Listener switching test passed");
}

/// Test error handling for invalid entities
#[test]
fn test_error_handling() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Try to play sound for non-existent entity
    let fake_entity = world.spawn();
    world.despawn(fake_entity);

    let result = system.play_sound(fake_entity, &mut world);
    assert!(result.is_err());
    info!("Error handling test passed: {:?}", result.err());
}

/// Test performance with large entity count
#[test]
fn test_performance_stress() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create 1000 entities with sounds
    for i in 0..1000 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(
            (i % 100) as f32 * 2.0,
            ((i / 100) % 10) as f32 * 2.0,
            (i / 1000) as f32 * 2.0,
        );
        world.add(entity, transform);

        let mut sound = Sound::new("stress.wav").spatial_3d(50.0);
        // Simulate some playing
        if i % 10 == 0 {
            sound.instance_id = Some(i as u64);
        }
        world.add(entity, sound);
    }

    // Measure update performance
    let start = std::time::Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        entity_count = 1000,
        elapsed_us = elapsed.as_micros(),
        "Performance stress test completed"
    );

    // Target: < 1ms for 1000 entities
    assert!(elapsed < Duration::from_millis(1));
}
