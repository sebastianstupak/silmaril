//! AABB wireframe rendering (Phase A.1.2)
//!
//! Renders axis-aligned bounding boxes for all colliders in the physics world.
//! Colors AABBs by sleep state: green for awake, blue for sleeping.

use crate::debug_render::DebugRenderer;
use crate::PhysicsWorld;
use engine_math::Vec3;

/// AABB rendering options
#[derive(Debug, Clone)]
pub struct AabbRenderOptions {
    /// Color for awake bodies (RGB, 0.0-1.0)
    pub awake_color: [f32; 3],
    /// Color for sleeping bodies (RGB, 0.0-1.0)
    pub sleeping_color: [f32; 3],
    /// Whether to render AABBs for static bodies
    pub show_static: bool,
}

impl Default for AabbRenderOptions {
    fn default() -> Self {
        Self {
            awake_color: [0.0, 1.0, 0.0],    // Green for awake
            sleeping_color: [0.0, 0.5, 1.0], // Blue for sleeping
            show_static: false,              // Don't clutter with static bodies
        }
    }
}

impl DebugRenderer {
    /// Render AABBs for all colliders in the physics world
    ///
    /// This is the main Phase A.1.2 feature. It extracts AABB data from Rapier
    /// and renders wireframe boxes color-coded by sleep state.
    ///
    /// # Arguments
    ///
    /// * `world` - Physics world to render AABBs from
    /// * `options` - Rendering options (colors, filters)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_physics::{PhysicsWorld, PhysicsConfig};
    /// # use engine_physics::debug_render::{DebugRenderer, AabbRenderOptions};
    /// # let world = PhysicsWorld::new(PhysicsConfig::default());
    /// # let mut debug_renderer = DebugRenderer::default();
    /// debug_renderer.begin_frame();
    /// debug_renderer.render_aabbs(&world, &AabbRenderOptions::default());
    /// let lines = debug_renderer.end_frame();
    /// // Submit lines to renderer...
    /// ```
    pub fn render_aabbs(&mut self, world: &PhysicsWorld, options: &AabbRenderOptions) {
        use rapier3d::prelude::*;

        // Get collider set from world
        let colliders = world.collider_set();
        let rigid_bodies = world.rigid_body_set();

        for (_handle, collider) in colliders.iter() {
            // Get rigid body for this collider
            let body_handle = collider.parent().expect("Collider should have parent body");
            let body = &rigid_bodies[body_handle];

            // Skip static bodies if not showing them
            if !options.show_static && body.is_static() {
                continue;
            }

            // Get AABB
            let aabb = collider.compute_aabb();

            // Convert Rapier AABB to our Vec3
            let min = Vec3::new(aabb.mins.x, aabb.mins.y, aabb.mins.z);
            let max = Vec3::new(aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);

            // Choose color based on sleep state
            let color = if body.is_sleeping() {
                options.sleeping_color
            } else {
                options.awake_color
            };

            // Draw wireframe box
            self.draw_box(min, max, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PhysicsConfig, RigidBody, Collider};
    use engine_math::Quat;

    #[test]
    fn test_render_aabbs_basic() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add a ground plane
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

        // Add a dynamic box
        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Render AABBs
        debug_renderer.begin_frame();
        debug_renderer.render_aabbs(&world, &AabbRenderOptions::default());

        // Should have 1 box (12 lines) - static body is hidden by default
        assert_eq!(
            debug_renderer.line_count(),
            12,
            "Should render 1 AABB (dynamic body only)"
        );
    }

    #[test]
    fn test_render_aabbs_with_static() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add a ground plane
        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

        // Add a dynamic box
        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        // Render AABBs with static bodies enabled
        let options = AabbRenderOptions {
            show_static: true,
            ..Default::default()
        };

        debug_renderer.begin_frame();
        debug_renderer.render_aabbs(&world, &options);

        // Should have 2 boxes (24 lines)
        assert_eq!(
            debug_renderer.line_count(),
            24,
            "Should render 2 AABBs (static + dynamic)"
        );
    }

    #[test]
    fn test_render_aabbs_color_by_sleep_state() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Add a dynamic box (initially awake)
        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::ONE));

        let options = AabbRenderOptions::default();

        // Render while awake
        debug_renderer.begin_frame();
        debug_renderer.render_aabbs(&world, &options);
        let lines_awake = debug_renderer.end_frame().to_vec();

        // All lines should be awake color (green)
        for line in &lines_awake {
            assert_eq!(line.color, options.awake_color, "Awake body should use awake color");
        }

        // Step physics until body sleeps (falls and settles)
        for _ in 0..240 {
            // 4 seconds @ 60Hz
            world.step(1.0 / 60.0);
        }

        // Render after sleeping
        debug_renderer.begin_frame();
        debug_renderer.render_aabbs(&world, &options);
        let lines_sleeping = debug_renderer.end_frame().to_vec();

        // All lines should be sleeping color (blue)
        for line in &lines_sleeping {
            assert_eq!(
                line.color, options.sleeping_color,
                "Sleeping body should use sleeping color"
            );
        }
    }
}
