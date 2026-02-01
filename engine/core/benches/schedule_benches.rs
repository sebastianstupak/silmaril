//! Benchmarks for system scheduling and parallelization
//!
//! Measures:
//! - Scheduling overhead
//! - Parallel vs sequential execution
//! - Dependency graph construction
//! - Complex game loop scenarios

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, Schedule, System, SystemAccess, World};
use std::time::Duration;

// ============================================================================
// Benchmark Components
// ============================================================================

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Position {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Rotation {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}
impl Component for Rotation {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Damage {
    amount: f32,
}
impl Component for Damage {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct AI {
    state: u32,
}
impl Component for AI {}

// ============================================================================
// Benchmark Systems
// ============================================================================

struct PhysicsSystem;
impl System for PhysicsSystem {
    fn name(&self) -> &str {
        "PhysicsSystem"
    }
    fn run(&mut self, _world: &mut World) {
        // Simulate some work
        black_box(1 + 1);
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Velocity>().writes::<Position>()
    }
}

struct RenderSystem;
impl System for RenderSystem {
    fn name(&self) -> &str {
        "RenderSystem"
    }
    fn run(&mut self, _world: &mut World) {
        black_box(1 + 1);
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Position>().reads::<Rotation>()
    }
}

struct AISystem;
impl System for AISystem {
    fn name(&self) -> &str {
        "AISystem"
    }
    fn run(&mut self, _world: &mut World) {
        black_box(1 + 1);
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<AI>().writes::<Velocity>()
    }
}

struct DamageSystem;
impl System for DamageSystem {
    fn name(&self) -> &str {
        "DamageSystem"
    }
    fn run(&mut self, _world: &mut World) {
        black_box(1 + 1);
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Damage>().writes::<Health>()
    }
}

struct RotationSystem;
impl System for RotationSystem {
    fn name(&self) -> &str {
        "RotationSystem"
    }
    fn run(&mut self, _world: &mut World) {
        black_box(1 + 1);
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().writes::<Rotation>()
    }
}

// System with actual work
struct WorkSystem {
    work_duration: Duration,
}

impl System for WorkSystem {
    fn name(&self) -> &str {
        "WorkSystem"
    }
    fn run(&mut self, _world: &mut World) {
        // Simulate actual work
        let start = std::time::Instant::now();
        while start.elapsed() < self.work_duration {
            black_box(1 + 1);
        }
    }
    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Position>()
    }
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_schedule_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("schedule_build");

    // Benchmark building schedule with different numbers of systems
    for num_systems in [1, 5, 10, 20, 50] {
        group.throughput(Throughput::Elements(num_systems));
        group.bench_with_input(
            BenchmarkId::new("systems", num_systems),
            &num_systems,
            |b, &num| {
                b.iter(|| {
                    let mut schedule = Schedule::new();

                    // Add systems
                    for i in 0..num {
                        match i % 5 {
                            0 => schedule.add_system(PhysicsSystem),
                            1 => schedule.add_system(RenderSystem),
                            2 => schedule.add_system(AISystem),
                            3 => schedule.add_system(DamageSystem),
                            _ => schedule.add_system(RotationSystem),
                        }
                    }

                    schedule.build();
                    black_box(&schedule);
                });
            },
        );
    }

    group.finish();
}

fn bench_schedule_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("schedule_execution");

    // Benchmark executing a schedule
    let mut schedule = Schedule::new();
    schedule.add_system(PhysicsSystem);
    schedule.add_system(RenderSystem);
    schedule.add_system(AISystem);
    schedule.add_system(DamageSystem);
    schedule.add_system(RotationSystem);
    schedule.build();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Rotation>();
    world.register::<Health>();
    world.register::<Damage>();
    world.register::<AI>();

    group.bench_function("5_systems", |b| {
        b.iter(|| {
            schedule.run(black_box(&mut world));
        });
    });

    group.finish();
}

fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_vs_sequential");

    // Create systems with actual work
    let work_duration = Duration::from_micros(100);

    // Parallel schedule (independent systems)
    let mut parallel_schedule = Schedule::new();
    for _ in 0..4 {
        parallel_schedule.add_system(WorkSystem {
            work_duration,
        });
    }
    parallel_schedule.build();

    let mut world_parallel = World::new();
    world_parallel.register::<Position>();

    group.bench_function("parallel_4_systems", |b| {
        b.iter(|| {
            parallel_schedule.run(black_box(&mut world_parallel));
        });
    });

    // Sequential schedule (dependent systems)
    #[allow(dead_code)]
    struct SequentialWorkSystem {
        index: usize,
        work_duration: Duration,
    }

    impl System for SequentialWorkSystem {
        fn name(&self) -> &str {
            "SequentialWorkSystem"
        }
        fn run(&mut self, _world: &mut World) {
            let start = std::time::Instant::now();
            while start.elapsed() < self.work_duration {
                black_box(1 + 1);
            }
        }
        fn access(&self) -> SystemAccess {
            // Each system writes to Position, forcing sequential execution
            SystemAccess::new().writes::<Position>()
        }
    }

    let mut sequential_schedule = Schedule::new();
    for i in 0..4 {
        sequential_schedule.add_system(SequentialWorkSystem {
            index: i,
            work_duration,
        });
    }
    sequential_schedule.build();

    let mut world_sequential = World::new();
    world_sequential.register::<Position>();

    group.bench_function("sequential_4_systems", |b| {
        b.iter(|| {
            sequential_schedule.run(black_box(&mut world_sequential));
        });
    });

    group.finish();
}

