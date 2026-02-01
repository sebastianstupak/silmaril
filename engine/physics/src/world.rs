//! Physics world - wraps Rapier physics engine
//!
//! This module is **standalone** - works without ECS integration.
//! ECS sync will be added later via a separate system.

use crate::components::{Collider, ColliderShape, RigidBody, RigidBodyType};
use crate::config::{PhysicsConfig, PhysicsMode};
use engine_math::{Quat, Vec3};
use rapier3d::na::{Quaternion, UnitQuaternion};
use rapier3d::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Contact force event wrapper
#[derive(Debug, Clone)]
pub struct PhysicsContactForceEvent {
    /// Collider 1
    pub collider1: ColliderHandle,
    /// Collider 2
    pub collider2: ColliderHandle,
    /// Total force magnitude
    pub total_force_magnitude: Real,
}

use crossbeam_channel;
use std::collections::HashSet;

/// Result of a raycast query
#[derive(Debug, Clone)]
pub struct RaycastHit {
    /// Entity that was hit
    pub entity: u64,
    /// Distance along ray to hit point
    pub distance: f32,
    /// World-space hit point
    pub point: Vec3,
    /// Surface normal at hit point
    pub normal: Vec3,
}

/// Physics world - manages all physics simulation
///
/// This wraps Rapier and provides a clean API with our configuration system.
/// Can run standalone or be integrated with ECS.
pub struct PhysicsWorld {
    /// Configuration (determines execution mode)
    config: PhysicsConfig,

    /// Rapier physics pipeline
    pipeline: PhysicsPipeline,

    /// Gravity
    gravity: Vector<Real>,

    /// Integration parameters
    integration_params: IntegrationParameters,

    /// Island manager (sleeping body optimization)
    islands: IslandManager,

    /// Broad phase (spatial partitioning for collision detection)
    broad_phase: BroadPhase,

    /// Narrow phase (precise collision detection)
    narrow_phase: NarrowPhase,

    /// Rigid body set
    rigid_body_set: RigidBodySet,

    /// Collider set
    collider_set: ColliderSet,

    /// Joint set (impulse joints)
    impulse_joint_set: ImpulseJointSet,

    /// Multibody joint set (articulations/ragdolls)
    multibody_joint_set: MultibodyJointSet,

    /// CCD solver
    ccd_solver: CCDSolver,

    /// Query pipeline (raycasts, shapecast, overlaps)
    query_pipeline: QueryPipeline,

    /// Entity ID <-> RigidBodyHandle mapping
    entity_to_body: HashMap<u64, RigidBodyHandle>,
    body_to_entity: HashMap<RigidBodyHandle, u64>,

    /// ColliderHandle -> Entity ID mapping (for event translation)
    collider_to_entity: HashMap<ColliderHandle, u64>,

    /// Entity ID -> desired mass mapping (for density calculation)
    entity_desired_mass: HashMap<u64, f32>,

    /// Collision events from last step
    collision_events: Vec<CollisionEvent>,

    /// Contact force events from last step
    contact_force_events: Vec<PhysicsContactForceEvent>,

    /// Trigger enter events from last step
    trigger_enter_events: Vec<(u64, u64)>,

    /// Trigger exit events from last step
    trigger_exit_events: Vec<(u64, u64)>,

    /// Active trigger pairs (trigger_entity, other_entity)
    /// Used to detect enter/exit events
    active_trigger_pairs: HashSet<(u64, u64)>,

    /// Accumulated time for fixed timestep
    accumulator: f32,

    /// Frame counter for debugging
    frame_count: u64,
}

