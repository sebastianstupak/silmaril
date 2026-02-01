# Phase 3.1: Physics Integration

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** High (core gameplay functionality)

---

## 🎯 **Objective**

Integrate Rapier physics engine with server-authoritative simulation. Provides collision detection, rigidbody dynamics, and efficient physics stepping for networked gameplay.

**Features:**
- Rapier physics integration
- Server-authoritative physics simulation
- Collision detection and response
- Rigidbody component (dynamic/kinematic/static)
- Physics-ECS synchronization
- Efficient physics stepping

---

## 📋 **Detailed Tasks**

### **1. Physics World Setup** (Day 1)

**File:** `engine/physics/src/world.rs`

```rust
use rapier3d::prelude::*;
use glam::{Vec3, Quat};
use std::collections::HashMap;

/// Physics world managing all physics simulation
pub struct PhysicsWorld {
    /// Rapier physics pipeline
    pipeline: PhysicsPipeline,

    /// Gravity
    gravity: Vec3,

    /// Integration parameters
    integration_params: IntegrationParameters,

    /// Island manager (for sleeping bodies)
    islands: IslandManager,

    /// Broad phase (collision detection)
    broad_phase: BroadPhase,

    /// Narrow phase (precise collision)
    narrow_phase: NarrowPhase,

    /// Rigidbody set
    rigid_body_set: RigidBodySet,

    /// Collider set
    collider_set: ColliderSet,

    /// Joint set
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,

    /// CCD solver
    ccd_solver: CCDSolver,

    /// Entity to rigidbody mapping
    entity_to_body: HashMap<u64, RigidBodyHandle>,
    body_to_entity: HashMap<RigidBodyHandle, u64>,

    /// Collision events
    collision_events: Vec<CollisionEvent>,
    contact_force_events: Vec<ContactForceEvent>,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        let mut integration_params = IntegrationParameters::default();
        integration_params.dt = 1.0 / 60.0; // Fixed 60Hz timestep

        Self {
            pipeline: PhysicsPipeline::new(),
            gravity,
            integration_params,
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            collision_events: Vec::new(),
            contact_force_events: Vec::new(),
        }
    }

    /// Step physics simulation
    pub fn step(&mut self) {
        let gravity = vector![self.gravity.x, self.gravity.y, self.gravity.z];

        let physics_hooks = ();
        let event_handler = ChannelEventCollector::new();

        // Step simulation
        self.pipeline.step(
            &gravity,
            &self.integration_params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None, // query_pipeline
            &physics_hooks,
            &event_handler,
        );

        // Collect events
        self.collision_events.clear();
        self.contact_force_events.clear();

        while let Ok(event) = event_handler.collision_events.try_recv() {
            self.collision_events.push(event);
        }

        while let Ok(event) = event_handler.contact_force_events.try_recv() {
            self.contact_force_events.push(event);
        }

        tracing::trace!(
            "Physics step: {} bodies, {} colliders, {} collisions",
            self.rigid_body_set.len(),
            self.collider_set.len(),
            self.collision_events.len()
        );
    }

    /// Add rigidbody for entity
    pub fn add_rigidbody(
        &mut self,
        entity: u64,
        body_type: RigidBodyType,
        position: Vec3,
        rotation: Quat,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::new(body_type)
            .translation(vector![position.x, position.y, position.z])
            .rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w])
            .build();

        let handle = self.rigid_body_set.insert(rigid_body);

        self.entity_to_body.insert(entity, handle);
        self.body_to_entity.insert(handle, entity);

        tracing::debug!("Added rigidbody for entity {}: {:?}", entity, body_type);

        handle
    }

    /// Remove rigidbody for entity
    pub fn remove_rigidbody(&mut self, entity: u64) {
        if let Some(handle) = self.entity_to_body.remove(&entity) {
            self.rigid_body_set.remove(
                handle,
                &mut self.islands,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
            self.body_to_entity.remove(&handle);

            tracing::debug!("Removed rigidbody for entity {}", entity);
        }
    }

    /// Add collider to rigidbody
    pub fn add_collider(
        &mut self,
        entity: u64,
        shape: SharedShape,
        density: f32,
    ) -> Option<ColliderHandle> {
        let handle = self.entity_to_body.get(&entity)?;

        let collider = ColliderBuilder::new(shape)
            .density(density)
            .build();

        let collider_handle = self.collider_set.insert_with_parent(
            collider,
            *handle,
            &mut self.rigid_body_set,
        );

        tracing::debug!("Added collider to entity {}", entity);

        Some(collider_handle)
    }

    /// Get rigidbody position/rotation
    pub fn get_transform(&self, entity: u64) -> Option<(Vec3, Quat)> {
        let handle = self.entity_to_body.get(&entity)?;
        let body = self.rigid_body_set.get(*handle)?;

        let pos = body.translation();
        let rot = body.rotation();

        Some((
            Vec3::new(pos.x, pos.y, pos.z),
            Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w),
        ))
    }

    /// Set rigidbody position/rotation
    pub fn set_transform(&mut self, entity: u64, position: Vec3, rotation: Quat) {
        if let Some(handle) = self.entity_to_body.get(&entity) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.set_translation(vector![position.x, position.y, position.z], true);
                body.set_rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w], true);
            }
        }
    }

    /// Apply force to rigidbody
    pub fn apply_force(&mut self, entity: u64, force: Vec3) {
        if let Some(handle) = self.entity_to_body.get(&entity) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.add_force(vector![force.x, force.y, force.z], true);
            }
        }
    }

    /// Apply impulse to rigidbody
    pub fn apply_impulse(&mut self, entity: u64, impulse: Vec3) {
        if let Some(handle) = self.entity_to_body.get(&entity) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
            }
        }
    }

    /// Get collision events
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    /// Get contact force events
    pub fn contact_force_events(&self) -> &[ContactForceEvent] {
        &self.contact_force_events
    }
}

/// Event collector for physics events
struct ChannelEventCollector {
    collision_events: crossbeam_channel::Receiver<CollisionEvent>,
    contact_force_events: crossbeam_channel::Receiver<ContactForceEvent>,
}

impl ChannelEventCollector {
    fn new() -> Self {
        let (collision_send, collision_recv) = crossbeam_channel::unbounded();
        let (contact_send, contact_recv) = crossbeam_channel::unbounded();

        Self {
            collision_events: collision_recv,
            contact_force_events: contact_recv,
        }
    }
}

impl EventHandler for ChannelEventCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        // Send collision event
        let _ = self.collision_events.send(event);
    }

    fn handle_contact_force_event(
        &self,
        dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        contact_pair: &ContactPair,
        total_force_magnitude: Real,
    ) {
        // Create event
        let event = ContactForceEvent {
            dt,
            collider1: contact_pair.collider1,
            collider2: contact_pair.collider2,
            total_force_magnitude,
        };

        let _ = self.contact_force_events.send(event);
    }
}
```

