//! Physics state snapshot system for agentic debugging
//!
//! Captures complete physics state for a single frame, including:
//! - Entity transforms, velocities, forces, masses
//! - Collider shapes and materials
//! - Constraint states and applied impulses
//! - Solver islands and partitioning
//! - **A.0.5**: Contact manifolds with detailed collision data (contact points, normals, impulses)
//! - **A.0.5**: Broadphase pairs (spatial partitioning collision candidates)
//!
//! # A.0.5 - Rapier Solver Internals Export
//!
//! This implementation successfully extracts internal solver data from Rapier 0.18's public API:
//!
//! ## What We Extract:
//!
//! 1. **Contact Manifolds** - Complete narrow-phase collision data:
//!    - Contact normal (world-space)
//!    - All contact points with positions, penetration depths
//!    - Per-point friction and restitution coefficients
//!    - Total impulse magnitude applied by solver
//!    - Relative dominance (affects solver resolution order)
//!
//! 2. **Broadphase Pairs** - Spatial partitioning data:
//!    - Entity pairs that are spatially close (AABB proximity)
//!    - Distance between AABBs
//!    - Whether pair is in narrowphase processing
//!
//! 3. **Joint Entity Mapping** - Constraint details:
//!    - Both entity IDs connected by joint (via `ImpulseJoint.body1/body2`)
//!    - Applied impulse magnitude
//!    - Joint type classification
//!
//! ## API Used:
//!
//! - `NarrowPhase::contact_pairs()` - Iterates all contact pairs
//! - `ContactPair::manifolds` - Access contact manifolds (public field)
//! - `ContactManifoldData::solver_contacts` - Contact points used by solver
//! - `ImpulseJoint::body1/body2` - Public body handle fields
//! - `ImpulseJoint::impulses` - Public impulse vector field
//!
//! ## Known Limitations:
//!
//! These features are NOT available in Rapier 0.18's public API:
//!
//! 1. **Island Iteration** - `IslandManager` doesn't expose per-island iteration
//!    - Current workaround: Group active/sleeping bodies into pseudo-islands
//!    - Missing: Actual island IDs, per-island solver iteration counts
//!
//! 2. **Solver Residual** - Constraint error after solving not exposed
//!    - Would require accessing internal solver state
//!
//! 3. **True Broadphase Pairs** - `BroadPhase` doesn't expose pair iteration
//!    - Current workaround: Use narrowphase pairs as proxy
//!    - Missing: Pairs that passed broadphase but failed narrowphase
//!
//! 4. **Per-Island Convergence** - Can't determine if individual islands converged
//!    - Would require internal solver access
//!
//! ## Future Improvements:
//!
//! If Rapier exposes these APIs in future versions, we can add:
//! - True island iteration with per-island solver stats
//! - Constraint residual tracking for debugging instability
//! - Complete broadphase pair listing (including rejected pairs)
//! - Per-island convergence metrics
//!
//! Snapshots are fully serializable to JSON/JSONL for AI agent consumption.

use engine_math::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Complete physics state snapshot for a single frame
///
/// Contains all data needed to reconstruct or analyze physics simulation state.
/// Designed to be serialized to JSONL/SQLite for AI agent debugging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicsDebugSnapshot {
    /// Frame number
    pub frame: u64,

    /// Simulation timestamp (seconds)
    pub timestamp: f64,

    /// All entity states
    pub entities: Vec<EntityState>,

    /// All collider states
    pub colliders: Vec<ColliderState>,

    /// All constraint/joint states
    pub constraints: Vec<ConstraintState>,

    /// Solver island partitioning
    pub islands: Vec<IslandState>,

    /// Contact manifolds (collision details with contact points, normals, impulses)
    pub contact_manifolds: Vec<ContactManifoldState>,

    /// Broadphase pairs (spatial partitioning collision candidates)
    pub broadphase_pairs: Vec<BroadphasePairState>,

    /// Global physics parameters
    pub config: PhysicsConfigSnapshot,
}

