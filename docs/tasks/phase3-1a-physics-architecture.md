# Phase 3.1A: Physics Architecture & Core Implementation

**Status:** 📋 Planning
**Estimated Time:** 5-7 days
**Priority:** CRITICAL (Foundation for all physics)

---

## 🎯 **Objective**

Design and implement a **configuration-driven physics system** that:
- Runs on server, client, or both based on **runtime configuration**
- Reuses the same `PhysicsWorld` code everywhere (#[shared])
- Supports multiple authority modes (server-auth, client-prediction, deterministic, local-only)
- Matches AAA performance: < 5ms for 1000 bodies

---

## 🏗️ **Architecture Philosophy**

### **Core Principle: Configuration Over Compilation**

**❌ BAD (Hardcoded):**
```rust
// Separate implementations - code duplication
#[cfg(feature = "server")]
fn physics_step_server() { /* server code */ }

#[cfg(feature = "client")]
fn physics_step_client() { /* client code */ }
```

**✅ GOOD (Configurable):**
```rust
// Same implementation, runtime decision
#[shared]
fn physics_step(world: &mut PhysicsWorld, dt: f32) {
    match world.config.mode {
        PhysicsMode::ServerAuthoritative => { /* ... */ }
        PhysicsMode::ClientPrediction => { /* ... */ }
        // ... more modes
    }
}
```

### **Inspired By:**

- **Unity**: [NetworkRigidbody](https://docs.unity3d.com/Packages/com.unity.netcode.gameobjects@2.7/manual/components/helper/networkrigidbody.html) with authority mirroring NetworkTransform
- **Unreal**: [Networked Physics modes](https://dev.epicgames.com/documentation/en-us/unreal-engine/networked-physics-overview) (Default, Predictive Interpolation, Resimulation)
- **FishNet**: [Runtime authority switching](https://github.com/FirstGearGames/FishNet/discussions/492)

---

## 📋 **Detailed Implementation Plan**

### **Task 1: Core Configuration Types** (Day 1 - Morning)

**File:** `engine/physics/src/config.rs`

```rust
use engine_math::Vec3;
use serde::{Deserialize, Serialize};

/// Physics execution mode (runtime decision)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsMode {
    /// Server simulates authoritatively, clients interpolate received state
    ///
    /// Best for: MMOs, authoritative gameplay
    /// Network: Server sends full state updates
    /// Performance: Server pays full cost, clients minimal
    ServerAuthoritative,

    /// Client predicts locally, server reconciles
    ///
    /// Best for: Fast-paced games (FPS, racing)
    /// Network: Client sends inputs, server sends corrections
    /// Performance: Both simulate, bandwidth reduced
    ClientPrediction {
        /// Threshold for triggering reconciliation (distance in meters)
        reconciliation_threshold: f32,

        /// Number of frames to keep for rollback
        history_frames: u32,
    },

    /// Both client and server simulate identically (lockstep)
    ///
    /// Best for: RTS, MOBA, fighting games
    /// Network: Only inputs sent, deterministic simulation
    /// Performance: Both simulate, minimal bandwidth
    /// **Requires**: Deterministic math (fixed-point or careful f32)
    Deterministic {
        /// Use fixed-point math (slower but deterministic)
        use_fixed_point: bool,
    },

    /// Local simulation only (singleplayer, editor)
    ///
    /// Best for: Offline games, testing
    /// Network: None
    /// Performance: Full simulation locally
    LocalOnly,

    /// Physics disabled (UI-only games, no gameplay physics)
    Disabled,
}

/// Physics world configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Execution mode
    pub mode: PhysicsMode,

    /// Gravity vector (m/s²)
    /// Default: Vec3::new(0.0, -9.81, 0.0)
    pub gravity: Vec3,

    /// Physics timestep in Hz (60 = 60 updates/second)
    /// Default: 60
    pub timestep_hz: u32,

    /// Maximum sub-steps per frame (prevents spiral of death)
    /// Default: 4
    pub max_substeps: u32,

    /// Enable Continuous Collision Detection (CCD)
    /// Prevents fast-moving objects from tunneling
    /// Default: true
    pub enable_ccd: bool,

    /// Number of solver iterations (higher = more stable, slower)
    /// Default: 8
    pub solver_iterations: u32,

    /// Enable parallel physics (uses rayon)
    /// Only useful for 500+ bodies
    /// Default: true
    pub enable_parallel: bool,

    /// Enable SIMD optimizations
    /// Requires CPU support (detected at runtime)
    /// Default: true
    pub enable_simd: bool,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            mode: PhysicsMode::LocalOnly, // Safe default
            gravity: Vec3::new(0.0, -9.81, 0.0),
            timestep_hz: 60,
            max_substeps: 4,
            enable_ccd: true,
            solver_iterations: 8,
            enable_parallel: true,
            enable_simd: true,
        }
    }
}

impl PhysicsConfig {
    /// Create configuration for server-authoritative mode
    pub fn server_authoritative() -> Self {
        Self {
            mode: PhysicsMode::ServerAuthoritative,
            ..Default::default()
        }
    }

    /// Create configuration for client-side prediction
    pub fn client_prediction(reconciliation_threshold: f32) -> Self {
        Self {
            mode: PhysicsMode::ClientPrediction {
                reconciliation_threshold,
                history_frames: 60, // 1 second at 60Hz
            },
            ..Default::default()
        }
    }

    /// Create configuration for deterministic lockstep
    pub fn deterministic(use_fixed_point: bool) -> Self {
        Self {
            mode: PhysicsMode::Deterministic { use_fixed_point },
            // Deterministic mode needs stricter settings
            enable_parallel: false, // Parallelism breaks determinism
            enable_simd: false,     // SIMD may vary across CPUs
            solver_iterations: 10,  // More iterations for stability
            ..Default::default()
        }
    }

    /// Get timestep in seconds
    pub fn timestep(&self) -> f32 {
        1.0 / self.timestep_hz as f32
    }
}
```

**Tests:** `engine/physics/tests/config_tests.rs`
```rust
#[test]
fn test_physics_config_defaults() {
    let config = PhysicsConfig::default();
    assert_eq!(config.mode, PhysicsMode::LocalOnly);
    assert_eq!(config.timestep(), 1.0 / 60.0);
}

#[test]
fn test_server_authoritative_config() {
    let config = PhysicsConfig::server_authoritative();
    assert!(matches!(config.mode, PhysicsMode::ServerAuthoritative));
}

#[test]
fn test_deterministic_disables_parallelism() {
    let config = PhysicsConfig::deterministic(false);
    assert!(!config.enable_parallel);
    assert!(!config.enable_simd);
}
```

---

### **Task 2: Physics Components** (Day 1 - Afternoon)

**File:** `engine/physics/src/components.rs`

```rust
use engine_core::ecs::Component;
use engine_math::{Vec3, Quat};
use serde::{Deserialize, Serialize};

/// Rigid body component
///
/// Represents a physics object that can be simulated.
/// Attach to an entity along with Transform and Collider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody {
    /// Body type determines how it interacts with forces
    pub body_type: RigidBodyType,

    /// Mass in kilograms (ignored for kinematic/static)
    pub mass: f32,

    /// Linear velocity (m/s)
    pub linear_velocity: Vec3,

    /// Angular velocity (rad/s)
    pub angular_velocity: Vec3,

    /// Linear damping (air resistance)
    /// Range: 0.0 (no damping) to 1.0 (full damping)
    pub linear_damping: f32,

    /// Angular damping (rotational air resistance)
    /// Range: 0.0 to 1.0
    pub angular_damping: f32,

    /// Gravity scale multiplier
    /// 1.0 = normal gravity, 0.0 = no gravity, 2.0 = double gravity
    pub gravity_scale: f32,

    /// Lock translation axes (prevent movement on X/Y/Z)
    pub lock_translation: [bool; 3],

    /// Lock rotation axes (prevent rotation around X/Y/Z)
    pub lock_rotation: [bool; 3],

    /// Enable Continuous Collision Detection for this body
    /// Prevents tunneling for fast-moving objects
    pub ccd_enabled: bool,

    /// Is this body currently sleeping? (optimization)
    /// Sleeping bodies don't simulate until disturbed
    pub is_sleeping: bool,
}

impl Component for RigidBody {}

/// Type of rigid body
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RigidBodyType {
    /// Dynamic: Affected by forces, gravity, collisions
    /// Use for: Player, enemies, physics props
    Dynamic,

    /// Kinematic: Not affected by forces, moved by velocity
    /// Use for: Moving platforms, doors, elevators
    Kinematic,

    /// Static: Never moves, infinite mass
    /// Use for: Walls, floors, terrain
    Static,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            mass: 1.0,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            linear_damping: 0.01,     // Slight air resistance
            angular_damping: 0.05,    // More rotational damping
            gravity_scale: 1.0,
            lock_translation: [false; 3],
            lock_rotation: [false; 3],
            ccd_enabled: false,
            is_sleeping: false,
        }
    }
}

impl RigidBody {
    /// Create a dynamic body with given mass
    pub fn dynamic(mass: f32) -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            mass,
            ..Default::default()
        }
    }

    /// Create a kinematic body
    pub fn kinematic() -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            ..Default::default()
        }
    }

    /// Create a static body
    pub fn static_body() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            is_sleeping: true, // Static bodies always sleep
            ..Default::default()
        }
    }

    /// Lock Y-axis translation (2D platformer physics)
    pub fn lock_2d_platform(mut self) -> Self {
        self.lock_translation[1] = true; // Lock Y
        self.lock_rotation[0] = true;    // Lock X rotation
        self.lock_rotation[2] = true;    // Lock Z rotation
        self
    }

    /// Apply impulse (immediate velocity change)
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.body_type == RigidBodyType::Dynamic {
            self.linear_velocity += impulse / self.mass;
        }
    }

    /// Calculate kinetic energy (for sleeping detection)
    pub fn kinetic_energy(&self) -> f32 {
        let linear_ke = 0.5 * self.mass * self.linear_velocity.length_squared();
        let angular_ke = 0.5 * self.mass * self.angular_velocity.length_squared();
        linear_ke + angular_ke
    }
}

/// Collider shape component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider {
    /// Geometric shape
    pub shape: ColliderShape,

    /// Physics material properties
    pub material: PhysicsMaterial,

    /// Is this a trigger? (no collision response, only events)
    pub is_sensor: bool,

    /// Collision layer (what group am I in?)
    /// Bit mask: bit 0-31 represents layers 0-31
    pub collision_layer: u32,

    /// Collision mask (what do I collide with?)
    /// Bit mask: set bit N to collide with layer N
    pub collision_mask: u32,
}

impl Component for Collider {}

/// Collision shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderShape {
    /// Box collider (half-extents from center)
    /// Use for: Walls, crates, simple objects
    Box { half_extents: Vec3 },

    /// Sphere collider (radius)
    /// Use for: Balls, round objects, simple characters
    Sphere { radius: f32 },

    /// Capsule collider (cylinder with hemispherical ends)
    /// Use for: Character controllers (best for slopes/stairs)
    Capsule {
        half_height: f32, // Half height of cylinder part
        radius: f32,      // Radius of cylinder and spheres
    },

    /// Cylinder collider
    /// Use for: Pillars, trees
    Cylinder {
        half_height: f32,
        radius: f32,
    },

    /// Convex mesh (simplified geometry)
    /// Use for: Complex static objects
    /// **Note**: More expensive than primitives
    ConvexMesh {
        vertices: Vec<Vec3>,
    },

    /// Triangle mesh (concave, static only)
    /// Use for: Terrain, level geometry
    /// **Warning**: Very expensive, static bodies only
    TriangleMesh {
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
    },
}

/// Physics material properties
///
/// Determines friction and bounciness of collisions.
/// See: [Unity Physics Materials](https://docs.unity3d.com/Packages/com.unity.physics@1.0/manual/custom-materials.html)
/// See: [Friction/Restitution Combining](https://www.gamedev.net/tutorials/programming/math-and-physics/combining-material-friction-and-restitution-values-r4227/)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicsMaterial {
    /// Friction coefficient (0.0 = ice, 1.0 = rubber)
    /// Default: 0.5
    pub friction: f32,

    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    /// Default: 0.0
    pub restitution: f32,

    /// Density (kg/m³) - used to calculate mass from volume
    /// Default: 1000.0 (water density)
    pub density: f32,

    /// How to combine friction with other materials
    pub friction_combine: CombineMode,

    /// How to combine restitution with other materials
    pub restitution_combine: CombineMode,
}

/// Material combining mode
///
/// When two materials collide, we need to combine their properties.
/// See: [Godot Proposal](https://github.com/godotengine/godot-proposals/issues/11715)
/// See: [PhysX Material Combining](https://docs.o3de.org/docs/user-guide/interactivity/physics/nvidia-physx/materials/)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombineMode {
    /// Average: (a + b) / 2
    /// Most realistic for friction
    Average,

    /// Minimum: min(a, b)
    /// Use when you want the slipperier material to dominate
    Minimum,

    /// Maximum: max(a, b)
    /// Use when you want the grippier material to dominate
    Maximum,

    /// Multiply: a * b
    /// Use for restitution (bounciness compounds)
    Multiply,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.5,
            restitution: 0.0,
            density: 1000.0, // Water density
            friction_combine: CombineMode::Average,
            restitution_combine: CombineMode::Maximum,
        }
    }
}

impl PhysicsMaterial {
    /// Ice (very low friction)
    pub const ICE: Self = Self {
        friction: 0.05,
        restitution: 0.1,
        density: 920.0, // Ice density
        friction_combine: CombineMode::Minimum,
        restitution_combine: CombineMode::Average,
    };

    /// Rubber (high friction, bouncy)
    pub const RUBBER: Self = Self {
        friction: 0.9,
        restitution: 0.8,
        density: 1200.0,
        friction_combine: CombineMode::Maximum,
        restitution_combine: CombineMode::Maximum,
    };

    /// Metal (medium friction, some bounce)
    pub const METAL: Self = Self {
        friction: 0.4,
        restitution: 0.3,
        density: 7850.0, // Steel density
        friction_combine: CombineMode::Average,
        restitution_combine: CombineMode::Average,
    };

    /// Wood (medium properties)
    pub const WOOD: Self = Self {
        friction: 0.6,
        restitution: 0.2,
        density: 700.0,
        friction_combine: CombineMode::Average,
        restitution_combine: CombineMode::Average,
    };

    /// Combine two materials
    pub fn combine(&self, other: &Self) -> (f32, f32) {
        let friction = self.combine_values(
            self.friction,
            other.friction,
            self.friction_combine,
            other.friction_combine,
        );

        let restitution = self.combine_values(
            self.restitution,
            other.restitution,
            self.restitution_combine,
            other.restitution_combine,
        );

        (friction, restitution)
    }

    fn combine_values(&self, a: f32, b: f32, mode_a: CombineMode, mode_b: CombineMode) -> f32 {
        // Use the mode with higher precedence
        let mode = if mode_b as u8 > mode_a as u8 { mode_b } else { mode_a };

        match mode {
            CombineMode::Average => (a + b) / 2.0,
            CombineMode::Minimum => a.min(b),
            CombineMode::Maximum => a.max(b),
            CombineMode::Multiply => a * b,
        }
    }
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: Vec3::ONE,
            },
            material: PhysicsMaterial::default(),
            is_sensor: false,
            collision_layer: 1,      // Layer 0
            collision_mask: 0xFFFFFFFF, // Collide with all layers
        }
    }
}

impl Collider {
    /// Box collider
    pub fn box_collider(half_extents: Vec3) -> Self {
        Self {
            shape: ColliderShape::Box { half_extents },
            ..Default::default()
        }
    }

    /// Sphere collider
    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere { radius },
            ..Default::default()
        }
    }

    /// Capsule collider (best for character controllers)
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape::Capsule { half_height, radius },
            ..Default::default()
        }
    }

    /// Create a sensor/trigger collider
    pub fn sensor(shape: ColliderShape) -> Self {
        Self {
            shape,
            is_sensor: true,
            ..Default::default()
        }
    }

    /// Set collision layer (what group am I in?)
    pub fn with_layer(mut self, layer: u32) -> Self {
        self.collision_layer = 1 << layer.min(31);
        self
    }

    /// Set collision mask (what do I collide with?)
    pub fn with_mask(mut self, mask: u32) -> Self {
        self.collision_mask = mask;
        self
    }

    /// Check if this collider can collide with another layer
    pub fn can_collide_with(&self, other_layer: u32) -> bool {
        (self.collision_mask & other_layer) != 0
    }
}
```

**Tests:** `engine/physics/tests/component_tests.rs`
```rust
#[test]
fn test_rigidbody_default() {
    let rb = RigidBody::default();
    assert_eq!(rb.body_type, RigidBodyType::Dynamic);
    assert_eq!(rb.mass, 1.0);
}

#[test]
fn test_apply_impulse() {
    let mut rb = RigidBody::dynamic(2.0);
    rb.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
    assert_eq!(rb.linear_velocity.x, 5.0); // 10 / 2
}

#[test]
fn test_material_combine() {
    let ice = PhysicsMaterial::ICE;
    let rubber = PhysicsMaterial::RUBBER;

    let (friction, restitution) = ice.combine(&rubber);

    // Friction: min(0.05, 0.9) = 0.05 (ice mode is Minimum)
    assert!((friction - 0.05).abs() < 0.01);
}

#[test]
fn test_collision_layers() {
    let collider = Collider::sphere(1.0)
        .with_layer(2)  // I'm on layer 2
        .with_mask(0b101); // I collide with layers 0 and 2

    assert!(collider.can_collide_with(1 << 0)); // Layer 0: yes
    assert!(!collider.can_collide_with(1 << 1)); // Layer 1: no
    assert!(collider.can_collide_with(1 << 2)); // Layer 2: yes
}
```

---

### **Task 3: PhysicsWorld Core** (Day 2)

**File:** `engine/physics/src/world.rs`

```rust
use rapier3d::prelude::*;
use engine_math::{Vec3, Quat};
use std::collections::HashMap;
use super::config::{PhysicsConfig, PhysicsMode};
use super::components::{RigidBody, Collider, RigidBodyType};

/// Physics world - wraps Rapier with configuration
///
/// This is `#[shared]` - same code runs on client/server/singleplayer.
/// Behavior changes based on `config.mode`.
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

    /// Entity <-> RigidBodyHandle mapping
    entity_to_body: HashMap<u64, RigidBodyHandle>,
    body_to_entity: HashMap<RigidBodyHandle, u64>,

    /// Collision events from last step
    collision_events: Vec<CollisionEvent>,

    /// Contact force events from last step
    contact_force_events: Vec<ContactForceEvent>,

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

        // Configure Rapier based on our config
        if let PhysicsMode::Deterministic { .. } = config.mode {
            // Deterministic mode needs special settings
            // Disable features that break determinism
            integration_params.max_ccd_substeps = config.max_substeps as usize;
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
            collision_events: Vec::new(),
            contact_force_events: Vec::new(),
            accumulator: 0.0,
            frame_count: 0,
            config,
        }
    }

    /// Step physics simulation with delta time
    ///
    /// Uses fixed timestep internally for stability.
    /// See: [Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/)
    pub fn step(&mut self, dt: f32) {
        match self.config.mode {
            PhysicsMode::Disabled => return,
            _ => {}
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
                "Physics spiral of death detected! {} substeps exceeded max {}",
                steps,
                self.config.max_substeps
            );
        }
    }

    /// Internal step (one fixed timestep)
    fn step_internal(&mut self, dt: f32) {
        self.frame_count += 1;

        // Update integration parameters if timestep changed
        self.integration_params.dt = dt;

        // Event collector
        let event_handler = ChannelEventCollector::new();

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
            Some(&mut self.query_pipeline), // For spatial queries
            &(),                             // No physics hooks
            &event_handler,
        );

        // Collect collision events
        self.collision_events.clear();
        self.contact_force_events.clear();

        while let Ok(event) = event_handler.collision_events.try_recv() {
            self.collision_events.push(event);
        }

        while let Ok(event) = event_handler.contact_force_events.try_recv() {
            self.contact_force_events.push(event);
        }

        tracing::trace!(
            frame = self.frame_count,
            bodies = self.rigid_body_set.len(),
            colliders = self.collider_set.len(),
            collisions = self.collision_events.len(),
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

        let mut rigid_body = RigidBodyBuilder::new(rapier_type)
            .translation(vector![position.x, position.y, position.z])
            .rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w])
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

        tracing::debug!(entity = entity_id, "Added rigidbody");

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

        // Build collider
        let mut collider = ColliderBuilder::new(shape)
            .friction(collider_component.material.friction)
            .restitution(collider_component.material.restitution)
            .density(collider_component.material.density)
            .sensor(collider_component.is_sensor)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(collider_component.collision_layer),
                Group::from_bits_truncate(collider_component.collision_mask),
            ))
            .build();

        let collider_handle = self.collider_set.insert_with_parent(
            collider,
            *rb_handle,
            &mut self.rigid_body_set,
        );

        tracing::debug!(entity = entity_id, "Added collider");

        Some(collider_handle)
    }

    /// Remove rigidbody and all associated colliders
    pub fn remove_rigidbody(&mut self, entity_id: u64) {
        if let Some(handle) = self.entity_to_body.remove(&entity_id) {
            self.rigid_body_set.remove(
                handle,
                &mut self.islands,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true, // Remove attached colliders
            );
            self.body_to_entity.remove(&handle);

            tracing::debug!(entity = entity_id, "Removed rigidbody");
        }
    }

    /// Get current transform of entity
    pub fn get_transform(&self, entity_id: u64) -> Option<(Vec3, Quat)> {
        let handle = self.entity_to_body.get(&entity_id)?;
        let body = self.rigid_body_set.get(*handle)?;

        let pos = body.translation();
        let rot = body.rotation();

        Some((
            Vec3::new(pos.x, pos.y, pos.z),
            Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w),
        ))
    }

    /// Set transform of entity
    pub fn set_transform(&mut self, entity_id: u64, position: Vec3, rotation: Quat) {
        if let Some(handle) = self.entity_to_body.get(&entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.set_translation(vector![position.x, position.y, position.z], true);
                body.set_rotation(
                    UnitQuaternion::from_quaternion(Quaternion::new(
                        rotation.w,
                        rotation.x,
                        rotation.y,
                        rotation.z,
                    )),
                    true,
                );
            }
        }
    }

    /// Convert collider shape component to Rapier shape
    fn convert_shape(shape: &super::components::ColliderShape) -> SharedShape {
        use super::components::ColliderShape;

        match shape {
            ColliderShape::Box { half_extents } => {
                SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z)
            }
            ColliderShape::Sphere { radius } => SharedShape::ball(*radius),
            ColliderShape::Capsule {
                half_height,
                radius,
            } => SharedShape::capsule_y(*half_height, *radius),
            ColliderShape::Cylinder {
                half_height,
                radius,
            } => SharedShape::cylinder(*half_height, *radius),
            ColliderShape::ConvexMesh { vertices } => {
                let points: Vec<Point<Real>> = vertices
                    .iter()
                    .map(|v| Point::new(v.x, v.y, v.z))
                    .collect();
                SharedShape::convex_hull(&points).unwrap_or_else(|| SharedShape::ball(0.5))
            }
            ColliderShape::TriangleMesh { vertices, indices } => {
                let points: Vec<Point<Real>> = vertices
                    .iter()
                    .map(|v| Point::new(v.x, v.y, v.z))
                    .collect();
                SharedShape::trimesh(points, indices.clone())
            }
        }
    }

    /// Get collision events from last step
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    /// Get contact force events from last step
    pub fn contact_force_events(&self) -> &[ContactForceEvent] {
        &self.contact_force_events
    }

    /// Get entity ID from rigidbody handle
    pub fn body_to_entity(&self, handle: RigidBodyHandle) -> Option<u64> {
        self.body_to_entity.get(&handle).copied()
    }

    /// Raycast (find first hit)
    pub fn raycast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        filter: QueryFilter,
    ) -> Option<(u64, f32)> {
        let ray = Ray::new(
            point![origin.x, origin.y, origin.z],
            vector![direction.x, direction.y, direction.z],
        );

        self.query_pipeline
            .cast_ray(
                &self.rigid_body_set,
                &self.collider_set,
                &ray,
                max_distance,
                true, // solid
                filter,
            )
            .and_then(|(handle, toi)| {
                let collider = self.collider_set.get(handle)?;
                let rb_handle = collider.parent()?;
                let entity_id = self.body_to_entity.get(&rb_handle)?;
                Some((*entity_id, toi))
            })
    }

    /// Raycast all (find all hits)
    pub fn raycast_all(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        filter: QueryFilter,
    ) -> Vec<(u64, f32)> {
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
            true,
            filter,
            |handle, intersection| {
                if let Some(collider) = self.collider_set.get(handle) {
                    if let Some(rb_handle) = collider.parent() {
                        if let Some(&entity_id) = self.body_to_entity.get(&rb_handle) {
                            hits.push((entity_id, intersection.time_of_impact));
                        }
                    }
                }
                true // Continue iterating
            },
        );

        hits
    }
}

