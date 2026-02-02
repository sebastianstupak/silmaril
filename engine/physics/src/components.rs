//! Physics components

use engine_core::ecs::Component;
use engine_math::Vec3;
use serde::{Deserialize, Serialize};

/// Velocity component - movement in 3D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    /// Linear velocity vector
    pub linear: Vec3,
}

impl Component for Velocity {}

impl Velocity {
    /// Zero velocity
    pub const ZERO: Self = Self { linear: Vec3::ZERO };

    /// Create a new velocity
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { linear: Vec3::new(x, y, z) }
    }

    /// Get X component
    pub fn x(&self) -> f32 {
        self.linear.x
    }

    /// Get Y component
    pub fn y(&self) -> f32 {
        self.linear.y
    }

    /// Get Z component
    pub fn z(&self) -> f32 {
        self.linear.z
    }
}

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
            linear_damping: 0.01,  // Slight air resistance
            angular_damping: 0.05, // More rotational damping
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
        Self { body_type: RigidBodyType::Dynamic, mass, ..Default::default() }
    }

    /// Create a kinematic body
    pub fn kinematic() -> Self {
        Self { body_type: RigidBodyType::Kinematic, ..Default::default() }
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
        self.lock_rotation[0] = true; // Lock X rotation
        self.lock_rotation[2] = true; // Lock Z rotation
        self
    }

    /// Set linear damping (air resistance for linear velocity)
    pub fn with_linear_damping(mut self, damping: f32) -> Self {
        self.linear_damping = damping;
        self
    }

    /// Set angular damping (air resistance for angular velocity)
    pub fn with_angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = damping;
        self
    }

    /// Set gravity scale (1.0 = normal, 0.0 = no gravity, 2.0 = double)
    pub fn with_gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
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
    Box {
        /// Half extents from center (width/2, height/2, depth/2)
        half_extents: Vec3,
    },

    /// Sphere collider (radius)
    /// Use for: Balls, round objects, simple characters
    Sphere {
        /// Radius of the sphere
        radius: f32,
    },

    /// Capsule collider (cylinder with hemispherical ends)
    /// Use for: Character controllers (best for slopes/stairs)
    Capsule {
        /// Half height of cylinder part
        half_height: f32,
        /// Radius of cylinder and spheres
        radius: f32,
    },

    /// Cylinder collider
    /// Use for: Pillars, trees
    Cylinder {
        /// Half height of cylinder
        half_height: f32,
        /// Radius of cylinder
        radius: f32,
    },
}

/// Physics material properties
///
/// Determines friction and bounciness of collisions.
/// See: Unity Physics Materials, Unreal Physical Materials
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
            shape: ColliderShape::Box { half_extents: Vec3::ONE },
            material: PhysicsMaterial::default(),
            is_sensor: false,
            collision_layer: 1,         // Layer 0
            collision_mask: 0xFFFFFFFF, // Collide with all layers
        }
    }
}

impl Collider {
    /// Box collider
    pub fn box_collider(half_extents: Vec3) -> Self {
        Self { shape: ColliderShape::Box { half_extents }, ..Default::default() }
    }

    /// Sphere collider
    pub fn sphere(radius: f32) -> Self {
        Self { shape: ColliderShape::Sphere { radius }, ..Default::default() }
    }

