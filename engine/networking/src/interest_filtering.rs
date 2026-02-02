//! Interest-based filtering for network updates
//!
//! This module integrates the interest management system with
//! the networking layer to filter entity updates based on client visibility.

use engine_core::ecs::{Entity, World};
use engine_core::Vec3;
use engine_interest::{AreaOfInterest, InterestManager};

/// Interest filter for network updates
///
/// Wraps InterestManager to provide filtering for network packets.
/// This reduces bandwidth by only sending relevant entity updates to each client.
///
/// # Architecture
///
/// - Maintains an InterestManager instance
/// - Provides filtering methods for entity lists
/// - Computes bandwidth reduction metrics
///
/// # Usage
///
/// ```no_run
/// use engine_networking::InterestFilter;
/// use engine_core::{World, Vec3};
/// use engine_interest::AreaOfInterest;
///
/// let mut world = World::new();
/// let mut filter = InterestFilter::new(50.0);
///
/// // Update from world state
/// filter.update_from_world(&world);
///
/// // Register clients
/// filter.register_client(1, Vec3::ZERO, 100.0);
///
/// // Filter entities for a specific client
/// let all_entities = vec![]; // Your entities
/// let visible = filter.filter_updates(1, &all_entities);
/// ```
pub struct InterestFilter {
    manager: InterestManager,
}

impl InterestFilter {
    /// Create a new interest filter
    ///
    /// # Arguments
    ///
    /// * `cell_size` - Spatial grid cell size (world units)
    pub fn new(cell_size: f32) -> Self {
        Self { manager: InterestManager::new(cell_size) }
    }

    /// Update spatial grid from world state
    ///
    /// Should be called every frame/tick before filtering.
    pub fn update_from_world(&mut self, world: &World) {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!(
            "interest_filter_update",
            agent_game_engine_profiling::ProfileCategory::Networking
        );

        self.manager.update_from_world(world);
    }

    /// Register a client with an area of interest
    ///
    /// # Arguments
    ///
    /// * `client_id` - Unique client identifier
    /// * `position` - Client's current position
    /// * `radius` - AOI radius in world units
    pub fn register_client(&mut self, client_id: u64, position: Vec3, radius: f32) {
        let aoi = AreaOfInterest::new(position, radius);
        self.manager.set_client_interest(client_id, aoi);
    }

    /// Update a client's position
    ///
    /// This updates the AOI center without changing the radius.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Client to update
    /// * `new_position` - New center position
    pub fn update_client_position(&mut self, client_id: u64, new_position: Vec3) {
        if let Some(mut aoi) = self.manager.get_client_aoi(client_id) {
            aoi.set_center(new_position);
            self.manager.set_client_interest(client_id, aoi);
        }
    }

    /// Unregister a client
    ///
    /// Should be called when a client disconnects.
    pub fn unregister_client(&mut self, client_id: u64) {
        self.manager.clear_client(client_id);
    }

    /// Filter entity updates for a specific client
    ///
    /// Returns only entities that are visible to the client.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Client to filter for
    /// * `all_entities` - All entities to consider
    ///
    /// # Returns
    ///
    /// Filtered list of visible entities
    pub fn filter_updates(&self, client_id: u64, all_entities: &[Entity]) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!(
            "interest_filter_entities",
            agent_game_engine_profiling::ProfileCategory::Networking
        );

        // If no filtering registered for client, return all
        if !self.manager.has_client_interest(client_id) {
            return all_entities.to_vec();
        }

        // Get visible entities
        let visible_set: std::collections::HashSet<Entity> =
            self.manager.calculate_visibility(client_id).into_iter().collect();

