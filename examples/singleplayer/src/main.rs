//! Singleplayer Example Game
//!
//! This example demonstrates audio integration with the ECS in a singleplayer context.
//! It showcases:
//! - 2D audio (music, UI sounds)
//! - 3D spatial audio (footsteps at entity positions)
//! - Doppler effect (moving sound sources)
//! - Audio effects (reverb on explosion)

use engine_audio::{AudioEffect, AudioListener, AudioSystem, ReverbEffect, Sound};
use engine_core::ecs::World;
use engine_math::{Quat, Transform, Vec3};
use std::path::Path;
use tracing::{error, info};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    info!("Singleplayer Example - Audio Demo Starting");

    // Ensure audio assets exist
    if !Path::new("assets/audio/footstep.wav").exists() {
        error!("Audio assets not found. Run: cargo run --bin generate-audio-assets");
        eprintln!("\nError: Audio assets not found!");
        eprintln!("Please run: cargo run --bin generate-audio-assets");
        eprintln!("This will generate test audio files in assets/audio/\n");
        std::process::exit(1);
    }

    // Initialize game world
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Initialize audio system
    let mut audio_system = match AudioSystem::new() {
        Ok(system) => {
            info!("Audio system initialized");
            system
        }
        Err(e) => {
            error!("Failed to initialize audio system: {}", e);
            eprintln!("\nError: Failed to initialize audio system: {}", e);
            std::process::exit(1);
        }
    };

    // Load all sound assets
    if let Err(e) = load_audio_assets(&mut audio_system) {
        error!("Failed to load audio assets: {}", e);
        eprintln!("\nError: Failed to load audio assets: {}", e);
        std::process::exit(1);
    }

    info!("Loaded {} sounds", audio_system.engine().loaded_sound_count());

    // Create camera entity with audio listener
    let camera = create_camera(&mut world);
    info!("Created camera entity with AudioListener: {:?}", camera);

    // Create entities with different audio behaviors
    create_background_music(&mut world);
    create_footstep_emitter(&mut world, Vec3::new(5.0, 0.0, 0.0));
    create_moving_sound_source(&mut world);
    create_explosion_emitter(&mut world, Vec3::new(-5.0, 0.0, 5.0));

    info!("Created {} entities", world.entity_count());

    // Run game loop for demonstration (5 seconds, 60 fps)
    run_game_loop(&mut world, &mut audio_system);

    info!("Singleplayer Example - Audio Demo Complete");
}

/// Load all audio assets
fn load_audio_assets(audio_system: &mut AudioSystem) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading audio assets...");

    audio_system.load_sound("footstep", "assets/audio/footstep.wav")?;
    audio_system.load_sound("ambient", "assets/audio/ambient.wav")?;
    audio_system.load_sound("explosion", "assets/audio/explosion.wav")?;
    audio_system.load_sound("music", "assets/audio/music.wav")?;

    info!("All audio assets loaded successfully");
    Ok(())
}

/// Create camera entity with audio listener
fn create_camera(world: &mut World) -> engine_core::ecs::Entity {
    let camera = world.spawn();

    let transform = Transform::new(
        Vec3::new(0.0, 1.8, 0.0), // Head height
        Quat::IDENTITY,
        Vec3::ONE,
    );

    world.add(camera, transform);
    world.add(camera, AudioListener::new());

    camera
}

/// Create background music (2D, non-spatial)
fn create_background_music(world: &mut World) {
    let entity = world.spawn();

    world.add(entity, Transform::default());

    let sound = Sound::new("music").non_spatial().with_volume(0.3).looping().auto_play();

    world.add(entity, sound);

    info!("Created background music entity: {:?}", entity);
}

/// Create 3D footstep sound emitter
fn create_footstep_emitter(world: &mut World, position: Vec3) {
    let entity = world.spawn();

    let transform = Transform::new(position, Quat::IDENTITY, Vec3::ONE);

    world.add(entity, transform);

    let sound = Sound::new("footstep")
        .spatial_3d(50.0)
        .with_volume(0.8)
        .looping()
        .auto_play()
        .without_doppler(); // Footsteps don't need Doppler

    world.add(entity, sound);

    info!("Created footstep emitter at {:?}, entity: {:?}", position, entity);
}

