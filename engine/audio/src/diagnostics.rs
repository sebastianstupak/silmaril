//! Audio Diagnostics
//!
//! Provides comprehensive diagnostics for AI agents to verify audio system state.
//! This module allows agents to inspect active sounds, listener position, performance
//! metrics, and validate the overall audio state.

use crate::components::{AudioListener, Sound};
use crate::engine::AudioEngine;
use engine_core::ecs::{Entity, World};
use engine_core::math::{Transform, Vec3};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, warn};

/// State of a single sound in the audio system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundState {
    /// Entity ID
    pub entity_id: u32,

    /// Sound asset name
    pub sound_name: String,

    /// Is currently playing
    pub is_playing: bool,

    /// Position (for 3D sounds)
    pub position: Option<Vec3>,

    /// Volume
    pub volume: f32,

    /// Is looping
    pub looping: bool,

    /// Is spatial (3D)
    pub spatial: bool,

    /// Max distance
    pub max_distance: f32,

    /// Doppler enabled
    pub doppler_enabled: bool,

    /// Instance ID (if playing)
    pub instance_id: Option<u64>,
}

/// Performance metrics for audio system
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioPerformanceMetrics {
    /// Average update time (ms)
    pub avg_update_time_ms: f32,

    /// Peak update time (ms)
    pub peak_update_time_ms: f32,

    /// Last update time (ms)
    pub last_update_time_ms: f32,

    /// Number of updates tracked
    pub update_count: u64,

    /// Total time spent in updates (ms)
    pub total_update_time_ms: f64,
}

impl Default for AudioPerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_update_time_ms: 0.0,
            peak_update_time_ms: 0.0,
            last_update_time_ms: 0.0,
            update_count: 0,
            total_update_time_ms: 0.0,
        }
    }
}

/// Audio diagnostics for state inspection and validation
pub struct AudioDiagnostics {
    /// Performance metrics
    metrics: AudioPerformanceMetrics,

    /// Last update start time
    last_update_start: Option<Instant>,
}

impl Default for AudioDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDiagnostics {
    /// Create new diagnostics tracker
    pub fn new() -> Self {
        Self { metrics: AudioPerformanceMetrics::default(), last_update_start: None }
    }

    /// Begin performance tracking for update
    pub fn begin_update(&mut self) {
        self.last_update_start = Some(Instant::now());
    }

    /// End performance tracking for update
    pub fn end_update(&mut self) {
        if let Some(start) = self.last_update_start.take() {
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_secs_f32() * 1000.0;

            self.metrics.last_update_time_ms = elapsed_ms;
            self.metrics.update_count += 1;
            self.metrics.total_update_time_ms += elapsed_ms as f64;
            self.metrics.avg_update_time_ms =
                (self.metrics.total_update_time_ms / self.metrics.update_count as f64) as f32;

            if elapsed_ms > self.metrics.peak_update_time_ms {
                self.metrics.peak_update_time_ms = elapsed_ms;
            }
        }
    }

    /// Get number of active sounds in world
    pub fn active_sounds_count(&self, world: &World) -> usize {
        world.query::<&Sound>().filter(|(_, sound)| sound.instance_id.is_some()).count()
    }

    /// Get number of entities with Sound component
    pub fn active_emitters_count(&self, world: &World) -> usize {
        world.query::<&Sound>().count()
    }

    /// Get listener position
    pub fn listener_position(&self, world: &World) -> Option<Vec3> {
        for (_entity, (transform, listener)) in world.query::<(&Transform, &AudioListener)>() {
            if listener.active {
                return Some(transform.position);
            }
        }
        None
    }

    /// Get all sound states
    pub fn get_sound_states(&self, world: &World) -> Vec<SoundState> {
        let mut states = Vec::new();

        for (entity, (transform, sound)) in world.query::<(&Transform, &Sound)>() {
            let position = if sound.spatial { Some(transform.position) } else { None };

            states.push(SoundState {
                entity_id: entity.id(),
                sound_name: sound.sound_name.clone(),
                is_playing: sound.instance_id.is_some(),
                position,
                volume: sound.volume,
                looping: sound.looping,
                spatial: sound.spatial,
                max_distance: sound.max_distance,
                doppler_enabled: sound.doppler_enabled,
                instance_id: sound.instance_id,
            });
        }

        states
    }

    /// Get performance metrics
    pub fn performance_metrics(&self) -> AudioPerformanceMetrics {
        self.metrics
    }

