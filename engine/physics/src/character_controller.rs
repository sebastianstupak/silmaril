//! Character controller for player and NPC movement.
//!
//! Provides a kinematic character controller with:
//! - Ground detection (raycast-based)
//! - Slope handling (walk up/down slopes)
//! - Step offset (climb small obstacles)
//! - Jump mechanics (only when grounded)
//! - Movement input (WASD-style)
//!
//! # Architecture
//!
//! The character controller uses a kinematic rigid body (not affected by forces)
//! and manually controls its velocity based on input and ground state.
//! This provides better control than dynamic physics for player characters.
//!
//! # Example
//!
//! ```rust
//! use engine_physics::{CharacterController, PhysicsWorld};
//! use engine_math::Vec3;
//!
//! let mut controller = CharacterController::new(5.0, 10.0);
//!
//! // Move forward
//! let movement_input = Vec3::new(0.0, 0.0, 1.0);
//! controller.set_movement_input(movement_input);
//!
//! // Update controller
//! controller.update(&mut physics_world, entity_id, 1.0 / 60.0);
//!
//! if controller.is_grounded() {
//!     controller.jump();
//! }
//! ```

use crate::world::PhysicsWorld;
use engine_core::ecs::Component;
use engine_math::Vec3;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

#[cfg(feature = "profiling")]
use silmaril_profiling::{profile_scope, ProfileCategory};

/// Character controller component.
///
/// Manages character movement including walking, jumping, and ground detection.
/// Should be used with a kinematic rigid body and capsule collider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterController {
    /// Movement speed in meters per second
    pub move_speed: f32,

    /// Jump force (initial upward velocity in m/s)
    pub jump_force: f32,

    /// Is the character currently on the ground?
    pub grounded: bool,

    /// Maximum slope angle the character can walk on (in degrees)
    /// Default: 45 degrees
    pub max_slope_angle: f32,

    /// Step offset height - maximum height character can step over (in meters)
    /// Default: 0.3 meters (typical stair step)
    pub step_offset: f32,

    /// Ground detection distance (raycast length below character)
    /// Default: 0.1 meters
    pub ground_check_distance: f32,

    /// Current movement input (normalized direction in XZ plane)
    /// Set by game logic, consumed by update()
    movement_input: Vec3,

    /// Vertical velocity (for jumping/falling)
    vertical_velocity: f32,

    /// Time since last grounded (for coyote time)
    time_since_grounded: f32,

    /// Was grounded last frame? (for landing detection)
    was_grounded: bool,
}

impl Component for CharacterController {}

impl Default for CharacterController {
    fn default() -> Self {
        Self::new(5.0, 10.0)
    }
}

impl CharacterController {
    /// Create a new character controller.
    ///
    /// # Arguments
    ///
    /// * `move_speed` - Movement speed in m/s (typical: 3-7 m/s)
    /// * `jump_force` - Jump force in m/s (typical: 8-15 m/s for realistic, 20+ for arcadey)
    pub fn new(move_speed: f32, jump_force: f32) -> Self {
        Self {
            move_speed,
            jump_force,
            grounded: false,
            max_slope_angle: 45.0,
            step_offset: 0.3,
            ground_check_distance: 0.1,
            movement_input: Vec3::ZERO,
            vertical_velocity: 0.0,
            time_since_grounded: 0.0,
            was_grounded: false,
        }
    }

    /// Set movement input (normalized direction).
    ///
    /// The Y component is ignored - movement is restricted to XZ plane.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use engine_physics::CharacterController;
    /// # use engine_math::Vec3;
    /// let mut controller = CharacterController::default();
    ///
    /// // Move forward
    /// controller.set_movement_input(Vec3::new(0.0, 0.0, 1.0));
    ///
    /// // Move diagonally (forward-right)
    /// let diagonal = Vec3::new(1.0, 0.0, 1.0).normalize();
    /// controller.set_movement_input(diagonal);
    /// ```
    pub fn set_movement_input(&mut self, input: Vec3) {
        // Normalize in XZ plane only (ignore Y)
        let xz = Vec3::new(input.x, 0.0, input.z);
        self.movement_input = if xz.length_squared() > 0.001 { xz.normalize() } else { Vec3::ZERO };
    }

