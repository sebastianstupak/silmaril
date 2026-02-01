use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::{Component, World};

// Test components
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Acceleration {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Acceleration {}

#[derive(Debug, Clone, Copy)]
struct Mass {
    value: f32,
}

impl Component for Mass {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

fn setup_world_single_component(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(
            e,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    world
}

fn setup_world_two_components(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(
            e,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Velocity {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    world
}

fn setup_world_three_components(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Acceleration>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(
            e,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Velocity {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Acceleration {
                x: 0.1,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    world
}

fn setup_world_five_components(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Acceleration>();
    world.register::<Mass>();
    world.register::<Health>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(
            e,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Velocity {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Acceleration {
                x: 0.1,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(e, Mass { value: 1.0 });
        world.add(
            e,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );
    }

    world
}

fn bench_query_single_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_single_component");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_single_component(size);
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, pos) in world.query::<&Position>() {
                    sum += black_box(pos.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_single_component_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_single_component_mut");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_world_single_component(size),
                |mut world| {
                    for (_entity, pos) in world.query_mut::<&mut Position>() {
                        pos.x += black_box(1.0);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_query_two_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_two_components");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_two_components(size);
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                    sum += black_box(pos.x + vel.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_two_components_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_two_components_mut");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_world_two_components(size),
                |mut world| {
                    for (_entity, (pos, vel)) in
                        world.query_mut::<(&mut Position, &mut Velocity)>()
                    {
                        pos.x += black_box(vel.x);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_query_three_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_three_components");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_three_components(size);
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, (pos, vel, acc)) in
                    world.query::<(&Position, &Velocity, &Acceleration)>()
                {
                    sum += black_box(pos.x + vel.x + acc.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_five_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_five_components");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_five_components(size);
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, (pos, vel, acc, mass, health)) in
                    world.query::<(&Position, &Velocity, &Acceleration, &Mass, &Health)>()
                {
                    sum += black_box(pos.x + vel.x + acc.x + mass.value + health.current);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_physics_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_physics_simulation");

    for size in [1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_world_three_components(size),
                |mut world| {
                    // Simulate a physics update step
                    for (_e, (pos, vel, acc)) in
                        world.query_mut::<(&mut Position, &mut Velocity, &mut Acceleration)>()
                    {
                        vel.x += black_box(acc.x * 0.016); // dt = 16ms
                        vel.y += black_box(acc.y * 0.016);
                        vel.z += black_box(acc.z * 0.016);

                        pos.x += black_box(vel.x * 0.016);
                        pos.y += black_box(vel.y * 0.016);
                        pos.z += black_box(vel.z * 0.016);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_query_sparse_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_sparse_components");

    for size in [1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Position>();
            world.register::<Velocity>();

            // Only 10% of entities have both components
            for i in 0..size {
                let e = world.spawn();
                world.add(
                    e,
                    Position {
                        x: i as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                );
                if i % 10 == 0 {
                    world.add(
                        e,
                        Velocity {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                    );
                }
            }

            b.iter(|| {
                let mut count = 0;
                for (_entity, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
                    count += 1;
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_query_single_component,
    bench_query_single_component_mut,
    bench_query_two_components,
    bench_query_two_components_mut,
    bench_query_three_components,
    bench_query_five_components,
    bench_query_physics_simulation,
    bench_query_sparse_components,
);
criterion_main!(benches);