---

### **2. Rigidbody Component** (Day 2)

**File:** `engine/ecs/src/components/rigidbody.rs`

```rust
use serde::{Deserialize, Serialize};
use glam::Vec3;

/// Rigidbody component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rigidbody {
    /// Body type
    pub body_type: RigidBodyType,

    /// Mass (kg)
    pub mass: f32,

    /// Linear velocity (m/s)
    pub linear_velocity: Vec3,

    /// Angular velocity (rad/s)
    pub angular_velocity: Vec3,

    /// Linear damping
    pub linear_damping: f32,

    /// Angular damping
    pub angular_damping: f32,

    /// Lock axes (prevent movement/rotation on certain axes)
    pub lock_translation: [bool; 3],
    pub lock_rotation: [bool; 3],

    /// CCD (continuous collision detection) enabled
    pub ccd_enabled: bool,

    /// Gravity scale (1.0 = normal gravity)
    pub gravity_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RigidBodyType {
    /// Dynamic body (affected by forces)
    Dynamic,

    /// Kinematic body (controlled by velocity)
    Kinematic,

    /// Static body (never moves)
    Static,
}

impl Default for Rigidbody {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            mass: 1.0,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            linear_damping: 0.0,
            angular_damping: 0.0,
            lock_translation: [false; 3],
            lock_rotation: [false; 3],
            ccd_enabled: false,
            gravity_scale: 1.0,
        }
    }
}

impl Rigidbody {
    pub fn dynamic(mass: f32) -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            mass,
            ..Default::default()
        }
    }

    pub fn kinematic() -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            ..Default::default()
        }
    }

    pub fn static_body() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            ..Default::default()
        }
    }
}

/// Collider component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider {
    /// Shape
    pub shape: ColliderShape,

    /// Density (kg/m³)
    pub density: f32,

    /// Friction coefficient (0.0 - 1.0)
    pub friction: f32,

    /// Restitution (bounciness, 0.0 - 1.0)
    pub restitution: f32,

    /// Is sensor (doesn't cause collision response)
    pub is_sensor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderShape {
    /// Box (half extents)
    Box { half_extents: Vec3 },

    /// Sphere (radius)
    Sphere { radius: f32 },

    /// Capsule (half height, radius)
    Capsule { half_height: f32, radius: f32 },

    /// Cylinder (half height, radius)
    Cylinder { half_height: f32, radius: f32 },
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: Vec3::ONE,
            },
            density: 1.0,
            friction: 0.5,
            restitution: 0.0,
            is_sensor: false,
        }
    }
}

impl Collider {
    pub fn box_collider(half_extents: Vec3) -> Self {
        Self {
            shape: ColliderShape::Box { half_extents },
            ..Default::default()
        }
    }

    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere { radius },
            ..Default::default()
        }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape::Capsule { half_height, radius },
            ..Default::default()
        }
    }
}
```