fn bench_complex_game_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_game_loop");

    // Simulate a realistic game loop with many systems
    let mut schedule = Schedule::new();

    // Add many systems with various dependencies
    for _ in 0..10 {
        schedule.add_system(AISystem);
    }
    for _ in 0..5 {
        schedule.add_system(PhysicsSystem);
    }
    for _ in 0..3 {
        schedule.add_system(DamageSystem);
    }
    for _ in 0..2 {
        schedule.add_system(RenderSystem);
    }

    schedule.build();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Rotation>();
    world.register::<Health>();
    world.register::<Damage>();
    world.register::<AI>();

    // Add entities
    for _ in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        world.add(entity, Health { current: 100.0, max: 100.0 });
    }

    group.throughput(Throughput::Elements(1000)); // 1000 entities
    group.bench_function("20_systems_1000_entities", |b| {
        b.iter(|| {
            schedule.run(black_box(&mut world));
        });
    });

    group.finish();
}

fn bench_dependency_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_analysis");

    // Benchmark dependency analysis with varying numbers of systems
    for num_systems in [5, 10, 20, 50] {
        group.throughput(Throughput::Elements(num_systems));
        group.bench_with_input(
            BenchmarkId::new("systems", num_systems),
            &num_systems,
            |b, &num| {
                b.iter(|| {
                    let mut schedule = Schedule::new();

                    for i in 0..num {
                        match i % 5 {
                            0 => schedule.add_system(PhysicsSystem),
                            1 => schedule.add_system(RenderSystem),
                            2 => schedule.add_system(AISystem),
                            3 => schedule.add_system(DamageSystem),
                            _ => schedule.add_system(RotationSystem),
                        }
                    }

                    // Only measure the build time
                    schedule.build();
                    black_box(&schedule);
                });
            },
        );
    }

    group.finish();
}

fn bench_scheduling_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduling_overhead");

    // Measure pure scheduling overhead with minimal system work
    let mut schedule = Schedule::new();
    schedule.add_system(PhysicsSystem);
    schedule.build();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    group.bench_function("minimal_overhead", |b| {
        b.iter(|| {
            schedule.run(black_box(&mut world));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_schedule_build,
    bench_schedule_execution,
    bench_parallel_vs_sequential,
    bench_complex_game_loop,
    bench_dependency_analysis,
    bench_scheduling_overhead,
);

criterion_main!(benches);
