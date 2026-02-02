//! Physics event stream for temporal analysis
//!
//! Records discrete events that occur during simulation:
//! - Collisions (start/end, impulses)
//! - Constraint breaks
//! - Entity sleep/wake transitions
//! - Solver convergence failures
//! - User inputs (forces, teleports)
//! - Divergence detection (multiplayer)
//!
//! Events are timestamped and can be exported to JSONL for AI agent analysis.

use engine_math::Vec3;
use serde::{Deserialize, Serialize};

/// Physics event that occurred during simulation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type")]
pub enum PhysicsEvent {
    /// Collision started between two entities
    CollisionStart {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// First entity
        entity_a: u64,
        /// Second entity
        entity_b: u64,
        /// Contact point (world space)
        contact_point: Vec3,
        /// Contact normal (from A to B)
        normal: Vec3,
        /// Collision impulse magnitude
        impulse: f32,
        /// Relative velocity at contact
        relative_velocity: f32,
    },

    /// Collision ended between two entities
    CollisionEnd {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// First entity
        entity_a: u64,
        /// Second entity
        entity_b: u64,
        /// Total frames in contact
        contact_duration_frames: u64,
    },

    /// Constraint/joint broken due to excessive force
    ConstraintBreak {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Constraint ID
        constraint_id: u64,
        /// First entity
        entity_a: u64,
        /// Second entity
        entity_b: u64,
        /// Force magnitude that caused break
        force_magnitude: f32,
        /// Breaking force threshold
        break_threshold: f32,
    },

    /// Entity woke from sleep state
    EntityWake {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Entity ID
        entity_id: u64,
        /// Reason for waking
        reason: WakeReason,
    },

    /// Entity went to sleep
    EntitySleep {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Entity ID
        entity_id: u64,
        /// Velocity when fell asleep
        linear_velocity: f32,
        /// Angular velocity when fell asleep
        angular_velocity: f32,
    },

    /// Constraint solver failed to converge
    SolverFailure {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Island ID that failed
        island_id: usize,
        /// Iterations attempted
        iterations: u32,
        /// Final residual (constraint error)
        residual: f32,
        /// Entities in failed island
        entities: Vec<u64>,
    },

    /// User applied force/impulse to entity
    ForceApplied {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Entity ID
        entity_id: u64,
        /// Force vector applied
        force: Vec3,
        /// Torque vector applied
        torque: Vec3,
        /// Was this an impulse (instant) or continuous force?
        is_impulse: bool,
    },

    /// User teleported entity (non-physical movement)
    EntityTeleport {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Entity ID
        entity_id: u64,
        /// Previous position
        old_position: Vec3,
        /// New position
        new_position: Vec3,
        /// Distance teleported
        distance: f32,
    },

    /// Divergence detected between client and server (multiplayer)
    DivergenceDetected {
        /// Frame number
        frame: u64,
        /// Timestamp (seconds)
        timestamp: f64,
        /// Entity ID that diverged
        entity_id: u64,
        /// Client position
        client_position: Vec3,
        /// Server position (authoritative)
        server_position: Vec3,
        /// Position delta magnitude
        position_delta: f32,
        /// Client velocity
        client_velocity: Vec3,
        /// Server velocity
        server_velocity: Vec3,
        /// Velocity delta magnitude
        velocity_delta: f32,
    },
}

/// Reason entity woke from sleep
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WakeReason {
    /// Collision with another entity
    Collision,
    /// Force/impulse applied
    ForceApplied,
    /// Constraint/joint interaction
    Constraint,
    /// Manual wake by user
    Manual,
    /// Nearby entity woke (cascading wake)
    Proximity,
}

impl PhysicsEvent {
    /// Get frame number for this event
    pub fn frame(&self) -> u64 {
        match self {
            PhysicsEvent::CollisionStart { frame, .. } => *frame,
            PhysicsEvent::CollisionEnd { frame, .. } => *frame,
            PhysicsEvent::ConstraintBreak { frame, .. } => *frame,
            PhysicsEvent::EntityWake { frame, .. } => *frame,
            PhysicsEvent::EntitySleep { frame, .. } => *frame,
            PhysicsEvent::SolverFailure { frame, .. } => *frame,
            PhysicsEvent::ForceApplied { frame, .. } => *frame,
            PhysicsEvent::EntityTeleport { frame, .. } => *frame,
            PhysicsEvent::DivergenceDetected { frame, .. } => *frame,
        }
    }

    /// Get timestamp for this event
    pub fn timestamp(&self) -> f64 {
        match self {
            PhysicsEvent::CollisionStart { timestamp, .. } => *timestamp,
            PhysicsEvent::CollisionEnd { timestamp, .. } => *timestamp,
            PhysicsEvent::ConstraintBreak { timestamp, .. } => *timestamp,
            PhysicsEvent::EntityWake { timestamp, .. } => *timestamp,
            PhysicsEvent::EntitySleep { timestamp, .. } => *timestamp,
            PhysicsEvent::SolverFailure { timestamp, .. } => *timestamp,
            PhysicsEvent::ForceApplied { timestamp, .. } => *timestamp,
            PhysicsEvent::EntityTeleport { timestamp, .. } => *timestamp,
            PhysicsEvent::DivergenceDetected { timestamp, .. } => *timestamp,
        }
    }

