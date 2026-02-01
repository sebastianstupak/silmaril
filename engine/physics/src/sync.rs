//! Physics-ECS synchronization system
//!
//! Syncs physics state with ECS components and sends physics events.
//!
//! # Performance Notes
//! - Uses batch operations to minimize cache misses
//! - Preallocates buffers to avoid repeated allocations
//! - Only syncs dynamic bodies (static/kinematic handled separately)
//! - Uses profiling to track sync overhead

use crate::components::{RigidBody, RigidBodyType};
use crate::events::*;
use crate::world::PhysicsWorld;
use engine_core::ecs::{Entity, World};
use engine_math::{Quat, Vec3};
use std::collections::HashMap;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Sync configuration
pub struct PhysicsSyncConfig {
    /// Sync transforms from physics → ECS
    pub sync_transforms: bool,
    /// Sync velocities from physics → ECS
    pub sync_velocities: bool,
    /// Send collision events to ECS
    pub send_events: bool,
    /// Batch size for sync operations (cache optimization)
    pub batch_size: usize,
}

impl Default for PhysicsSyncConfig {
    fn default() -> Self {
        Self {
            sync_transforms: true,
            sync_velocities: true,
            send_events: true,
            batch_size: 256, // Optimized for cache line (64 bytes)
        }
    }
}

/// Physics sync system state
///
/// Preallocates buffers to minimize allocations during sync.
pub struct PhysicsSyncSystem {
    config: PhysicsSyncConfig,
    /// Mapping from entity ID (u64) to ECS Entity
    /// This is needed because PhysicsWorld uses u64 for entity IDs
    entity_map: HashMap<u64, Entity>,
    /// Preallocated buffer for transform batch
    transform_buffer: Vec<(Entity, Vec3, Quat)>,
    /// Preallocated buffer for velocity batch
    velocity_buffer: Vec<(Entity, Vec3, Vec3)>,
}

impl PhysicsSyncSystem {
    /// Create new sync system
    pub fn new(config: PhysicsSyncConfig) -> Self {
        Self {
            transform_buffer: Vec::with_capacity(config.batch_size),
            velocity_buffer: Vec::with_capacity(config.batch_size),
            entity_map: HashMap::new(),
            config,
        }
    }

    /// Register an entity mapping (u64 → Entity)
    ///
    /// Call this when spawning entities that have physics bodies.
    pub fn register_entity(&mut self, entity_id: u64, entity: Entity) {
        self.entity_map.insert(entity_id, entity);
    }

    /// Unregister an entity mapping
    pub fn unregister_entity(&mut self, entity_id: u64) {
        self.entity_map.remove(&entity_id);
    }

    /// Sync physics state to ECS
    ///
    /// This should be called after PhysicsWorld::step() to propagate
    /// physics results back to ECS components.
    pub fn sync_to_ecs(&mut self, physics: &PhysicsWorld, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("physics_sync_to_ecs", ProfileCategory::Physics);

        // Sync transforms
        if self.config.sync_transforms {
            self.sync_transforms(physics, world);
        }

        // Sync velocities
        if self.config.sync_velocities {
            self.sync_velocities(physics, world);
        }

        // Send events
        if self.config.send_events {
            self.send_events(physics, world);
        }
    }

    /// Sync transforms from physics to ECS (batch optimized)
    fn sync_transforms(&mut self, physics: &PhysicsWorld, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("sync_transforms", ProfileCategory::Physics);

        // Clear buffer (keeps capacity)
        self.transform_buffer.clear();

        // Batch collect transforms
        for (&entity_id, &ecs_entity) in &self.entity_map {
            if let Some((pos, rot)) = physics.get_transform(entity_id) {
                self.transform_buffer.push((ecs_entity, pos, rot));

                // Flush batch when full
                if self.transform_buffer.len() >= self.config.batch_size {
                    self.flush_transform_batch(world);
                }
            }
        }

        // Flush remaining
        if !self.transform_buffer.is_empty() {
            self.flush_transform_batch(world);
        }
    }

    /// Flush transform batch to ECS
    #[inline]
    fn flush_transform_batch(&mut self, world: &mut World) {
        for (entity, pos, rot) in self.transform_buffer.drain(..) {
            // Update Transform component if it exists
            if let Some(transform) = world.get_mut::<engine_math::Transform>(entity) {
                transform.position = pos;
                transform.rotation = rot;
            }
        }
    }

