//! Interest Manager - Client visibility tracking and AOI management

use engine_core::ecs::{Entity, World};
use engine_core::math::Vec3;
use engine_core::spatial::{Aabb, SpatialGrid, SpatialGridConfig};
use std::collections::{HashMap, HashSet};

/// Area of Interest for a client.
///
/// Defines the region around a client where entities are visible.
/// Entities outside this region are not sent to the client.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaOfInterest {
    /// Center position (typically the player's position)
    pub center: Vec3,
    /// Radius of the AOI in world units
    pub radius: f32,
}

impl AreaOfInterest {
    /// Create a new circular AOI
    #[inline]
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Check if a position is within this AOI
    #[inline]
    pub fn contains(&self, position: Vec3) -> bool {
        (position - self.center).length_squared() <= self.radius * self.radius
    }

    /// Update the center position
    #[inline]
    pub fn set_center(&mut self, center: Vec3) {
        self.center = center;
    }
}

impl Default for AreaOfInterest {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 100.0)
    }
}

/// Visibility change information for a client
#[derive(Debug, Clone)]
pub struct VisibilityChange {
    /// Entities that entered the AOI
    pub entered: Vec<Entity>,
    /// Entities that exited the AOI
    pub exited: Vec<Entity>,
}

impl VisibilityChange {
    /// Check if there are any changes
    #[inline]
    pub fn has_changes(&self) -> bool {
        !self.entered.is_empty() || !self.exited.is_empty()
    }
}

/// Interest Manager - Tracks client visibility and manages AOIs
///
/// This is the main entry point for interest management. It wraps
/// the spatial grid from engine-core and adds per-client tracking.
///
/// # Architecture
///
/// - Uses SpatialGrid for efficient spatial queries
/// - Tracks per-client visibility sets
/// - Detects enter/exit events for bandwidth optimization
///
/// # Performance
///
/// - O(1) amortized entity updates (grid cell changes)
/// - O(k) visibility calculation where k = entities in AOI
/// - Target: <1ms per client for 1K entities
pub struct InterestManager {
    /// Spatial grid for entity partitioning
    spatial_grid: SpatialGrid,
    /// Client AOIs (client_id -> AOI)
    pub(crate) client_interests: HashMap<u64, AreaOfInterest>,
    /// Cached visibility per client (client_id -> entity set)
    visibility_cache: HashMap<u64, HashSet<Entity>>,
    /// Entity positions (for distance checks)
    entity_positions: HashMap<Entity, Vec3>,
}

impl InterestManager {
    /// Create a new interest manager with the specified cell size
    ///
    /// # Arguments
    ///
    /// * `cell_size` - Size of spatial grid cells (world units)
    ///
    /// # Recommendations
    ///
    /// - Use cell_size ≈ typical AOI radius / 2 for optimal performance
    /// - Smaller cells: Better culling, more memory
    /// - Larger cells: Less memory, more false positives
    pub fn new(cell_size: f32) -> Self {
        let config = SpatialGridConfig {
            cell_size,
            entities_per_cell: 16, // Reasonable default
        };

        Self {
            spatial_grid: SpatialGrid::new(config),
            client_interests: HashMap::new(),
            visibility_cache: HashMap::new(),
            entity_positions: HashMap::new(),
        }
    }

    /// Update spatial grid from world state
    ///
    /// This should be called every frame/tick to synchronize the
    /// interest manager with the current world state.
    ///
    /// # Performance
    ///
    /// - O(N) where N = entities with Aabb components
    /// - Uses profiling when enabled
    pub fn update_from_world(&mut self, world: &World) {
        #[cfg(feature = "profiling")]
        silmaril_profiling::profile_scope!(
            "interest_update_from_world",
            silmaril_profiling::ProfileCategory::Networking
        );

        // Clear and rebuild spatial grid
        self.spatial_grid.clear();
        self.entity_positions.clear();

        // Use World query API to get all entities with Aabb components
        for (entity, aabb) in world.query::<&Aabb>() {
            let center = aabb.center();
            self.spatial_grid.insert(entity, *aabb);
            self.entity_positions.insert(entity, center);
        }
    }

    /// Set or update a client's area of interest
    ///
    /// # Arguments
    ///
    /// * `client_id` - Unique client identifier
    /// * `aoi` - Area of interest configuration
    pub fn set_client_interest(&mut self, client_id: u64, aoi: AreaOfInterest) {
        self.client_interests.insert(client_id, aoi);

        // Initialize visibility cache if needed
        self.visibility_cache.entry(client_id).or_default();
    }

    /// Remove a client from interest management
    ///
    /// Should be called when a client disconnects.
    pub fn clear_client(&mut self, client_id: u64) {
        self.client_interests.remove(&client_id);
        self.visibility_cache.remove(&client_id);
    }

