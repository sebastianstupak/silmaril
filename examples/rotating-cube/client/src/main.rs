//! rotating-cube - Client Binary
//!
//! A simple demo showing the rendering pipeline in action:
//! - Creates a rotating cube using ECS components
//! - Renders it using the Vulkan renderer
//! - Demonstrates Templates → ECS → Rendering flow

use rotating_cube_shared::components::{MeshRenderer, RotationSpeed, Transform};
use rotating_cube_shared::systems::rotation_system;
use silmaril_assets::{AssetId, MeshData};
use silmaril_core::{Camera, Entity, World};
use silmaril_renderer::{Renderer, WindowConfig};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, Level};

// Simple asset manager for demo (just stores meshes in memory)
struct SimpleAssetManager {
    meshes: HashMap<AssetId, MeshData>,
}

impl SimpleAssetManager {
    fn new() -> Self {
        Self {
            meshes: HashMap::new(),
        }
    }

    fn add_mesh(&mut self, id: AssetId, mesh: MeshData) {
        self.meshes.insert(id, mesh);
    }

    fn get_mesh(&self, id: AssetId) -> Option<&MeshData> {
        self.meshes.get(&id)
    }
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("rotating-cube client starting...");

    // 1. Create asset manager and load cube mesh
    let mut assets = SimpleAssetManager::new();
    let cube_mesh = MeshData::cube();
    let mesh_id = AssetId::from_seed_and_params(1, b"mesh"); // mesh_id = 1
    assets.add_mesh(mesh_id, cube_mesh);
    info!("Loaded cube mesh with {} vertices", assets.get_mesh(mesh_id).unwrap().vertex_count());

    // 2. Create ECS world and spawn entities
    let mut world = World::new();

    // Create camera entity
    let camera_entity = world.spawn();
    let camera_transform = engine_math::Transform::new(
        glam::Vec3::new(0.0, 2.0, 8.0), // Position camera back and up
        glam::Quat::IDENTITY,
        glam::Vec3::ONE,
    );
    world.add(camera_entity, camera_transform);
    world.add(camera_entity, Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0));
    info!("Created camera entity at {:?}", camera_transform.position);

    // Load cube template (manual parsing for simplicity)
    // In a real game, you'd use engine-templating
    let cube_entity = world.spawn();

    // Convert from array-based Transform to engine_math::Transform
    let template_transform = Transform {
        position: [0.0, 0.0, -5.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    let ecs_transform = engine_math::Transform::new(
        glam::Vec3::from_array(template_transform.position),
        glam::Quat::from_xyzw(
            template_transform.rotation[0],
            template_transform.rotation[1],
            template_transform.rotation[2],
            template_transform.rotation[3],
        ),
        glam::Vec3::from_array(template_transform.scale),
    );

    world.add(cube_entity, ecs_transform);
    world.add(
        cube_entity,
        engine_core::MeshRenderer::new(1), // mesh_id = 1
    );

    // Store rotation speed component in shared format for systems
    let rotation_speed = RotationSpeed::new(1.0);

    info!(
        entity = ?cube_entity,
        position = ?template_transform.position,
        "Created rotating cube entity"
    );

    // 3. Initialize renderer
    info!("Initializing Vulkan renderer...");
    let window_config = WindowConfig {
        title: "Rotating Cube Demo".to_string(),
        width: 1280,
        height: 720,
        resizable: true,
        visible: true, // Make window visible
    };

    let mut renderer = Renderer::new(window_config, "RotatingCube")?;
    renderer.set_clear_color([0.1, 0.1, 0.15, 1.0]); // Dark blue background
    info!("Renderer initialized successfully");

    // 4. Main loop
    info!("Entering main loop (press ESC to exit)");
    let start_time = Instant::now();
    let mut last_frame_time = start_time;
    let mut frame_count = 0u64;

    loop {
        // Calculate delta time
        let current_time = Instant::now();
        let dt = (current_time - last_frame_time).as_secs_f32();
        last_frame_time = current_time;

        // Update rotation system
        // Get transform components from world and apply rotation
        let entities: Vec<Entity> = world.entities().collect();
        for entity in entities {
            // Only rotate entities that have both Transform and mesh renderer
            // (skip camera)
            if let Some(mesh_renderer) = world.get::<engine_core::MeshRenderer>(entity) {
                if mesh_renderer.mesh_id == 1 {
                    // This is our cube
                    if let Some(mut transform) = world.get_mut::<engine_math::Transform>(entity) {
                        // Convert to shared format for system
                        let mut shared_transform = Transform {
                            position: transform.position.to_array(),
                            rotation: [
                                transform.rotation.x,
                                transform.rotation.y,
                                transform.rotation.z,
                                transform.rotation.w,
                            ],
                            scale: transform.scale.to_array(),
                        };

                        // Apply rotation
                        rotation_system(&mut shared_transform, &rotation_speed, dt);

                        // Write back to ECS
                        transform.rotation = glam::Quat::from_xyzw(
                            shared_transform.rotation[0],
                            shared_transform.rotation[1],
                            shared_transform.rotation[2],
                            shared_transform.rotation[3],
                        );
                    }
                }
            }
        }

        // Render frame
        match renderer.render_meshes(&world, &assets) {
            Ok(_) => {
                frame_count += 1;

                // Log FPS every second
                if frame_count % 60 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f32();
                    let fps = frame_count as f32 / elapsed;
                    info!(frame = frame_count, fps = fps as u32, "Running");
                }
            }
            Err(e) => {
                // Handle swapchain out of date (window resize)
                if e.to_string().contains("out of date") {
                    info!("Swapchain out of date, recreating...");
                    // In a real app, you'd recreate the swapchain here
                    // For now, just continue
                    continue;
                } else {
                    tracing::error!(error = ?e, "Render failed");
                    break;
                }
            }
        }

        // Simple exit condition (run for 10 seconds in automated tests)
        // In a real game, you'd check for window close events
        if current_time.duration_since(start_time).as_secs() > 10 {
            info!("Demo time limit reached");
            break;
        }

        // Limit frame rate to ~60 FPS
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    info!(
        frames = frame_count,
        duration = ?start_time.elapsed(),
        "rotating-cube client shutting down"
    );

    Ok(())
}
