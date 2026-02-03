//! End-to-End Audio Test
//!
//! Validates the complete audio system integration:
//! - Sound loading and playback
//! - 2D and 3D spatial audio
//! - Doppler effect
//! - Audio effects (reverb)
//! - Performance targets

use engine_audio::{AudioEffect, AudioListener, AudioSystem, ReverbEffect, Sound};
use engine_core::ecs::World;
use engine_math::{Quat, Transform, Vec3};
use std::time::Instant;
use tracing::info;

/// Performance thresholds
const MAX_FRAME_TIME_MS: u128 = 17; // 60 fps = 16.67ms
const MAX_AUDIO_UPDATE_MS: u128 = 5; // Audio update should be < 5ms
const TARGET_FPS: u32 = 60;
const TEST_FRAMES: u32 = 180; // 3 seconds

#[test]
fn test_audio_e2e_integration() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .try_init();

    info!("Starting audio E2E integration test");

    // Check if audio assets exist (skip test if not available)
    if !std::path::Path::new("assets/audio/footstep.wav").exists() {
        eprintln!("Skipping test: Audio assets not found");
        eprintln!("Run: cargo run --bin generate-audio-assets");
        return;
    }

    // Initialize ECS world
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Initialize audio system
    let mut audio_system = AudioSystem::new().expect("Failed to create audio system");

    // Load test sounds
    load_test_sounds(&mut audio_system);

    // Create test entities
    let camera = create_test_camera(&mut world);
    let music_entity = create_background_music(&mut world);
    let footstep_entity = create_spatial_sound(&mut world, Vec3::new(5.0, 0.0, 0.0));
    let moving_entity = create_moving_sound(&mut world);
    let explosion_entity = create_explosion(&mut world);

    info!("Created {} test entities", world.entity_count());

    // Run simulation
    let mut frame_times = Vec::new();
    let mut audio_update_times = Vec::new();
    let frame_time = 1.0 / TARGET_FPS as f32;

    let mut explosion_effect_applied = false;

    for frame in 0..TEST_FRAMES {
        let frame_start = Instant::now();

        // Update entity positions
        update_test_entities(&mut world, frame as f32 * frame_time);

        // Update audio system
        let audio_start = Instant::now();
        audio_system.update(&mut world, frame_time);
        let audio_elapsed = audio_start.elapsed().as_millis();
        audio_update_times.push(audio_elapsed);

        // Apply reverb to explosion (after it starts playing)
        if !explosion_effect_applied && frame > 1 {
            if let Some(sound) = world.get::<Sound>(explosion_entity) {
                if let Some(instance_id) = sound.instance_id {
                    let reverb = ReverbEffect::large_hall();
                    audio_system
                        .engine_mut()
                        .add_effect(instance_id, AudioEffect::Reverb(reverb))
                        .expect("Failed to add reverb effect");
                    info!("Applied reverb effect to explosion");
                    explosion_effect_applied = true;
                }
            }
        }

        let frame_elapsed = frame_start.elapsed().as_millis();
        frame_times.push(frame_elapsed);

        // Log progress periodically
        if frame % TARGET_FPS == 0 {
            let active_sounds = audio_system.engine().active_sound_count();
            info!(
                "Frame {}/{} - Active sounds: {} - Frame time: {}ms - Audio update: {}ms",
                frame, TEST_FRAMES, active_sounds, frame_elapsed, audio_elapsed
            );
        }
    }

    // Verify entity states
    verify_entities(&world, camera, music_entity, footstep_entity, moving_entity, explosion_entity);

    // Verify audio state
    verify_audio_state(&audio_system);

    // Verify performance
    verify_performance(&frame_times, &audio_update_times);

    info!("Audio E2E integration test PASSED");
}

