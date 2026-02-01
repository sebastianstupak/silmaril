//! World container for managing all ECS data
//!
//! The World owns all entities and their components, providing the main
//! interface for creating entities and managing component data.

use super::change_detection::Tick;
use super::events::{Event, EventReader, Events};
use super::storage::ComponentStorage;
use super::{Component, ComponentDescriptor, Entity, EntityAllocator, SparseSet};
use std::any::TypeId;
use std::collections::HashMap;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// World container - owns all entities and components
///
/// The World is the central data structure in the ECS. It manages:
/// - Entity allocation and lifecycle
/// - Component storage (one SparseSet per component type)
/// - Component registration
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::{World, Component};
/// # #[derive(Debug)]
/// # struct Position { x: f32, y: f32, z: f32 }
/// # impl Component for Position {}
/// let mut world = World::new();
///
/// // Register component types
/// world.register::<Position>();
///
/// // Spawn an entity and add components
/// let entity = world.spawn();
/// world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
///
/// // Query components
/// if let Some(pos) = world.get::<Position>(entity) {
///     println!("Position: {:?}", pos);
/// }
/// ```
pub struct World {
    /// Entity allocator
    entities: EntityAllocator,
    /// Component storage (TypeId → type-erased SparseSet)
    pub(crate) components: HashMap<TypeId, Box<dyn ComponentStorage>>,
    /// Component metadata for debugging
    descriptors: HashMap<TypeId, ComponentDescriptor>,
    /// Current tick for change detection
    current_tick: Tick,
    /// Event storage for inter-system communication
    events: Events,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("entity_count", &self.entities.len())
            .field("component_types", &self.components.len())
            .field("current_tick", &self.current_tick)
            .finish()
    }
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
            descriptors: HashMap::new(),
            current_tick: Tick::new(),
            events: Events::new(),
        }
    }

    /// Register a component type
    ///
    /// This must be called before using a component type. It creates
    /// the storage for that component type.
    ///
    /// It's safe to call multiple times - subsequent calls are no-ops.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut world = World::new();
    /// world.register::<Health>();
    /// ```
    pub fn register<T: Component>(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("component_register", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        if let std::collections::hash_map::Entry::Vacant(e) = self.components.entry(type_id) {
            e.insert(Box::new(SparseSet::<T>::new()) as Box<dyn ComponentStorage>);
            self.descriptors.insert(type_id, ComponentDescriptor::new::<T>());
        }
    }

    /// Spawn a new entity
    ///
    /// Returns an entity handle that can be used to add components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::World;
    /// let mut world = World::new();
    /// let entity = world.spawn();
    /// assert!(world.is_alive(entity));
    /// ```
    #[inline]
    pub fn spawn(&mut self) -> Entity {
        #[cfg(feature = "profiling")]
        profile_scope!("entity_spawn", ProfileCategory::ECS);

        self.entities.allocate()
    }

    /// Despawn an entity (removes all components)
    ///
    /// Returns `true` if the entity was alive and successfully despawned,
    /// `false` if the entity was already dead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::World;
    /// let mut world = World::new();
    /// let entity = world.spawn();
    ///
    /// assert!(world.despawn(entity));
    /// assert!(!world.is_alive(entity));
    /// ```
    #[inline]
    pub fn despawn(&mut self, entity: Entity) -> bool {
        #[cfg(feature = "profiling")]
        profile_scope!("entity_despawn", ProfileCategory::ECS);

        if !self.entities.is_alive(entity) {
            return false;
        }

        // Remove from all component storages using type-erased method
        for storage in self.components.values_mut() {
            storage.remove_entity(entity);
        }

        self.entities.free(entity)
    }

    /// Add a component to an entity
    ///
    /// If the entity already has this component, it will be replaced.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The entity is not alive (debug builds only)
    /// - The component type is not registered (call `register::<T>()` first)
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Debug)]
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut world = World::new();
    /// world.register::<Health>();
    ///
    /// let entity = world.spawn();
    /// world.add(entity, Health { current: 100.0, max: 100.0 });
    /// ```
    #[inline]
    pub fn add<T: Component>(&mut self, entity: Entity, component: T) {
        #[cfg(feature = "profiling")]
        profile_scope!("component_add", ProfileCategory::ECS);

        // Extra defensive: verify entity is alive
        // Note: This check is always active for safety, even in release builds
        assert!(
            self.entities.is_alive(entity),
            "Cannot add component to dead entity {:?}",
            entity
        );

        let type_id = TypeId::of::<T>();
        let storage = self
            .components
            .get_mut(&type_id)
            .unwrap_or_else(|| {
                panic!(
                    "Component type {} not registered. Call world.register::<{}>() first.",
                    std::any::type_name::<T>(),
                    std::any::type_name::<T>()
                )
            })
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("Component storage type mismatch (internal error)");

        storage.insert(entity, component, self.current_tick);
    }

    /// Get a typed storage reference directly (internal use for queries)
    ///
    /// This bypasses the ComponentStorage trait to avoid virtual dispatch overhead.
    /// Returns None if the component type is not registered.
    ///
    /// SAFETY: This is pub(crate) to keep the API clean - only used by query internals.
    #[inline(always)]
    pub(crate) fn get_storage<T: Component>(&self) -> Option<&SparseSet<T>> {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id)?.as_any().downcast_ref::<SparseSet<T>>()
    }

    /// Get an immutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component or
    /// if the component type is not registered.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # struct Position { x: f32, y: f32, z: f32 }
    /// # impl Component for Position {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// # let entity = world.spawn();
    /// # world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
    /// if let Some(pos) = world.get::<Position>(entity) {
    ///     println!("x: {}", pos.x);
    /// }
    /// ```
    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        #[cfg(feature = "profiling")]
        profile_scope!("component_get", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let storage = self.components.get(&type_id)?;
        let storage = storage.as_any().downcast_ref::<SparseSet<T>>()?;
        storage.get(entity)
    }

    /// Get a mutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component or
    /// if the component type is not registered.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// # let mut world = World::new();
    /// # world.register::<Health>();
    /// # let entity = world.spawn();
    /// # world.add(entity, Health { current: 100.0, max: 100.0 });
    /// if let Some(health) = world.get_mut::<Health>(entity) {
    ///     health.current -= 10.0;
    /// }
    /// ```
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        #[cfg(feature = "profiling")]
        profile_scope!("component_get_mut", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?;
        let storage = storage.as_any_mut().downcast_mut::<SparseSet<T>>()?;
        storage.get_mut(entity)
    }

    /// Remove a component from an entity
    ///
    /// Returns `Some(component)` if the entity had the component,
    /// `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// # impl Component for Velocity {}
    /// # let mut world = World::new();
    /// # world.register::<Velocity>();
    /// # let entity = world.spawn();
    /// # world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
    /// let velocity = world.remove::<Velocity>(entity);
    /// assert!(velocity.is_some());
    /// ```
    #[inline]
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        #[cfg(feature = "profiling")]
        profile_scope!("component_remove", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?;
        let storage = storage.as_any_mut().downcast_mut::<SparseSet<T>>()?;
        storage.remove(entity)
    }

    /// Check if an entity has a component
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # struct Name(String);
    /// # impl Component for Name {}
    /// # let mut world = World::new();
    /// # world.register::<Name>();
    /// # let entity = world.spawn();
    /// # world.add(entity, Name("Player".to_string()));
    /// assert!(world.has::<Name>(entity));
    /// ```
    #[inline]
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Check if an entity is alive
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::World;
    /// let mut world = World::new();
    /// let entity = world.spawn();
    ///
    /// assert!(world.is_alive(entity));
    /// world.despawn(entity);
    /// assert!(!world.is_alive(entity));
    /// ```
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// Get the number of alive entities
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::World;
    /// let mut world = World::new();
    /// assert_eq!(world.entity_count(), 0);
    ///
    /// world.spawn();
    /// world.spawn();
    /// assert_eq!(world.entity_count(), 2);
    /// ```
    pub fn entity_count(&self) -> usize {
        self.entities.alive_count()
    }

    /// Clear all entities and components
    ///
    /// This removes all data from the world but keeps component registrations.
    pub fn clear(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("world_clear", ProfileCategory::ECS);

        self.entities.clear();
        for storage in self.components.values_mut() {
            storage.clear();
        }
    }

    /// Iterate all alive entities
    ///
    /// This is useful for serialization and debugging.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::World;
    /// let mut world = World::new();
    /// world.spawn();
    /// world.spawn();
    ///
    /// let count = world.entities().count();
    /// assert_eq!(count, 2);
    /// ```
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.entities()
    }

    /// Spawn an entity with a specific ID and generation (for deserialization)
    ///
    /// This is used when restoring world state from a snapshot.
    ///
    /// # Panics
    ///
    /// Panics if the entity is already alive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Entity};
    /// let mut world = World::new();
    /// let entity = Entity::new(42, 5);
    /// world.spawn_with_id(entity);
    /// assert!(world.is_alive(entity));
    /// ```
    pub fn spawn_with_id(&mut self, entity: Entity) {
        #[cfg(feature = "profiling")]
        profile_scope!("entity_spawn_with_id", ProfileCategory::ECS);

        self.entities.allocate_with_id(entity);
    }

    /// Get all components for an entity (for serialization)
    ///
    /// Returns a vector of ComponentData containing all components
    /// attached to the entity. Used for world state snapshots.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # let world = World::new();
    /// # let entity = world.entities().next().unwrap();
    /// let components = world.get_all_components(entity);
    /// println!("Entity has {} components", components.len());
    /// ```
    pub fn get_all_components(&self, entity: Entity) -> Vec<crate::serialization::ComponentData> {
        #[cfg(feature = "profiling")]
        profile_scope!("world_get_all_components", ProfileCategory::ECS);

        let mut result = Vec::new();

        // Check each registered component type
        for storage in self.components.values() {
            if let Some(component_data) = storage.get_component_data(entity) {
                result.push(component_data);
            }
        }

        result
    }

    /// Add a component from ComponentData enum (for deserialization)
    ///
    /// This is used when restoring world state from a snapshot.
    ///
    /// # Panics
    ///
    /// Panics if the entity is not alive or if the component type is not registered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_core::serialization::ComponentData;
    /// # use engine_core::math::Transform;
    /// # let mut world = World::new();
    /// # let entity = world.spawn();
    /// # world.register::<Transform>();
    /// let component_data = ComponentData::Transform(Transform::default());
    /// world.add_component_data(entity, component_data);
    /// ```
    pub fn add_component_data(
        &mut self,
        entity: Entity,
        component_data: crate::serialization::ComponentData,
    ) {
        #[cfg(feature = "profiling")]
        profile_scope!("world_add_component_data", ProfileCategory::ECS);

        match component_data {
            crate::serialization::ComponentData::Transform(t) => {
                self.add(entity, t);
            }
            crate::serialization::ComponentData::Health(h) => {
                self.add(entity, h);
            }
            crate::serialization::ComponentData::Velocity(v) => {
                self.add(entity, v);
            }
            crate::serialization::ComponentData::MeshRenderer(m) => {
                self.add(entity, m);
            }
        }
    }

    /// Get component descriptor for debugging
    ///
    /// Returns `None` if the component type is not registered.
    pub fn get_component_descriptor<T: Component>(&self) -> Option<&ComponentDescriptor> {
        let type_id = TypeId::of::<T>();
        self.descriptors.get(&type_id)
    }

    /// Check if an entity has a component by TypeId (internal use for query filters)
    ///
    /// This is less efficient than the typed `has<T>()` method, but allows
    /// checking component existence without knowing the component type at compile time.
    #[allow(dead_code)]
    pub(crate) fn has_component_by_id(&self, entity: Entity, type_id: TypeId) -> bool {
        self.components
            .get(&type_id)
            .map(|storage| storage.contains_entity(entity))
            .unwrap_or(false)
    }

    // ========================================================================
    // Change Detection
    // ========================================================================

    /// Get the current tick
    ///
    /// The tick is incremented each time `increment_tick()` is called,
    /// typically between system executions.
    #[inline]
    pub fn current_tick(&self) -> Tick {
        self.current_tick
    }

    /// Increment the tick counter
    ///
    /// This should be called between system executions to enable change detection.
    /// Systems can track which tick they last ran and query only components
    /// that changed since then.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Component)]
    /// # struct Transform { x: f32 }
    /// let mut world = World::new();
    /// world.register::<Transform>();
    ///
    /// let entity = world.spawn();
    /// world.add(entity, Transform { x: 0.0 });
    ///
    /// // Simulate system execution
    /// world.increment_tick();
    ///
    /// // Components added after this tick will be detected as changed
    /// let entity2 = world.spawn();
    /// world.add(entity2, Transform { x: 1.0 });
    /// ```
    #[inline]
    pub fn increment_tick(&mut self) {
        self.current_tick.increment();
    }

    /// Mark a component as changed
    ///
    /// This is used when getting mutable access to a component to track
    /// that it has been modified.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Component)]
    /// # struct Transform { x: f32 }
    /// let mut world = World::new();
    /// world.register::<Transform>();
    ///
    /// let entity = world.spawn();
    /// world.add(entity, Transform { x: 0.0 });
    ///
    /// // Modify component and mark as changed
    /// if let Some(transform) = world.get_mut::<Transform>(entity) {
    ///     transform.x = 10.0;
    /// }
    /// world.mark_changed::<Transform>(entity);
    /// ```
    pub fn mark_changed<T: Component>(&mut self, entity: Entity) {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.components.get_mut(&type_id) {
            let storage = storage
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect("Component storage type mismatch");
            storage.mark_changed(entity, self.current_tick);
        }
    }

    // ========================================================================
    // Event System
    // ========================================================================

    /// Send an event
    ///
    /// Events can be used for inter-system communication without tight coupling.
    /// Multiple systems can read the same events.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine_core::ecs::{World, Event};
    /// # #[derive(Debug, Clone)]
    /// # struct CollisionEvent { entity_a: u64, entity_b: u64 }
    /// # impl Event for CollisionEvent {}
    /// let mut world = World::new();
    ///
    /// // Physics system sends collision event
    /// world.send_event(CollisionEvent { entity_a: 1, entity_b: 2 });
    /// ```
    pub fn send_event<E: Event>(&mut self, event: E) {
        self.events.send(event);
    }

    /// Get an event reader for a specific event type
    ///
    /// Event readers track which events have been read, allowing multiple
    /// systems to independently process events.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine_core::ecs::{World, Event};
    /// # #[derive(Debug, Clone)]
    /// # struct CollisionEvent { entity_a: u64, entity_b: u64 }
    /// # impl Event for CollisionEvent {}
    /// let world = World::new();
    /// let mut reader = world.get_event_reader::<CollisionEvent>();
    /// ```
    pub fn get_event_reader<E: Event>(&self) -> EventReader<E> {
        self.events.get_reader()
    }

    /// Read events with a reader
    ///
    /// Returns an iterator over all unread events of the specified type.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine_core::ecs::{World, Event};
    /// # #[derive(Debug, Clone)]
    /// # struct CollisionEvent { entity_a: u64, entity_b: u64 }
    /// # impl Event for CollisionEvent {}
    /// let mut world = World::new();
    /// world.send_event(CollisionEvent { entity_a: 1, entity_b: 2 });
    ///
    /// let mut reader = world.get_event_reader::<CollisionEvent>();
    /// for event in world.read_events(&mut reader) {
    ///     println!("Collision: {} and {}", event.entity_a, event.entity_b);
    /// }
    /// ```
    pub fn read_events<'a, E: Event>(
        &'a self,
        reader: &mut EventReader<E>,
    ) -> impl Iterator<Item = &'a E> + 'a {
        self.events.read(reader)
    }

    /// Clear all events of a specific type
    pub fn clear_events<E: Event>(&mut self) {
        self.events.clear::<E>();
    }

    /// Clear all events
    pub fn clear_all_events(&mut self) {
        self.events.clear_all();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Transform {
        x: f32,
        y: f32,
        z: f32,
    }

    impl Component for Transform {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Health {
        current: f32,
        max: f32,
    }

    impl Component for Health {}

    #[test]
    fn test_world_spawn_despawn() {
        let mut world = World::new();
        let entity = world.spawn();

        assert!(world.is_alive(entity));

        world.despawn(entity);

        assert!(!world.is_alive(entity));
    }

    #[test]
    fn test_world_add_get_component() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform { x: 1.0, y: 2.0, z: 3.0 });

        assert!(world.get::<Transform>(entity).is_some());
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.x, 1.0);
    }

    #[test]
    #[should_panic(expected = "Cannot add component to dead entity")]
    fn test_world_add_to_dead_entity_panics() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.despawn(entity);

        world.add(entity, Transform { x: 0.0, y: 0.0, z: 0.0 }); // Should panic
    }

    #[test]
    #[should_panic(expected = "not registered")]
    fn test_world_add_unregistered_component_panics() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, Transform { x: 0.0, y: 0.0, z: 0.0 }); // Should panic - not registered
    }

    #[test]
    fn test_world_remove_component() {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, Health { current: 100.0, max: 100.0 });

        let removed = world.remove::<Health>(entity);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().current, 100.0);
        assert!(world.get::<Health>(entity).is_none());
    }

    #[test]
    fn test_world_has_component() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();

        assert!(!world.has::<Transform>(entity));

        world.add(entity, Transform { x: 0.0, y: 0.0, z: 0.0 });

        assert!(world.has::<Transform>(entity));
    }

    #[test]
    fn test_world_get_mut() {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, Health { current: 100.0, max: 100.0 });

        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }

        assert_eq!(world.get::<Health>(entity).unwrap().current, 50.0);
    }

    #[test]
    fn test_world_multiple_components() {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, Transform { x: 1.0, y: 2.0, z: 3.0 });
        world.add(entity, Health { current: 100.0, max: 100.0 });

        assert!(world.has::<Transform>(entity));
        assert!(world.has::<Health>(entity));
    }

    #[test]
    fn test_world_entity_count() {
        let mut world = World::new();

        assert_eq!(world.entity_count(), 0);

        let e1 = world.spawn();
        let e2 = world.spawn();

        assert_eq!(world.entity_count(), 2);

        world.despawn(e1);

        assert_eq!(world.entity_count(), 1);

        world.despawn(e2);

        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_world_register_idempotent() {
        let mut world = World::new();

        world.register::<Transform>();
        world.register::<Transform>(); // Should not panic

        let entity = world.spawn();
        world.add(entity, Transform { x: 1.0, y: 2.0, z: 3.0 });

        assert!(world.has::<Transform>(entity));
    }

    #[test]
    fn test_world_component_descriptor() {
        let mut world = World::new();
        world.register::<Transform>();

        let descriptor = world.get_component_descriptor::<Transform>();
        assert!(descriptor.is_some());
        assert_eq!(descriptor.unwrap().type_id, TypeId::of::<Transform>());
    }
}