/// Event collector using channels (thread-safe)
struct ChannelEventCollector {
    collision_events: crossbeam_channel::Sender<CollisionEvent>,
    contact_force_events: crossbeam_channel::Sender<ContactForceEvent>,
    collision_recv: crossbeam_channel::Receiver<CollisionEvent>,
    contact_recv: crossbeam_channel::Receiver<ContactForceEvent>,
}

impl ChannelEventCollector {
    fn new() -> Self {
        let (collision_send, collision_recv) = crossbeam_channel::unbounded();
        let (contact_send, contact_recv) = crossbeam_channel::unbounded();

        Self {
            collision_events: collision_send,
            contact_force_events: contact_send,
            collision_recv,
            contact_recv,
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

### **Task 4: ECS Integration System** (Day 3)

**File:** `engine/physics/src/systems/physics_system.rs`

```rust
use engine_core::ecs::{World, Query};
use engine_math::Transform;
use super::super::{PhysicsWorld, RigidBody, Collider};

/// Physics system - synchronizes ECS with PhysicsWorld
///
/// This is `#[shared]` - runs on both client and server.
/// The PhysicsWorld config determines what actually happens.
#[shared]
pub struct PhysicsSystem {
    physics_world: PhysicsWorld,
}

impl PhysicsSystem {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            physics_world: PhysicsWorld::new(config),
        }
    }

    /// Sync ECS components -> Physics world
    pub fn sync_to_physics(&mut self, world: &World) {
        // Add new rigid bodies that don't exist in physics world yet
        for (entity, (transform, rigidbody)) in world
            .query::<(&Transform, &RigidBody)>()
            .iter()
        {
            let entity_id = entity.id();

            // Check if already exists
            if self.physics_world.entity_to_body.contains_key(&entity_id) {
                continue;
            }

            // Add to physics world
            let rb_handle = self.physics_world.add_rigidbody(
                entity_id,
                rigidbody,
                transform.translation,
                transform.rotation,
            );

            // Add collider if entity has one
            if let Some(collider) = world.get::<Collider>(entity) {
                self.physics_world.add_collider(entity_id, collider);
            }
        }

        // TODO: Remove deleted entities
    }

    /// Step physics simulation
    pub fn step(&mut self, dt: f32) {
        self.physics_world.step(dt);
    }

    /// Sync Physics world -> ECS components
    pub fn sync_from_physics(&mut self, world: &mut World) {
        for (entity, transform) in world.query::<&mut Transform>().iter_mut() {
            if let Some((position, rotation)) = self.physics_world.get_transform(entity.id()) {
                transform.translation = position;
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
                        // TODO: Emit event to ECS event system
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

    /// Access physics world for queries (raycasts, etc.)
    pub fn physics_world(&self) -> &PhysicsWorld {
        &self.physics_world
    }

    pub fn physics_world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.physics_world
    }
}
```

---

### **Task 5: Benchmarks** (Day 4)

Create comprehensive benchmarks comparing to Unity/Unreal performance targets.

**File:** `engine/physics/benches/world_benchmarks.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use engine_physics::{PhysicsWorld, PhysicsConfig, RigidBody, Collider};
use engine_math::{Vec3, Quat};

/// Benchmark: Cube tower (stress test for stacking stability)
///
/// Unity reference: ~3-5ms for 1000 cubes
/// Unreal reference: ~4-7ms for 1000 cubes (Chaos)
/// Target: < 5ms
fn bench_cube_tower(c: &mut Criterion) {
    let mut group = c.benchmark_group("cube_tower");

    for size in [100, 500, 1000, 2000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let config = PhysicsConfig::server_authoritative();
            let mut world = PhysicsWorld::new(config);

            // Create ground plane
            let ground_id = 0;
            world.add_rigidbody(
                ground_id,
                &RigidBody::static_body(),
                Vec3::new(0.0, -0.5, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(
                ground_id,
                &Collider::box_collider(Vec3::new(50.0, 0.5, 50.0)),
            );

            // Stack cubes
            for i in 0..size {
                let entity_id = i + 1;
                let y = 1.0 + i as f32 * 2.1;

                world.add_rigidbody(
                    entity_id,
                    &RigidBody::dynamic(1.0),
                    Vec3::new(0.0, y, 0.0),
                    Quat::IDENTITY,
                );
                world.add_collider(entity_id, &Collider::box_collider(Vec3::ONE));
            }

            b.iter(|| {
                world.step(1.0 / 60.0);
            });
        });
    }

    group.finish();
}

/// Benchmark: Sphere drop (collision detection stress test)
///
/// Reference: [PassMark Physics Benchmark](https://www.passmark.com/products/performancetest/pt_advphys.php)
/// Target: < 3ms for 1000 spheres
fn bench_sphere_drop(c: &mut Criterion) {
    let mut group = c.benchmark_group("sphere_drop");

    for size in [100, 500, 1000, 2000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let config = PhysicsConfig::server_authoritative();
            let mut world = PhysicsWorld::new(config);

            // Create ground
            let ground_id = 0;
            world.add_rigidbody(
                ground_id,
                &RigidBody::static_body(),
                Vec3::ZERO,
                Quat::IDENTITY,
            );
            world.add_collider(
                ground_id,
                &Collider::box_collider(Vec3::new(50.0, 0.5, 50.0)),
            );

            // Drop spheres in grid pattern
            let grid_size = (size as f32).sqrt() as usize;
            for x in 0..grid_size {
                for z in 0..grid_size {
                    let entity_id = x * grid_size + z + 1;
                    let pos = Vec3::new(x as f32 * 2.0, 10.0, z as f32 * 2.0);

                    world.add_rigidbody(
                        entity_id,
                        &RigidBody::dynamic(1.0),
                        pos,
                        Quat::IDENTITY,
                    );
                    world.add_collider(entity_id, &Collider::sphere(0.5));
                }
            }

            b.iter(|| {
                world.step(1.0 / 60.0);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_cube_tower, bench_sphere_drop);
criterion_main!(benches);
```

---

### **Task 6: Integration Tests** (Day 5)

**File:** `engine/physics/tests/integration_tests.rs`

```rust
use engine_physics::*;
use engine_math::{Vec3, Quat};

#[test]
fn test_falling_box() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create falling box
    let box_id = 1;
    world.add_rigidbody(
        box_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Simulate 1 second
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    // Box should have fallen due to gravity
    let (pos, _) = world.get_transform(box_id).unwrap();
    assert!(pos.y < 10.0, "Box should have fallen");
}

#[test]
fn test_collision_events() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Ground
    let ground_id = 0;
    world.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::ZERO,
        Quat::IDENTITY,
    );
    world.add_collider(
        ground_id,
        &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)),
    );

    // Falling box
    let box_id = 1;
    world.add_rigidbody(
        box_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Simulate until collision
    let mut collision_detected = false;
    for _ in 0..120 {
        world.step(1.0 / 60.0);

        if !world.collision_events().is_empty() {
            collision_detected = true;
            break;
        }
    }

    assert!(collision_detected, "Box should collide with ground");
}

#[test]
fn test_raycast() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create box at origin
    let box_id = 1;
    world.add_rigidbody(
        box_id,
        &RigidBody::static_body(),
        Vec3::ZERO,
        Quat::IDENTITY,
    );
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Step once to update query pipeline
    world.step(1.0 / 60.0);

    // Raycast should hit
    let hit = world.raycast(
        Vec3::new(0.0, 5.0, 0.0),  // Origin above box
        Vec3::new(0.0, -1.0, 0.0), // Downward direction
        10.0,                      // Max distance
        QueryFilter::default(),
    );

    assert!(hit.is_some(), "Raycast should hit box");
    let (entity_id, _distance) = hit.unwrap();
    assert_eq!(entity_id, box_id);
}

#[test]
fn test_physics_mode_config() {
    // Server-authoritative mode
    let server_config = PhysicsConfig::server_authoritative();
    assert!(matches!(
        server_config.mode,
        PhysicsMode::ServerAuthoritative
    ));

    // Client prediction mode
    let client_config = PhysicsConfig::client_prediction(0.1);
    assert!(matches!(
        client_config.mode,
        PhysicsMode::ClientPrediction { .. }
    ));

    // Deterministic mode
    let det_config = PhysicsConfig::deterministic(false);
    assert!(matches!(
        det_config.mode,
        PhysicsMode::Deterministic { .. }
    ));
    assert!(!det_config.enable_parallel); // Determinism requires no parallelism
}
```

---

## ✅ **Acceptance Criteria**

### **Functionality**
- [ ] PhysicsConfig supports all modes (ServerAuth, ClientPrediction, Deterministic, LocalOnly, Disabled)
- [ ] PhysicsWorld wraps Rapier correctly
- [ ] RigidBody and Collider components work
- [ ] Collision events are collected
- [ ] Raycasting works
- [ ] ECS integration syncs both ways

### **Performance** (Benchmarks vs AAA Targets)
- [ ] 1000 dynamic cubes: < 5ms/step (vs Unity 3-5ms, Unreal 4-7ms)
- [ ] 1000 spheres: < 3ms/step
- [ ] 100 raycasts: < 0.5ms

### **Tests**
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Config tests verify mode switching
- [ ] Collision detection verified
- [ ] Raycasting verified

### **Code Quality**
- [ ] All public APIs documented
- [ ] TDD approach followed
- [ ] No println!/unwrap() in production code
- [ ] Structured logging (tracing) used

---

## 📚 **References**

### **Physics Engine Design**
- [Baraff's Rigid Body Dynamics](https://www.cs.cmu.edu/~baraff/sigcourse/notesd1.pdf) - Mathematical foundations
- [Video Game Physics Tutorial](https://www.toptal.com/game/video-game-physics-part-i-an-introduction-to-rigid-body-dynamics) - Implementation guide
- [Rapier Physics Engine](https://rapier.rs/) - Our chosen engine

### **Numerical Integration**
- [Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/) - Fixed timestep integration
- [Integration Basics](https://gafferongames.com/post/integration_basics/) - Euler vs Verlet

### **Networked Physics**
- [Unity NetworkRigidbody](https://docs.unity3d.com/Packages/com.unity.netcode.gameobjects@2.7/manual/components/helper/networkrigidbody.html)
- [Unreal Networked Physics](https://dev.epicgames.com/documentation/en-us/unreal-engine/networked-physics-overview)
- [Client-Side Prediction](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html)

### **Deterministic Physics**
- [Floating Point Determinism](https://gafferongames.com/post/floating_point_determinism/)
- [Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/)

### **Benchmarking**
- [PassMark Physics Benchmark](https://www.passmark.com/products/performancetest/pt_advphys.php)
- [SimBenchmark](https://leggedrobotics.github.io/SimBenchmark/) - Robotics physics comparison

---

**Next Task:** [phase3-1b-raycasting-triggers.md](phase3-1b-raycasting-triggers.md)

