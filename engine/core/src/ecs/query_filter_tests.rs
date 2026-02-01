//! Tests for query filters (.with() and .without())

#[cfg(test)]
mod tests {
    use crate::ecs::{Component, World};

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
        current: f32,
        max: f32,
    }
    impl Component for Health {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Alive;
    impl Component for Alive {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Dead;
    impl Component for Dead {}

    #[test]
    fn test_query_with_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Alive>();

        // Create entities - some alive, some not
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        world.add(e1, Alive);

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        // e2 does NOT have Alive

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Alive);

        // Query for Position with Alive filter
        let mut count = 0;
        for (entity, pos) in world.query::<&Position>().with::<Alive>() {
            assert!(entity == e1 || entity == e3);
            assert!(pos.x == 1.0 || pos.x == 3.0);
            count += 1;
        }

        assert_eq!(count, 2); // Only e1 and e3
    }

    #[test]
    fn test_query_without_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Dead>();

        // Create entities
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        // e1 is NOT dead

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Dead);

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        // e3 is NOT dead

        // Query for Position without Dead filter
        let mut count = 0;
        for (entity, pos) in world.query::<&Position>().without::<Dead>() {
            assert!(entity == e1 || entity == e3);
            assert!(pos.x == 1.0 || pos.x == 3.0);
            count += 1;
        }

        assert_eq!(count, 2); // Only e1 and e3 (not dead)
    }

    #[test]
    fn test_query_chained_filters() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Alive>();
        world.register::<Health>();

        // Create entities with different component combinations
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        world.add(e1, Alive);
        world.add(e1, Health { current: 100.0, max: 100.0 });

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Alive);
        // e2 does NOT have Health

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Health { current: 50.0, max: 100.0 });
        // e3 does NOT have Alive

        let e4 = world.spawn();
        world.add(e4, Position { x: 4.0, y: 4.0 });
        // e4 has neither Alive nor Health

        // Query for Position with Alive and WITHOUT Health
        let mut count = 0;
        for (entity, pos) in world.query::<&Position>().with::<Alive>().without::<Health>() {
            assert_eq!(entity, e2);
            assert_eq!(pos.x, 2.0);
            count += 1;
        }

        assert_eq!(count, 1); // Only e2
    }

    #[test]
    fn test_query_multi_component_with_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Alive>();

        // Create entities
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        world.add(e1, Velocity { x: 0.1, y: 0.1 });
        world.add(e1, Alive);

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Velocity { x: 0.2, y: 0.2 });
        // e2 does NOT have Alive

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Velocity { x: 0.3, y: 0.3 });
        world.add(e3, Alive);

        // Query for (Position, Velocity) with Alive filter
        let mut count = 0;
        for (entity, (pos, vel)) in world.query::<(&Position, &Velocity)>().with::<Alive>() {
            assert!(entity == e1 || entity == e3);
            assert!(pos.x == 1.0 || pos.x == 3.0);
            assert!(vel.x == 0.1 || vel.x == 0.3);
            count += 1;
        }

        assert_eq!(count, 2); // Only e1 and e3
    }

    #[test]
    fn test_query_multi_component_without_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Dead>();

        // Create entities
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        world.add(e1, Velocity { x: 0.1, y: 0.1 });
        // e1 is NOT dead

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Velocity { x: 0.2, y: 0.2 });
        world.add(e2, Dead);

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Velocity { x: 0.3, y: 0.3 });
        // e3 is NOT dead

        // Query for (Position, Velocity) without Dead filter
        let mut count = 0;
        for (entity, (pos, vel)) in world.query::<(&Position, &Velocity)>().without::<Dead>() {
            assert!(entity == e1 || entity == e3);
            assert!(pos.x == 1.0 || pos.x == 3.0);
            assert!(vel.x == 0.1 || vel.x == 0.3);
            count += 1;
        }

        assert_eq!(count, 2); // Only e1 and e3
    }

    #[test]
    fn test_query_multiple_with_filters() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Alive>();
        world.register::<Health>();

        // Create entities
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        world.add(e1, Alive);
        world.add(e1, Health { current: 100.0, max: 100.0 });

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Alive);
        // e2 does NOT have Health

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Health { current: 50.0, max: 100.0 });
        // e3 does NOT have Alive

        // Query for Position with both Alive AND Health
        let mut count = 0;
        for (entity, pos) in world.query::<&Position>().with::<Alive>().with::<Health>() {
            assert_eq!(entity, e1);
            assert_eq!(pos.x, 1.0);
            count += 1;
        }

        assert_eq!(count, 1); // Only e1
    }

    #[test]
    fn test_query_multiple_without_filters() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Dead>();
        world.register::<Health>();

        // Create entities
        let e1 = world.spawn();
        world.add(e1, Position { x: 1.0, y: 1.0 });
        // e1 has neither Dead nor Health

        let e2 = world.spawn();
        world.add(e2, Position { x: 2.0, y: 2.0 });
        world.add(e2, Dead);

        let e3 = world.spawn();
        world.add(e3, Position { x: 3.0, y: 3.0 });
        world.add(e3, Health { current: 50.0, max: 100.0 });

        let e4 = world.spawn();
        world.add(e4, Position { x: 4.0, y: 4.0 });
        world.add(e4, Dead);
        world.add(e4, Health { current: 25.0, max: 100.0 });

        // Query for Position without Dead AND without Health
        let mut count = 0;
        for (entity, pos) in world.query::<&Position>().without::<Dead>().without::<Health>() {
            assert_eq!(entity, e1);
            assert_eq!(pos.x, 1.0);
            count += 1;
        }

        assert_eq!(count, 1); // Only e1
    }

    #[test]
    fn test_query_empty_with_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Alive>();

        // Create entities without Alive
        let _e1 = world.spawn();
        world.add(_e1, Position { x: 1.0, y: 1.0 });

        let _e2 = world.spawn();
        world.add(_e2, Position { x: 2.0, y: 2.0 });

        // Query with filter should return nothing
        let count = world.query::<&Position>().with::<Alive>().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_empty_without_filter() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Dead>();

        // Create entities ALL with Dead
        let _e1 = world.spawn();
        world.add(_e1, Position { x: 1.0, y: 1.0 });
        world.add(_e1, Dead);

        let _e2 = world.spawn();
        world.add(_e2, Position { x: 2.0, y: 2.0 });
        world.add(_e2, Dead);

        // Query without Dead should return nothing
        let count = world.query::<&Position>().without::<Dead>().count();
        assert_eq!(count, 0);
    }
}