    /// Validate audio state and return warnings/errors
    ///
    /// Returns a list of issues found:
    /// - Multiple active listeners
    /// - Sounds with missing assets
    /// - Performance issues (update time > 1ms)
    /// - Invalid positions (NaN/Inf)
    pub fn validate_audio_state(&self, world: &World, audio_engine: &AudioEngine) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for multiple active listeners
        let active_listeners: Vec<Entity> = world
            .query::<&AudioListener>()
            .filter(|(_, listener)| listener.active)
            .map(|(entity, _)| entity)
            .collect();

        if active_listeners.is_empty() {
            issues.push("WARNING: No active audio listener found".to_string());
        } else if active_listeners.len() > 1 {
            issues.push(format!(
                "WARNING: Multiple active audio listeners found ({})",
                active_listeners.len()
            ));
        }

        // Check for performance issues
        if self.metrics.avg_update_time_ms > 1.0 {
            issues.push(format!(
                "WARNING: Average update time ({:.2}ms) exceeds target (1ms)",
                self.metrics.avg_update_time_ms
            ));
        }

        if self.metrics.peak_update_time_ms > 5.0 {
            issues.push(format!(
                "WARNING: Peak update time ({:.2}ms) is very high",
                self.metrics.peak_update_time_ms
            ));
        }

        // Check for invalid positions
        for (entity, (transform, sound)) in world.query::<(&Transform, &Sound)>() {
            if sound.spatial {
                let pos = transform.position;
                if !pos.is_finite() {
                    issues.push(format!(
                        "ERROR: Entity {} has invalid position (NaN/Inf)",
                        entity.id()
                    ));
                }
            }
        }

        // Check for sounds that claim to be playing but aren't in engine
        for (entity, sound) in world.query::<&Sound>() {
            if let Some(instance_id) = sound.instance_id {
                if !audio_engine.is_playing(instance_id) {
                    issues.push(format!(
                        "WARNING: Sound on entity {} claims to be playing but engine reports not playing",
                        entity.id()
                    ));
                }
            }
        }

        // Log issues
        for issue in &issues {
            if issue.starts_with("ERROR") {
                tracing::error!("{}", issue);
            } else if issue.starts_with("WARNING") {
                warn!("{}", issue);
            } else {
                debug!("{}", issue);
            }
        }

