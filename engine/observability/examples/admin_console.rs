//! Example: Admin telnet console for server management
//!
//! This example demonstrates:
//! 1. Starting the admin console server
//! 2. Processing admin commands
//! 3. Integrating with game server loop
//!
//! Run with:
//! ```bash
//! cargo run --example admin_console --features admin
//! ```
//!
//! Then connect with telnet:
//! ```bash
//! telnet localhost 8888
//! ```
//!
//! Available commands:
//! - help      - Show help
//! - status    - Show server status
//! - metrics   - Show metrics
//! - spawn 100 - Spawn 100 entities
//! - quit      - Disconnect

#[cfg(feature = "admin")]
use engine_observability::admin::{AdminCommand, AdminConsole};

#[cfg(feature = "admin")]
#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create admin console
    let mut console = AdminConsole::new("127.0.0.1:8888");

    // Get command receiver
    let mut command_rx = console.command_receiver();

    // Start console in background
    tokio::spawn(async move {
        if let Err(e) = console.start().await {
            eprintln!("Admin console error: {}", e);
        }
    });

    info!("Server starting...");
    info!("Connect with: telnet localhost 8888");

    // Simulate game server state
    let mut entity_count = 0;
    let mut tick_count = 0;
    let mut paused = false;
    let mut max_clients = 100;

    // Server loop
    loop {
        let tick_start = Instant::now();

        // Process admin commands (non-blocking)
        while let Ok((command, response_tx)) = command_rx.try_recv() {
            let response = process_command(
                &command,
                &mut entity_count,
                &mut paused,
                &mut max_clients,
                tick_count,
            );
            let _ = response_tx.send(response);
        }

        // Only tick if not paused
        if !paused {
            // Simulate game tick
            simulate_tick(&mut entity_count);
            tick_count += 1;

            if tick_count % 60 == 0 {
                info!("Tick {}: {} entities, paused={}", tick_count, entity_count, paused);
            }
        }

        // Maintain tick rate
        let elapsed = tick_start.elapsed();
        let target_tick = Duration::from_millis(16); // 60 TPS
        if elapsed < target_tick {
            tokio::time::sleep(target_tick - elapsed).await;
        }
    }
}

#[cfg(feature = "admin")]
fn process_command(
    command: &AdminCommand,
    entity_count: &mut i32,
    paused: &mut bool,
    max_clients: &mut u32,
    tick_count: u64,
) -> String {
    match command {
        AdminCommand::Status => {
            format!(
                "Server Status:\r\n\
                 - Tick: {}\r\n\
                 - Entities: {}\r\n\
                 - Paused: {}\r\n\
                 - Max Clients: {}",
                tick_count, entity_count, paused, max_clients
            )
        }
        AdminCommand::Metrics => {
            format!(
                "Metrics:\r\n\
                 - entity_count: {}\r\n\
                 - tick_count: {}\r\n\
                 - paused: {}",
                entity_count, tick_count, paused
            )
        }
        AdminCommand::Pause => {
            *paused = true;
            "Server paused".to_string()
        }
        AdminCommand::Resume => {
            *paused = false;
            "Server resumed".to_string()
        }
        AdminCommand::Spawn(count) => {
            *entity_count += *count as i32;
            format!("Spawned {} entities (total: {})", count, entity_count)
        }
        AdminCommand::Despawn(count) => {
            let actual = (*count as i32).min(*entity_count);
            *entity_count -= actual;
            format!("Despawned {} entities (total: {})", actual, entity_count)
        }
        AdminCommand::SetConfig { key, value } => match key.as_str() {
            "max_clients" => match value.parse::<u32>() {
                Ok(val) => {
                    *max_clients = val;
                    format!("Set max_clients = {}", val)
                }
                Err(_) => "Error: max_clients must be a number".to_string(),
            },
            _ => format!("Error: Unknown config key '{}'", key),
        },
        AdminCommand::GetConfig { key } => match key.as_str() {
            "max_clients" => format!("max_clients = {}", max_clients),
            _ => format!("Error: Unknown config key '{}'", key),
        },
        _ => "Command not implemented".to_string(),
    }
}

#[cfg(feature = "admin")]
fn simulate_tick(entity_count: &mut i32) {
    // Simulate some entity spawning/despawning
    if rand::random::<f32>() < 0.01 && *entity_count < 1000 {
        *entity_count += 1;
    } else if rand::random::<f32>() < 0.005 && *entity_count > 0 {
        *entity_count -= 1;
    }

    // Simulate work
    std::thread::sleep(Duration::from_micros(500));
}

#[cfg(not(feature = "admin"))]
fn main() {
    println!("This example requires the 'admin' feature");
    println!("Run with: cargo run --example admin_console --features admin");
}
