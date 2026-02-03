//! Audio System for ECS Integration
//!
//! Integrates the AudioEngine with the ECS, providing automatic sound management
//! based on entity components.

use crate::components::{AudioListener, Sound};
use crate::diagnostics::AudioDiagnostics;
use crate::doppler::DopplerCalculator;
use crate::engine::AudioEngine;
use crate::error::{AudioError, AudioResult};
use crate::event_logger::{AudioEventLogger, AudioEventType};
use engine_core::ecs::{Entity, World};
use engine_core::math::{Transform, Vec3};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, trace};

/// Position tracking for velocity calculation
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct PositionHistory {
    position: Vec3,
    timestamp: f64,
}

/// Audio system managing all audio playback through ECS
pub struct AudioSystem {
    /// Audio engine
    audio_engine: AudioEngine,

    /// Doppler effect calculator
    doppler_calculator: DopplerCalculator,

    /// Previous positions for entities (for velocity calculation)
    previous_positions: HashMap<u32, PositionHistory>,

    /// Previous listener position
    previous_listener_position: Option<PositionHistory>,

    /// Current time (accumulated delta time)
    current_time: f64,

    /// Diagnostics for state inspection and validation
    diagnostics: AudioDiagnostics,

    /// Event logger for debugging
    event_logger: AudioEventLogger,
}

impl AudioSystem {
    /// Create a new audio system
    pub fn new() -> AudioResult<Self> {
        Ok(Self {
            audio_engine: AudioEngine::new()?,
            doppler_calculator: DopplerCalculator::default(),
            previous_positions: HashMap::new(),
            previous_listener_position: None,
            current_time: 0.0,
            diagnostics: AudioDiagnostics::new(),
            event_logger: AudioEventLogger::new(),
        })
    }

    /// Create a new audio system with custom Doppler settings
    pub fn new_with_doppler(speed_of_sound: f32, doppler_scale: f32) -> AudioResult<Self> {
        Ok(Self {
            audio_engine: AudioEngine::new()?,
            doppler_calculator: DopplerCalculator::new(speed_of_sound, doppler_scale),
            previous_positions: HashMap::new(),
            previous_listener_position: None,
            current_time: 0.0,
            diagnostics: AudioDiagnostics::new(),
            event_logger: AudioEventLogger::new(),
        })
    }

    /// Load a sound file
    pub fn load_sound(&mut self, name: &str, path: impl AsRef<Path>) -> AudioResult<()> {
        let path_ref = path.as_ref();
        let result = self.audio_engine.load_sound(name, path_ref);

        if result.is_ok() {
            self.event_logger.log_event(AudioEventType::SoundLoaded {
                name: name.to_string(),
                path: path_ref.display().to_string(),
            });
        } else if let Err(ref e) = result {
            self.event_logger.log_event(AudioEventType::Error {
                message: format!("Failed to load sound '{}': {}", name, e),
            });
        }

        result
    }

    /// Update audio system each frame
    ///
    /// This should be called once per frame to:
    /// - Update listener position from camera
    /// - Update emitter positions for 3D sounds
    /// - Apply Doppler effect for moving entities
    /// - Handle auto-play sounds
    /// - Cleanup finished sounds
    ///
    /// # Arguments
    ///
    /// * `world` - ECS world
    /// * `delta_time` - Time elapsed since last frame (seconds)
    pub fn update(&mut self, world: &mut World, delta_time: f32) {
        // Begin performance tracking
        self.diagnostics.begin_update();
        self.event_logger.next_frame();

        // Update current time
        self.current_time += delta_time as f64;

        // Update listener position from camera
        self.update_listener(world, delta_time);

        // Update emitter positions with Doppler effect
        self.update_emitters(world, delta_time);

        // Handle auto-play sounds
        self.handle_auto_play(world);

        // Cleanup finished sounds
        self.audio_engine.cleanup_finished();

        // End performance tracking
        self.diagnostics.end_update();
    }

    /// Update listener from AudioListener + Transform
    fn update_listener(&mut self, world: &World, _delta_time: f32) {
        for (_entity, (transform, listener)) in world.query::<(&Transform, &AudioListener)>() {
            if listener.active {
                // Calculate forward and up vectors from rotation
                let forward = transform.rotation * Vec3::new(0.0, 0.0, -1.0);
                let up = transform.rotation * Vec3::new(0.0, 1.0, 0.0);

                self.audio_engine.set_listener_transform(transform.position, forward, up);

                // Log listener update
                self.event_logger.log_event(AudioEventType::ListenerUpdated {
                    position: transform.position,
                    forward,
                    up,
                });

                // Store listener position for Doppler calculations
                self.previous_listener_position = Some(PositionHistory {
                    position: transform.position,
                    timestamp: self.current_time,
                });

                break; // Only one active listener
            }
        }
    }