---

### **3. Physics System** (Day 3-4)

**File:** `engine/physics/src/systems.rs`

```rust
use crate::world::PhysicsWorld;
use engine_ecs::prelude::*;

/// Physics synchronization system
pub struct PhysicsSystem {
    physics_world: PhysicsWorld,
}

impl PhysicsSystem {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            physics_world: PhysicsWorld::new(gravity),
        }
    }

    /// Sync ECS components to physics world
    pub fn sync_to_physics(&mut self, world: &World) {
        // Add new rigidbodies
        for (entity, (transform, rigidbody)) in world
            .query::<(&Transform, &Rigidbody)>()
            .iter()
        {
            // Check if body already exists
            if !self.physics_world.has_rigidbody(entity.id()) {
                // Convert body type
                let body_type = match rigidbody.body_type {
                    RigidBodyType::Dynamic => rapier3d::prelude::RigidBodyType::Dynamic,
                    RigidBodyType::Kinematic => rapier3d::prelude::RigidBodyType::KinematicVelocityBased,
                    RigidBodyType::Static => rapier3d::prelude::RigidBodyType::Fixed,
                };

                // Add to physics world
                let handle = self.physics_world.add_rigidbody(
                    entity.id(),
                    body_type,
                    transform.position,
                    transform.rotation,
                );

                // Add collider if present
                if let Some(collider) = world.get_component::<Collider>(entity) {
                    let shape = Self::convert_shape(&collider.shape);
                    self.physics_world.add_collider(
                        entity.id(),
                        shape,
                        collider.density,
                    );
                }

                tracing::debug!("Synced entity {} to physics", entity.id());
            }
        }

        // Remove deleted rigidbodies
        // (Would need to track deleted entities)
    }

    /// Step physics simulation
    pub fn step(&mut self) {
        self.physics_world.step();
    }

    /// Sync physics results back to ECS
    pub fn sync_from_physics(&mut self, world: &mut World) {
        for (entity, transform) in world.query::<&mut Transform>().iter() {
            if let Some((position, rotation)) = self.physics_world.get_transform(entity.id()) {
                transform.position = position;
                transform.rotation = rotation;
            }
        }
    }

    /// Process collision events
    pub fn process_collisions(&self, world: &mut World) {
        for event in self.physics_world.collision_events() {
            match event {
                CollisionEvent::Started(h1, h2, _flags) => {
                    if let (Some(e1), Some(e2)) = (
                        self.physics_world.body_to_entity(*h1),
                        self.physics_world.body_to_entity(*h2),
                    ) {
                        tracing::debug!("Collision started: {} <-> {}", e1, e2);

                        // Emit collision event
                        // world.emit_event(CollisionStartedEvent { entity1: e1, entity2: e2 });
                    }
                }
                CollisionEvent::Stopped(h1, h2, _flags) => {
                    if let (Some(e1), Some(e2)) = (
                        self.physics_world.body_to_entity(*h1),
                        self.physics_world.body_to_entity(*h2),
                    ) {
                        tracing::debug!("Collision stopped: {} <-> {}", e1, e2);
                    }
                }
            }
        }
    }

    /// Convert collider shape to Rapier shape
    fn convert_shape(shape: &ColliderShape) -> SharedShape {
        match shape {
            ColliderShape::Box { half_extents } => {
                SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z)
            }
            ColliderShape::Sphere { radius } => {
                SharedShape::ball(*radius)
            }
            ColliderShape::Capsule { half_height, radius } => {
                SharedShape::capsule_y(*half_height, *radius)
            }
            ColliderShape::Cylinder { half_height, radius } => {
                SharedShape::cylinder(*half_height, *radius)
            }
        }
    }
}
```

