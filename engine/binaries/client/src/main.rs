use anyhow::Result;
use engine_core::ecs::World;
use engine_renderer::{Renderer, WindowConfig};
use std::time::{Duration, Instant};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,silmaril=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Silmaril Client starting...");

    // Create window configuration (headless for now, until event loop integration)
    let window_config = WindowConfig {
        title: "Silmaril Client".to_string(),
        width: 1280,
        height: 720,
        resizable: true,
        visible: false, // Headless until proper event loop integration
    };

    // Initialize renderer (creates window internally)
    let mut renderer = Renderer::new(window_config, "Silmaril")?;
    info!("Renderer initialized (Vulkan, headless mode)");

    // Initialize ECS world
    let mut world = World::new();
    register_components(&mut world);
    info!("ECS world initialized");

    // TODO: Initialize networking
    // let network_client = TcpClient::connect("127.0.0.1:7777").await?;

    info!("Client initialized successfully, entering game loop");

    // Frame timing
    let target_frame_time = Duration::from_micros(16_667); // 60 FPS
    let mut last_frame = Instant::now();
    let mut frame_count = 0u64;

    // Main game loop
    loop {
        // TODO: Poll window events when visible window is enabled
        // if renderer.window().should_close() {
        //     info!("Window close requested, shutting down");
        //     break;
        // }

        // Calculate delta time
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        last_frame = now;

        // TODO: Process input
        // TODO: Update client prediction
        // TODO: Process network messages

        // Update ECS world (stubbed for now)
        // run_client_systems(&mut world, dt.as_secs_f32());

        // Render frame
        if let Some(recorder) = renderer.begin_frame() {
            renderer.end_frame(recorder);
        }

        frame_count += 1;

        // Log FPS every 60 frames
        if frame_count % 60 == 0 {
            let fps = if dt.as_secs_f32() > 0.0 { 1.0 / dt.as_secs_f32() } else { 0.0 };
            info!(fps = fps as u32, frame = frame_count, "Frame rendered");
        }

        // Sleep for remaining frame time
        if dt < target_frame_time {
            std::thread::sleep(target_frame_time - dt);
        }

        // For now, run for 60 frames then exit (demo mode)
        // Remove this once proper event handling is added
        if frame_count >= 60 {
            info!("Demo mode: 60 frames rendered, exiting");
            break;
        }
    }

    info!("Client shutdown complete");
    Ok(())
}

/// Register all components used by the client
fn register_components(_world: &mut World) {
    // TODO: Register components when they're implemented
    // world.register::<Transform>();
    // world.register::<Velocity>();
    // world.register::<MeshRenderer>();
    // world.register::<Camera>();
}
