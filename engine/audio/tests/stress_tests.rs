//! Stress tests for audio system
//!
//! Tests the audio system under high load conditions:
//! - 100+ simultaneous sounds
//! - 1000+ emitter position updates per frame
//! - Rapid sound creation/deletion
//! - Memory pressure scenarios
//! - Long-running stability tests

use engine_audio::{AudioEngine, AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Test playing many simultaneous sounds
#[test]
fn test_100_simultaneous_sounds() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create 100 sound emitters
    const SOUND_COUNT: usize = 100;
    let mut entities = Vec::with_capacity(SOUND_COUNT);

    for i in 0..SOUND_COUNT {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position =
            Vec3::new((i as f32 % 10.0) * 5.0, (i as f32 / 10.0).floor() * 5.0, 0.0);
        world.add(entity, transform);

        // Create non-auto-play sound to avoid actual playback
        let sound = Sound::new("test.wav")
            .with_volume(0.1) // Low volume
            .spatial_3d(100.0);
        world.add(entity, sound);

        entities.push(entity);
    }

    // Update system - should handle all entities without crashing
    let start = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        sound_count = SOUND_COUNT,
        elapsed_ms = elapsed.as_millis(),
        "Processed {} sound emitters",
        SOUND_COUNT
    );

    // Should complete within frame budget (16.67ms for 60 FPS)
    assert!(elapsed < Duration::from_millis(17), "Update took too long: {:?}", elapsed);
}

/// Test with extreme number of position updates
#[test]
fn test_1000_position_updates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create 1000 sound emitters
    const EMITTER_COUNT: usize = 1000;
    let mut entities = Vec::with_capacity(EMITTER_COUNT);

    for i in 0..EMITTER_COUNT {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(
            (i as f32 % 32.0) * 3.0,
            ((i / 32) as f32 % 32.0) * 3.0,
            (i / 1024) as f32 * 3.0,
        );
        world.add(entity, transform);

        let sound = Sound::new("test.wav").spatial_3d(50.0);
        world.add(entity, sound);

        entities.push(entity);
    }

    // First update to establish baseline
    system.update(&mut world, 0.016);

    // Move all entities
    for entity in &entities {
        if let Some(transform) = world.get_mut::<Transform>(*entity) {
            transform.position += Vec3::new(0.1, 0.1, 0.1);
        }
    }

    // Second update - measures position update performance
    let start = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        emitter_count = EMITTER_COUNT,
        elapsed_ms = elapsed.as_millis(),
        "Updated {} emitter positions",
        EMITTER_COUNT
    );

    // Should still meet performance targets (relaxed for 1000 entities)
    assert!(elapsed < Duration::from_millis(50), "Update took too long: {:?}", elapsed);
}

/// Test rapid sound creation and deletion
#[test]
fn test_rapid_creation_deletion() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    const ITERATIONS: usize = 100;
    const SOUNDS_PER_ITERATION: usize = 10;

    let start = Instant::now();

    for iteration in 0..ITERATIONS {
        // Create sounds
        let mut entities = Vec::new();
        for i in 0..SOUNDS_PER_ITERATION {
            let entity = world.spawn();
            let mut transform = Transform::default();
            transform.position = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
            world.add(entity, transform);
            world.add(entity, Sound::new("test.wav"));
            entities.push(entity);
        }

        // Update
        system.update(&mut world, 0.016);

        // Delete half the sounds
        for i in 0..SOUNDS_PER_ITERATION / 2 {
            world.despawn(entities[i]);
        }

        if iteration % 10 == 0 {
            info!(iteration = iteration, "Rapid creation/deletion test");
        }
    }

    let elapsed = start.elapsed();

    info!(
        iterations = ITERATIONS,
        sounds_per_iteration = SOUNDS_PER_ITERATION,
        total_elapsed_ms = elapsed.as_millis(),
        avg_iteration_us = elapsed.as_micros() / (ITERATIONS as u128),
        "Completed rapid creation/deletion test"
    );

    // Should complete all iterations in reasonable time (< 5 seconds)
    assert!(elapsed < Duration::from_secs(5), "Test took too long: {:?}", elapsed);
}