/// Create moving sound source (demonstrates Doppler effect)
fn create_moving_sound_source(world: &mut World) {
    let entity = world.spawn();

    let transform = Transform::new(
        Vec3::new(-20.0, 0.0, 0.0), // Start far left
        Quat::IDENTITY,
        Vec3::ONE,
    );

    world.add(entity, transform);

    let sound = Sound::new("ambient")
        .spatial_3d(100.0)
        .with_volume(1.0)
        .looping()
        .auto_play()
        .with_doppler(1.0); // Enable Doppler effect

    world.add(entity, sound);

    info!("Created moving sound source entity: {:?}", entity);
}

/// Create explosion emitter (demonstrates audio effects)
fn create_explosion_emitter(world: &mut World, position: Vec3) {
    let entity = world.spawn();

    let transform = Transform::new(position, Quat::IDENTITY, Vec3::ONE);

    world.add(entity, transform);

    let sound = Sound::new("explosion").spatial_3d(100.0).with_volume(1.0).auto_play(); // One-shot sound

    world.add(entity, sound);

    info!("Created explosion emitter at {:?}, entity: {:?}", position, entity);
}

/// Run the game loop
fn run_game_loop(world: &mut World, audio_system: &mut AudioSystem) {
    const TARGET_FPS: u32 = 60;
    const FRAME_TIME: f32 = 1.0 / TARGET_FPS as f32;
    const TOTAL_FRAMES: u32 = TARGET_FPS * 5; // 5 seconds

    info!("Starting game loop ({}s at {} fps)", TOTAL_FRAMES / TARGET_FPS, TARGET_FPS);

    // Apply reverb effect to explosion (after first frame, when instance_id is assigned)
    let mut explosion_effect_applied = false;

    for frame in 0..TOTAL_FRAMES {
        let elapsed = frame as f32 * FRAME_TIME;

        // Update moving sound source (moves right, demonstrating Doppler)
        update_moving_entities(world, elapsed);

        // Update audio system (must be called every frame)
        audio_system.update(world, FRAME_TIME);

        // Apply reverb to explosion after it starts playing
        if !explosion_effect_applied && frame > 1 {
            if let Some(explosion_entity) = find_explosion_entity(world) {
                if let Some(sound) = world.get::<Sound>(explosion_entity) {
                    if let Some(instance_id) = sound.instance_id {
                        let reverb = ReverbEffect::large_hall();
                        if audio_system
                            .engine_mut()
                            .add_effect(instance_id, AudioEffect::Reverb(reverb))
                            .is_ok()
                        {
                            info!("Applied reverb effect to explosion");
                            explosion_effect_applied = true;
                        }
                    }
                }
            }
        }

        // Log progress periodically
        if frame % TARGET_FPS == 0 {
            let active_sounds = audio_system.engine().active_sound_count();
            info!(
                "Frame {}/{} - Active sounds: {} - Elapsed: {:.1}s",
                frame, TOTAL_FRAMES, active_sounds, elapsed
            );
        }

        // Sleep to maintain frame rate (in real game, use proper frame timing)
        std::thread::sleep(std::time::Duration::from_secs_f32(FRAME_TIME));
    }

    info!("Game loop complete");
}

/// Update positions of moving entities
fn update_moving_entities(world: &mut World, elapsed: f32) {
    // Find the moving sound source and update its position
    for (_entity, (transform, sound)) in world.query_mut::<(&mut Transform, &Sound)>() {
        // Identify moving entity by checking if it has ambient sound with Doppler
        if sound.sound_name == "ambient" && sound.doppler_enabled {
            // Move from left (-20) to right (20) over 5 seconds
            let progress = (elapsed / 5.0).clamp(0.0, 1.0);
            let x = -20.0 + (progress * 40.0); // -20 to +20

            transform.position = Vec3::new(x, 0.0, 0.0);
        }
    }
}

/// Find explosion entity
fn find_explosion_entity(world: &World) -> Option<engine_core::ecs::Entity> {
    for (entity, sound) in world.query::<&Sound>() {
        if sound.sound_name == "explosion" {
            return Some(entity);
        }
    }
    None
}
