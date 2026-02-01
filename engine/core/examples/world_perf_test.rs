use engine_core::ecs::{Component, World};
use std::time::Instant;

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

fn main() {
    const ITERATIONS: usize = 100_000;

    // Test spawn
    let start = Instant::now();
    let mut world = World::new();
    for _ in 0..ITERATIONS {
        world.spawn();
    }
    let spawn_time = start.elapsed();
    println!("spawn: {:?} ({:.2} ns/op)", spawn_time, spawn_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test add component
    world.register::<Position>();
    let entities: Vec<_> = (0..ITERATIONS).map(|_| world.spawn()).collect();

    let start = Instant::now();
    for &entity in &entities {
        world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
    }
    let add_time = start.elapsed();
    println!("add: {:?} ({:.2} ns/op)", add_time, add_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test get component
    let start = Instant::now();
    for &entity in &entities {
        let _ = world.get::<Position>(entity);
    }
    let get_time = start.elapsed();
    println!("get: {:?} ({:.2} ns/op)", get_time, get_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test get_mut component
    let start = Instant::now();
    for &entity in &entities {
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x += 1.0;
        }
    }
    let get_mut_time = start.elapsed();
    println!("get_mut: {:?} ({:.2} ns/op)", get_mut_time, get_mut_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test remove component
    let start = Instant::now();
    for &entity in &entities {
        world.remove::<Position>(entity);
    }
    let remove_time = start.elapsed();
    println!("remove: {:?} ({:.2} ns/op)", remove_time, remove_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test despawn
    world.register::<Velocity>();
    let entities: Vec<_> = (0..ITERATIONS).map(|i| {
        let e = world.spawn();
        world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        e
    }).collect();

    let start = Instant::now();
    for &entity in &entities {
        world.despawn(entity);
    }
    let despawn_time = start.elapsed();
    println!("despawn: {:?} ({:.2} ns/op)", despawn_time, despawn_time.as_nanos() as f64 / ITERATIONS as f64);
}