impl PhysicsWorld {
    /// Create a new physics world with configuration
    pub fn new(config: PhysicsConfig) -> Self {
        let mut integration_params = IntegrationParameters::default();
        integration_params.dt = config.timestep();

        // Configure based on mode
        if let PhysicsMode::Deterministic { .. } = config.mode {
            integration_params.max_ccd_substeps = config.max_substeps as usize;
        }

        // Apply deterministic configuration
        if config.deterministic {
            // Ensure fixed timestep with no variation
            integration_params.dt = config.timestep();
            // Disable non-deterministic features at integration level
            integration_params.max_ccd_substeps = config.max_substeps as usize;

            tracing::info!(
                timestep = %config.timestep(),
                solver_iterations = config.solver_iterations,
                "Physics world initialized in deterministic mode"
            );
        }

        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: vector![config.gravity.x, config.gravity.y, config.gravity.z],
            integration_params,
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            collider_to_entity: HashMap::new(),
            entity_desired_mass: HashMap::new(),
            collision_events: Vec::new(),
            contact_force_events: Vec::new(),
            trigger_enter_events: Vec::new(),
            trigger_exit_events: Vec::new(),
            active_trigger_pairs: HashSet::new(),
            accumulator: 0.0,
            frame_count: 0,
            config,
        }
    }

    /// Step physics simulation with delta time
    ///
    /// Uses fixed timestep internally for stability.
    /// See: https://gafferongames.com/post/fix_your_timestep/
    pub fn step(&mut self, dt: f32) {
        #[cfg(feature = "profiling")]
        profile_scope!("physics_step", ProfileCategory::Physics);

        if matches!(self.config.mode, PhysicsMode::Disabled) {
            return;
        }

        self.accumulator += dt;

        let fixed_dt = self.config.timestep();
        let mut steps = 0;

        // Fixed timestep integration
        while self.accumulator >= fixed_dt && steps < self.config.max_substeps {
            self.step_internal(fixed_dt);
            self.accumulator -= fixed_dt;
            steps += 1;
        }

        // Prevent spiral of death
        if self.accumulator > fixed_dt * self.config.max_substeps as f32 {
            self.accumulator = 0.0;
            tracing::warn!(
                steps = steps,
                max = self.config.max_substeps,
                "Physics spiral of death - too many substeps"
            );
        }
    }

    /// Internal step (one fixed timestep)
    fn step_internal(&mut self, _dt: f32) {
        #[cfg(feature = "profiling")]
        profile_scope!("physics_step_internal", ProfileCategory::Physics);

        self.frame_count += 1;

        // Event collector
        let (collision_send, collision_recv) = crossbeam_channel::unbounded();
        let (contact_send, contact_recv) = crossbeam_channel::unbounded();

        let event_handler = ChannelEventCollector { collision_send, contact_send };

        // Step the physics simulation
        self.pipeline.step(
            &self.gravity,
            &self.integration_params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &event_handler,
        );

        // Collect collision events
        self.collision_events.clear();
        self.contact_force_events.clear();

        while let Ok(event) = collision_recv.try_recv() {
            self.collision_events.push(event);
        }

        while let Ok(event) = contact_recv.try_recv() {
            self.contact_force_events.push(event);
        }

        // Process trigger events
        self.process_trigger_events();

        tracing::trace!(
            frame = self.frame_count,
            bodies = self.rigid_body_set.len(),
            colliders = self.collider_set.len(),
            collisions = self.collision_events.len(),
            triggers_enter = self.trigger_enter_events.len(),
            triggers_exit = self.trigger_exit_events.len(),
            "Physics step complete"
        );
    }

    /// Add rigid body for entity
    pub fn add_rigidbody(
        &mut self,
        entity_id: u64,
        rb_component: &RigidBody,
        position: Vec3,
        rotation: Quat,
    ) -> RigidBodyHandle {
        // Convert component to Rapier type
        let rapier_type = match rb_component.body_type {
            RigidBodyType::Dynamic => rapier3d::prelude::RigidBodyType::Dynamic,
            RigidBodyType::Kinematic => rapier3d::prelude::RigidBodyType::KinematicVelocityBased,
            RigidBodyType::Static => rapier3d::prelude::RigidBodyType::Fixed,
        };

        // Convert quaternion to Rapier's UnitQuaternion
        let rapier_rotation = UnitQuaternion::from_quaternion(Quaternion::new(
            rotation.w, rotation.x, rotation.y, rotation.z,
        ));

        let mut rigid_body = RigidBodyBuilder::new(rapier_type)
            .translation(vector![position.x, position.y, position.z])
            .rotation(rapier_rotation.scaled_axis())
            .linvel(vector![
                rb_component.linear_velocity.x,
                rb_component.linear_velocity.y,
                rb_component.linear_velocity.z
            ])
            .angvel(vector![
                rb_component.angular_velocity.x,
                rb_component.angular_velocity.y,
                rb_component.angular_velocity.z
            ])
            .linear_damping(rb_component.linear_damping)
            .angular_damping(rb_component.angular_damping)
            .gravity_scale(rb_component.gravity_scale)
            .ccd_enabled(rb_component.ccd_enabled && self.config.enable_ccd)
            .build();

        // Apply axis locks
        let mut locked_axes = LockedAxes::empty();
        if rb_component.lock_translation[0] {
            locked_axes |= LockedAxes::TRANSLATION_LOCKED_X;
        }
        if rb_component.lock_translation[1] {
            locked_axes |= LockedAxes::TRANSLATION_LOCKED_Y;
        }
        if rb_component.lock_translation[2] {
            locked_axes |= LockedAxes::TRANSLATION_LOCKED_Z;
        }
        if rb_component.lock_rotation[0] {
            locked_axes |= LockedAxes::ROTATION_LOCKED_X;
        }
        if rb_component.lock_rotation[1] {
            locked_axes |= LockedAxes::ROTATION_LOCKED_Y;
        }
        if rb_component.lock_rotation[2] {
            locked_axes |= LockedAxes::ROTATION_LOCKED_Z;
        }
        rigid_body.set_locked_axes(locked_axes, true);

        let handle = self.rigid_body_set.insert(rigid_body);

        self.entity_to_body.insert(entity_id, handle);
        self.body_to_entity.insert(handle, entity_id);

        // Store desired mass for density calculation
        if rb_component.body_type == RigidBodyType::Dynamic {
            self.entity_desired_mass.insert(entity_id, rb_component.mass);
        }

        tracing::debug!(entity = entity_id, mass = rb_component.mass, "Added rigidbody");

        handle
    }

    /// Add collider to entity's rigidbody
    pub fn add_collider(
        &mut self,
        entity_id: u64,
        collider_component: &Collider,
    ) -> Option<ColliderHandle> {
        let rb_handle = self.entity_to_body.get(&entity_id)?;

        // Convert shape
        let shape = Self::convert_shape(&collider_component.shape);

        // Calculate density from desired mass for dynamic bodies
        // For static/kinematic bodies, use material density directly
        let rb = self.rigid_body_set.get(*rb_handle)?;
        let density = if rb.is_dynamic() {
            // Get desired mass from stored value
            if let Some(&desired_mass) = self.entity_desired_mass.get(&entity_id) {
                // Calculate volume of the shape
                let volume = Self::calculate_shape_volume(&collider_component.shape);

                // Calculate density to achieve desired mass
                // density = mass / volume
                let calculated_density = desired_mass / volume;

                tracing::debug!(
                    entity = entity_id,
                    desired_mass = desired_mass,
                    volume = volume,
                    density = calculated_density,
                    "Calculated collider density from desired mass"
                );

                calculated_density
            } else {
                // Fallback to material density if no desired mass found
                collider_component.material.density
            }
        } else {
            collider_component.material.density
        };

        // Build collider with active events enabled
        let collider = ColliderBuilder::new(shape)
            .friction(collider_component.material.friction)
            .restitution(collider_component.material.restitution)
            .density(density)
            .sensor(collider_component.is_sensor)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(collider_component.collision_layer),
                Group::from_bits_truncate(collider_component.collision_mask),
            ))
            .active_events(ActiveEvents::COLLISION_EVENTS) // Enable collision events!
            .build();

        let collider_handle =
            self.collider_set
                .insert_with_parent(collider, *rb_handle, &mut self.rigid_body_set);

        // Register collider->entity mapping for event translation
        self.collider_to_entity.insert(collider_handle, entity_id);

        tracing::debug!(entity = entity_id, "Added collider");

        Some(collider_handle)
    }

    /// Remove rigidbody and all associated colliders
    pub fn remove_rigidbody(&mut self, entity_id: u64) {
        if let Some(handle) = self.entity_to_body.remove(&entity_id) {
            // Remove collider mappings before removing rigidbody
            // (Rapier will remove colliders when removing the body)
            self.collider_to_entity.retain(|_, &mut ent_id| ent_id != entity_id);

            self.rigid_body_set.remove(
                handle,
                &mut self.islands,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
            self.body_to_entity.remove(&handle);
            self.entity_desired_mass.remove(&entity_id);

            tracing::debug!(entity = entity_id, "Removed rigidbody");
        }
    }

    /// Get current transform of entity
    pub fn get_transform(&self, entity_id: u64) -> Option<(Vec3, Quat)> {
        let handle = self.entity_to_body.get(&entity_id)?;
        let body = self.rigid_body_set.get(*handle)?;

        let pos = body.translation();
        let rot = body.rotation();

        Some((Vec3::new(pos.x, pos.y, pos.z), Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w)))
    }

    /// Set transform of entity
    pub fn set_transform(&mut self, entity_id: u64, position: Vec3, rotation: Quat) {
        if let Some(handle) = self.entity_to_body.get(&entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.set_translation(vector![position.x, position.y, position.z], true);
                body.set_rotation(
                    UnitQuaternion::from_quaternion(Quaternion::new(
                        rotation.w, rotation.x, rotation.y, rotation.z,
                    )),
                    true,
                );
            }
        }
    }

    /// Get velocity of entity
    pub fn get_velocity(&self, entity_id: u64) -> Option<(Vec3, Vec3)> {
        let handle = self.entity_to_body.get(&entity_id)?;
        let body = self.rigid_body_set.get(*handle)?;

        let linvel = body.linvel();
        let angvel = body.angvel();

        Some((Vec3::new(linvel.x, linvel.y, linvel.z), Vec3::new(angvel.x, angvel.y, angvel.z)))
    }

    /// Set velocity of entity
    pub fn set_velocity(&mut self, entity_id: u64, linear: Vec3, angular: Vec3) {
        if let Some(handle) = self.entity_to_body.get(&entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.set_linvel(vector![linear.x, linear.y, linear.z], true);
                body.set_angvel(vector![angular.x, angular.y, angular.z], true);
            }
        }
    }

    /// Apply force to entity
    pub fn apply_force(&mut self, entity_id: u64, force: Vec3) {
        if let Some(handle) = self.entity_to_body.get(&entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.add_force(vector![force.x, force.y, force.z], true);
            }
        }
    }

    /// Apply impulse to entity
    pub fn apply_impulse(&mut self, entity_id: u64, impulse: Vec3) {
        if let Some(handle) = self.entity_to_body.get(&entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
            }
        }
    }

    /// Add joint between two entities
    ///
    /// Both entities must have rigid bodies already added.
    /// Returns the joint handle if successful.
    pub fn add_joint(
        &mut self,
        entity1: u64,
        entity2: u64,
        joint: &crate::joints::Joint,
    ) -> Option<crate::joints::JointHandle> {
        #[cfg(feature = "profiling")]
        profile_scope!("add_joint", ProfileCategory::Physics);

        let body1 = *self.entity_to_body.get(&entity1)?;
        let body2 = *self.entity_to_body.get(&entity2)?;

        let rapier_joint = joint.to_rapier();
        let handle = self.impulse_joint_set.insert(body1, body2, rapier_joint, true);

        tracing::debug!(
            entity1 = entity1,
            entity2 = entity2,
            joint_handle = ?handle,
            "Added joint"
        );

        Some(handle)
    }

    /// Remove joint
    pub fn remove_joint(&mut self, handle: crate::joints::JointHandle) -> bool {
        #[cfg(feature = "profiling")]
        profile_scope!("remove_joint", ProfileCategory::Physics);

        let removed = self.impulse_joint_set.remove(handle, true).is_some();

        if removed {
            tracing::debug!(joint_handle = ?handle, "Removed joint");
        }

        removed
    }

    /// Get number of joints
    pub fn joint_count(&self) -> usize {
        self.impulse_joint_set.len()
    }

    /// Raycast (find first hit)
    ///
    /// Returns: (entity_id, distance, hit_point, normal)
    pub fn raycast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<RaycastHit> {
        #[cfg(feature = "profiling")]
        profile_scope!("raycast_single", ProfileCategory::Physics);

        let ray = Ray::new(
            point![origin.x, origin.y, origin.z],
            vector![direction.x, direction.y, direction.z],
        );

        self.query_pipeline
            .cast_ray_and_get_normal(
                &self.rigid_body_set,
                &self.collider_set,
                &ray,
                max_distance,
                true,
                QueryFilter::default(),
            )
            .and_then(|(handle, intersection)| {
                let collider = self.collider_set.get(handle)?;
                let rb_handle = collider.parent()?;
                let entity_id = self.body_to_entity.get(&rb_handle)?;

                let hit_point = origin + direction * intersection.toi;
                let normal =
                    Vec3::new(intersection.normal.x, intersection.normal.y, intersection.normal.z);

                Some(RaycastHit {
                    entity: *entity_id,
                    distance: intersection.toi,
                    point: hit_point,
                    normal,
                })
            })
    }

    /// Raycast all (find all hits along ray, sorted by distance)
    ///
    /// Returns all hits sorted from nearest to farthest.
    pub fn raycast_all(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Vec<RaycastHit> {
        #[cfg(feature = "profiling")]
        profile_scope!("raycast_all", ProfileCategory::Physics);

        let ray = Ray::new(
            point![origin.x, origin.y, origin.z],
            vector![direction.x, direction.y, direction.z],
        );

        let mut hits = Vec::new();

        self.query_pipeline.intersections_with_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_distance,
            QueryFilter::default(),
            |handle, intersection| {
                if let Some(collider) = self.collider_set.get(handle) {
                    if let Some(rb_handle) = collider.parent() {
                        if let Some(&entity_id) = self.body_to_entity.get(&rb_handle) {
                            let hit_point = origin + direction * intersection.toi;
                            let normal = Vec3::new(
                                intersection.normal.x,
                                intersection.normal.y,
                                intersection.normal.z,
                            );

                            hits.push(RaycastHit {
                                entity: entity_id,
                                distance: intersection.toi,
                                point: hit_point,
                                normal,
                            });
                        }
                    }
                }
                true // Continue iterating
            },
        );

        // Sort by distance (nearest first)
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        tracing::trace!(hits = hits.len(), "Raycast all complete");

        hits
    }

    /// Get collision events from last step
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    /// Get contact force events from last step
    pub fn contact_force_events(&self) -> &[PhysicsContactForceEvent] {
        &self.contact_force_events
    }

    /// Get number of active bodies
    pub fn body_count(&self) -> usize {
        self.rigid_body_set.len()
    }

    /// Get number of colliders
    pub fn collider_count(&self) -> usize {
        self.collider_set.len()
    }

    /// Get current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get entity ID from collider handle (for event translation)
    pub fn get_entity_from_collider(&self, handle: ColliderHandle) -> Option<&u64> {
        self.collider_to_entity.get(&handle)
    }

    /// Get all entity IDs in the physics world
    ///
    /// Returns an iterator over all entity IDs that have physics bodies.
    /// This is used for deterministic state hashing and snapshots.
    pub fn entity_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.entity_to_body.keys().copied()
    }

    /// Get trigger enter events from last step
    pub fn trigger_enter_events(&self) -> &[(u64, u64)] {
        &self.trigger_enter_events
    }

    /// Get trigger exit events from last step
    pub fn trigger_exit_events(&self) -> &[(u64, u64)] {
        &self.trigger_exit_events
    }

    /// Process trigger events from collision events
    ///
    /// Detects when entities enter/exit sensor colliders and generates
    /// trigger events accordingly.
    fn process_trigger_events(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("process_trigger_events", ProfileCategory::Physics);

        self.trigger_enter_events.clear();
        self.trigger_exit_events.clear();

        // Build set of currently active trigger pairs from collision events
        let mut current_pairs = HashSet::new();

        for collision_event in &self.collision_events {
            match collision_event {
                CollisionEvent::Started(h1, h2, _flags) => {
                    // Check if either collider is a sensor
                    let c1 = self.collider_set.get(*h1);
                    let c2 = self.collider_set.get(*h2);

                    if let (Some(c1), Some(c2)) = (c1, c2) {
                        let is_sensor_1 = c1.is_sensor();
                        let is_sensor_2 = c2.is_sensor();

                        // Only process if at least one is a sensor
                        if is_sensor_1 || is_sensor_2 {
                            // Get entity IDs
                            if let (Some(&e1), Some(&e2)) = (
                                self.collider_to_entity.get(h1),
                                self.collider_to_entity.get(h2),
                            ) {
                                // Determine which is the trigger (sensor) and which is the other
                                let (trigger, other) = if is_sensor_1 && !is_sensor_2 {
                                    (e1, e2)
                                } else if is_sensor_2 && !is_sensor_1 {
                                    (e2, e1)
                                } else {
                                    // Both are sensors or neither, use first as trigger
                                    (e1, e2)
                                };

                                current_pairs.insert((trigger, other));
                            }
                        }
                    }
                }
                CollisionEvent::Stopped(_, _, _) => {
                    // We'll detect exits by comparing with previous frame
                }
            }
        }

        // Detect new trigger pairs (enters)
        for &pair in &current_pairs {
            if !self.active_trigger_pairs.contains(&pair) {
                self.trigger_enter_events.push(pair);
                tracing::trace!(trigger = pair.0, other = pair.1, "Trigger enter");
            }
        }

        // Detect removed trigger pairs (exits)
        for &pair in &self.active_trigger_pairs {
            if !current_pairs.contains(&pair) {
                self.trigger_exit_events.push(pair);
                tracing::trace!(trigger = pair.0, other = pair.1, "Trigger exit");
            }
        }

        // Update active pairs
        self.active_trigger_pairs = current_pairs;
    }

    /// Convert collider shape component to Rapier shape
    fn convert_shape(shape: &ColliderShape) -> SharedShape {
        match shape {
            ColliderShape::Box { half_extents } => {
                SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z)
            }
            ColliderShape::Sphere { radius } => SharedShape::ball(*radius),
            ColliderShape::Capsule { half_height, radius } => {
                SharedShape::capsule_y(*half_height, *radius)
            }
            ColliderShape::Cylinder { half_height, radius } => {
                SharedShape::cylinder(*half_height, *radius)
            }
        }
    }

    /// Calculate volume of a collider shape
    fn calculate_shape_volume(shape: &ColliderShape) -> f32 {
        match shape {
            ColliderShape::Box { half_extents } => {
                // Volume = (2 * half_extent)³ = 8 * hx * hy * hz
                8.0 * half_extents.x * half_extents.y * half_extents.z
            }
            ColliderShape::Sphere { radius } => {
                // Volume = (4/3) * π * r³
                (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3)
            }
            ColliderShape::Capsule { half_height, radius } => {
                // Volume = cylinder + sphere
                // Cylinder: π * r² * (2 * half_height)
                // Sphere: (4/3) * π * r³
                let cylinder_vol = std::f32::consts::PI * radius.powi(2) * (2.0 * half_height);
                let sphere_vol = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
                cylinder_vol + sphere_vol
            }
            ColliderShape::Cylinder { half_height, radius } => {
                // Volume = π * r² * (2 * half_height)
                std::f32::consts::PI * radius.powi(2) * (2.0 * half_height)
            }
        }
    }
}

