//! Arrow rendering for velocity and force visualization
//!
//! Phase A.1.4: Velocity vector arrows
//! Phase A.1.7: Force/torque vector rendering

use crate::debug_render::DebugRenderer;
use crate::PhysicsWorld;
use engine_math::Vec3;

/// Velocity rendering options
#[derive(Debug, Clone)]
pub struct VelocityRenderOptions {
    /// Velocity scale factor (visual scaling)
    pub scale: f32,
    /// Minimum velocity magnitude to render (filter noise)
    pub min_magnitude: f32,
    /// Maximum velocity magnitude to clamp visualization
    pub max_magnitude: f32,
    /// Color for low velocity (RGB, 0.0-1.0)
    pub low_velocity_color: [f32; 3],
    /// Color for high velocity (RGB, 0.0-1.0)
    pub high_velocity_color: [f32; 3],
    /// Whether to show linear velocity
    pub show_linear: bool,
    /// Whether to show angular velocity
    pub show_angular: bool,
}

impl Default for VelocityRenderOptions {
    fn default() -> Self {
        Self {
            scale: 0.2,                         // 1 m/s = 0.2m arrow
            min_magnitude: 0.1,                 // Filter out very small velocities
            max_magnitude: 50.0,                // Clamp extreme velocities
            low_velocity_color: [0.0, 1.0, 0.0], // Green for slow
            high_velocity_color: [1.0, 0.0, 0.0], // Red for fast
            show_linear: true,
            show_angular: false, // Angular velocity is complex to visualize
        }
    }
}

/// Force rendering options
#[derive(Debug, Clone)]
pub struct ForceRenderOptions {
    /// Force scale factor (visual scaling)
    pub scale: f32,
    /// Minimum force magnitude to render
    pub min_magnitude: f32,
    /// Color for forces (RGB, 0.0-1.0)
    pub force_color: [f32; 3],
    /// Color for torques (RGB, 0.0-1.0)
    pub torque_color: [f32; 3],
    /// Whether to show forces
    pub show_forces: bool,
    /// Whether to show torques
    pub show_torques: bool,
}

impl Default for ForceRenderOptions {
    fn default() -> Self {
        Self {
            scale: 0.1,                       // Visual scaling
            min_magnitude: 0.1,               // Filter small forces
            force_color: [1.0, 1.0, 0.0],     // Yellow for forces
            torque_color: [1.0, 0.0, 1.0],    // Magenta for torques
            show_forces: true,
            show_torques: false, // Torques are complex to visualize
        }
    }
}

impl DebugRenderer {
    /// Render velocity arrows for all dynamic bodies
    ///
    /// Phase A.1.4 implementation. Draws arrows from body center of mass
    /// in the direction of velocity, scaled and color-coded by magnitude.
    ///
    /// # Arguments
    ///
    /// * `world` - Physics world to render velocities from
    /// * `options` - Rendering options (scale, colors, filters)
    pub fn render_velocities(&mut self, world: &PhysicsWorld, options: &VelocityRenderOptions) {
        let rigid_bodies = world.rigid_body_set();

        for (_handle, body) in rigid_bodies.iter() {
            // Skip static/kinematic bodies (no interesting velocity)
            if !body.is_dynamic() {
                continue;
            }

            if options.show_linear {
                // Get linear velocity
                let velocity = body.linvel();
                let velocity = Vec3::new(velocity.x, velocity.y, velocity.z);
                let magnitude = velocity.length();

                // Filter small velocities
                if magnitude < options.min_magnitude {
                    continue;
                }

                // Get position
                let position = body.translation();
                let start = Vec3::new(position.x, position.y, position.z);

                // Clamp and scale velocity for visualization
                let clamped_magnitude = magnitude.min(options.max_magnitude);
                let visual_velocity = velocity.normalize() * clamped_magnitude * options.scale;
                let end = start + visual_velocity;

                // Interpolate color based on magnitude
                let t = (magnitude / options.max_magnitude).min(1.0);
                let color = [
                    options.low_velocity_color[0] * (1.0 - t) + options.high_velocity_color[0] * t,
                    options.low_velocity_color[1] * (1.0 - t) + options.high_velocity_color[1] * t,
                    options.low_velocity_color[2] * (1.0 - t) + options.high_velocity_color[2] * t,
                ];

                self.draw_arrow(start, end, color);
            }

            if options.show_angular {
                // Angular velocity visualization (torque axis + magnitude)
                let angvel = body.angvel();
                let angvel = Vec3::new(angvel.x, angvel.y, angvel.z);
                let magnitude = angvel.length();

                if magnitude < options.min_magnitude {
                    continue;
                }

                // Draw angular velocity as axis
                let position = body.translation();
                let start = Vec3::new(position.x, position.y, position.z);
                let clamped_magnitude = magnitude.min(options.max_magnitude);
                let end = start + angvel.normalize() * clamped_magnitude * options.scale;

                // Use cyan for angular velocity
                self.draw_arrow(start, end, [0.0, 1.0, 1.0]);
            }
        }
    }