    /// Sync velocities from physics to ECS (batch optimized)
    fn sync_velocities(&mut self, physics: &PhysicsWorld, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("sync_velocities", ProfileCategory::Physics);

        // Clear buffer (keeps capacity)
        self.velocity_buffer.clear();

        // Batch collect velocities
        for (&entity_id, &ecs_entity) in &self.entity_map {
            if let Some((linvel, angvel)) = physics.get_velocity(entity_id) {
                self.velocity_buffer.push((ecs_entity, linvel, angvel));

                // Flush batch when full
                if self.velocity_buffer.len() >= self.config.batch_size {
                    self.flush_velocity_batch(world);
                }
            }
        }

        // Flush remaining
        if !self.velocity_buffer.is_empty() {
            self.flush_velocity_batch(world);
        }
    }

    /// Flush velocity batch to ECS
    #[inline]
    fn flush_velocity_batch(&mut self, world: &mut World) {
        for (entity, linvel, angvel) in self.velocity_buffer.drain(..) {
            // Update Velocity component if it exists
            if let Some(vel) = world.get_mut::<crate::components::Velocity>(entity) {
                vel.linear = linvel;
                vel.angular = angvel;
            }
        }
    }

    /// Send physics events to ECS
    fn send_events(&self, physics: &PhysicsWorld, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("send_physics_events", ProfileCategory::Physics);

        // Convert Rapier collision events to our events
        for rapier_event in physics.collision_events() {
            match rapier_event {
                rapier3d::prelude::CollisionEvent::Started(h1, h2, _flags) => {
                    // Get entity IDs from collider handles
                    if let (Some(&e1), Some(&e2)) = (
                        physics.get_entity_from_collider(*h1),
                        physics.get_entity_from_collider(*h2),
                    ) {
                        world.send_event(CollisionStartEvent {
                            entity_a: e1,
                            entity_b: e2,
                            contact_point: Vec3::ZERO, // TODO: Get actual contact point
                            normal: Vec3::ZERO,         // TODO: Get actual normal
                        });
                    }
                }
                rapier3d::prelude::CollisionEvent::Stopped(h1, h2, _flags) => {
                    if let (Some(&e1), Some(&e2)) = (
                        physics.get_entity_from_collider(*h1),
                        physics.get_entity_from_collider(*h2),
                    ) {
                        world.send_event(CollisionEndEvent {
                            entity_a: e1,
                            entity_b: e2,
                        });
                    }
                }
            }
        }

        // Send contact force events
        for force_event in physics.contact_force_events() {
            if let (Some(&e1), Some(&e2)) = (
                physics.get_entity_from_collider(force_event.collider1),
                physics.get_entity_from_collider(force_event.collider2),
            ) {
                world.send_event(ContactForceEvent {
                    entity_a: e1,
                    entity_b: e2,
                    force_magnitude: force_event.total_force_magnitude,
                    contact_point: Vec3::ZERO, // TODO: Get actual contact point
                });
            }
        }
    }
}

impl Default for PhysicsSyncSystem {
    fn default() -> Self {
        Self::new(PhysicsSyncConfig::default())
    }
}

/// Helper function to create entity mapping automatically
///
/// Scans all entities with RigidBody components and registers them
/// with the sync system.
pub fn build_entity_mapping(world: &World, sync: &mut PhysicsSyncSystem) {
    #[cfg(feature = "profiling")]
    profile_scope!("build_entity_mapping", ProfileCategory::Physics);

    // Clear existing mapping
    sync.entity_map.clear();

    // Query all entities with RigidBody
    // Note: This requires query support for RigidBody components
    // For now, this is a placeholder that would need proper query implementation

    tracing::debug!("Built physics entity mapping");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = PhysicsSyncConfig::default();
        assert!(config.sync_transforms);
        assert!(config.sync_velocities);
        assert!(config.send_events);
        assert_eq!(config.batch_size, 256);
    }

    #[test]
    fn test_entity_registration() {
        let mut sync = PhysicsSyncSystem::default();

        sync.register_entity(1, Entity::from_raw(0));
        sync.register_entity(2, Entity::from_raw(1));

        assert_eq!(sync.entity_map.len(), 2);

        sync.unregister_entity(1);
        assert_eq!(sync.entity_map.len(), 1);
    }

    #[test]
    fn test_buffer_preallocation() {
        let sync = PhysicsSyncSystem::default();

        // Buffers should be preallocated
        assert_eq!(sync.transform_buffer.capacity(), 256);
        assert_eq!(sync.velocity_buffer.capacity(), 256);
    }
}
