//! Integration tests for system scheduling and parallelization
//!
//! Tests:
//! - Dependency detection
//! - Parallel execution correctness
//! - Data race prevention
//! - Execution order guarantees

use engine_core::ecs::{Component, Schedule, System, SystemAccess, World};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Test Components
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health {
    value: f32,
}

impl Component for Health {}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
struct Damage {
    value: f32,
}

impl Component for Damage {}

// ============================================================================
// Test Systems
// ============================================================================

/// System that tracks execution order
struct OrderTrackingSystem {
    name: String,
    access: SystemAccess,
    execution_log: Arc<Mutex<Vec<String>>>,
}

impl OrderTrackingSystem {
    fn new(name: &str, access: SystemAccess, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name: name.to_string(),
            access,
            execution_log: log,
        }
    }
}

impl System for OrderTrackingSystem {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, _world: &mut World) {
        let mut log = self.execution_log.lock().unwrap();
        log.push(self.name.clone());
    }

    fn access(&self) -> SystemAccess {
        self.access.clone()
    }
}

/// System that simulates actual work
struct WorkSystem {
    name: String,
    access: SystemAccess,
    duration: Duration,
}

impl WorkSystem {
    fn new(name: &str, access: SystemAccess, duration: Duration) -> Self {
        Self {
            name: name.to_string(),
            access,
            duration,
        }
    }
}

impl System for WorkSystem {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, _world: &mut World) {
        std::thread::sleep(self.duration);
    }

    fn access(&self) -> SystemAccess {
        self.access.clone()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_single_system_execution() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    schedule.add_system(OrderTrackingSystem::new(
        "System1",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.build();

    let mut world = World::new();
    world.register::<Position>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 1);
    assert_eq!(execution_log[0], "System1");
}

#[test]
fn test_independent_systems_parallel() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Two systems that don't conflict - should be in same stage
    schedule.add_system(OrderTrackingSystem::new(
        "System1",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "System2",
        SystemAccess::new().reads::<Velocity>(),
        Arc::clone(&log),
    ));

    schedule.build();

    // Both systems should be in stage 0 (parallel)
    assert_eq!(schedule.stage_count(), 1);

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 2);
    // Both ran, order doesn't matter in same stage
    assert!(execution_log.contains(&"System1".to_string()));
    assert!(execution_log.contains(&"System2".to_string()));
}

#[test]
fn test_dependent_systems_sequential() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // System1 writes Position, System2 reads Position
    // System1 must run before System2
    schedule.add_system(OrderTrackingSystem::new(
        "Writer",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "Reader",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.build();

    // Should have 2 stages (sequential)
    assert_eq!(schedule.stage_count(), 2);

    let mut world = World::new();
    world.register::<Position>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 2);

    // Writer must run before Reader
    assert_eq!(execution_log[0], "Writer");
    assert_eq!(execution_log[1], "Reader");
}

#[test]
fn test_write_write_conflict() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Two systems that both write Position - must be sequential
    schedule.add_system(OrderTrackingSystem::new(
        "Writer1",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "Writer2",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    schedule.build();

    // Must be sequential
    assert_eq!(schedule.stage_count(), 2);

    let mut world = World::new();
    world.register::<Position>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 2);
}

#[test]
fn test_read_read_no_conflict() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Two systems that both read Position - can be parallel
    schedule.add_system(OrderTrackingSystem::new(
        "Reader1",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "Reader2",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.build();

    // Should be parallel (1 stage)
    assert_eq!(schedule.stage_count(), 1);

    let mut world = World::new();
    world.register::<Position>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 2);
}

#[test]
fn test_complex_dependency_chain() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Create a complex dependency chain:
    // System1: writes Position
    // System2: reads Position, writes Velocity
    // System3: reads Velocity
    //
    // Expected order (because of add order):
    // Stage 0: System1
    // Stage 1: System2
    // Stage 2: System3

    schedule.add_system(OrderTrackingSystem::new(
        "System1",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "System2",
        SystemAccess::new().reads::<Position>().writes::<Velocity>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "System3",
        SystemAccess::new().reads::<Velocity>(),
        Arc::clone(&log),
    ));

    schedule.build();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 3);

    // Verify order - due to our simplified dependency analysis,
    // systems are ordered based on their addition order when they conflict
    let sys1_idx = execution_log.iter().position(|s| s == "System1").unwrap();
    let sys2_idx = execution_log.iter().position(|s| s == "System2").unwrap();
    let sys3_idx = execution_log.iter().position(|s| s == "System3").unwrap();

    assert!(sys1_idx < sys2_idx, "System1 should run before System2");
    assert!(sys2_idx < sys3_idx, "System2 should run before System3");
}