/// Single entity's physics state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityState {
    /// Entity ID
    pub id: u64,

    /// Position (world space)
    pub position: Vec3,

    /// Rotation (world space)
    pub rotation: Quat,

    /// Linear velocity
    pub linear_velocity: Vec3,

    /// Angular velocity
    pub angular_velocity: Vec3,

    /// Total forces applied this frame
    pub forces: Vec3,

    /// Total torques applied this frame
    pub torques: Vec3,

    /// Mass (kg)
    pub mass: f32,

    /// Linear damping coefficient
    pub linear_damping: f32,

    /// Angular damping coefficient
    pub angular_damping: f32,

    /// Gravity scale multiplier
    pub gravity_scale: f32,

    /// Is entity sleeping?
    pub sleeping: bool,

    /// Is entity static (immovable)?
    pub is_static: bool,

    /// Is entity kinematic (velocity-driven, no forces)?
    pub is_kinematic: bool,

    /// Can entity sleep?
    pub can_sleep: bool,

    /// CCD (Continuous Collision Detection) enabled?
    pub ccd_enabled: bool,
}

impl EntityState {
    /// Compute kinetic energy (for debugging/analysis)
    pub fn kinetic_energy(&self) -> f32 {
        let linear_ke = 0.5 * self.mass * self.linear_velocity.length_squared();
        // Note: Angular KE would need inertia tensor, using approximation
        let angular_ke = 0.5 * self.mass * self.angular_velocity.length_squared();
        linear_ke + angular_ke
    }

    /// Is entity moving significantly?
    pub fn is_moving(&self, velocity_threshold: f32) -> bool {
        self.linear_velocity.length() > velocity_threshold
            || self.angular_velocity.length() > velocity_threshold
    }

    /// Check if position/velocity are finite (not NaN/Inf)
    pub fn is_valid(&self) -> bool {
        self.position.is_finite()
            && self.rotation.is_finite()
            && self.linear_velocity.is_finite()
            && self.angular_velocity.is_finite()
            && self.forces.is_finite()
            && self.torques.is_finite()
            && self.mass.is_finite()
    }
}

/// Collider shape and properties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColliderState {
    /// Entity this collider is attached to
    pub entity_id: u64,

    /// Collider shape type
    pub shape_type: ShapeType,

    /// Shape-specific parameters
    pub shape_params: ShapeParams,

    /// Axis-Aligned Bounding Box (world space)
    pub aabb: AABB,

    /// Physics material properties
    pub material: MaterialState,

    /// Is this a sensor/trigger (no collision response)?
    pub is_sensor: bool,

    /// Collision groups/layers (bitfield)
    pub collision_groups: u32,

    /// Collision mask (bitfield)
    pub collision_mask: u32,
}

/// Collider shape type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShapeType {
    /// Box/cuboid
    Box,
    /// Sphere
    Sphere,
    /// Capsule
    Capsule,
    /// Cylinder
    Cylinder,
    /// Convex hull
    ConvexHull,
    /// Triangle mesh (static geometry)
    TriMesh,
}

/// Shape-specific parameters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ShapeParams {
    /// Box: half-extents (x, y, z)
    Box { half_extents: Vec3 },

    /// Sphere: radius
    Sphere { radius: f32 },

    /// Capsule: half-height (along Y), radius
    Capsule { half_height: f32, radius: f32 },

    /// Cylinder: half-height (along Y), radius
    Cylinder { half_height: f32, radius: f32 },

    /// Convex hull: number of vertices
    ConvexHull { vertex_count: usize },

    /// Triangle mesh: number of triangles
    TriMesh { triangle_count: usize },
}

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AABB {
    /// Minimum corner
    pub min: Vec3,
    /// Maximum corner
    pub max: Vec3,
}

impl AABB {
    /// Compute center of AABB
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Compute half-extents
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Compute volume
    pub fn volume(&self) -> f32 {
        let size = self.max - self.min;
        size.x * size.y * size.z
    }
}

/// Physics material properties
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MaterialState {
    /// Friction coefficient (0.0 = frictionless, 1.0 = high friction)
    pub friction: f32,

    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,

    /// Density (kg/m³)
    pub density: f32,
}