fn load_test_sounds(audio_system: &mut AudioSystem) {
    audio_system
        .load_sound("footstep", "assets/audio/footstep.wav")
        .expect("Failed to load footstep");

    audio_system
        .load_sound("ambient", "assets/audio/ambient.wav")
        .expect("Failed to load ambient");

    audio_system
        .load_sound("explosion", "assets/audio/explosion.wav")
        .expect("Failed to load explosion");

    audio_system
        .load_sound("music", "assets/audio/music.wav")
        .expect("Failed to load music");

    assert_eq!(audio_system.engine().loaded_sound_count(), 4);
    info!("Loaded 4 test sounds");
}

fn create_test_camera(world: &mut World) -> engine_core::ecs::Entity {
    let camera = world.spawn();

    world.add(camera, Transform::new(Vec3::new(0.0, 1.8, 0.0), Quat::IDENTITY, Vec3::ONE));

    world.add(camera, AudioListener::new());

    info!("Created camera with AudioListener");
    camera
}

fn create_background_music(world: &mut World) -> engine_core::ecs::Entity {
    let entity = world.spawn();

    world.add(entity, Transform::default());

    let sound = Sound::new("music").non_spatial().with_volume(0.3).looping().auto_play();

    world.add(entity, sound);

    info!("Created background music entity");
    entity
}

fn create_spatial_sound(world: &mut World, position: Vec3) -> engine_core::ecs::Entity {
    let entity = world.spawn();

    world.add(entity, Transform::new(position, Quat::IDENTITY, Vec3::ONE));

    let sound = Sound::new("footstep")
        .spatial_3d(50.0)
        .with_volume(0.8)
        .looping()
        .auto_play()
        .without_doppler();

    world.add(entity, sound);

    info!("Created spatial sound at {:?}", position);
    entity
}