/// Test memory stability with long-running updates
#[test]
fn test_long_running_stability() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create moderate number of sounds
    const SOUND_COUNT: usize = 50;
    let mut entities = Vec::with_capacity(SOUND_COUNT);

    for i in 0..SOUND_COUNT {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i as f32 % 10.0) * 3.0, (i as f32 / 10.0) * 3.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").with_doppler(1.0));
        entities.push(entity);
    }

    // Run for simulated 10 seconds at 60 FPS
    const FRAME_COUNT: usize = 600;
    let start = Instant::now();

    for frame in 0..FRAME_COUNT {
        // Move entities
        for entity in &entities {
            if let Some(transform) = world.get_mut::<Transform>(*entity) {
                transform.position += Vec3::new(
                    (frame as f32 * 0.01).sin() * 0.1,
                    (frame as f32 * 0.01).cos() * 0.1,
                    0.0,
                );
            }
        }

        system.update(&mut world, 0.016);

        if frame % 60 == 0 {
            info!(
                frame = frame,
                elapsed_ms = start.elapsed().as_millis(),
                "Long-running stability test"
            );
        }
    }

    let elapsed = start.elapsed();

    info!(
        frame_count = FRAME_COUNT,
        total_elapsed_ms = elapsed.as_millis(),
        avg_frame_us = elapsed.as_micros() / (FRAME_COUNT as u128),
        "Completed long-running stability test"
    );

    // Should maintain consistent performance
    assert!(elapsed < Duration::from_secs(10), "Test took too long: {:?}", elapsed);
}

/// Test extreme sound distances
#[test]
fn test_extreme_distances() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener at origin
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create sounds at extreme distances
    let distances = vec![
        0.001,    // Very close
        1.0,      // Normal
        100.0,    // Far
        1000.0,   // Very far
        10000.0,  // Extremely far
        100000.0, // Beyond render distance
    ];

    for distance in distances.iter() {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(*distance, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").spatial_3d(*distance * 2.0).with_doppler(1.0));
    }

    // Should handle all distances without crashing
    let start = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        distance_count = distances.len(),
        elapsed_us = elapsed.as_micros(),
        "Processed sounds at extreme distances"
    );

    assert!(elapsed < Duration::from_millis(5));
}

/// Test with many listeners (only one should be active)
#[test]
fn test_multiple_listeners() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create 100 listeners (only first should be processed)
    for i in 0..100 {
        let listener = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        world.add(listener, transform);
        world.add(listener, AudioListener::new());
    }

    // Should efficiently skip all but first active listener
    let start = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        listener_count = 100,
        elapsed_us = elapsed.as_micros(),
        "Processed multiple listeners"
    );

    // Should be very fast since we break on first active listener
    assert!(elapsed < Duration::from_millis(5));
}

/// Test Doppler calculations under high velocity
#[test]
fn test_high_velocity_doppler() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create 50 fast-moving entities
    const ENTITY_COUNT: usize = 50;
    let mut entities = Vec::with_capacity(ENTITY_COUNT);

    for i in 0..ENTITY_COUNT {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i as f32 % 10.0) * 5.0, (i as f32 / 10.0) * 5.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").spatial_3d(100.0).with_doppler(1.0));
        entities.push(entity);
    }

    // First update to establish baseline
    system.update(&mut world, 0.016);

    // Move entities at high velocity (100 m/s)
    for entity in &entities {
        if let Some(transform) = world.get_mut::<Transform>(*entity) {
            transform.position += Vec3::new(1.6, 0.0, 0.0); // 100 m/s at 60 FPS
        }
    }

    // Second update - calculates Doppler shift
    let start = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed = start.elapsed();

    info!(
        entity_count = ENTITY_COUNT,
        elapsed_us = elapsed.as_micros(),
        "Calculated Doppler shifts for high-velocity entities"
    );

    // Doppler calculations should be fast
    assert!(elapsed < Duration::from_millis(10));
}