    /// Get event type as string (for filtering)
    pub fn event_type(&self) -> &'static str {
        match self {
            PhysicsEvent::CollisionStart { .. } => "CollisionStart",
            PhysicsEvent::CollisionEnd { .. } => "CollisionEnd",
            PhysicsEvent::ConstraintBreak { .. } => "ConstraintBreak",
            PhysicsEvent::EntityWake { .. } => "EntityWake",
            PhysicsEvent::EntitySleep { .. } => "EntitySleep",
            PhysicsEvent::SolverFailure { .. } => "SolverFailure",
            PhysicsEvent::ForceApplied { .. } => "ForceApplied",
            PhysicsEvent::EntityTeleport { .. } => "EntityTeleport",
            PhysicsEvent::DivergenceDetected { .. } => "DivergenceDetected",
        }
    }

    /// Get entities involved in this event
    pub fn involved_entities(&self) -> Vec<u64> {
        match self {
            PhysicsEvent::CollisionStart { entity_a, entity_b, .. } => vec![*entity_a, *entity_b],
            PhysicsEvent::CollisionEnd { entity_a, entity_b, .. } => vec![*entity_a, *entity_b],
            PhysicsEvent::ConstraintBreak { entity_a, entity_b, .. } => vec![*entity_a, *entity_b],
            PhysicsEvent::EntityWake { entity_id, .. } => vec![*entity_id],
            PhysicsEvent::EntitySleep { entity_id, .. } => vec![*entity_id],
            PhysicsEvent::SolverFailure { entities, .. } => entities.clone(),
            PhysicsEvent::ForceApplied { entity_id, .. } => vec![*entity_id],
            PhysicsEvent::EntityTeleport { entity_id, .. } => vec![*entity_id],
            PhysicsEvent::DivergenceDetected { entity_id, .. } => vec![*entity_id],
        }
    }

    /// Is this a critical event that likely indicates a bug?
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            PhysicsEvent::SolverFailure { .. } | PhysicsEvent::DivergenceDetected { .. }
        )
    }

    /// Is this event related to collisions?
    pub fn is_collision_event(&self) -> bool {
        matches!(self, PhysicsEvent::CollisionStart { .. } | PhysicsEvent::CollisionEnd { .. })
    }

    /// Is this event related to constraints?
    pub fn is_constraint_event(&self) -> bool {
        matches!(self, PhysicsEvent::ConstraintBreak { .. })
    }

    /// Is this event related to sleep/wake?
    pub fn is_sleep_event(&self) -> bool {
        matches!(self, PhysicsEvent::EntityWake { .. } | PhysicsEvent::EntitySleep { .. })
    }
}

/// Event recorder that collects events during simulation
///
/// Events are accumulated in a buffer and can be drained periodically.
#[derive(Debug, Default)]
pub struct EventRecorder {
    /// Events for current frame (not yet exported)
    current_events: Vec<PhysicsEvent>,

    /// Total events recorded (for statistics)
    total_recorded: usize,

    /// Is recording enabled?
    enabled: bool,
}

impl EventRecorder {
    /// Create a new event recorder (disabled by default)
    pub fn new() -> Self {
        Self { current_events: Vec::new(), total_recorded: 0, enabled: false }
    }

    /// Enable event recording
    pub fn enable(&mut self) {
        self.enabled = true;
        tracing::info!("Physics event recording enabled");
    }

    /// Disable event recording
    pub fn disable(&mut self) {
        self.enabled = false;
        tracing::info!("Physics event recording disabled");
    }

    /// Is recording enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record an event
    pub fn record(&mut self, event: PhysicsEvent) {
        if !self.enabled {
            return;
        }

        if event.is_critical() {
            tracing::warn!(
                event_type = event.event_type(),
                frame = event.frame(),
                "Critical physics event recorded"
            );
        }

        self.current_events.push(event);
        self.total_recorded += 1;
    }

    /// Drain all events (consumes and returns them)
    pub fn drain_events(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.current_events)
    }

    /// Get current event count
    pub fn current_count(&self) -> usize {
        self.current_events.len()
    }

    /// Get total events recorded since creation
    pub fn total_recorded(&self) -> usize {
        self.total_recorded
    }

    /// Clear all pending events without returning them
    pub fn clear(&mut self) {
        self.current_events.clear();
    }
}

/// Event statistics for analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventStatistics {
    /// Total events
    pub total: usize,

    /// Collision events
    pub collisions: usize,

    /// Constraint breaks
    pub constraint_breaks: usize,

    /// Wake events
    pub wakes: usize,

    /// Sleep events
    pub sleeps: usize,

    /// Solver failures
    pub solver_failures: usize,

    /// Force applications
    pub forces_applied: usize,

    /// Teleports
    pub teleports: usize,

    /// Divergences
    pub divergences: usize,
}