#[test]
fn test_independent_systems_same_stage() {
    let mut schedule = Schedule::new();

    let duration = Duration::from_millis(10);

    // Two independent systems that could theoretically run in parallel
    schedule.add_system(WorkSystem::new(
        "Work1",
        SystemAccess::new().reads::<Position>(),
        duration,
    ));

    schedule.add_system(WorkSystem::new(
        "Work2",
        SystemAccess::new().reads::<Velocity>(),
        duration,
    ));

    schedule.build();

    // Should be in same stage (could be parallel)
    assert_eq!(schedule.stage_count(), 1);

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Just verify it runs without error
    schedule.run(&mut world);

    // Note: Current implementation runs systems sequentially even in same stage.
    // True parallel execution would require more sophisticated system storage.
}

#[test]
fn test_mixed_parallel_and_sequential() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Stage 0: System1 and System2 (parallel, different components)
    schedule.add_system(OrderTrackingSystem::new(
        "System1",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    schedule.add_system(OrderTrackingSystem::new(
        "System2",
        SystemAccess::new().writes::<Velocity>(),
        Arc::clone(&log),
    ));

    // Stage 1: System3 (reads both)
    schedule.add_system(OrderTrackingSystem::new(
        "System3",
        SystemAccess::new().reads::<Position>().reads::<Velocity>(),
        Arc::clone(&log),
    ));

    schedule.build();

    // Should have 2 stages
    assert_eq!(schedule.stage_count(), 2);

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    schedule.run(&mut world);

    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 3);

    // System3 must run last
    assert_eq!(execution_log[2], "System3");
}

#[test]
#[should_panic(expected = "must be built")]
fn test_run_without_build_panics() {
    let mut schedule = Schedule::new();
    let mut world = World::new();

    schedule.run(&mut world);
}

#[test]
fn test_empty_schedule() {
    let mut schedule = Schedule::new();
    schedule.build();

    let mut world = World::new();
    schedule.run(&mut world);

    // Should not panic
}

#[test]
fn test_rebuild_after_add() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    schedule.add_system(OrderTrackingSystem::new(
        "System1",
        SystemAccess::new().reads::<Position>(),
        Arc::clone(&log),
    ));

    schedule.build();
    assert_eq!(schedule.stage_count(), 1);

    // Add another system
    schedule.add_system(OrderTrackingSystem::new(
        "System2",
        SystemAccess::new().writes::<Position>(),
        Arc::clone(&log),
    ));

    // Must rebuild
    schedule.build();

    // Now should have 2 stages
    assert_eq!(schedule.stage_count(), 2);
}

#[test]
fn test_no_data_races() {
    // This test verifies that the schedule prevents data races by
    // ensuring systems that conflict are not run in parallel

    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Create two systems that would race if run in parallel
    let log1 = Arc::clone(&log);
    schedule.add_system(OrderTrackingSystem::new(
        "Incrementer",
        SystemAccess::new().writes::<Position>(),
        log1,
    ));

    let log2 = Arc::clone(&log);
    schedule.add_system(OrderTrackingSystem::new(
        "Reader",
        SystemAccess::new().reads::<Position>(),
        log2,
    ));

    schedule.build();

    // Systems must be sequential (2 stages)
    assert_eq!(schedule.stage_count(), 2);

    let mut world = World::new();
    world.register::<Position>();

    schedule.run(&mut world);

    // Both systems should have run
    let execution_log = log.lock().unwrap();
    assert_eq!(execution_log.len(), 2);
}

#[test]
fn test_system_access_builder() {
    let access = SystemAccess::new()
        .reads::<Position>()
        .reads::<Velocity>()
        .writes::<Health>();

    assert_eq!(access.reads.len(), 2);
    assert_eq!(access.writes.len(), 1);
}

#[test]
fn test_debug_info() {
    let mut schedule = Schedule::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    schedule.add_system(OrderTrackingSystem::new(
        "TestSystem",
        SystemAccess::new().reads::<Position>(),
        log,
    ));

    schedule.build();

    let info = schedule.debug_info();
    assert!(info.contains("TestSystem"));
    assert!(info.contains("1 systems"));
}
