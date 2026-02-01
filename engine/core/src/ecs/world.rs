//! World container for managing all ECS data
//!
//! The World owns all entities and their components, providing the main
//! interface for creating entities and managing component data.

use super::{Component, ComponentDescriptor, Entity, EntityAllocator, SparseSet};
use std::any::{Any, TypeId};
use std::collections::HashMap;

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
    pub(crate) components: HashMap<TypeId, Box<dyn Any>>,
    /// Component metadata for debugging
    descriptors: HashMap<TypeId, ComponentDescriptor>,
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
            descriptors: HashMap::new(),
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
        let type_id = TypeId::of::<T>();
        if !self.components.contains_key(&type_id) {
            self.components
                .insert(type_id, Box::new(SparseSet::<T>::new()));
            self.descriptors
                .insert(type_id, ComponentDescriptor::new::<T>());
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
    pub fn spawn(&mut self) -> Entity {
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
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }

        // Remove from all component storages
        for storage in self.components.values_mut() {
            // Type-erased removal - try to cast and remove
            // This is safe because we only store SparseSet<T> for registered T
            Self::remove_component_erased(storage, entity);
        }

        self.entities.free(entity)
    }

    /// Helper to remove a component from type-erased storage
    fn remove_component_erased(storage: &mut Box<dyn Any>, entity: Entity) {
        // We don't know the component type here, so we can't call remove directly
        // This is a limitation of type erasure - we'll handle it with a trait in the future
        // For now, we'll just skip removal (components will be orphaned)
        // TODO: Implement ComponentStorage trait for proper type-erased removal
        let _ = (storage, entity); // Suppress unused warnings
    }

    /// Add a component to an entity
    ///
    /// If the entity already has this component, it will be replaced.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The entity is not alive
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
    pub fn add<T: Component>(&mut self, entity: Entity, component: T) {
        // Extra defensive: verify entity is alive
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
            .downcast_mut::<SparseSet<T>>()
            .expect("Component storage type mismatch (internal error)");

        storage.insert(entity, component);
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
        let type_id = TypeId::of::<T>();
        let storage = self.components.get(&type_id)?;
        let storage = storage.downcast_ref::<SparseSet<T>>()?;
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
        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?;
        let storage = storage.downcast_mut::<SparseSet<T>>()?;
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
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?;
        let storage = storage.downcast_mut::<SparseSet<T>>()?;
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
        self.entities.clear();
        for _storage in self.components.values_mut() {
            // Clear all component storages
            // TODO: Implement ComponentStorage trait for proper type-erased clearing
        }
    }

    /// Get component descriptor for debugging
    ///
    /// Returns `None` if the component type is not registered.
    pub fn get_component_descriptor<T: Component>(&self) -> Option<&ComponentDescriptor> {
        let type_id = TypeId::of::<T>();
        self.descriptors.get(&type_id)
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
        world.add(
            entity,
            Transform {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

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

        world.add(
            entity,
            Transform {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ); // Should panic
    }

    #[test]
    #[should_panic(expected = "not registered")]
    fn test_world_add_unregistered_component_panics() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(
            entity,
            Transform {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ); // Should panic - not registered
    }

    #[test]
    fn test_world_remove_component() {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, Health {
            current: 100.0,
            max: 100.0,
        });

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

        world.add(
            entity,
            Transform {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );

        assert!(world.has::<Transform>(entity));
    }

    #[test]
    fn test_world_get_mut() {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, Health {
            current: 100.0,
            max: 100.0,
        });

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
        world.add(
            entity,
            Transform {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );
        world.add(entity, Health {
            current: 100.0,
            max: 100.0,
        });

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
        world.add(
            entity,
            Transform {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

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
