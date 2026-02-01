use engine_core::ecs::{World, Component};

#[derive(Debug)]
struct Position { x: f32, y: f32 }
impl Component for Position {}

#[derive(Debug)]
struct Alive;
impl Component for Alive {}

fn main() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Alive>();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Alive);

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });
    world.add(e3, Alive);

    println!("Querying with .with::<Alive>() filter:");
    for (entity, pos) in world.query::<&Position>().with::<Alive>() {
        println!("  Entity {:?} at ({}, {})", entity, pos.x, pos.y);
    }
}
