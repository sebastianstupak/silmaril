//! Collision point and normal visualization for debugging contact resolution
//!
//! Visualizes active contact points with color-coded normals to help debug
//! collision detection and resolution issues.

use super::lines::arrow;
use super::renderer::DebugRenderer;
use crate::world::PhysicsWorld;
use engine_math::Vec3;

/// Options for rendering collision points and normals
#[derive(Debug, Clone)]
pub struct CollisionRenderOptions {
    /// Color for collision point markers [R, G, B]
    pub point_color: [f32; 3],

    /// Color for contact normals [R, G, B]
    pub normal_color: [f32; 3],

    /// Scale for normal arrows (multiplier of penetration depth)
    pub normal_scale: f32,

    /// Minimum penetration depth to render (filter trivial contacts)
    pub min_penetration: f32,

    /// Maximum contacts to render per frame (performance limit)
    pub max_contacts: usize,

    /// Render point as small cross marker
    pub show_points: bool,

    /// Render surface normals as arrows
    pub show_normals: bool,
}

impl Default for CollisionRenderOptions {
    fn default() -> Self {
        Self {
            point_color: [1.0, 0.0, 1.0],    // Magenta for contact points
            normal_color: [0.0, 1.0, 1.0],   // Cyan for normals
            normal_scale: 2.0,                // Normal length = 2x penetration
            min_penetration: 0.001,           // 1mm threshold
            max_contacts: 1000,               // Limit for performance
            show_points: true,
            show_normals: true,
        }
    }
}

impl DebugRenderer {
    /// Render active collision points and surface normals
    ///
    /// Visualizes contact manifolds from the current physics step:
    /// - Magenta crosses mark contact points
    /// - Cyan arrows show surface normals (point away from collision)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_physics::{PhysicsWorld, PhysicsConfig};
    /// # use engine_physics::debug_render::{DebugRenderer, CollisionRenderOptions};
    /// # let mut world = PhysicsWorld::new(PhysicsConfig::default());
    /// let mut debug_renderer = DebugRenderer::new(None);
    ///
    /// debug_renderer.begin_frame();
    /// debug_renderer.render_collisions(&world, &CollisionRenderOptions::default());
    /// let lines = debug_renderer.end_frame();
    /// // Submit lines to renderer...
    /// ```
    pub fn render_collisions(&mut self, world: &PhysicsWorld, options: &CollisionRenderOptions) {
        if !self.is_enabled() {
            return;
        }

        // Get narrow phase for contact manifolds
        let narrow_phase = world.narrow_phase();

        let mut contact_count = 0;

        // Iterate through all contact pairs
        for contact_pair in narrow_phase.contact_pairs() {
            if contact_count >= options.max_contacts {
                break;
            }

            // Iterate through all manifolds in this contact pair
            for manifold in &contact_pair.manifolds {
                // Get contact normal (points from object 1 to object 2)
                let normal_rapier = manifold.data.normal;
                let normal = Vec3::new(normal_rapier.x, normal_rapier.y, normal_rapier.z);

                // Iterate through all solver contacts in this manifold
                for solver_contact in &manifold.data.solver_contacts {
                    if contact_count >= options.max_contacts {
                        break;
                    }

                    // Filter by penetration depth
                    let penetration = solver_contact.dist;
                    if penetration > -options.min_penetration {
                        // Positive or near-zero distance = no contact
                        continue;
                    }

                    // Get contact point in world space
                    let contact_point = Vec3::new(
                        solver_contact.point.x,
                        solver_contact.point.y,
                        solver_contact.point.z,
                    );

                    // Render contact point as small cross
                    if options.show_points {
                        let size = 0.05; // 5cm cross
                        self.add_line(
                            contact_point - Vec3::X * size,
                            contact_point + Vec3::X * size,
                            options.point_color,
                        );
                        self.add_line(
                            contact_point - Vec3::Y * size,
                            contact_point + Vec3::Y * size,
                            options.point_color,
                        );
                        self.add_line(
                            contact_point - Vec3::Z * size,
                            contact_point + Vec3::Z * size,
                            options.point_color,
                        );
                    }

                    // Render surface normal as arrow
                    if options.show_normals {
                        let arrow_length = penetration.abs() * options.normal_scale;
                        let arrow_end = contact_point + normal * arrow_length;

                        let arrow_lines = arrow(contact_point, arrow_end, options.normal_color);
                        for line in &arrow_lines {
                            self.add_line(line.start, line.end, line.color);
                        }
                    }

                    contact_count += 1;
                }
            }
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
    fn test_render_collisions_basic() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add ground plane
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 0.1, 100.0)));

        // Add box slightly penetrating ground
        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 0.4, 0.0), // Slightly overlapping
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

        // Step once to generate contacts
        world.step(1.0 / 60.0);

        let options = CollisionRenderOptions::default();

        debug_renderer.begin_frame();
        debug_renderer.render_collisions(&world, &options);
        let lines = debug_renderer.end_frame();

        // Should have some contact visualization lines
        // (3 cross lines + 4 arrow lines per contact, possibly multiple contacts)
        assert!(
            !lines.is_empty(),
            "Should render collision visualization"
        );
    }

    #[test]
    fn test_collision_render_filters_by_penetration() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add two boxes far apart (no contact)
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::ONE));

        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 10.0, 0.0), // 10m above
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Step once (no contacts expected)
        world.step(1.0 / 60.0);

        let options = CollisionRenderOptions {
            min_penetration: 0.001,
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_collisions(&world, &options);
        let lines = debug_renderer.end_frame();

        // No contacts = no lines
        assert_eq!(lines.len(), 0, "Should not render when no collisions");
    }

    #[test]
    fn test_collision_render_toggle_points_normals() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Setup collision scenario
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 0.1, 100.0)));

        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 0.4, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

        world.step(1.0 / 60.0);

        // Test with only points
        let options_points_only = CollisionRenderOptions {
            show_points: true,
            show_normals: false,
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_collisions(&world, &options_points_only);
        let lines_points = debug_renderer.end_frame().len();

        // Test with only normals
        let options_normals_only = CollisionRenderOptions {
            show_points: false,
            show_normals: true,
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_collisions(&world, &options_normals_only);
        let lines_normals = debug_renderer.end_frame().len();

        // Points should render (3 lines per contact)
        // Normals should render (4 lines per contact)
        // Both should be > 0 individually
        assert!(lines_points > 0, "Should render contact points");
        assert!(lines_normals > 0, "Should render normals");
    }
}