/// Joint/constraint state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintState {
    /// Unique constraint ID
    pub id: u64,

    /// First entity
    pub entity_a: u64,

    /// Second entity
    pub entity_b: u64,

    /// Constraint type
    pub constraint_type: ConstraintType,

    /// Current constraint error (position/angle deviation)
    pub current_error: f32,

    /// Impulse applied by solver this frame
    pub applied_impulse: f32,

    /// Is constraint broken?
    pub broken: bool,

    /// Breaking force threshold
    pub break_force: Option<f32>,

    /// Is constraint enabled?
    pub enabled: bool,
}

/// Constraint/joint type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintType {
    /// Fixed joint (all DOF locked)
    Fixed,

    /// Revolute joint (1 rotational DOF)
    Revolute {
        /// Current angle (radians)
        angle: i32, // Stored as fixed-point for JSON compatibility
        /// Has angle limits?
        has_limits: bool,
    },

    /// Prismatic joint (1 translational DOF)
    Prismatic {
        /// Current translation
        translation: i32, // Fixed-point
        /// Has distance limits?
        has_limits: bool,
    },

    /// Spherical joint (3 rotational DOF)
    Spherical,

    /// Generic 6-DOF joint
    Generic { locked_axes: u8 }, // Bitfield
}

/// Solver island (group of interacting bodies)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IslandState {
    /// Island ID
    pub id: usize,

    /// Entities in this island
    pub entities: Vec<u64>,

    /// Is entire island sleeping?
    pub sleeping: bool,

    /// Solver iterations used
    pub iterations: u32,

    /// Final constraint residual (error)
    pub residual: f32,

    /// Did solver converge?
    pub converged: bool,
}

/// Global physics configuration snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicsConfigSnapshot {
    /// Gravity vector
    pub gravity: Vec3,

    /// Timestep (seconds)
    pub timestep: f32,

    /// Solver iterations per step
    pub solver_iterations: u32,

    /// Velocity threshold for sleeping
    pub sleep_threshold: f32,

    /// CCD enabled globally?
    pub ccd_enabled: bool,
}

/// Contact manifold state (collision details from narrow-phase)
///
/// Represents a contact manifold between two colliders, containing
/// all contact points, normals, and solver-applied impulses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactManifoldState {
    /// First entity involved
    pub entity_a: u64,

    /// Second entity involved
    pub entity_b: u64,

    /// Contact normal (world space)
    pub normal: Vec3,

    /// All contact points in this manifold
    pub contact_points: Vec<ContactPointState>,

    /// Total impulse magnitude applied by this manifold
    pub total_impulse_magnitude: f32,

    /// Has any active contact?
    pub has_active_contact: bool,

    /// Relative dominance of bodies (affects solver resolution order)
    pub relative_dominance: i16,
}

/// Single contact point in a manifold
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactPointState {
    /// Contact point position (world space)
    pub point: Vec3,

    /// Penetration depth (negative = penetrating, positive = separated)
    pub distance: f32,

    /// Effective friction coefficient at this point
    pub friction: f32,

    /// Effective restitution coefficient at this point
    pub restitution: f32,
}

/// Broadphase collision pair (spatial partitioning candidates)
///
/// Represents pairs of colliders that are spatially close and may collide.
/// The narrowphase will then perform precise collision detection on these pairs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BroadphasePairState {
    /// First entity involved
    pub entity_a: u64,

    /// Second entity involved
    pub entity_b: u64,

    /// Distance between AABBs (approximate)
    pub aabb_distance: f32,

    /// Is this pair currently in narrowphase processing?
    pub in_narrowphase: bool,
}