---

### **4. Server Integration** (Day 4-5)

**File:** `server/src/physics_tick.rs`

```rust
use engine_physics::PhysicsSystem;
use engine_ecs::World;
use std::time::Duration;

/// Server physics tick
pub struct PhysicsTick {
    physics_system: PhysicsSystem,
    tick_rate: u32,
    accumulator: Duration,
}

impl PhysicsTick {
    pub fn new(tick_rate: u32) -> Self {
        Self {
            physics_system: PhysicsSystem::new(Vec3::new(0.0, -9.81, 0.0)),
            tick_rate,
            accumulator: Duration::ZERO,
        }
    }

    /// Update physics with delta time
    pub fn update(&mut self, world: &mut World, dt: Duration) {
        self.accumulator += dt;

        let tick_duration = Duration::from_secs_f32(1.0 / self.tick_rate as f32);

        // Fixed timestep physics
        while self.accumulator >= tick_duration {
            // Sync ECS -> Physics
            self.physics_system.sync_to_physics(world);

            // Step physics
            let start = std::time::Instant::now();
            self.physics_system.step();
            let elapsed = start.elapsed();

            if elapsed.as_millis() > 5 {
                tracing::warn!("Physics step took {}ms (target: <5ms)", elapsed.as_millis());
            }

            // Sync Physics -> ECS
            self.physics_system.sync_from_physics(world);

            // Process collisions
            self.physics_system.process_collisions(world);

            self.accumulator -= tick_duration;
        }
    }
}
```

**File:** `examples/physics_demo.rs`

```rust
use engine_ecs::prelude::*;
use engine_physics::PhysicsSystem;
use glam::{Vec3, Quat};

fn main() {
    // Create world
    let mut world = World::new();

    // Create ground plane
    let ground = world.spawn();
    world.add_component(ground, Transform {
        position: Vec3::new(0.0, -1.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    });
    world.add_component(ground, Rigidbody::static_body());
    world.add_component(ground, Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Create falling boxes
    for i in 0..100 {
        let entity = world.spawn();
        world.add_component(entity, Transform {
            position: Vec3::new(
                (i % 10) as f32 * 2.0 - 10.0,
                10.0 + (i / 10) as f32 * 2.0,
                0.0,
            ),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        world.add_component(entity, Rigidbody::dynamic(1.0));
        world.add_component(entity, Collider::box_collider(Vec3::ONE));
    }

    // Create physics system
    let mut physics = PhysicsSystem::new(Vec3::new(0.0, -9.81, 0.0));

    // Simulate
    for tick in 0..600 {
        physics.sync_to_physics(&world);

        let start = std::time::Instant::now();
        physics.step();
        let elapsed = start.elapsed();

        physics.sync_from_physics(&mut world);
        physics.process_collisions(&mut world);

        if tick % 60 == 0 {
            println!("Tick {}: {}ms", tick, elapsed.as_millis());
        }
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Rapier physics engine integrated
- [ ] PhysicsWorld manages simulation
- [ ] Rigidbody component (Dynamic/Kinematic/Static)
- [ ] Collider component (Box/Sphere/Capsule/Cylinder)
- [ ] PhysicsSystem syncs ECS <-> Physics
- [ ] Server-authoritative physics tick
- [ ] Collision events processed
- [ ] Physics step < 5ms for 1000 bodies
- [ ] No physics desyncs between clients
- [ ] Example demonstrates physics

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Physics step (1000 bodies) | < 5ms | < 10ms |
| ECS -> Physics sync | < 1ms | < 3ms |
| Physics -> ECS sync | < 1ms | < 3ms |
| Collision detection | < 2ms | < 5ms |
| Add rigidbody | < 0.1ms | < 0.5ms |
| Remove rigidbody | < 0.1ms | < 0.5ms |

---

**Dependencies:** [phase1-ecs-core.md](phase1-ecs-core.md), [phase2-server-tick.md](phase2-server-tick.md)
**Next:** [phase3-audio.md](phase3-audio.md)
