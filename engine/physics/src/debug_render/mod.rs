//! Visual debugging features for physics
//!
//! This module provides human-centric visual debugging tools that complement
//! the AI-readable data export from `agentic_debug`. It renders physics state
//! directly into the scene for real-time debugging.
//!
//! # Features (Phase A.1)
//!
//! - AABB wireframe rendering
//! - Collision point + normal visualization
//! - Velocity vector arrows
//! - Constraint/joint line rendering
//! - Center of mass markers
//! - Force/torque vector rendering
//!
//! # Usage
//!
//! ```no_run
//! # use engine_physics::{PhysicsWorld, PhysicsConfig};
//! # use engine_renderer::Renderer;
//! # let mut world = PhysicsWorld::new(PhysicsConfig::default());
//! # let mut renderer: Renderer = todo!();
//! #[cfg(feature = "debug-render")]
//! {
//!     use engine_physics::debug_render::DebugRenderer;
//!
//!     let mut debug_renderer = DebugRenderer::new(&renderer)?;
//!
//!     // Every frame
//!     debug_renderer.begin_frame();
//!     debug_renderer.render_aabbs(&world);
//!     debug_renderer.render_velocities(&world);
//!     debug_renderer.end_frame(&mut renderer)?;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Cargo Feature
//!
//! This module is only available when the `debug-render` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! engine-physics = { path = "../physics", features = ["debug-render"] }
//! ```

mod aabb;
mod arrows;
mod center_of_mass;
mod collisions;
mod joints;
mod lines;
mod renderer;

pub use aabb::*;
pub use arrows::*;
pub use center_of_mass::*;
pub use collisions::*;
pub use joints::*;
pub use lines::*;
pub use renderer::*;