impl PhysicsDebugSnapshot {
    /// Create an empty snapshot
    pub fn new(frame: u64, timestamp: f64) -> Self {
        Self {
            frame,
            timestamp,
            entities: Vec::new(),
            colliders: Vec::new(),
            constraints: Vec::new(),
            islands: Vec::new(),
            contact_manifolds: Vec::new(),
            broadphase_pairs: Vec::new(),
            config: PhysicsConfigSnapshot {
                gravity: Vec3::new(0.0, -9.81, 0.0),
                timestep: 1.0 / 60.0,
                solver_iterations: 4,
                sleep_threshold: 0.01,
                ccd_enabled: false,
            },
        }
    }

    /// Get entity by ID
    pub fn get_entity(&self, entity_id: u64) -> Option<&EntityState> {
        self.entities.iter().find(|e| e.id == entity_id)
    }

    /// Get colliders for entity
    pub fn get_entity_colliders(&self, entity_id: u64) -> Vec<&ColliderState> {
        self.colliders.iter().filter(|c| c.entity_id == entity_id).collect()
    }

    /// Get constraints involving entity
    pub fn get_entity_constraints(&self, entity_id: u64) -> Vec<&ConstraintState> {
        self.constraints
            .iter()
            .filter(|c| c.entity_a == entity_id || c.entity_b == entity_id)
            .collect()
    }

    /// Count total entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Count active (non-sleeping) entities
    pub fn active_entity_count(&self) -> usize {
        self.entities.iter().filter(|e| !e.sleeping).count()
    }

    /// Count sleeping entities
    pub fn sleeping_entity_count(&self) -> usize {
        self.entities.iter().filter(|e| e.sleeping).count()
    }

    /// Compute total kinetic energy
    pub fn total_kinetic_energy(&self) -> f32 {
        self.entities.iter().map(|e| e.kinetic_energy()).sum()
    }

    /// Check if all entities have valid (finite) state
    pub fn is_valid(&self) -> bool {
        self.entities.iter().all(|e| e.is_valid())
    }

    /// Find entities with velocity above threshold
    pub fn find_high_velocity_entities(&self, threshold: f32) -> Vec<u64> {
        self.entities
            .iter()
            .filter(|e| e.linear_velocity.length() > threshold)
            .map(|e| e.id)
            .collect()
    }

    /// Find broken constraints
    pub fn find_broken_constraints(&self) -> Vec<u64> {
        self.constraints.iter().filter(|c| c.broken).map(|c| c.id).collect()
    }

    /// Find islands that didn't converge
    pub fn find_unconverged_islands(&self) -> Vec<usize> {
        self.islands.iter().filter(|i| !i.converged).map(|i| i.id).collect()
    }

    /// Get contact manifolds involving entity
    pub fn get_entity_contact_manifolds(&self, entity_id: u64) -> Vec<&ContactManifoldState> {
        self.contact_manifolds
            .iter()
            .filter(|m| m.entity_a == entity_id || m.entity_b == entity_id)
            .collect()
    }

    /// Count total contact points across all manifolds
    pub fn total_contact_points(&self) -> usize {
        self.contact_manifolds.iter().map(|m| m.contact_points.len()).sum()
    }

    /// Find high-impulse contact manifolds (above threshold)
    pub fn find_high_impulse_contacts(&self, threshold: f32) -> Vec<(u64, u64, f32)> {
        self.contact_manifolds
            .iter()
            .filter(|m| m.total_impulse_magnitude > threshold)
            .map(|m| (m.entity_a, m.entity_b, m.total_impulse_magnitude))
            .collect()
    }

    /// Get broadphase pairs involving entity
    pub fn get_entity_broadphase_pairs(&self, entity_id: u64) -> Vec<&BroadphasePairState> {
        self.broadphase_pairs
            .iter()
            .filter(|p| p.entity_a == entity_id || p.entity_b == entity_id)
            .collect()
    }

    /// Compute state hash for determinism validation
    pub fn compute_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash frame and entity count
        self.frame.hash(&mut hasher);
        self.entities.len().hash(&mut hasher);

        // Hash each entity's core state
        for entity in &self.entities {
            entity.id.hash(&mut hasher);
            // Hash position/rotation as fixed-point to avoid float precision issues
            hash_vec3(&mut hasher, entity.position);
            hash_quat(&mut hasher, entity.rotation);
            hash_vec3(&mut hasher, entity.linear_velocity);
            hash_vec3(&mut hasher, entity.angular_velocity);
        }

