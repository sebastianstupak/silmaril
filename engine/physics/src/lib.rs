//! Engine Physics
//!
//! Provides physics simulation:
//! - Rigid body dynamics
//! - Collision detection
//! - Raycasting
//! - Character controller

#![warn(missing_docs)]

pub mod world;
pub mod rigidbody;
pub mod collider;
pub mod raycast;

// Re-export commonly used types
pub use world::PhysicsWorld;
pub use rigidbody::RigidBody;
pub use collider::Collider;