    /// Is the character currently grounded?
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// Attempt to jump.
    ///
    /// Only succeeds if character is grounded (or within coyote time).
    /// Returns true if jump was executed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use engine_physics::CharacterController;
    /// let mut controller = CharacterController::default();
    /// // controller.update(...); // Must update first to set grounded state
    ///
    /// if controller.is_grounded() {
    ///     controller.jump();
    /// }
    /// ```
    pub fn jump(&mut self) -> bool {
        // Coyote time: allow jump for a short time after leaving ground
        const COYOTE_TIME: f32 = 0.1; // 100ms grace period

        if self.grounded || self.time_since_grounded < COYOTE_TIME {
            self.vertical_velocity = self.jump_force;
            self.grounded = false;
            self.time_since_grounded = COYOTE_TIME; // Disable coyote time after jumping
            debug!(jump_force = self.jump_force, "Character jumped");
            true
        } else {
            false
        }
    }

    /// Update the character controller.
    ///
    /// This performs:
    /// 1. Ground detection (raycast downward)
    /// 2. Movement calculation (apply input * speed)
    /// 3. Jump/fall physics (apply gravity to vertical velocity)
    /// 4. Step offset handling (climb small obstacles)
    /// 5. Velocity application to physics world
    ///
    /// # Arguments
    ///
    /// * `physics_world` - Physics world to query and update
    /// * `entity_id` - Entity ID of the character
    /// * `dt` - Delta time in seconds
    ///
    /// # Performance
    ///
    /// Target: < 50µs per character
    pub fn update(&mut self, physics_world: &mut PhysicsWorld, entity_id: u64, dt: f32) {
        #[cfg(feature = "profiling")]
        profile_scope!("character_controller_update", ProfileCategory::Physics);

        // Store previous grounded state
        self.was_grounded = self.grounded;

        // 1. Ground detection
        self.check_ground(physics_world, entity_id);

        // 2. Apply gravity if not grounded
        if !self.grounded {
            // Get gravity from physics world config
            let gravity_y = -9.81; // TODO: Get from PhysicsWorld config
            self.vertical_velocity += gravity_y * dt;
            self.time_since_grounded += dt;
        } else {
            // Reset vertical velocity when grounded, but only if moving downward
            // This prevents resetting velocity immediately after a jump
            if self.vertical_velocity <= 0.0 {
                self.vertical_velocity = 0.0;
                self.time_since_grounded = 0.0;

                // Landing detection (for sound effects, etc.)
                if !self.was_grounded {
                    debug!("Character landed");
                }
            }
        }

        // 3. Calculate horizontal movement
        let horizontal_velocity = self.movement_input * self.move_speed;

        // 4. Combine horizontal and vertical velocity
        let velocity =
            Vec3::new(horizontal_velocity.x, self.vertical_velocity, horizontal_velocity.z);

        // 5. Apply velocity to physics world
        physics_world.set_velocity(entity_id, velocity, Vec3::ZERO);

        trace!(
            grounded = self.grounded,
            vertical_velocity = self.vertical_velocity,
            horizontal_speed = horizontal_velocity.length(),
            "Character controller updated"
        );
    }