    /// Capsule collider (best for character controllers)
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self { shape: ColliderShape::Capsule { half_height, radius }, ..Default::default() }
    }

    /// Create a sensor/trigger collider
    pub fn sensor(shape: ColliderShape) -> Self {
        Self { shape, is_sensor: true, ..Default::default() }
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

    /// Set physics material (friction, restitution, etc.)
    pub fn with_material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }

    /// Check if this collider can collide with another layer
    pub fn can_collide_with(&self, other_layer: u32) -> bool {
        (self.collision_mask & other_layer) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_zero() {
        let v = Velocity::ZERO;
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
    }

    #[test]
    fn test_velocity_new() {
        let v = Velocity::new(1.0, 2.0, 3.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
    }

    #[test]
    fn test_rigidbody_default() {
        let rb = RigidBody::default();
        assert_eq!(rb.body_type, RigidBodyType::Dynamic);
        assert_eq!(rb.mass, 1.0);
        assert_eq!(rb.gravity_scale, 1.0);
        assert!(!rb.ccd_enabled);
    }

    #[test]
    fn test_rigidbody_constructors() {
        let dynamic = RigidBody::dynamic(5.0);
        assert_eq!(dynamic.body_type, RigidBodyType::Dynamic);
        assert_eq!(dynamic.mass, 5.0);

        let kinematic = RigidBody::kinematic();
        assert_eq!(kinematic.body_type, RigidBodyType::Kinematic);

        let static_body = RigidBody::static_body();
        assert_eq!(static_body.body_type, RigidBodyType::Static);
        assert!(static_body.is_sleeping);
    }

    #[test]
    fn test_apply_impulse() {
        let mut rb = RigidBody::dynamic(2.0);
        rb.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(rb.linear_velocity.x, 5.0); // 10 / 2

        // Kinematic doesn't respond to impulses
        let mut kinematic = RigidBody::kinematic();
        kinematic.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(kinematic.linear_velocity.x, 0.0);
    }

    #[test]
    fn test_kinetic_energy() {
        let mut rb = RigidBody::dynamic(2.0);
        rb.linear_velocity = Vec3::new(3.0, 4.0, 0.0);

        // KE = 0.5 * m * v^2
        // v^2 = 3^2 + 4^2 = 25
        // KE = 0.5 * 2 * 25 = 25
        assert_eq!(rb.kinetic_energy(), 25.0);
    }

    #[test]
    fn test_lock_2d_platform() {
        let rb = RigidBody::default().lock_2d_platform();
        assert!(rb.lock_translation[1]); // Y locked
        assert!(rb.lock_rotation[0]); // X rotation locked
        assert!(rb.lock_rotation[2]); // Z rotation locked
    }

    #[test]
    fn test_material_combine() {
        let ice = PhysicsMaterial::ICE;
        let rubber = PhysicsMaterial::RUBBER;

        let (friction, restitution) = ice.combine(&rubber);

        // Friction: ice=Minimum, rubber=Maximum -> Maximum wins
        // max(0.05, 0.9) = 0.9
        assert!((friction - 0.9).abs() < 0.01);

        // Restitution: both use Maximum
        // max(0.1, 0.8) = 0.8
        assert!((restitution - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_material_combine_modes() {
        let mat_a = PhysicsMaterial {
            friction: 0.6,
            restitution: 0.3,
            friction_combine: CombineMode::Average,
            restitution_combine: CombineMode::Multiply,
            ..Default::default()
        };

        let mat_b = PhysicsMaterial {
            friction: 0.4,
            restitution: 0.5,
            friction_combine: CombineMode::Minimum,
            restitution_combine: CombineMode::Average,
            ..Default::default()
        };

        let (friction, restitution) = mat_a.combine(&mat_b);

        // Friction: Minimum has higher precedence -> min(0.6, 0.4) = 0.4
        assert!((friction - 0.4).abs() < 0.01);

        // Restitution: Multiply has higher precedence -> 0.3 * 0.5 = 0.15
        assert!((restitution - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_collider_constructors() {
        let box_collider = Collider::box_collider(Vec3::ONE);
        assert!(matches!(box_collider.shape, ColliderShape::Box { .. }));

        let sphere = Collider::sphere(1.5);
        assert!(
            matches!(sphere.shape, ColliderShape::Sphere { radius } if (radius - 1.5).abs() < 0.01)
        );

        let capsule = Collider::capsule(2.0, 0.5);
        assert!(matches!(
            capsule.shape,
            ColliderShape::Capsule { half_height, radius }
            if (half_height - 2.0).abs() < 0.01 && (radius - 0.5).abs() < 0.01
        ));
    }

    #[test]
    fn test_collision_layers() {
        let collider = Collider::sphere(1.0)
            .with_layer(2) // I'm on layer 2
            .with_mask(0b101); // I collide with layers 0 and 2

        assert_eq!(collider.collision_layer, 1 << 2);

        assert!(collider.can_collide_with(1 << 0)); // Layer 0: yes
        assert!(!collider.can_collide_with(1 << 1)); // Layer 1: no
        assert!(collider.can_collide_with(1 << 2)); // Layer 2: yes
    }

    #[test]
    fn test_sensor_collider() {
        let sensor = Collider::sensor(ColliderShape::Box { half_extents: Vec3::ONE });
        assert!(sensor.is_sensor);
    }
}