    /// Get a client's area of interest
    ///
    /// Returns None if the client has no registered AOI.
    pub fn get_client_interest(&self, client_id: u64) -> Option<&AreaOfInterest> {
        self.client_interests.get(&client_id)
    }

    /// Calculate currently visible entities for a client
    ///
    /// This performs a spatial query and returns all entities
    /// within the client's AOI.
    ///
    /// # Returns
    ///
    /// Vector of entities visible to the client, or empty if
    /// the client has no AOI registered.
    ///
    /// # Performance
    ///
    /// - Target: <1ms for 1K entities
    /// - Uses spatial grid for efficient querying
    pub fn calculate_visibility(&self, client_id: u64) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        silmaril_profiling::profile_scope!(
            "interest_calculate_visibility",
            silmaril_profiling::ProfileCategory::Networking
        );

        let aoi = match self.client_interests.get(&client_id) {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Query spatial grid for candidates
        let candidates = self.spatial_grid.query_radius(aoi.center, aoi.radius);

        // Filter by precise distance check
        let mut visible = Vec::new();
        for entity in candidates {
            if let Some(&pos) = self.entity_positions.get(&entity) {
                if aoi.contains(pos) {
                    visible.push(entity);
                }
            }
        }

        visible
    }

    /// Get visibility changes since last calculation
    ///
    /// This compares the current visibility set against the cached
    /// visibility and returns entities that entered or exited.
    ///
    /// # Returns
    ///
    /// Tuple of (entered, exited) entity vectors
    ///
    /// # Side Effects
    ///
    /// Updates the visibility cache for this client
    pub fn get_visibility_changes(&mut self, client_id: u64) -> (Vec<Entity>, Vec<Entity>) {
        #[cfg(feature = "profiling")]
        silmaril_profiling::profile_scope!(
            "interest_get_visibility_changes",
            silmaril_profiling::ProfileCategory::Networking
        );

        // Calculate current visibility
        let current_visible: HashSet<Entity> =
            self.calculate_visibility(client_id).into_iter().collect();

        // Get cached visibility
        let cached_visible = self.visibility_cache.entry(client_id).or_default();

        // Calculate differences
        let entered: Vec<Entity> = current_visible.difference(cached_visible).copied().collect();

        let exited: Vec<Entity> = cached_visible.difference(&current_visible).copied().collect();

        // Update cache
        *cached_visible = current_visible;

        (entered, exited)
    }

    /// Get all registered clients
    pub fn client_count(&self) -> usize {
        self.client_interests.len()
    }

    /// Get total entity count in spatial grid
    pub fn entity_count(&self) -> usize {
        self.spatial_grid.entity_count()
    }

    /// Get average entities visible per client
    pub fn average_visible_entities(&self) -> f32 {
        if self.visibility_cache.is_empty() {
            return 0.0;
        }

        let total: usize = self.visibility_cache.values().map(|set| set.len()).sum();
        total as f32 / self.visibility_cache.len() as f32
    }

    /// Compute bandwidth reduction metrics
    ///
    /// # Returns
    ///
    /// Tuple of (without_interest, with_interest, reduction_percentage)
    /// where values represent total entity updates per tick.
    ///
    /// # Example
    ///
    /// ```text
    /// (100000, 5000, 95.0) = 95% bandwidth reduction
    /// ```
    pub fn compute_bandwidth_reduction(&self) -> (usize, usize, f32) {
        let total_clients = self.client_count();
        let total_entities = self.entity_count();

        if total_clients == 0 || total_entities == 0 {
            return (0, 0, 0.0);
        }

        // Without interest: all clients see all entities
        let without_interest = total_clients * total_entities;

        // With interest: sum of visible entities per client
        let with_interest: usize = self.visibility_cache.values().map(|set| set.len()).sum();

        let reduction = if without_interest > 0 {
            (1.0 - (with_interest as f32 / without_interest as f32)) * 100.0
        } else {
            0.0
        };

        (without_interest, with_interest, reduction)
    }

    /// Check if a client has an AOI registered
    ///
    /// # Arguments
    ///
    /// * `client_id` - Client to check
    ///
    /// # Returns
    ///
    /// True if the client has an AOI registered
    pub fn has_client_interest(&self, client_id: u64) -> bool {
        self.client_interests.contains_key(&client_id)
    }

    /// Get a client's current AOI (if registered)
    ///
    /// # Arguments
    ///
    /// * `client_id` - Client to query
    ///
    /// # Returns
    ///
    /// The client's AOI if registered, None otherwise
    pub fn get_client_aoi(&self, client_id: u64) -> Option<AreaOfInterest> {
        self.client_interests.get(&client_id).copied()
    }
}