        // Filter input entities
        all_entities.iter().filter(|e| visible_set.contains(e)).copied().collect()
    }

    /// Get visibility changes for a client
    ///
    /// Returns entities that entered or exited the AOI since last call.
    /// Updates internal cache.
    ///
    /// # Returns
    ///
    /// Tuple of (entered, exited) entity vectors
    pub fn get_visibility_changes(&mut self, client_id: u64) -> (Vec<Entity>, Vec<Entity>) {
        self.manager.get_visibility_changes(client_id)
    }

    /// Compute bandwidth reduction metrics
    ///
    /// # Arguments
    ///
    /// * `clients` - Number of clients (for simulation)
    /// * `entities` - Number of entities (for simulation)
    ///
    /// # Returns
    ///
    /// Tuple of (without_interest, with_interest, reduction_percentage)
    ///
    /// # Example
    ///
    /// ```text
    /// (100000, 5000, 95.0) means:
    /// - Without interest: 100,000 entity updates per tick
    /// - With interest: 5,000 entity updates per tick
    /// - Reduction: 95%
    /// ```
    pub fn compute_bandwidth_reduction(
        &self,
        clients: usize,
        entities: usize,
    ) -> (usize, usize, f32) {
        // If using actual data from manager
        if self.manager.client_count() > 0 && self.manager.entity_count() > 0 {
            return self.manager.compute_bandwidth_reduction();
        }

        // Simulation mode (for testing)
        let without = clients * entities;
        let avg_visible = self.manager.average_visible_entities();
        let with = if avg_visible > 0.0 {
            (clients as f32 * avg_visible) as usize
        } else {
            // Assume 10% visibility if no data
            clients * entities / 10
        };

        let reduction =
            if without > 0 { (1.0 - (with as f32 / without as f32)) * 100.0 } else { 0.0 };

        (without, with, reduction)
    }

    /// Get statistics
    pub fn stats(&self) -> InterestFilterStats {
        InterestFilterStats {
            client_count: self.manager.client_count(),
            entity_count: self.manager.entity_count(),
            avg_visible_entities: self.manager.average_visible_entities(),
        }
    }

    /// Access the underlying interest manager (for advanced use cases)
    pub fn manager(&self) -> &InterestManager {
        &self.manager
    }

    /// Mutable access to the underlying interest manager
    pub fn manager_mut(&mut self) -> &mut InterestManager {
        &mut self.manager
    }
}

impl Default for InterestFilter {
    fn default() -> Self {
        Self::new(50.0)
    }
}

/// Interest filter statistics
#[derive(Debug, Clone, Copy)]
pub struct InterestFilterStats {
    /// Number of registered clients
    pub client_count: usize,
    /// Total entities in spatial grid
    pub entity_count: usize,
    /// Average entities visible per client
    pub avg_visible_entities: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::{Aabb, World};

    #[test]
    fn test_interest_filter_basic() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn entities
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new((i as f32) * 10.0, 0.0, 0.0);
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        }

        let mut filter = InterestFilter::new(50.0);
        filter.update_from_world(&world);

        // Register client
        filter.register_client(1, Vec3::ZERO, 50.0);

        let stats = filter.stats();
        assert_eq!(stats.client_count, 1);
        assert_eq!(stats.entity_count, 10);
    }

    #[test]
    fn test_filter_updates() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn entities at different distances
        let mut all_entities = Vec::new();

        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new((i as f32) * 20.0, 0.0, 0.0);
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
            all_entities.push(entity);
        }

        let mut filter = InterestFilter::new(50.0);
        filter.update_from_world(&world);

        // Register client at origin with 50 unit radius
        filter.register_client(1, Vec3::ZERO, 50.0);

        // Filter should only return nearby entities
        let visible = filter.filter_updates(1, &all_entities);

        assert!(visible.len() < 10, "Should filter some entities");
        assert!(visible.len() > 0, "Should have some visible entities");
    }

    #[test]
    fn test_visibility_changes() {
        let mut world = World::new();
        world.register::<Aabb>();

        let entity = world.spawn();
        world.add(entity, Aabb::from_center_half_extents(Vec3::new(10.0, 0.0, 0.0), Vec3::ONE));

        let mut filter = InterestFilter::new(50.0);
        filter.update_from_world(&world);
        filter.register_client(1, Vec3::ZERO, 50.0);

        // First call - entity should enter
        let (entered, exited) = filter.get_visibility_changes(1);
        assert_eq!(entered.len(), 1);
        assert_eq!(exited.len(), 0);

        // Second call - no changes
        let (entered, exited) = filter.get_visibility_changes(1);
        assert_eq!(entered.len(), 0);
        assert_eq!(exited.len(), 0);
    }

    #[test]
    fn test_bandwidth_reduction() {
        let filter = InterestFilter::new(50.0);

        // Simulated: 100 clients, 1000 entities
        let (without, with, reduction) = filter.compute_bandwidth_reduction(100, 1000);

        assert_eq!(without, 100_000); // 100 × 1000
        assert!(with < without, "With interest should be less");
        assert!(reduction > 0.0, "Should have some reduction");
    }

    #[test]
    fn test_update_client_position() {
        let mut filter = InterestFilter::new(50.0);

        filter.register_client(1, Vec3::ZERO, 100.0);

        // Update position
        filter.update_client_position(1, Vec3::new(50.0, 0.0, 0.0));

        // Client should still be registered
        let stats = filter.stats();
        assert_eq!(stats.client_count, 1);
    }

    #[test]
    fn test_unregister_client() {
        let mut filter = InterestFilter::new(50.0);

        filter.register_client(1, Vec3::ZERO, 100.0);
        assert_eq!(filter.stats().client_count, 1);

        filter.unregister_client(1);
        assert_eq!(filter.stats().client_count, 0);
    }
}