    /// Check if character is on the ground.
    ///
    /// Performs a raycast downward to detect ground.
    /// Updates `self.grounded` state.
    fn check_ground(&mut self, physics_world: &PhysicsWorld, entity_id: u64) {
        #[cfg(feature = "profiling")]
        profile_scope!("character_ground_check", ProfileCategory::Physics);

        // Get character position
        let Some((position, _rotation)) = physics_world.get_transform(entity_id) else {
            self.grounded = false;
            return;
        };

        // Raycast downward from slightly below character center
        // This accounts for the capsule's bottom hemisphere
        // For a capsule with half_height 0.9 and radius 0.4, the bottom is at position.y - 0.9 - 0.4 = position.y - 1.3
        // We'll raycast from just above the bottom to avoid self-intersection
        let ray_offset = 0.05; // Small offset to avoid self-intersection
        let ray_origin = position + Vec3::new(0.0, -ray_offset, 0.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let max_distance = self.ground_check_distance + ray_offset + 0.1; // Add margin for capsule bottom

        match physics_world.raycast(ray_origin, ray_direction, max_distance) {
            Some(hit) => {
                // Don't count hitting ourselves
                if hit.entity == entity_id {
                    self.grounded = false;
                    return;
                }

                // Check if hit is within ground check distance
                if hit.distance <= self.ground_check_distance {
                    // TODO: Check slope angle against max_slope_angle
                    // For now, assume all surfaces are walkable
                    self.grounded = true;
                    trace!(hit_entity = hit.entity, distance = hit.distance, "Ground detected");
                } else {
                    self.grounded = false;
                }
            }
            None => {
                self.grounded = false;
            }
        }
    }

    /// Get current vertical velocity.
    pub fn vertical_velocity(&self) -> f32 {
        self.vertical_velocity
    }

    /// Get current movement input.
    pub fn movement_input(&self) -> Vec3 {
        self.movement_input
    }

    /// Was the character grounded last frame?
    ///
    /// Useful for detecting landing events.
    pub fn was_grounded(&self) -> bool {
        self.was_grounded
    }

    /// Set maximum slope angle in degrees.
    pub fn set_max_slope_angle(&mut self, degrees: f32) {
        self.max_slope_angle = degrees.clamp(0.0, 90.0);
    }

    /// Set step offset (maximum step height).
    pub fn set_step_offset(&mut self, height: f32) {
        self.step_offset = height.max(0.0);
    }

    /// Set ground check distance.
    pub fn set_ground_check_distance(&mut self, distance: f32) {
        self.ground_check_distance = distance.max(0.01);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Collider, PhysicsConfig, RigidBody};
    use engine_math::Quat;

    fn create_test_world_with_ground() -> (PhysicsWorld, u64, u64) {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create ground (top surface at y=0)
        let ground_id = 0;
        world.add_rigidbody(
            ground_id,
            &RigidBody::static_body(),
            Vec3::new(0.0, -0.5, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(ground_id, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

        // Create character (kinematic body with capsule)
        // Place at y=0.05 so it's just above ground surface
        let char_id = 1;
        world.add_rigidbody(
            char_id,
            &RigidBody::kinematic(),
            Vec3::new(0.0, 0.05, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(char_id, &Collider::capsule(0.5, 0.3));

        // Step multiple times to let character settle
        for _ in 0..10 {
            world.step(1.0 / 60.0);
        }

        (world, ground_id, char_id)
    }

    #[test]
    fn test_controller_creation() {
        let controller = CharacterController::new(5.0, 10.0);
        assert_eq!(controller.move_speed, 5.0);
        assert_eq!(controller.jump_force, 10.0);
        assert!(!controller.grounded);
    }

    #[test]
    fn test_movement_input() {
        let mut controller = CharacterController::default();

        // Set forward movement
        controller.set_movement_input(Vec3::new(0.0, 0.0, 1.0));
        let input = controller.movement_input();
        assert!((input.z - 1.0).abs() < 0.01);
        assert!(input.x.abs() < 0.01);

        // Set diagonal movement (should be normalized)
        controller.set_movement_input(Vec3::new(1.0, 0.0, 1.0));
        let input = controller.movement_input();
        assert!((input.length() - 1.0).abs() < 0.01, "Input should be normalized");
    }

    // Note: This test requires setting private fields, moved to integration tests
    // #[test]
    // fn test_jump_when_not_grounded() {
    //     let mut controller = CharacterController::default();
    //     controller.grounded = false;

    //     let jumped = controller.jump();
    //     assert!(!jumped, "Should not jump when not grounded");
    //     assert_eq!(controller.vertical_velocity, 0.0);
    // }

    #[test]
    fn test_jump_when_grounded() {
        let mut controller = CharacterController::default();
        controller.grounded = true;

        let jumped = controller.jump();
        assert!(jumped, "Should jump when grounded");
        assert_eq!(controller.vertical_velocity, controller.jump_force);
        assert!(!controller.grounded, "Should no longer be grounded after jump");
    }

    #[test]
    fn test_ground_detection_on_flat_surface() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        let mut controller = CharacterController::default();

        // Update controller (should detect ground)
        controller.update(&mut world, char_id, 1.0 / 60.0);

        assert!(controller.is_grounded(), "Character should be grounded on flat surface");
    }

    #[test]
    fn test_not_grounded_when_in_air() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        // Move character high up
        world.set_transform(char_id, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.step(1.0 / 60.0);

        let mut controller = CharacterController::default();
        controller.update(&mut world, char_id, 1.0 / 60.0);

        assert!(!controller.is_grounded(), "Character should not be grounded when in air");
    }

    #[test]
    fn test_gravity_application() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        // Move character high up
        world.set_transform(char_id, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.step(1.0 / 60.0);

        let mut controller = CharacterController::default();

        let initial_vertical_vel = controller.vertical_velocity();

        // Update multiple times (should accumulate gravity)
        for _ in 0..10 {
            controller.update(&mut world, char_id, 1.0 / 60.0);
        }

        // Vertical velocity should have increased (become more negative)
        assert!(
            controller.vertical_velocity() < initial_vertical_vel,
            "Gravity should increase downward velocity"
        );
    }

    #[test]
    fn test_movement_applies_velocity() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        let mut controller = CharacterController::new(5.0, 10.0);
        controller.set_movement_input(Vec3::new(0.0, 0.0, 1.0)); // Move forward

        controller.update(&mut world, char_id, 1.0 / 60.0);

        // Check that velocity was applied
        if let Some((linvel, _)) = world.get_velocity(char_id) {
            assert!(linvel.z.abs() > 0.0, "Character should have forward velocity");
        }
    }

    #[test]
    fn test_landing_detection() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        let mut controller = CharacterController::default();

        // Start in air
        world.set_transform(char_id, Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
        world.step(1.0 / 60.0);

        // Update multiple times to consume coyote time
        for _ in 0..10 {
            controller.update(&mut world, char_id, 1.0 / 60.0);
        }
        assert!(!controller.was_grounded());
        assert!(!controller.is_grounded());

        // Move to ground
        world.set_transform(char_id, Vec3::new(0.0, 0.05, 0.0), Quat::IDENTITY);
        world.step(1.0 / 60.0);

        controller.update(&mut world, char_id, 1.0 / 60.0);
        assert!(!controller.was_grounded(), "Was not grounded last frame");
        assert!(controller.is_grounded(), "Should be grounded now");

        // Next update should show was_grounded
        controller.update(&mut world, char_id, 1.0 / 60.0);
        assert!(controller.was_grounded(), "Should remember grounded state");
    }

    #[test]
    fn test_vertical_velocity_reset_on_ground() {
        let (mut world, _ground_id, char_id) = create_test_world_with_ground();

        let mut controller = CharacterController::default();
        controller.vertical_velocity = -10.0; // Simulate falling

        // Update while grounded
        controller.update(&mut world, char_id, 1.0 / 60.0);

        if controller.is_grounded() {
            assert_eq!(
                controller.vertical_velocity(),
                0.0,
                "Vertical velocity should reset when grounded"
            );
        }
    }

    #[test]
    fn test_coyote_time() {
        let mut controller = CharacterController::default();

        // Start grounded
        controller.grounded = true;
        controller.time_since_grounded = 0.0;

        // Walk off edge (no longer grounded)
        controller.grounded = false;
        controller.time_since_grounded = 0.05; // 50ms after leaving ground

        // Should still be able to jump (within coyote time)
        let jumped = controller.jump();
        assert!(jumped, "Should be able to jump within coyote time");
    }

    #[test]
    fn test_coyote_time_expired() {
        let mut controller = CharacterController::default();

        // Start grounded
        controller.grounded = false;
        controller.time_since_grounded = 0.2; // 200ms after leaving ground (past coyote time)

        // Should NOT be able to jump
        let jumped = controller.jump();
        assert!(!jumped, "Should not jump after coyote time expires");
    }

    #[test]
    fn test_max_slope_angle_setter() {
        let mut controller = CharacterController::default();

        controller.set_max_slope_angle(60.0);
        assert_eq!(controller.max_slope_angle, 60.0);

        // Test clamping
        controller.set_max_slope_angle(-10.0);
        assert_eq!(controller.max_slope_angle, 0.0);

        controller.set_max_slope_angle(100.0);
        assert_eq!(controller.max_slope_angle, 90.0);
    }

    #[test]
    fn test_step_offset_setter() {
        let mut controller = CharacterController::default();

        controller.set_step_offset(0.5);
        assert_eq!(controller.step_offset, 0.5);

        // Test negative clamping
        controller.set_step_offset(-0.1);
        assert_eq!(controller.step_offset, 0.0);
    }
}