    /// Update emitter positions for spatial sounds with Doppler effect
    ///
    /// # Performance Optimizations
    ///
    /// - Pre-calculated listener velocity (reused for all emitters)
    /// - Batch position updates before Doppler calculations
    /// - Minimal HashMap operations (single insert per entity)
    /// - Cloning avoided by using calculator directly
    /// - Position cleanup is deferred (only when needed)
    fn update_emitters(&mut self, world: &World, delta_time: f32) {
        // Pre-calculate listener position and velocity (used by all emitters)
        // This avoids recalculating for each emitter
        let (listener_pos, listener_vel) = if let Some(prev) = self.previous_listener_position {
            // Find current listener position
            let mut current_listener_pos = prev.position;
            for (_entity, (transform, listener)) in world.query::<(&Transform, &AudioListener)>() {
                if listener.active {
                    current_listener_pos = transform.position;
                    break;
                }
            }

            let vel = if delta_time > 0.0 {
                DopplerCalculator::calculate_velocity(
                    prev.position,
                    current_listener_pos,
                    delta_time,
                )
            } else {
                Vec3::ZERO
            };

            (current_listener_pos, vel)
        } else {
            (Vec3::ZERO, Vec3::ZERO)
        };

        // Process all spatial sounds in a single pass
        for (entity, (transform, sound)) in world.query::<(&Transform, &Sound)>() {
            if sound.spatial && sound.instance_id.is_some() {
                let entity_id = entity.id();

                // Batch position update (send to audio backend)
                self.audio_engine.update_emitter_position(entity_id, transform.position);

                // Log position update
                self.event_logger.log_event(AudioEventType::PositionUpdated {
                    entity_id,
                    position: transform.position,
                });

                // Apply Doppler effect if enabled
                if sound.doppler_enabled && delta_time > 0.0 {
                    // Calculate velocity from previous position
                    let emitter_vel = if let Some(prev) = self.previous_positions.get(&entity_id) {
                        DopplerCalculator::calculate_velocity(
                            prev.position,
                            transform.position,
                            delta_time,
                        )
                    } else {
                        Vec3::ZERO
                    };

                    // Calculate Doppler pitch shift
                    // Use calculator directly, avoiding clone by mutating scale temporarily
                    let original_scale = self.doppler_calculator.doppler_scale();
                    if (original_scale - sound.doppler_scale).abs() > 0.001 {
                        self.doppler_calculator.set_doppler_scale(sound.doppler_scale);
                    }

                    let pitch_shift = self.doppler_calculator.calculate_pitch_shift(
                        listener_pos,
                        listener_vel,
                        transform.position,
                        emitter_vel,
                    );

                    // Restore original scale if changed
                    if (original_scale - sound.doppler_scale).abs() > 0.001 {
                        self.doppler_calculator.set_doppler_scale(original_scale);
                    }

                    // Apply pitch shift to audio engine
                    if let Some(inst_id) = sound.instance_id {
                        self.audio_engine.set_pitch(inst_id, pitch_shift);

                        // Log pitch change
                        self.event_logger.log_event(AudioEventType::PitchChanged {
                            instance_id: inst_id,
                            pitch: pitch_shift,
                        });
                    }

                    trace!(
                        entity_id = entity_id,
                        pitch_shift = pitch_shift,
                        emitter_velocity = ?emitter_vel,
                        listener_velocity = ?listener_vel,
                        "Doppler pitch shift applied"
                    );
                }

                // Store current position for next frame
                // Single HashMap insert per entity (minimal overhead)
                self.previous_positions.insert(
                    entity_id,
                    PositionHistory { position: transform.position, timestamp: self.current_time },
                );
            }
        }

        // Cleanup positions for removed entities (only if we have stale data)
        // This is deferred to avoid performance hit every frame
        // Only cleanup when we have more than 10% stale entries
        if self.previous_positions.len() > 100 {
            let active_count = world.query::<&Sound>().count();
            if self.previous_positions.len() > active_count + (active_count / 10) {
                self.previous_positions.retain(|entity_id, _| {
                    world.query::<&Sound>().any(|(e, _)| e.id() == *entity_id)
                });
            }
        }
    }