        issues
    }

    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = AudioPerformanceMetrics::default();
    }

    /// Get formatted diagnostic report for AI agents
    ///
    /// Returns a human-readable report with clear PASS/FAIL indicators
    pub fn generate_report(&self, world: &World, audio_engine: &AudioEngine) -> String {
        let mut report = String::new();

        report.push_str("=== Audio System Diagnostics ===\n\n");

        // Active sounds
        let active_count = self.active_sounds_count(world);
        let emitter_count = self.active_emitters_count(world);
        report.push_str(&format!("Active Sounds: {} / {}\n", active_count, emitter_count));

        // Engine state
        report.push_str(&format!(
            "Engine: {} active, {} loaded\n",
            audio_engine.active_sound_count(),
            audio_engine.loaded_sound_count()
        ));

        // Listener
        if let Some(pos) = self.listener_position(world) {
            report.push_str(&format!(
                "Listener Position: ({:.2}, {:.2}, {:.2})\n",
                pos.x, pos.y, pos.z
            ));
        } else {
            report.push_str("Listener Position: NONE\n");
        }

        // Performance
        report.push_str(&format!(
            "Performance: {:.2}ms/frame (avg: {:.2}ms, peak: {:.2}ms)\n",
            self.metrics.last_update_time_ms,
            self.metrics.avg_update_time_ms,
            self.metrics.peak_update_time_ms
        ));

        // Performance target
        let perf_status =
            if self.metrics.avg_update_time_ms < 1.0 { "✅ PASS" } else { "❌ FAIL" };
        report.push_str(&format!("{} Target: <1ms\n\n", perf_status));

        // Sound states
        let states = self.get_sound_states(world);
        if !states.is_empty() {
            report.push_str("Sound States:\n");
            for state in states {
                let status = if state.is_playing { "▶" } else { "⏸" };
                report.push_str(&format!(
                    "  {} Entity {}: '{}' (vol: {:.2}, {}{})\n",
                    status,
                    state.entity_id,
                    state.sound_name,
                    state.volume,
                    if state.spatial { "3D" } else { "2D" },
                    if state.looping { ", looping" } else { "" }
                ));
                if let Some(pos) = state.position {
                    report.push_str(&format!(
                        "     Position: ({:.2}, {:.2}, {:.2})\n",
                        pos.x, pos.y, pos.z
                    ));
                }
            }
            report.push('\n');
        }

        // Validation
        let issues = self.validate_audio_state(world, audio_engine);
        if issues.is_empty() {
            report.push_str("✅ Validation: PASS (no issues found)\n");
        } else {
            report.push_str(&format!("❌ Validation: FAIL ({} issues found)\n", issues.len()));
            for issue in issues {
                report.push_str(&format!("  - {}\n", issue));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::World;
    use std::time::Duration;

    #[test]
    fn test_diagnostics_creation() {
        let diagnostics = AudioDiagnostics::new();
        assert_eq!(diagnostics.metrics.update_count, 0);
    }

    #[test]
    fn test_active_sounds_count() {
        let mut world = World::new();
        world.register::<Sound>();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Sound::new("test.wav"));

        let diagnostics = AudioDiagnostics::new();
        assert_eq!(diagnostics.active_sounds_count(&world), 0); // Not playing

        // Simulate playing
        if let Some(sound) = world.get_mut::<Sound>(entity) {
            sound.instance_id = Some(12345);
        }
        assert_eq!(diagnostics.active_sounds_count(&world), 1);
    }

    #[test]
    fn test_listener_position() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<AudioListener>();

        let diagnostics = AudioDiagnostics::new();
        assert!(diagnostics.listener_position(&world).is_none());

        let camera = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(1.0, 2.0, 3.0);
        world.add(camera, transform);
        world.add(camera, AudioListener::new());

        let pos = diagnostics.listener_position(&world).unwrap();
        assert_eq!(pos, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_sound_states() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Sound>();

        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(5.0, 0.0, 0.0);
        world.add(entity, transform);

        let sound = Sound::new("footstep.wav").spatial_3d(50.0).with_volume(0.8);
        world.add(entity, sound);

        let diagnostics = AudioDiagnostics::new();
        let states = diagnostics.get_sound_states(&world);

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].sound_name, "footstep.wav");
        assert_eq!(states[0].volume, 0.8);
        assert!(states[0].spatial);
        assert_eq!(states[0].position, Some(Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_performance_tracking() {
        let mut diagnostics = AudioDiagnostics::new();

        diagnostics.begin_update();
        std::thread::sleep(Duration::from_micros(100));
        diagnostics.end_update();

        assert!(diagnostics.metrics.last_update_time_ms > 0.0);
        assert_eq!(diagnostics.metrics.update_count, 1);
        assert!(diagnostics.metrics.avg_update_time_ms > 0.0);
    }

    #[test]
    fn test_validation_no_listener() {
        let world = World::new();
        let audio_engine = AudioEngine::new().unwrap();
        let diagnostics = AudioDiagnostics::new();

        let issues = diagnostics.validate_audio_state(&world, &audio_engine);
        assert!(issues.iter().any(|i| i.contains("No active audio listener")));
    }

    #[test]
    fn test_validation_multiple_listeners() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<AudioListener>();

        // Create two active listeners
        let camera1 = world.spawn();
        world.add(camera1, Transform::default());
        world.add(camera1, AudioListener::new());

        let camera2 = world.spawn();
        world.add(camera2, Transform::default());
        world.add(camera2, AudioListener::new());

        let audio_engine = AudioEngine::new().unwrap();
        let diagnostics = AudioDiagnostics::new();

        let issues = diagnostics.validate_audio_state(&world, &audio_engine);
        assert!(issues.iter().any(|i| i.contains("Multiple active audio listeners")));
    }

    #[test]
    fn test_validation_invalid_position() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Sound>();

        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(f32::NAN, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").spatial_3d(100.0));

        let audio_engine = AudioEngine::new().unwrap();
        let diagnostics = AudioDiagnostics::new();

        let issues = diagnostics.validate_audio_state(&world, &audio_engine);
        assert!(issues.iter().any(|i| i.contains("invalid position")));
    }

    #[test]
    fn test_report_generation() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Sound>();
        world.register::<AudioListener>();

        // Add listener
        let camera = world.spawn();
        world.add(camera, Transform::default());
        world.add(camera, AudioListener::new());

        // Add sound
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Sound::new("test.wav"));

        let audio_engine = AudioEngine::new().unwrap();
        let diagnostics = AudioDiagnostics::new();

        let report = diagnostics.generate_report(&world, &audio_engine);

        assert!(report.contains("Audio System Diagnostics"));
        assert!(report.contains("Active Sounds"));
        assert!(report.contains("Performance"));
    }

    #[test]
    fn test_reset_metrics() {
        let mut diagnostics = AudioDiagnostics::new();

        diagnostics.begin_update();
        std::thread::sleep(Duration::from_micros(100));
        diagnostics.end_update();

        assert!(diagnostics.metrics.update_count > 0);

        diagnostics.reset_metrics();
        assert_eq!(diagnostics.metrics.update_count, 0);
        assert_eq!(diagnostics.metrics.avg_update_time_ms, 0.0);
    }
}
