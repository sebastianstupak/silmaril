//! Audio Event Logger
//!
//! Logs all audio events for debugging and agent inspection.
//! Provides queryable event history with filtering capabilities.

use engine_core::math::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

/// Maximum number of events to keep in history
const MAX_EVENT_HISTORY: usize = 100;

/// Audio event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioEventType {
    /// Sound loaded
    SoundLoaded {
        /// Sound name
        name: String,
        /// File path
        path: String,
    },

    /// Sound played (2D)
    Sound2DPlayed {
        /// Sound name
        name: String,
        /// Instance ID
        instance_id: u64,
        /// Volume
        volume: f32,
        /// Looping
        looping: bool,
    },

    /// Sound played (3D)
    Sound3DPlayed {
        /// Entity ID
        entity_id: u32,
        /// Sound name
        name: String,
        /// Instance ID
        instance_id: u64,
        /// Position
        position: Vec3,
        /// Volume
        volume: f32,
        /// Looping
        looping: bool,
    },

    /// Sound stopped
    SoundStopped {
        /// Instance ID
        instance_id: u64,
        /// Fade out duration (if any)
        fade_out: Option<f32>,
    },

    /// Position updated
    PositionUpdated {
        /// Entity ID
        entity_id: u32,
        /// New position
        position: Vec3,
    },

    /// Listener updated
    ListenerUpdated {
        /// Position
        position: Vec3,
        /// Forward direction
        forward: Vec3,
        /// Up direction
        up: Vec3,
    },

    /// Pitch changed (Doppler effect)
    PitchChanged {
        /// Instance ID
        instance_id: u64,
        /// New pitch
        pitch: f32,
    },

    /// Effect added
    EffectAdded {
        /// Instance ID
        instance_id: u64,
        /// Effect type
        effect_type: String,
        /// Effect index
        effect_index: usize,
    },

    /// Effect removed
    EffectRemoved {
        /// Instance ID
        instance_id: u64,
        /// Effect index
        effect_index: usize,
    },

    /// Error occurred
    Error {
        /// Error message
        message: String,
    },
}

/// Audio event with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEvent {
    /// Event type
    pub event_type: AudioEventType,

    /// Timestamp (relative to logger creation)
    pub timestamp: Duration,

    /// Frame number (if available)
    pub frame: Option<u64>,
}

/// Event filter for querying events
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Filter by entity ID
    pub entity_id: Option<u32>,

    /// Filter by instance ID
    pub instance_id: Option<u64>,

    /// Filter by sound name
    pub sound_name: Option<String>,

    /// Filter by event type (simple string matching)
    pub event_type_contains: Option<String>,

    /// Minimum timestamp
    pub min_timestamp: Option<Duration>,

    /// Maximum timestamp
    pub max_timestamp: Option<Duration>,
}

impl EventFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by entity ID
    pub fn with_entity(mut self, entity_id: u32) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    /// Filter by instance ID
    pub fn with_instance(mut self, instance_id: u64) -> Self {
        self.instance_id = Some(instance_id);
        self
    }

    /// Filter by sound name
    pub fn with_sound_name(mut self, name: impl Into<String>) -> Self {
        self.sound_name = Some(name.into());
        self
    }

    /// Filter by event type (partial string match)
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type_contains = Some(event_type.into());
        self
    }

    /// Filter by time range
    pub fn with_time_range(mut self, min: Duration, max: Duration) -> Self {
        self.min_timestamp = Some(min);
        self.max_timestamp = Some(max);
        self
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &AudioEvent) -> bool {
        // Check entity ID
        if let Some(entity_id) = self.entity_id {
            let event_has_entity = match &event.event_type {
                AudioEventType::Sound3DPlayed { entity_id: e, .. } => *e == entity_id,
                AudioEventType::PositionUpdated { entity_id: e, .. } => *e == entity_id,
                _ => false,
            };
            if !event_has_entity {
                return false;
            }
        }

        // Check instance ID
        if let Some(instance_id) = self.instance_id {
            let event_has_instance = match &event.event_type {
                AudioEventType::Sound2DPlayed { instance_id: i, .. } => *i == instance_id,
                AudioEventType::Sound3DPlayed { instance_id: i, .. } => *i == instance_id,
                AudioEventType::SoundStopped { instance_id: i, .. } => *i == instance_id,
                AudioEventType::PitchChanged { instance_id: i, .. } => *i == instance_id,
                AudioEventType::EffectAdded { instance_id: i, .. } => *i == instance_id,
                AudioEventType::EffectRemoved { instance_id: i, .. } => *i == instance_id,
                _ => false,
            };
            if !event_has_instance {
                return false;
            }
        }

        // Check sound name
        if let Some(ref sound_name) = self.sound_name {
            let event_has_sound = match &event.event_type {
                AudioEventType::SoundLoaded { name, .. } => name == sound_name,
                AudioEventType::Sound2DPlayed { name, .. } => name == sound_name,
                AudioEventType::Sound3DPlayed { name, .. } => name == sound_name,
                _ => false,
            };
            if !event_has_sound {
                return false;
            }
        }

        // Check event type
        if let Some(ref event_type_str) = self.event_type_contains {
            let event_name = format!("{:?}", event.event_type);
            if !event_name.contains(event_type_str) {
                return false;
            }
        }

        // Check timestamp range
        if let Some(min) = self.min_timestamp {
            if event.timestamp < min {
                return false;
            }
        }
        if let Some(max) = self.max_timestamp {
            if event.timestamp > max {
                return false;
            }
        }

        true
    }
}