impl Default for InterestManager {
    fn default() -> Self {
        Self::new(50.0) // Default 50 unit cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::World;

    #[test]
    fn test_area_of_interest_contains() {
        let aoi = AreaOfInterest::new(Vec3::ZERO, 10.0);

        assert!(aoi.contains(Vec3::ZERO));
        assert!(aoi.contains(Vec3::new(5.0, 0.0, 0.0)));
        assert!(aoi.contains(Vec3::new(0.0, 0.0, 9.0)));
        assert!(!aoi.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_interest_manager_basic() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn entities
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new((i as f32) * 10.0, 0.0, 0.0);
            let aabb = Aabb::from_center_half_extents(pos, Vec3::ONE);
            world.add(entity, aabb);
        }

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        assert_eq!(manager.entity_count(), 10);
    }

    #[test]
    fn test_visibility_calculation() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn entities at different distances
        let close_entity = world.spawn();
        world.add(
            close_entity,
            Aabb::from_center_half_extents(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE),
        );

        let far_entity = world.spawn();
        world.add(
            far_entity,
            Aabb::from_center_half_extents(Vec3::new(200.0, 0.0, 0.0), Vec3::ONE),
        );

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        // Set client AOI
        let client_id = 1;
        let aoi = AreaOfInterest::new(Vec3::ZERO, 50.0);
        manager.set_client_interest(client_id, aoi);

        // Calculate visibility
        let visible = manager.calculate_visibility(client_id);

        assert_eq!(visible.len(), 1);
        assert!(visible.contains(&close_entity));
        assert!(!visible.contains(&far_entity));
    }

    #[test]
    fn test_visibility_changes() {
        let mut world = World::new();
        world.register::<Aabb>();

        let entity = world.spawn();
        let initial_pos = Vec3::new(10.0, 0.0, 0.0);
        world.add(entity, Aabb::from_center_half_extents(initial_pos, Vec3::ONE));

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        let client_id = 1;
        let aoi = AreaOfInterest::new(Vec3::ZERO, 50.0);
        manager.set_client_interest(client_id, aoi);

        // First calculation - entity should enter
        let (entered, exited) = manager.get_visibility_changes(client_id);
        assert_eq!(entered.len(), 1);
        assert_eq!(exited.len(), 0);
        assert!(entered.contains(&entity));

        // Second calculation - no changes
        let (entered, exited) = manager.get_visibility_changes(client_id);
        assert_eq!(entered.len(), 0);
        assert_eq!(exited.len(), 0);

        // Move entity out of range
        world.remove::<Aabb>(entity);
        world.add(entity, Aabb::from_center_half_extents(Vec3::new(200.0, 0.0, 0.0), Vec3::ONE));
        manager.update_from_world(&world);

        // Third calculation - entity should exit
        let (entered, exited) = manager.get_visibility_changes(client_id);
        assert_eq!(entered.len(), 0);
        assert_eq!(exited.len(), 1);
        assert!(exited.contains(&entity));
    }

    #[test]
    fn test_multiple_clients() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn entities in a grid
        for x in 0..10 {
            for z in 0..10 {
                let entity = world.spawn();
                let pos = Vec3::new(x as f32 * 20.0, 0.0, z as f32 * 20.0);
                world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
            }
        }

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        // Add two clients at different locations
        manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 50.0));
        manager.set_client_interest(2, AreaOfInterest::new(Vec3::new(100.0, 0.0, 100.0), 50.0));

        let visible1 = manager.calculate_visibility(1);
        let visible2 = manager.calculate_visibility(2);

        // Each client should see different entities
        assert!(visible1.len() > 0);
        assert!(visible2.len() > 0);
        assert!(visible1.len() < 100); // Not all entities
        assert!(visible2.len() < 100); // Not all entities
    }

    #[test]
    fn test_bandwidth_reduction() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Spawn 1000 entities
        for i in 0..1000 {
            let entity = world.spawn();
            let x = (i % 100) as f32 * 10.0;
            let z = (i / 100) as f32 * 10.0;
            let pos = Vec3::new(x, 0.0, z);
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        }

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        // Add 10 clients spread across the world
        for i in 0..10 {
            let pos = Vec3::new((i as f32) * 100.0, 0.0, 0.0);
            manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));

            // Calculate initial visibility
            manager.get_visibility_changes(i);
        }

        let (without, with, reduction) = manager.compute_bandwidth_reduction();

        // Without interest: 10 clients × 1000 entities = 10,000 updates
        assert_eq!(without, 10_000);

        // With interest: should be much less
        assert!(with < without);

        // Should achieve significant reduction
        assert!(reduction > 50.0, "Reduction should be >50%, got {}", reduction);
    }

    #[test]
    fn test_clear_client() {
        let mut manager = InterestManager::new(50.0);

        manager.set_client_interest(1, AreaOfInterest::default());
        assert_eq!(manager.client_count(), 1);

        manager.clear_client(1);
        assert_eq!(manager.client_count(), 0);
    }
}
