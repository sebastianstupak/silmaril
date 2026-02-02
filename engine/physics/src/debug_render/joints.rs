//! Joint and constraint visualization for debugging mechanical connections
//!
//! Visualizes joints between rigid bodies with color-coded lines based on joint type
//! and constraint status.

use super::renderer::DebugRenderer;
use crate::world::PhysicsWorld;
use engine_math::Vec3;
use rapier3d::prelude::*;

/// Options for rendering joints and constraints
#[derive(Debug, Clone)]
pub struct JointRenderOptions {
    /// Color for fixed joints [R, G, B]
    pub fixed_color: [f32; 3],

    /// Color for revolute joints [R, G, B]
    pub revolute_color: [f32; 3],

    /// Color for prismatic joints [R, G, B]
    pub prismatic_color: [f32; 3],

    /// Color for spherical (ball) joints [R, G, B]
    pub spherical_color: [f32; 3],

    /// Color for generic joints [R, G, B]
    pub generic_color: [f32; 3],

    /// Line thickness scale (affects number of parallel lines)
    pub thickness: f32,

    /// Show joint anchors as small crosses
    pub show_anchors: bool,

    /// Anchor marker size
    pub anchor_size: f32,
}

impl Default for JointRenderOptions {
    fn default() -> Self {
        Self {
            fixed_color: [0.5, 0.5, 0.5],     // Gray for fixed
            revolute_color: [0.0, 1.0, 0.0],  // Green for revolute
            prismatic_color: [0.0, 0.0, 1.0], // Blue for prismatic
            spherical_color: [1.0, 1.0, 0.0], // Yellow for spherical
            generic_color: [1.0, 0.0, 1.0],   // Magenta for generic
            thickness: 1.0,
            show_anchors: true,
            anchor_size: 0.1,
        }
    }
}

impl DebugRenderer {
    /// Render joints connecting rigid bodies
    ///
    /// Visualizes joint constraints with color-coded lines:
    /// - Gray: Fixed joints (no DOF)
    /// - Green: Revolute joints (1 rotational DOF)
    /// - Blue: Prismatic joints (1 translational DOF)
    /// - Yellow: Spherical joints (3 rotational DOF)
    /// - Magenta: Generic joints (custom DOF)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_physics::{PhysicsWorld, PhysicsConfig};
    /// # use engine_physics::debug_render::{DebugRenderer, JointRenderOptions};
    /// # let mut world = PhysicsWorld::new(PhysicsConfig::default());
    /// let mut debug_renderer = DebugRenderer::new(None);
    ///
    /// debug_renderer.begin_frame();
    /// debug_renderer.render_joints(&world, &JointRenderOptions::default());
    /// let lines = debug_renderer.end_frame();
    /// // Submit lines to renderer...
    /// ```
    pub fn render_joints(&mut self, world: &PhysicsWorld, options: &JointRenderOptions) {
        if !self.is_enabled() {
            return;
        }

        let joint_set = world.impulse_joint_set();
        let rigid_bodies = world.rigid_body_set();

        for (_handle, joint) in joint_set.iter() {
            // Get bodies connected by this joint
            let body1_handle = joint.body1;
            let body2_handle = joint.body2;

            let body1 = match rigid_bodies.get(body1_handle) {
                Some(b) => b,
                None => continue,
            };
            let body2 = match rigid_bodies.get(body2_handle) {
                Some(b) => b,
                None => continue,
            };

            // Get body positions
            let pos1 = body1.translation();
            let pos2 = body2.translation();

            let point1 = Vec3::new(pos1.x, pos1.y, pos1.z);
            let point2 = Vec3::new(pos2.x, pos2.y, pos2.z);

            // Determine joint type and color
            let color = match joint.data {
                GenericJoint::Fixed(_) => options.fixed_color,
                GenericJoint::Revolute(_) => options.revolute_color,
                GenericJoint::Prismatic(_) => options.prismatic_color,
                GenericJoint::Spherical(_) => options.spherical_color,
                GenericJoint::Generic(_) => options.generic_color,
            };

            // Draw line connecting bodies
            self.add_line(point1, point2, color);

            // Draw anchor points if enabled
            if options.show_anchors {
                let size = options.anchor_size;

                // Anchor on body 1
                self.add_line(point1 - Vec3::X * size, point1 + Vec3::X * size, color);
                self.add_line(point1 - Vec3::Y * size, point1 + Vec3::Y * size, color);
                self.add_line(point1 - Vec3::Z * size, point1 + Vec3::Z * size, color);

                // Anchor on body 2
                self.add_line(point2 - Vec3::X * size, point2 + Vec3::X * size, color);
                self.add_line(point2 - Vec3::Y * size, point2 + Vec3::Y * size, color);
                self.add_line(point2 - Vec3::Z * size, point2 + Vec3::Z * size, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Collider, RigidBody};
    use crate::config::PhysicsConfig;
    use crate::joints::JointBuilder;
    use engine_math::Quat;

    #[test]
    fn test_render_joints_basic() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create two bodies
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(2.0, 0.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Add a fixed joint between them
        let joint = JointBuilder::fixed().build();
        world.add_joint(0, 1, &joint);

        let options = JointRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_joints(&world, &options);
        let lines = debug_renderer.end_frame();

        // Should have:
        // - 1 line connecting bodies
        // - 6 lines for anchors (3 per body)
        assert_eq!(lines.len(), 7, "Should render joint line and anchor markers");
    }

    #[test]
    fn test_render_joints_multiple_types() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies
        for i in 0..4 {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
        }

        // Add different joint types
        world.add_joint(0, 1, &JointBuilder::fixed().build());
        world.add_joint(1, 2, &JointBuilder::revolute().axis(Vec3::Y).build());
        world.add_joint(2, 3, &JointBuilder::spherical().build());

        let options = JointRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_joints(&world, &options);
        let lines = debug_renderer.end_frame();

        // 3 joints × (1 connection line + 6 anchor lines) = 21 lines
        assert_eq!(lines.len(), 21, "Should render all joint types");
    }

    #[test]
    fn test_render_joints_toggle_anchors() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create two bodies with a joint
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(2.0, 0.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        world.add_joint(0, 1, &JointBuilder::fixed().build());

        // Test with anchors disabled
        let options_no_anchors = JointRenderOptions { show_anchors: false, ..Default::default() };

        debug_renderer.begin_frame();
        debug_renderer.render_joints(&world, &options_no_anchors);
        let lines_no_anchors = debug_renderer.end_frame();

        // Should only have connection line
        assert_eq!(lines_no_anchors.len(), 1, "Should only render connection line without anchors");

        // Test with anchors enabled
        let options_with_anchors = JointRenderOptions { show_anchors: true, ..Default::default() };

        debug_renderer.begin_frame();
        debug_renderer.render_joints(&world, &options_with_anchors);
        let lines_with_anchors = debug_renderer.end_frame();

        // Should have connection + anchors
        assert_eq!(lines_with_anchors.len(), 7, "Should render connection and anchor markers");
    }

    #[test]
    fn test_render_joints_no_joints() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies but no joints
        world.add_rigidbody(0, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        let options = JointRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_joints(&world, &options);
        let lines = debug_renderer.end_frame();

        // No joints = no lines
        assert_eq!(lines.len(), 0, "Should not render when no joints exist");
    }
}