/// Audio event logger
pub struct AudioEventLogger {
    /// Event history (ring buffer)
    events: VecDeque<AudioEvent>,

    /// Start time
    start_time: Instant,

    /// Current frame number
    current_frame: u64,

    /// Maximum history size
    max_history: usize,
}

impl Default for AudioEventLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEventLogger {
    /// Create a new event logger
    pub fn new() -> Self {
        Self::with_capacity(MAX_EVENT_HISTORY)
    }

    /// Create a new event logger with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            start_time: Instant::now(),
            current_frame: 0,
            max_history: capacity,
        }
    }

    /// Log an event
    pub fn log_event(&mut self, event_type: AudioEventType) {
        let event = AudioEvent {
            event_type: event_type.clone(),
            timestamp: self.start_time.elapsed(),
            frame: Some(self.current_frame),
        };

        // Log via tracing
        match &event_type {
            AudioEventType::Error { message } => {
                debug!("Audio event: Error - {}", message);
            }
            _ => {
                trace!("Audio event: {:?}", event_type);
            }
        }

        // Add to history
        if self.events.len() >= self.max_history {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Increment frame counter
    pub fn next_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Get all events
    pub fn events(&self) -> &VecDeque<AudioEvent> {
        &self.events
    }

    /// Get events matching filter
    pub fn query(&self, filter: &EventFilter) -> Vec<&AudioEvent> {
        self.events.iter().filter(|event| filter.matches(event)).collect()
    }

    /// Get last N events
    pub fn last_n(&self, n: usize) -> Vec<&AudioEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Clear event history
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get formatted event log
    pub fn format_log(&self, max_events: Option<usize>) -> String {
        let mut log = String::new();

        log.push_str("=== Audio Event Log ===\n\n");
        log.push_str(&format!("Total Events: {}\n", self.events.len()));
        log.push_str(&format!("Current Frame: {}\n\n", self.current_frame));

        let events_to_show = max_events.unwrap_or(self.events.len());
        let events: Vec<_> = self.events.iter().rev().take(events_to_show).collect();

        for event in events.iter().rev() {
            log.push_str(&format!(
                "[{:>6.3}s] Frame {:>6} | ",
                event.timestamp.as_secs_f32(),
                event.frame.unwrap_or(0)
            ));

            match &event.event_type {
                AudioEventType::SoundLoaded { name, path } => {
                    log.push_str(&format!("Loaded: '{}' from '{}'", name, path));
                }
                AudioEventType::Sound2DPlayed { name, instance_id, volume, looping } => {
                    log.push_str(&format!(
                        "Play2D: '{}' (ID: {}, vol: {:.2}, loop: {})",
                        name, instance_id, volume, looping
                    ));
                }
                AudioEventType::Sound3DPlayed {
                    entity_id,
                    name,
                    instance_id,
                    position,
                    volume,
                    looping,
                } => {
                    log.push_str(&format!(
                        "Play3D: '{}' (Entity: {}, ID: {}, pos: ({:.1}, {:.1}, {:.1}), vol: {:.2}, loop: {})",
                        name, entity_id, instance_id, position.x, position.y, position.z, volume, looping
                    ));
                }
                AudioEventType::SoundStopped { instance_id, fade_out } => {
                    log.push_str(&format!("Stop: ID {} (fade: {:?})", instance_id, fade_out));
                }
                AudioEventType::PositionUpdated { entity_id, position } => {
                    log.push_str(&format!(
                        "PosUpdate: Entity {} -> ({:.1}, {:.1}, {:.1})",
                        entity_id, position.x, position.y, position.z
                    ));
                }
                AudioEventType::ListenerUpdated { position, forward, up } => {
                    log.push_str(&format!(
                        "Listener: pos ({:.1}, {:.1}, {:.1}), fwd ({:.2}, {:.2}, {:.2}), up ({:.2}, {:.2}, {:.2})",
                        position.x, position.y, position.z,
                        forward.x, forward.y, forward.z,
                        up.x, up.y, up.z
                    ));
                }
                AudioEventType::PitchChanged { instance_id, pitch } => {
                    log.push_str(&format!("Pitch: ID {} -> {:.3}", instance_id, pitch));
                }
                AudioEventType::EffectAdded { instance_id, effect_type, effect_index } => {
                    log.push_str(&format!(
                        "Effect+: ID {} [{}] '{}'",
                        instance_id, effect_index, effect_type
                    ));
                }
                AudioEventType::EffectRemoved { instance_id, effect_index } => {
                    log.push_str(&format!("Effect-: ID {} [{}]", instance_id, effect_index));
                }
                AudioEventType::Error { message } => {
                    log.push_str(&format!("ERROR: {}", message));
                }
            }

            log.push('\n');
        }

        log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_logger_creation() {
        let logger = AudioEventLogger::new();
        assert_eq!(logger.event_count(), 0);
        assert_eq!(logger.current_frame, 0);
    }

    #[test]
    fn test_log_event() {
        let mut logger = AudioEventLogger::new();

        logger.log_event(AudioEventType::SoundLoaded {
            name: "test.wav".to_string(),
            path: "assets/test.wav".to_string(),
        });

        assert_eq!(logger.event_count(), 1);
    }

    #[test]
    fn test_max_history() {
        let mut logger = AudioEventLogger::with_capacity(5);

        for i in 0..10 {
            logger.log_event(AudioEventType::SoundLoaded {
                name: format!("sound{}.wav", i),
                path: format!("assets/sound{}.wav", i),
            });
        }

        assert_eq!(logger.event_count(), 5);
    }

    #[test]
    fn test_frame_counter() {
        let mut logger = AudioEventLogger::new();

        logger.next_frame();
        logger.next_frame();

        assert_eq!(logger.current_frame, 2);
    }

    #[test]
    fn test_filter_by_entity() {
        let mut logger = AudioEventLogger::new();

        logger.log_event(AudioEventType::Sound3DPlayed {
            entity_id: 1,
            name: "sound1.wav".to_string(),
            instance_id: 100,
            position: Vec3::ZERO,
            volume: 1.0,
            looping: false,
        });

        logger.log_event(AudioEventType::Sound3DPlayed {
            entity_id: 2,
            name: "sound2.wav".to_string(),
            instance_id: 101,
            position: Vec3::ZERO,
            volume: 1.0,
            looping: false,
        });

        let filter = EventFilter::new().with_entity(1);
        let results = logger.query(&filter);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_by_sound_name() {
        let mut logger = AudioEventLogger::new();

        logger.log_event(AudioEventType::Sound2DPlayed {
            name: "music.ogg".to_string(),
            instance_id: 100,
            volume: 0.5,
            looping: true,
        });

        logger.log_event(AudioEventType::Sound2DPlayed {
            name: "sfx.wav".to_string(),
            instance_id: 101,
            volume: 1.0,
            looping: false,
        });

        let filter = EventFilter::new().with_sound_name("music.ogg");
        let results = logger.query(&filter);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_last_n() {
        let mut logger = AudioEventLogger::new();

        for i in 0..10 {
            logger.log_event(AudioEventType::SoundLoaded {
                name: format!("sound{}.wav", i),
                path: format!("assets/sound{}.wav", i),
            });
        }

        let last_3 = logger.last_n(3);
        assert_eq!(last_3.len(), 3);
    }

    #[test]
    fn test_clear() {
        let mut logger = AudioEventLogger::new();

        logger.log_event(AudioEventType::SoundLoaded {
            name: "test.wav".to_string(),
            path: "assets/test.wav".to_string(),
        });

        assert_eq!(logger.event_count(), 1);

        logger.clear();
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_format_log() {
        let mut logger = AudioEventLogger::new();

        logger.log_event(AudioEventType::SoundLoaded {
            name: "test.wav".to_string(),
            path: "assets/test.wav".to_string(),
        });

        let log = logger.format_log(None);
        assert!(log.contains("Audio Event Log"));
        assert!(log.contains("test.wav"));
    }
}
