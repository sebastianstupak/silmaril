//! Physics events for ECS integration
//!
//! Events are sent from the physics world to the ECS, allowing systems
//! to react to physics occurrences without tight coupling.

use engine_core::ecs::Event;
use engine_math::Vec3;

/// Collision started between two entities
#[derive(Debug, Clone)]
pub struct CollisionStartEvent {
    /// First entity involved in collision
    pub entity_a: u64,
    /// Second entity involved in collision
    pub entity_b: u64,
    /// Contact point in world space
    pub contact_point: Vec3,
    /// Contact normal (points from B to A)
    pub normal: Vec3,
}

impl Event for CollisionStartEvent {}

/// Collision ended between two entities
#[derive(Debug, Clone)]
pub struct CollisionEndEvent {
    /// First entity involved in collision
    pub entity_a: u64,
    /// Second entity involved in collision
    pub entity_b: u64,
}

impl Event for CollisionEndEvent {}

/// Contact force event (high-energy collisions)
#[derive(Debug, Clone)]
pub struct ContactForceEvent {
    /// First entity involved
    pub entity_a: u64,
    /// Second entity involved
    pub entity_b: u64,
    /// Total force magnitude (Newtons)
    pub force_magnitude: f32,
    /// Contact point in world space
    pub contact_point: Vec3,
}

impl Event for ContactForceEvent {}

/// Trigger entered event (sensor collider)
#[derive(Debug, Clone)]
pub struct TriggerEnterEvent {
    /// Trigger entity (the sensor)
    pub trigger: u64,
    /// Entity that entered the trigger
    pub other: u64,
}

impl Event for TriggerEnterEvent {}

/// Trigger exited event (sensor collider)
#[derive(Debug, Clone)]
pub struct TriggerExitEvent {
    /// Trigger entity (the sensor)
    pub trigger: u64,
    /// Entity that exited the trigger
    pub other: u64,
}

impl Event for TriggerExitEvent {}

/// Body started sleeping (optimization event)
#[derive(Debug, Clone)]
pub struct BodySleepEvent {
    /// Entity that started sleeping
    pub entity: u64,
}

impl Event for BodySleepEvent {}

/// Body woke up from sleep
#[derive(Debug, Clone)]
pub struct BodyWakeEvent {
    /// Entity that woke up
    pub entity: u64,
}

impl Event for BodyWakeEvent {}
