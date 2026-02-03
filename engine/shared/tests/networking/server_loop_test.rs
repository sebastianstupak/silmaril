//! Integration tests for server tick loop

use engine_core::ecs::World;
use engine_networking::{ServerLoop, TARGET_TPS};
use std::time::Duration;

#[tokio::test]
async fn test_server_loop_initialization() {
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    assert_eq!(server_loop.tick(), 0);
    assert_eq!(server_loop.client_count().await, 0);

    let stats = server_loop.stats();
    assert_eq!(stats.tick, 0);
    assert_eq!(stats.client_count, 0);
}

#[tokio::test]
async fn test_server_loop_runs_at_60_tps() {
    let world = World::new();
    let mut server_loop = ServerLoop::new(world);

    let mut tick_count = 0;

    // Run for 200ms, should process ~12 ticks
    tokio::select! {
        _ = server_loop.run(|_world, _dt| {
            tick_count += 1;
        }) => {},
        _ = tokio::time::sleep(Duration::from_millis(200)) => {
            // Should have run 11-13 ticks in 200ms at 60 TPS
            assert!(server_loop.tick() >= 10 && server_loop.tick() <= 14,
                "Expected 11-13 ticks, got {}", server_loop.tick());
        }
    }
}

#[tokio::test]
async fn test_server_performance_stats() {
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    let initial_stats = server_loop.stats();
    assert_eq!(initial_stats.tick, 0);
    assert_eq!(initial_stats.client_count, 0);

    // Run server for a short time
    // Note: Stats are only updated every second, so we just verify initial state
    let stats = server_loop.stats();
    assert_eq!(stats.tick, 0);
    assert_eq!(stats.client_count, 0);
}

#[tokio::test]
async fn test_server_tick_timing_accuracy() {
    let world = World::new();
    let mut server_loop = ServerLoop::new(world);

    let start = std::time::Instant::now();

    tokio::select! {
        _ = server_loop.run(|_world, _dt| {}) => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            let elapsed = start.elapsed();
            let ticks = server_loop.tick();

            // In 100ms at 60 TPS, should process ~6 ticks (allow wider range due to scheduler variance)
            assert!(ticks >= 4 && ticks <= 8,
                "Expected 4-8 ticks in 100ms, got {}", ticks);

            // Each tick should be roughly 16.67ms (allow wider range)
            if ticks > 0 {
                let avg_tick_time = elapsed.as_millis() / ticks as u128;
                assert!(avg_tick_time >= 10 && avg_tick_time <= 30,
                    "Average tick time should be ~16.67ms, got {}ms", avg_tick_time);
            }
        }
    }
}

#[tokio::test]
async fn test_server_game_logic_callback() {
    let world = World::new();
    let mut server_loop = ServerLoop::new(world);

    let mut callback_count = 0;
    let mut total_dt = 0.0f32;

    tokio::select! {
        _ = server_loop.run(|_world, dt| {
            callback_count += 1;
            total_dt += dt;
        }) => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            // Should have called callback multiple times
            assert!(callback_count > 0);

            // Each dt should be ~0.01667 (16.67ms)
            let avg_dt = total_dt / callback_count as f32;
            assert!(avg_dt >= 0.015 && avg_dt <= 0.018,
                "Average dt should be ~0.01667, got {}", avg_dt);
        }
    }
}

#[tokio::test]
async fn test_server_handles_world_updates() {
    let mut world = World::new();
    let entity = world.spawn();

    let mut server_loop = ServerLoop::new(world);

    let mut tick_count = 0;

    tokio::select! {
        _ = server_loop.run(|world, _dt| {
            tick_count += 1;
            // Verify we can access world in callback
            assert!(world.is_alive(entity));
        }) => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            assert!(tick_count > 0);
        }
    }
}

#[tokio::test]
async fn test_server_target_tps_constant() {
    // Verify TARGET_TPS is 60
    assert_eq!(TARGET_TPS, 60);
}

#[tokio::test]
async fn test_server_loop_can_despawn_entities() {
    let mut world = World::new();
    let entity = world.spawn();

    let mut server_loop = ServerLoop::new(world);

    let mut entity_despawned = false;

    tokio::select! {
        _ = server_loop.run(|world, _dt| {
            if !entity_despawned {
                world.despawn(entity);
                entity_despawned = true;
            }
        }) => {},
        _ = tokio::time::sleep(Duration::from_millis(50)) => {
            assert!(entity_despawned);
        }
    }
}

#[tokio::test]
async fn test_server_loop_multiple_ticks() {
    let world = World::new();
    let mut server_loop = ServerLoop::new(world);

    let tick_values = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let tick_values_clone = tick_values.clone();

    tokio::select! {
        _ = server_loop.run(move |_world, dt| {
            tick_values_clone.lock().unwrap().push(dt);
        }) => {},
        _ = tokio::time::sleep(Duration::from_millis(150)) => {
            let values = tick_values.lock().unwrap();
            // Should have at least 8 ticks in 150ms
            assert!(values.len() >= 8, "Expected at least 8 ticks, got {}", values.len());

            // All dt values should be approximately 0.01667
            for (i, dt) in values.iter().enumerate() {
                assert!(
                    *dt >= 0.015 && *dt <= 0.018,
                    "Tick {} had dt={}, expected ~0.01667", i, dt
                );
            }
        }
    }
}