fn create_moving_sound(world: &mut World) -> engine_core::ecs::Entity {
    let entity = world.spawn();

    world.add(entity, Transform::new(Vec3::new(-20.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE));

    let sound = Sound::new("ambient")
        .spatial_3d(100.0)
        .with_volume(1.0)
        .looping()
        .auto_play()
        .with_doppler(1.0);

    world.add(entity, sound);

    info!("Created moving sound entity");
    entity
}

fn create_explosion(world: &mut World) -> engine_core::ecs::Entity {
    let entity = world.spawn();

    world.add(entity, Transform::new(Vec3::new(-5.0, 0.0, 5.0), Quat::IDENTITY, Vec3::ONE));

    let sound = Sound::new("explosion").spatial_3d(100.0).with_volume(1.0).auto_play();

    world.add(entity, sound);

    info!("Created explosion entity");
    entity
}

fn update_test_entities(world: &mut World, elapsed: f32) {
    // Move the ambient sound source (demonstrates Doppler)
    for (_entity, (transform, sound)) in world.query_mut::<(&mut Transform, &Sound)>() {
        if sound.sound_name == "ambient" && sound.doppler_enabled {
            let progress = (elapsed / 3.0).clamp(0.0, 1.0); // 3 seconds
            let x = -20.0 + (progress * 40.0); // -20 to +20
            transform.position = Vec3::new(x, 0.0, 0.0);
        }
    }
}

fn verify_entities(
    world: &World,
    camera: engine_core::ecs::Entity,
    music_entity: engine_core::ecs::Entity,
    footstep_entity: engine_core::ecs::Entity,
    moving_entity: engine_core::ecs::Entity,
    explosion_entity: engine_core::ecs::Entity,
) {
    // Verify camera has listener
    assert!(world.get::<AudioListener>(camera).is_some(), "Camera should have AudioListener");

    // Verify music is non-spatial and looping
    let music = world.get::<Sound>(music_entity).expect("Music sound missing");
    assert!(!music.spatial, "Music should be non-spatial");
    assert!(music.looping, "Music should be looping");
    assert_eq!(music.sound_name, "music");

    // Verify footstep is spatial
    let footstep = world.get::<Sound>(footstep_entity).expect("Footstep sound missing");
    assert!(footstep.spatial, "Footstep should be spatial");
    assert!(!footstep.doppler_enabled, "Footstep should not have Doppler");

    // Verify moving sound has Doppler
    let moving = world.get::<Sound>(moving_entity).expect("Moving sound missing");
    assert!(moving.doppler_enabled, "Moving sound should have Doppler");

    // Verify moving sound actually moved
    let moving_transform = world.get::<Transform>(moving_entity).expect("Moving transform missing");
    assert!(
        moving_transform.position.x > -20.0,
        "Moving sound should have moved from start position"
    );

    // Verify explosion sound exists
    let explosion = world.get::<Sound>(explosion_entity).expect("Explosion sound missing");
    assert_eq!(explosion.sound_name, "explosion");

    info!("Entity verification PASSED");
}

fn verify_audio_state(audio_system: &AudioSystem) {
    // Verify sounds were loaded
    assert_eq!(audio_system.engine().loaded_sound_count(), 4, "Should have 4 loaded sounds");

    // Active sound count will vary (explosion finishes)
    let active_count = audio_system.engine().active_sound_count();
    assert!(
        active_count >= 3 && active_count <= 4,
        "Should have 3-4 active sounds (explosion may have finished)"
    );

    info!("Audio state verification PASSED - {} active sounds", active_count);
}

fn verify_performance(frame_times: &[u128], audio_update_times: &[u128]) {
    // Calculate statistics
    let avg_frame_time = frame_times.iter().sum::<u128>() / frame_times.len() as u128;
    let max_frame_time = *frame_times.iter().max().unwrap();
    let avg_audio_time = audio_update_times.iter().sum::<u128>() / audio_update_times.len() as u128;
    let max_audio_time = *audio_update_times.iter().max().unwrap();

    info!("Performance stats:");
    info!("  Avg frame time: {}ms", avg_frame_time);
    info!("  Max frame time: {}ms", max_frame_time);
    info!("  Avg audio update: {}ms", avg_audio_time);
    info!("  Max audio update: {}ms", max_audio_time);

    // Verify performance targets
    assert!(
        avg_frame_time < MAX_FRAME_TIME_MS,
        "Average frame time {}ms exceeds target {}ms",
        avg_frame_time,
        MAX_FRAME_TIME_MS
    );

    assert!(
        max_frame_time < MAX_FRAME_TIME_MS * 2,
        "Max frame time {}ms exceeds 2x target ({}ms)",
        max_frame_time,
        MAX_FRAME_TIME_MS * 2
    );

    assert!(
        avg_audio_time < MAX_AUDIO_UPDATE_MS,
        "Average audio update {}ms exceeds target {}ms",
        avg_audio_time,
        MAX_AUDIO_UPDATE_MS
    );

    info!("Performance verification PASSED");
}

#[test]
fn test_audio_system_creation() {
    let audio_system = AudioSystem::new();
    assert!(audio_system.is_ok(), "AudioSystem creation should succeed");

    let system = audio_system.unwrap();
    assert_eq!(system.engine().active_sound_count(), 0);
    assert_eq!(system.engine().loaded_sound_count(), 0);
}

#[test]
fn test_audio_listener_component() {
    let listener = AudioListener::new();
    assert!(listener.active, "New AudioListener should be active");

    let inactive = AudioListener::default();
    assert!(!inactive.active, "Default AudioListener should be inactive");
}

#[test]
fn test_sound_component_builder() {
    let sound = Sound::new("test.wav")
        .spatial_3d(100.0)
        .with_volume(0.5)
        .looping()
        .auto_play()
        .with_doppler(0.8);

    assert_eq!(sound.sound_name, "test.wav");
    assert!(sound.spatial);
    assert_eq!(sound.max_distance, 100.0);
    assert_eq!(sound.volume, 0.5);
    assert!(sound.looping);
    assert!(sound.auto_play);
    assert!(sound.doppler_enabled);
    assert_eq!(sound.doppler_scale, 0.8);
}