/// Event collector using channels (thread-safe)
struct ChannelEventCollector {
    collision_send: crossbeam_channel::Sender<CollisionEvent>,
    contact_send: crossbeam_channel::Sender<PhysicsContactForceEvent>,
}

impl EventHandler for ChannelEventCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        let _ = self.collision_send.send(event);
    }

    fn handle_contact_force_event(
        &self,
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        contact_pair: &ContactPair,
        total_force_magnitude: Real,
    ) {
        let event = PhysicsContactForceEvent {
            collider1: contact_pair.collider1,
            collider2: contact_pair.collider2,
            total_force_magnitude,
        };

        let _ = self.contact_send.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_creation() {
        let config = PhysicsConfig::default();
        let world = PhysicsWorld::new(config);

        assert_eq!(world.body_count(), 0);
        assert_eq!(world.collider_count(), 0);
        assert_eq!(world.frame_count(), 0);
    }

    #[test]
    fn test_add_rigidbody() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

        assert_eq!(world.body_count(), 1);
    }

    #[test]
    fn test_add_collider() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

        let collider = Collider::sphere(1.0);
        world.add_collider(1, &collider);

        assert_eq!(world.collider_count(), 1);
    }

    #[test]
    fn test_get_set_transform() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let (pos, _rot) = world.get_transform(1).unwrap();
        assert!((pos.x - 1.0).abs() < 0.01);
        assert!((pos.y - 2.0).abs() < 0.01);
        assert!((pos.z - 3.0).abs() < 0.01);

        world.set_transform(1, Vec3::new(5.0, 6.0, 7.0), Quat::IDENTITY);
        let (new_pos, _) = world.get_transform(1).unwrap();
        assert!((new_pos.x - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_velocity() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

        world.set_velocity(1, Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO);
        let (linvel, _) = world.get_velocity(1).unwrap();

        assert!((linvel.x - 1.0).abs() < 0.01);
        assert!((linvel.y - 2.0).abs() < 0.01);
        assert!((linvel.z - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_remove_rigidbody() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(world.body_count(), 1);

        world.remove_rigidbody(1);
        assert_eq!(world.body_count(), 0);
    }

    #[test]
    fn test_physics_step() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

        // Step once
        world.step(1.0 / 60.0);
        assert_eq!(world.frame_count(), 1);

        // Step multiple times
        for _ in 0..10 {
            world.step(1.0 / 60.0);
        }
        assert_eq!(world.frame_count(), 11);
    }
}