/// Test cleanup of finished sounds
#[test]
fn test_cleanup_efficiency() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Cleanup should be fast even if called many times
    let start = Instant::now();
    for _ in 0..1000 {
        engine.cleanup_finished();
    }
    let elapsed = start.elapsed();

    info!(
        cleanup_calls = 1000,
        elapsed_us = elapsed.as_micros(),
        "Cleanup efficiency test"
    );

    // Should be very fast
    assert!(elapsed < Duration::from_millis(10));
}

/// Test component query performance
#[test]
fn test_query_performance() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Create 1000 entities with mixed components
    for i in 0..1000 {
        let entity = world.spawn();

        // Every entity has transform
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);

        // Only some have sounds
        if i % 2 == 0 {
            world.add(entity, Sound::new("test.wav"));
        }

        // Only some have listeners
        if i % 100 == 0 {
            world.add(entity, AudioListener::new());
        }
    }

    // Query performance test
    let start = Instant::now();
    let mut sound_count = 0;
    let mut listener_count = 0;

    for _ in 0..100 {
        for (_entity, (_transform, _sound)) in world.query::<(&Transform, &Sound)>() {
            sound_count += 1;
        }

        for (_entity, (_transform, _listener)) in world.query::<(&Transform, &AudioListener)>() {
            listener_count += 1;
        }
    }

    let elapsed = start.elapsed();

    info!(
        iterations = 100,
        entities = 1000,
        sound_count = sound_count,
        listener_count = listener_count,
        elapsed_ms = elapsed.as_millis(),
        "Query performance test"
    );

    // Should be fast
    assert!(elapsed < Duration::from_millis(100));
}

/// Benchmark update performance with realistic scenario
#[test]
fn test_realistic_game_scenario() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener (player)
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create realistic sound distribution:
    // - 5 looping ambient sounds (wind, water, etc.)
    // - 10 music/speech sounds
    // - 20 footstep/movement sounds
    // - 15 weapon/impact sounds

    // Ambient sounds
    for i in 0..5 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 20.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(
            entity,
            Sound::new("ambient.wav")
                .spatial_3d(200.0)
                .looping()
                .with_volume(0.3)
                .without_doppler(),
        );
    }

    // Music/speech (non-spatial)
    for _ in 0..10 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Sound::new("music.wav").non_spatial().looping().with_volume(0.5));
    }

    // Footsteps (spatial, with Doppler)
    for i in 0..20 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i % 5) as f32 * 5.0, (i / 5) as f32 * 5.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("footstep.wav").spatial_3d(30.0).with_doppler(0.5));
    }

    // Weapons/impacts (spatial, high Doppler)
    for i in 0..15 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i % 4) as f32 * 10.0, (i / 4) as f32 * 10.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("gunshot.wav").spatial_3d(500.0).with_doppler(1.0));
    }

    // Run simulation for 60 frames
    let start = Instant::now();
    let mut max_frame_time = Duration::ZERO;
    let mut total_frame_time = Duration::ZERO;

    for frame in 0..60 {
        let frame_start = Instant::now();
        system.update(&mut world, 0.016);
        let frame_elapsed = frame_start.elapsed();

        max_frame_time = max_frame_time.max(frame_elapsed);
        total_frame_time += frame_elapsed;

        if frame % 15 == 0 {
            info!(
                frame = frame,
                frame_time_us = frame_elapsed.as_micros(),
                "Realistic scenario frame"
            );
        }
    }

    let total_elapsed = start.elapsed();
    let avg_frame_time = total_frame_time / 60;

    info!(
        total_frames = 60,
        total_elapsed_ms = total_elapsed.as_millis(),
        avg_frame_us = avg_frame_time.as_micros(),
        max_frame_us = max_frame_time.as_micros(),
        "Realistic game scenario completed"
    );

    // Average frame should meet 60 FPS target
    assert!(
        avg_frame_time < Duration::from_micros(16670),
        "Average frame time exceeded budget: {:?}",
        avg_frame_time
    );

    // Max frame should be within tolerance
    if max_frame_time >= Duration::from_millis(20) {
        warn!("Max frame time exceeded tolerance: {:?}", max_frame_time);
    }
}