impl EventStatistics {
    /// Compute statistics from event list
    pub fn from_events(events: &[PhysicsEvent]) -> Self {
        let mut stats = Self {
            total: events.len(),
            ..Default::default()
        };

        for event in events {
            match event {
                PhysicsEvent::CollisionStart { .. } | PhysicsEvent::CollisionEnd { .. } => {
                    stats.collisions += 1;
                }
                PhysicsEvent::ConstraintBreak { .. } => stats.constraint_breaks += 1,
                PhysicsEvent::EntityWake { .. } => stats.wakes += 1,
                PhysicsEvent::EntitySleep { .. } => stats.sleeps += 1,
                PhysicsEvent::SolverFailure { .. } => stats.solver_failures += 1,
                PhysicsEvent::ForceApplied { .. } => stats.forces_applied += 1,
                PhysicsEvent::EntityTeleport { .. } => stats.teleports += 1,
                PhysicsEvent::DivergenceDetected { .. } => stats.divergences += 1,
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = PhysicsEvent::CollisionStart {
            frame: 100,
            timestamp: 1.5,
            entity_a: 1,
            entity_b: 2,
            contact_point: Vec3::ZERO,
            normal: Vec3::Y,
            impulse: 10.0,
            relative_velocity: 5.0,
        };

        assert_eq!(event.frame(), 100);
        assert_eq!(event.timestamp(), 1.5);
        assert_eq!(event.event_type(), "CollisionStart");
        assert!(event.is_collision_event());
        assert!(!event.is_critical());

        let entities = event.involved_entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&1));
        assert!(entities.contains(&2));
    }

    #[test]
    fn test_critical_events() {
        let solver_failure = PhysicsEvent::SolverFailure {
            frame: 100,
            timestamp: 1.5,
            island_id: 0,
            iterations: 8,
            residual: 10.0,
            entities: vec![1, 2, 3],
        };

        assert!(solver_failure.is_critical());

        let divergence = PhysicsEvent::DivergenceDetected {
            frame: 100,
            timestamp: 1.5,
            entity_id: 42,
            client_position: Vec3::ZERO,
            server_position: Vec3::new(1.0, 0.0, 0.0),
            position_delta: 1.0,
            client_velocity: Vec3::ZERO,
            server_velocity: Vec3::ZERO,
            velocity_delta: 0.0,
        };

        assert!(divergence.is_critical());
    }

    #[test]
    fn test_event_recorder() {
        let mut recorder = EventRecorder::new();
        assert!(!recorder.is_enabled());
        assert_eq!(recorder.current_count(), 0);

        // Recording while disabled should do nothing
        recorder.record(PhysicsEvent::EntityWake {
            frame: 1,
            timestamp: 0.016,
            entity_id: 1,
            reason: WakeReason::Manual,
        });
        assert_eq!(recorder.current_count(), 0);

        // Enable and record
        recorder.enable();
        recorder.record(PhysicsEvent::EntityWake {
            frame: 1,
            timestamp: 0.016,
            entity_id: 1,
            reason: WakeReason::Manual,
        });
        assert_eq!(recorder.current_count(), 1);
        assert_eq!(recorder.total_recorded(), 1);

        // Drain events
        let events = recorder.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(recorder.current_count(), 0);
        assert_eq!(recorder.total_recorded(), 1); // Total persists
    }

    #[test]
    fn test_event_statistics() {
        let events = vec![
            PhysicsEvent::CollisionStart {
                frame: 1,
                timestamp: 0.016,
                entity_a: 1,
                entity_b: 2,
                contact_point: Vec3::ZERO,
                normal: Vec3::Y,
                impulse: 10.0,
                relative_velocity: 5.0,
            },
            PhysicsEvent::CollisionEnd {
                frame: 10,
                timestamp: 0.16,
                entity_a: 1,
                entity_b: 2,
                contact_duration_frames: 9,
            },
            PhysicsEvent::SolverFailure {
                frame: 20,
                timestamp: 0.32,
                island_id: 0,
                iterations: 8,
                residual: 10.0,
                entities: vec![1, 2],
            },
        ];

        let stats = EventStatistics::from_events(&events);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.collisions, 2);
        assert_eq!(stats.solver_failures, 1);
        assert_eq!(stats.constraint_breaks, 0);
    }

    #[test]
    fn test_event_serialization() {
        let event = PhysicsEvent::ConstraintBreak {
            frame: 100,
            timestamp: 1.5,
            constraint_id: 5,
            entity_a: 1,
            entity_b: 2,
            force_magnitude: 1000.0,
            break_threshold: 500.0,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&event).expect("Failed to serialize");

        // Deserialize back
        let deserialized: PhysicsEvent =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_wake_reasons() {
        let reasons = vec![
            WakeReason::Collision,
            WakeReason::ForceApplied,
            WakeReason::Constraint,
            WakeReason::Manual,
            WakeReason::Proximity,
        ];

        // All wake reasons should be serializable
        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            let _deserialized: WakeReason = serde_json::from_str(&json).unwrap();
        }
    }
}
