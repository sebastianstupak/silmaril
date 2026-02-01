use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::{Component, World};

// Test components
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
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

fn bench_world_spawn(c: &mut Criterion) {
    c.bench_function("world_spawn", |b| {
        let mut world = World::new();
        b.iter(|| {
            black_box(world.spawn());
        });
    });
}

fn bench_world_add_component(c: &mut Criterion) {
    c.bench_function("world_add_component", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                let entity = world.spawn();
                (world, entity)
            },
            |(mut world, entity)| {
                world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_world_get_component(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Position>();
    let entity = world.spawn();
    world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });

    c.bench_function("world_get_component", |b| {
        b.iter(|| {
            black_box(world.get::<Position>(entity));
        });
    });
}

fn bench_world_get_mut_component(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Position>();
    let entity = world.spawn();
    world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });

    c.bench_function("world_get_mut_component", |b| {
        b.iter(|| {
            if let Some(pos) = world.get_mut::<Position>(entity) {
                pos.x = black_box(pos.x + 1.0);
            }
        });
    });
}

fn bench_world_remove_component(c: &mut Criterion) {
    c.bench_function("world_remove_component", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                let entity = world.spawn();
                world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
                (world, entity)
            },
            |(mut world, entity)| {
                black_box(world.remove::<Position>(entity));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_world_despawn(c: &mut Criterion) {
    c.bench_function("world_despawn", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                let entity = world.spawn();
                world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                (world, entity)
            },
            |(mut world, entity)| {
                black_box(world.despawn(entity));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_world_has_component(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Position>();
    let entity = world.spawn();
    world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });

    c.bench_function("world_has_component", |b| {
        b.iter(|| {
            black_box(world.has::<Position>(entity));
        });
    });
}

fn bench_world_is_alive(c: &mut Criterion) {
    let mut world = World::new();
    let entity = world.spawn();

    c.bench_function("world_is_alive", |b| {
        b.iter(|| {
            black_box(world.is_alive(entity));
        });
    });
}

fn bench_world_add_multiple_components(c: &mut Criterion) {
    c.bench_function("world_add_3_components", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                world.register::<Health>();
                let entity = world.spawn();
                (world, entity)
            },
            |(mut world, entity)| {
                world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                world.add(entity, Health { current: 100.0, max: 100.0 });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_world_spawn_with_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_with_components");

    for entity_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                b.iter(|| {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();

                    for i in 0..count {
                        let e = world.spawn();
                        world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
                        world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_world_component_access_pattern(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create 1000 entities with components
    let entities: Vec<_> = (0..1000)
        .map(|i| {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            e
        })
        .collect();

    c.bench_function("world_random_component_access", |b| {
        let mut idx = 0;
        b.iter(|| {
            let entity = entities[idx % entities.len()];
            if let Some(pos) = world.get::<Position>(entity) {
                black_box(pos);
            }
            idx += 1;
        });
    });
}

// Removed bench_world_get_storage as get_storage is pub(crate)

criterion_group!(
    benches,
    bench_world_spawn,
    bench_world_add_component,
    bench_world_get_component,
    bench_world_get_mut_component,
    bench_world_remove_component,
    bench_world_despawn,
    bench_world_has_component,
    bench_world_is_alive,
    bench_world_add_multiple_components,
    bench_world_spawn_with_components,
    bench_world_component_access_pattern
);

criterion_main!(benches);