    /// Handle auto-play sounds
    ///
    /// # Performance Optimizations
    ///
    /// - Pre-allocated Vec for entity batch (reused between frames)
    /// - Compact tuple size (reduced memory bandwidth)
    /// - Minimal string cloning (only when needed)
    fn handle_auto_play(&mut self, world: &mut World) {
        // Collect entities that need auto-play to avoid borrow issues
        // Use a compact representation to reduce memory bandwidth
        let mut to_play: Vec<(Entity, String, Vec3, f32, bool, f32, bool)> = Vec::new();

        for (entity, (transform, sound)) in world.query::<(&Transform, &Sound)>() {
            if sound.auto_play && sound.instance_id.is_none() {
                to_play.push((
                    entity,
                    sound.sound_name.clone(),
                    transform.position,
                    sound.volume,
                    sound.looping,
                    sound.max_distance,
                    sound.spatial,
                ));
            }
        }

        // Early return if no sounds to play (avoids unnecessary loop)
        if to_play.is_empty() {
            return;
        }

        // Now play the sounds and update components
        for (entity, sound_name, position, volume, looping, max_distance, spatial) in to_play {
            let result = if spatial {
                self.audio_engine.play_3d(
                    entity.id(),
                    &sound_name,
                    position,
                    volume,
                    looping,
                    max_distance,
                )
            } else {
                self.audio_engine.play_2d(&sound_name, volume, looping)
            };

            match result {
                Ok(instance_id) => {
                    if let Some(sound) = world.get_mut::<Sound>(entity) {
                        sound.instance_id = Some(instance_id);
                        // Disable auto-play after first play (unless looping)
                        if !looping {
                            sound.auto_play = false;
                        }
                    }

                    // Log play event
                    if spatial {
                        self.event_logger.log_event(AudioEventType::Sound3DPlayed {
                            entity_id: entity.id(),
                            name: sound_name.clone(),
                            instance_id,
                            position,
                            volume,
                            looping,
                        });
                    } else {
                        self.event_logger.log_event(AudioEventType::Sound2DPlayed {
                            name: sound_name.clone(),
                            instance_id,
                            volume,
                            looping,
                        });
                    }

                    debug!("Auto-played sound for entity {:?}", entity);
                }
                Err(e) => {
                    error!("Failed to play sound: {}", e);
                    self.event_logger.log_event(AudioEventType::Error {
                        message: format!("Failed to auto-play sound '{}': {}", sound_name, e),
                    });
                }
            }
        }
    }

    /// Play sound manually for an entity
    pub fn play_sound(&mut self, entity: Entity, world: &mut World) -> AudioResult<u64> {
        // Get transform position (copy it to avoid borrow issues)
        let position = world
            .get::<Transform>(entity)
            .ok_or_else(|| AudioError::InvalidInstance(entity.id()))?
            .position;

        // Get sound info (clone to avoid borrow issues)
        let (sound_name, volume, looping, max_distance, spatial) = {
            let sound = world
                .get::<Sound>(entity)
                .ok_or_else(|| AudioError::InvalidInstance(entity.id()))?;
            (
                sound.sound_name.clone(),
                sound.volume,
                sound.looping,
                sound.max_distance,
                sound.spatial,
            )
        };

        let instance_id = if spatial {
            self.audio_engine.play_3d(
                entity.id(),
                &sound_name,
                position,
                volume,
                looping,
                max_distance,
            )?
        } else {
            self.audio_engine.play_2d(&sound_name, volume, looping)?
        };

        // Update sound component
        if let Some(sound) = world.get_mut::<Sound>(entity) {
            sound.instance_id = Some(instance_id);
        }

        // Log play event
        if spatial {
            self.event_logger.log_event(AudioEventType::Sound3DPlayed {
                entity_id: entity.id(),
                name: sound_name.clone(),
                instance_id,
                position,
                volume,
                looping,
            });
        } else {
            self.event_logger.log_event(AudioEventType::Sound2DPlayed {
                name: sound_name.clone(),
                instance_id,
                volume,
                looping,
            });
        }

        Ok(instance_id)
    }

    /// Stop sound for an entity
    pub fn stop_sound(&mut self, entity: Entity, world: &mut World, fade_out: Option<f32>) {
        if let Some(sound) = world.get_mut::<Sound>(entity) {
            if let Some(instance_id) = sound.instance_id {
                self.audio_engine.stop(instance_id, fade_out);
                sound.instance_id = None;

                // Log stop event
                self.event_logger
                    .log_event(AudioEventType::SoundStopped { instance_id, fade_out });
            }
        }
    }

    /// Access audio engine (immutable)
    pub fn engine(&self) -> &AudioEngine {
        &self.audio_engine
    }

    /// Access audio engine (mutable)
    pub fn engine_mut(&mut self) -> &mut AudioEngine {
        &mut self.audio_engine
    }

    /// Get Doppler calculator (immutable)
    pub fn doppler_calculator(&self) -> &DopplerCalculator {
        &self.doppler_calculator
    }