    /// Render force and torque arrows for all bodies
    ///
    /// Phase A.1.7 implementation. Note: Rapier doesn't expose per-body forces
    /// directly, so this renders user-applied forces that are tracked separately.
    ///
    /// # Arguments
    ///
    /// * `world` - Physics world
    /// * `options` - Rendering options
    ///
    /// # Note
    ///
    /// This requires tracking forces applied via `apply_force`. Internal solver
    /// forces (constraints, contacts) are not directly accessible from Rapier's
    /// public API.
    pub fn render_forces(&mut self, _world: &PhysicsWorld, _options: &ForceRenderOptions) {
        // TODO: Implement force tracking in PhysicsWorld
        // Rapier doesn't expose per-body accumulated forces in public API
        // We would need to track forces applied via apply_force/apply_impulse
        // and render those separately.
        //
        // For now, this is a placeholder for Phase A.1.7
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Collider, PhysicsConfig, RigidBody};
    use engine_math::Quat;

    #[test]
    fn test_render_velocities_filters_static() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add static body (should be filtered)
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        debug_renderer.begin_frame();
        debug_renderer.render_velocities(&world, &VelocityRenderOptions::default());

        assert_eq!(
            debug_renderer.line_count(),
            0,
            "Should not render velocities for static bodies"
        );
    }

    #[test]
    fn test_render_velocities_filters_small() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add dynamic body with very small velocity
        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Set tiny velocity (below filter threshold)
        world.set_velocity(1, Vec3::new(0.01, 0.0, 0.0), Vec3::ZERO);

        let options = VelocityRenderOptions {
            min_magnitude: 0.1, // Filter threshold
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_velocities(&world, &options);

        assert_eq!(
            debug_renderer.line_count(),
            0,
            "Should filter velocities below threshold"
        );
    }

    #[test]
    fn test_render_velocities_shows_significant() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add dynamic body with significant velocity
        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Set significant velocity
        world.set_velocity(1, Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO);

        debug_renderer.begin_frame();
        debug_renderer.render_velocities(&world, &VelocityRenderOptions::default());

        // Should render arrow (4 lines: 1 shaft + 3 head)
        assert_eq!(
            debug_renderer.line_count(),
            4,
            "Should render velocity arrow"
        );
    }

    #[test]
    fn test_velocity_color_interpolation() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add body with low velocity
        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));
        world.set_velocity(1, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);

        let options = VelocityRenderOptions {
            max_magnitude: 50.0,
            low_velocity_color: [0.0, 1.0, 0.0],  // Green
            high_velocity_color: [1.0, 0.0, 0.0], // Red
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_velocities(&world, &options);
        let lines = debug_renderer.end_frame();

        // Low velocity should be closer to green
        assert!(
            lines[0].color[1] > 0.8,
            "Low velocity should be mostly green"
        );

        // Test with high velocity
        world.set_velocity(1, Vec3::new(45.0, 0.0, 0.0), Vec3::ZERO);

        debug_renderer.begin_frame();
        debug_renderer.render_velocities(&world, &options);
        let lines = debug_renderer.end_frame();

        // High velocity should be closer to red
        assert!(lines[0].color[0] > 0.8, "High velocity should be mostly red");
    }
}
