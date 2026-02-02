//! Center of mass visualization for debugging inertia and balance
//!
//! Renders markers at the center of mass of each rigid body to help debug
//! stability, balance, and rotation issues.

use super::renderer::DebugRenderer;
use crate::world::PhysicsWorld;
use engine_math::Vec3;

/// Options for rendering center of mass markers
#[derive(Debug, Clone)]
pub struct CenterOfMassRenderOptions {
    /// Color for dynamic bodies [R, G, B]
    pub dynamic_color: [f32; 3],

    /// Color for kinematic bodies [R, G, B]
    pub kinematic_color: [f32; 3],

    /// Show static bodies
    pub show_static: bool,

    /// Color for static bodies [R, G, B]
    pub static_color: [f32; 3],

    /// Marker size (cross radius)
    pub marker_size: f32,
}

impl Default for CenterOfMassRenderOptions {
    fn default() -> Self {
        Self {
            dynamic_color: [1.0, 0.0, 0.0],   // Red for dynamic
            kinematic_color: [0.0, 0.0, 1.0], // Blue for kinematic
            show_static: false,               // Hide static by default
            static_color: [0.5, 0.5, 0.5],    // Gray for static
            marker_size: 0.2,                 // 20cm cross
        }
    }
}

impl DebugRenderer {
    /// Render center of mass markers for all rigid bodies
    ///
    /// Visualizes CoM with color-coded 3D cross markers:
    /// - Red: Dynamic bodies (affected by forces)
    /// - Blue: Kinematic bodies (controlled motion)
    /// - Gray: Static bodies (optional, disabled by default)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_physics::{PhysicsWorld, PhysicsConfig};
    /// # use engine_physics::debug_render::{DebugRenderer, CenterOfMassRenderOptions};
    /// # let mut world = PhysicsWorld::new(PhysicsConfig::default());
    /// let mut debug_renderer = DebugRenderer::new(None);
    ///
    /// debug_renderer.begin_frame();
    /// debug_renderer.render_center_of_mass(&world, &CenterOfMassRenderOptions::default());
    /// let lines = debug_renderer.end_frame();
    /// // Submit lines to renderer...
    /// ```
    pub fn render_center_of_mass(
        &mut self,
        world: &PhysicsWorld,
        options: &CenterOfMassRenderOptions,
    ) {
        if !self.is_enabled() {
            return;
        }

        let rigid_bodies = world.rigid_body_set();

        for (_handle, body) in rigid_bodies.iter() {
            // Determine color based on body type
            let color = if body.is_dynamic() {
                options.dynamic_color
            } else if body.is_kinematic() {
                options.kinematic_color
            } else {
                // Static body
                if !options.show_static {
                    continue;
                }
                options.static_color
            };

            // Get center of mass in world space
            let com = body.center_of_mass();
            let center = Vec3::new(com.translation.x, com.translation.y, com.translation.z);

            let size = options.marker_size;

            // Draw 3D cross marker (3 perpendicular lines)
            self.add_line(center - Vec3::X * size, center + Vec3::X * size, color);
            self.add_line(center - Vec3::Y * size, center + Vec3::Y * size, color);
            self.add_line(center - Vec3::Z * size, center + Vec3::Z * size, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Collider, RigidBody};
    use crate::config::PhysicsConfig;
    use engine_math::Quat;

    #[test]
    fn test_render_center_of_mass_basic() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add a dynamic body
        world.add_rigidbody(0, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        let options = CenterOfMassRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_center_of_mass(&world, &options);
        let lines = debug_renderer.end_frame();

        // 3 lines forming 3D cross
        assert_eq!(lines.len(), 3, "Should render 3D cross marker");

        // All lines should be red (dynamic)
        for line in &lines {
            assert_eq!(line.color, options.dynamic_color);
        }
    }

    #[test]
    fn test_render_center_of_mass_filters_static() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add static body
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        let options = CenterOfMassRenderOptions { show_static: false, ..Default::default() };

        debug_renderer.begin_frame();
        debug_renderer.render_center_of_mass(&world, &options);
        let lines = debug_renderer.end_frame();

        // Should not render static bodies by default
        assert_eq!(lines.len(), 0, "Should filter out static bodies");
    }

    #[test]
    fn test_render_center_of_mass_shows_static() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add static body
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        let options = CenterOfMassRenderOptions { show_static: true, ..Default::default() };

        debug_renderer.begin_frame();
        debug_renderer.render_center_of_mass(&world, &options);
        let lines = debug_renderer.end_frame();

        // Should render when show_static is true
        assert_eq!(lines.len(), 3, "Should render static body CoM");

        // All lines should be gray (static)
        for line in &lines {
            assert_eq!(line.color, options.static_color);
        }
    }

    #[test]
    fn test_render_center_of_mass_multiple_bodies() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add dynamic body
        world.add_rigidbody(0, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        // Add kinematic body
        world.add_rigidbody(1, &RigidBody::kinematic(), Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        let options = CenterOfMassRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_center_of_mass(&world, &options);
        let lines = debug_renderer.end_frame();

        // 2 bodies × 3 lines each = 6 lines
        assert_eq!(lines.len(), 6, "Should render CoM for both bodies");
    }
}