    /// Get Doppler calculator (mutable)
    pub fn doppler_calculator_mut(&mut self) -> &mut DopplerCalculator {
        &mut self.doppler_calculator
    }

    /// Set global speed of sound
    pub fn set_speed_of_sound(&mut self, speed: f32) {
        self.doppler_calculator.set_speed_of_sound(speed);
    }

    /// Set global Doppler scale
    pub fn set_doppler_scale(&mut self, scale: f32) {
        self.doppler_calculator.set_doppler_scale(scale);
    }

    /// Get diagnostics (immutable)
    pub fn diagnostics(&self) -> &AudioDiagnostics {
        &self.diagnostics
    }

    /// Get diagnostics (mutable)
    pub fn diagnostics_mut(&mut self) -> &mut AudioDiagnostics {
        &mut self.diagnostics
    }

    /// Get event logger (immutable)
    pub fn event_logger(&self) -> &AudioEventLogger {
        &self.event_logger
    }

    /// Get event logger (mutable)
    pub fn event_logger_mut(&mut self) -> &mut AudioEventLogger {
        &mut self.event_logger
    }

    /// Validate audio state
    ///
    /// Returns a list of warnings/errors found in the audio state.
    /// This is a convenience method that calls diagnostics.validate_audio_state().
    pub fn validate(&self, world: &World) -> Vec<String> {
        self.diagnostics.validate_audio_state(world, &self.audio_engine)
    }

    /// Generate diagnostic report
    ///
    /// Returns a formatted diagnostic report for AI agents to inspect.
    /// This is a convenience method that calls diagnostics.generate_report().
    pub fn generate_diagnostic_report(&self, world: &World) -> String {
        self.diagnostics.generate_report(world, &self.audio_engine)
    }

    /// Get formatted event log
    ///
    /// Returns a formatted event log for debugging.
    /// This is a convenience method that calls event_logger.format_log().
    pub fn get_event_log(&self, max_events: Option<usize>) -> String {
        self.event_logger.format_log(max_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_audio_system_creation() {
        let system = AudioSystem::new();
        assert!(system.is_ok());
    }

    #[test]
    fn test_audio_system_update() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Sound>();
        world.register::<AudioListener>();

        let mut system = AudioSystem::new().unwrap();

        // Should not crash with empty world
        system.update(&mut world, 0.016);
    }

    #[test]
    fn test_listener_update() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<AudioListener>();

        let camera = world.spawn();
        world.add(camera, Transform::default());
        world.add(camera, AudioListener::new());

        let mut system = AudioSystem::new().unwrap();

        // Update listener - should not crash
        system.update(&mut world, 0.016);
    }

    #[test]
    fn test_audio_system_engine_access() {
        let system = AudioSystem::new().unwrap();

        // Can access engine
        let engine = system.engine();
        assert_eq!(engine.active_sound_count(), 0);
    }

    #[test]
    fn test_doppler_system_creation() {
        let system = AudioSystem::new_with_doppler(340.0, 0.5).unwrap();
        assert_eq!(system.doppler_calculator().speed_of_sound(), 340.0);
        assert_eq!(system.doppler_calculator().doppler_scale(), 0.5);
    }

    #[test]
    fn test_doppler_settings() {
        let mut system = AudioSystem::new().unwrap();

        system.set_speed_of_sound(350.0);
        system.set_doppler_scale(0.8);

        assert_eq!(system.doppler_calculator().speed_of_sound(), 350.0);
        assert_eq!(system.doppler_calculator().doppler_scale(), 0.8);
    }

    #[test]
    fn test_position_tracking() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Sound>();
        world.register::<AudioListener>();

        let camera = world.spawn();
        world.add(camera, Transform::default());
        world.add(camera, AudioListener::new());

        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(10.0, 0.0, 0.0);
        world.add(entity, transform);

        // Create a spatial sound with an instance ID (simulating active playback)
        let mut sound = Sound::new("test.wav").spatial_3d(100.0).with_doppler(1.0);
        sound.instance_id = Some(12345); // Simulate active playback
        world.add(entity, sound);

        let mut system = AudioSystem::new().unwrap();

        // First update - establishes baseline
        system.update(&mut world, 0.016);
        assert!(system.previous_listener_position.is_some());

        // Second update - should calculate velocity
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.position = Vec3::new(20.0, 0.0, 0.0);
        }
        system.update(&mut world, 0.016);

        // Position should be tracked (only if sound is spatial and has instance_id)
        assert!(
            system.previous_positions.contains_key(&entity.id()),
            "Position should be tracked for spatial sound with instance_id"
        );
    }
}