        hasher.finish()
    }
}

// Helper functions for deterministic hashing of floats
fn hash_vec3<H: std::hash::Hasher>(hasher: &mut H, v: Vec3) {
    use std::hash::Hash;
    // Convert to fixed-point (4 decimal places)
    ((v.x * 10000.0) as i64).hash(hasher);
    ((v.y * 10000.0) as i64).hash(hasher);
    ((v.z * 10000.0) as i64).hash(hasher);
}

fn hash_quat<H: std::hash::Hasher>(hasher: &mut H, q: Quat) {
    use std::hash::Hash;
    ((q.x * 10000.0) as i64).hash(hasher);
    ((q.y * 10000.0) as i64).hash(hasher);
    ((q.z * 10000.0) as i64).hash(hasher);
    ((q.w * 10000.0) as i64).hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = PhysicsDebugSnapshot::new(100, 1.5);
        assert_eq!(snapshot.frame, 100);
        assert_eq!(snapshot.timestamp, 1.5);
        assert_eq!(snapshot.entity_count(), 0);
    }

    #[test]
    fn test_entity_state_validity() {
        let mut entity = EntityState {
            id: 1,
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(10.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        };

        assert!(entity.is_valid());
        assert!(entity.is_moving(5.0));
        assert!(!entity.is_moving(15.0));

        // Test invalid state (NaN)
        entity.position = Vec3::new(f32::NAN, 0.0, 0.0);
        assert!(!entity.is_valid());
    }

    #[test]
    fn test_entity_kinetic_energy() {
        let entity = EntityState {
            id: 1,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(10.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 2.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        };

        // KE = 0.5 * m * v^2 = 0.5 * 2.0 * 10.0^2 = 100.0
        let ke = entity.kinetic_energy();
        assert!((ke - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_aabb_calculations() {
        let aabb = AABB { min: Vec3::new(-1.0, -2.0, -3.0), max: Vec3::new(1.0, 2.0, 3.0) };

        let center = aabb.center();
        assert_eq!(center, Vec3::ZERO);

        let half_extents = aabb.half_extents();
        assert_eq!(half_extents, Vec3::new(1.0, 2.0, 3.0));

        let volume = aabb.volume();
        assert!((volume - 48.0).abs() < 0.01); // 2*4*6 = 48
    }

    #[test]
    fn test_snapshot_queries() {
        let mut snapshot = PhysicsDebugSnapshot::new(1, 0.016);

        // Add entities
        snapshot.entities.push(EntityState {
            id: 1,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(5.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        snapshot.entities.push(EntityState {
            id: 2,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(150.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        assert_eq!(snapshot.entity_count(), 2);
        assert_eq!(snapshot.active_entity_count(), 2);

        // Find high-velocity entities
        let high_vel = snapshot.find_high_velocity_entities(100.0);
        assert_eq!(high_vel.len(), 1);
        assert_eq!(high_vel[0], 2);
    }

    #[test]
    fn test_snapshot_hash_determinism() {
        let mut snapshot1 = PhysicsDebugSnapshot::new(100, 1.5);
        snapshot1.entities.push(EntityState {
            id: 1,
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(5.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        let snapshot2 = snapshot1.clone();

        // Same state should produce same hash
        assert_eq!(snapshot1.compute_hash(), snapshot2.compute_hash());

        // Different state should (likely) produce different hash
        let mut snapshot3 = snapshot1.clone();
        snapshot3.entities[0].position.x += 0.01;
        assert_ne!(snapshot1.compute_hash(), snapshot3.compute_hash());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut snapshot = PhysicsDebugSnapshot::new(100, 1.5);
        snapshot.entities.push(EntityState {
            id: 42,
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(5.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        // Serialize to JSON
        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");

        // Deserialize back
        let deserialized: PhysicsDebugSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Should be identical
        assert_eq!(snapshot, deserialized);
    }
}
