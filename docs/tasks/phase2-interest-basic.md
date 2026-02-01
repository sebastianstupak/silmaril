# Phase 2.8: Interest Management (Basic)

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (network scalability)

---

## 🎯 **Objective**

Implement basic interest management system using spatial grid to reduce bandwidth by only sending relevant entity updates to each client. Each client receives updates only for entities in their area of interest (AOI).

**Key Features:**
- Spatial grid partitioning
- Area-of-interest calculation per client
- Entity relevance filtering
- Efficient grid queries
- Dynamic grid updates

**Scalability Goal:**
- Without interest management: O(N × M) where N = clients, M = entities
- With interest management: O(N × K) where K = entities in AOI (K << M)

---

## 📋 **Detailed Tasks**

### **1. Spatial Grid** (Day 1-2)

**File:** `engine/networking/src/interest/spatial_grid.rs`

```rust
use std::collections::{HashMap, HashSet};
use glam::Vec3;

/// 2D spatial grid for efficient spatial queries
pub struct SpatialGrid {
    /// Cell size (world units)
    cell_size: f32,

    /// Grid cells (cell_key -> set of entity IDs)
    cells: HashMap<(i32, i32), HashSet<u32>>,

    /// Entity positions (entity_id -> cell_key)
    entity_cells: HashMap<u32, (i32, i32)>,
}

impl SpatialGrid {
    /// Create spatial grid
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Get cell key for position
    fn get_cell_key(&self, position: Vec3) -> (i32, i32) {
        let x = (position.x / self.cell_size).floor() as i32;
        let z = (position.z / self.cell_size).floor() as i32;
        (x, z)
    }

    /// Insert entity into grid
    pub fn insert(&mut self, entity_id: u32, position: Vec3) {
        let cell_key = self.get_cell_key(position);

        // Add to cell
        self.cells
            .entry(cell_key)
            .or_insert_with(HashSet::new)
            .insert(entity_id);

        // Track entity's cell
        self.entity_cells.insert(entity_id, cell_key);
    }

    /// Update entity position
    pub fn update(&mut self, entity_id: u32, new_position: Vec3) {
        let new_cell_key = self.get_cell_key(new_position);

        // Check if entity moved to different cell
        if let Some(&old_cell_key) = self.entity_cells.get(&entity_id) {
            if old_cell_key != new_cell_key {
                // Remove from old cell
                if let Some(cell) = self.cells.get_mut(&old_cell_key) {
                    cell.remove(&entity_id);

                    // Clean up empty cells
                    if cell.is_empty() {
                        self.cells.remove(&old_cell_key);
                    }
                }

                // Add to new cell
                self.cells
                    .entry(new_cell_key)
                    .or_insert_with(HashSet::new)
                    .insert(entity_id);

                // Update tracking
                self.entity_cells.insert(entity_id, new_cell_key);
            }
        } else {
            // Entity not in grid, insert it
            self.insert(entity_id, new_position);
        }
    }

    /// Remove entity from grid
    pub fn remove(&mut self, entity_id: u32) {
        if let Some(cell_key) = self.entity_cells.remove(&entity_id) {
            if let Some(cell) = self.cells.get_mut(&cell_key) {
                cell.remove(&entity_id);

                if cell.is_empty() {
                    self.cells.remove(&cell_key);
                }
            }
        }
    }

    /// Query entities in radius
    pub fn query_radius(&self, position: Vec3, radius: f32) -> Vec<u32> {
        let mut results = Vec::new();

        let center_cell = self.get_cell_key(position);

        // Calculate how many cells to check in each direction
        let cell_radius = (radius / self.cell_size).ceil() as i32;

        // Check all cells in range
        for dx in -cell_radius..=cell_radius {
            for dz in -cell_radius..=cell_radius {
                let cell_key = (center_cell.0 + dx, center_cell.1 + dz);

                if let Some(entities) = self.cells.get(&cell_key) {
                    results.extend(entities.iter().copied());
                }
            }
        }

        results
    }

    /// Query entities in rectangular area
    pub fn query_rect(&self, min: Vec3, max: Vec3) -> Vec<u32> {
        let mut results = Vec::new();

        let min_cell = self.get_cell_key(min);
        let max_cell = self.get_cell_key(max);

        for x in min_cell.0..=max_cell.0 {
            for z in min_cell.1..=max_cell.1 {
                let cell_key = (x, z);

                if let Some(entities) = self.cells.get(&cell_key) {
                    results.extend(entities.iter().copied());
                }
            }
        }

        results
    }

    /// Get all entities in grid
    pub fn all_entities(&self) -> Vec<u32> {
        self.entity_cells.keys().copied().collect()
    }

    /// Get cell count
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    /// Clear grid
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_grid_insert() {
        let mut grid = SpatialGrid::new(10.0);

        grid.insert(1, Vec3::new(5.0, 0.0, 5.0));
        grid.insert(2, Vec3::new(15.0, 0.0, 5.0));

        assert_eq!(grid.entity_count(), 2);
        assert_eq!(grid.cell_count(), 2); // Different cells
    }

    #[test]
    fn test_spatial_grid_update() {
        let mut grid = SpatialGrid::new(10.0);

        grid.insert(1, Vec3::new(5.0, 0.0, 5.0));
        assert_eq!(grid.cell_count(), 1);

        // Move to same cell
        grid.update(1, Vec3::new(6.0, 0.0, 6.0));
        assert_eq!(grid.cell_count(), 1);

        // Move to different cell
        grid.update(1, Vec3::new(15.0, 0.0, 5.0));
        assert_eq!(grid.cell_count(), 1);
    }

    #[test]
    fn test_spatial_grid_query() {
        let mut grid = SpatialGrid::new(10.0);

        // Add entities in pattern
        grid.insert(1, Vec3::new(0.0, 0.0, 0.0));
        grid.insert(2, Vec3::new(5.0, 0.0, 0.0));
        grid.insert(3, Vec3::new(50.0, 0.0, 0.0));

        // Query near origin
        let nearby = grid.query_radius(Vec3::ZERO, 10.0);
        assert!(nearby.contains(&1));
        assert!(nearby.contains(&2));
        assert!(!nearby.contains(&3));
    }
}
```

---

### **2. Area of Interest** (Day 2)

**File:** `engine/networking/src/interest/aoi.rs`

```rust
use glam::Vec3;
use std::collections::HashSet;

/// Area of interest for a client
#[derive(Debug, Clone)]
pub struct AreaOfInterest {
    /// Center position (player position)
    pub center: Vec3,

    /// Radius (world units)
    pub radius: f32,

    /// Shape type
    pub shape: AoiShape,
}

/// AOI shape
#[derive(Debug, Clone, Copy)]
pub enum AoiShape {
    /// Circular AOI
    Circle,

    /// Rectangular AOI
    Rectangle { width: f32, height: f32 },
}

impl AreaOfInterest {
    /// Create circular AOI
    pub fn circle(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius,
            shape: AoiShape::Circle,
        }
    }

    /// Create rectangular AOI
    pub fn rectangle(center: Vec3, width: f32, height: f32) -> Self {
        Self {
            center,
            radius: (width.max(height) / 2.0), // For grid query
            shape: AoiShape::Rectangle { width, height },
        }
    }

    /// Check if position is inside AOI
    pub fn contains(&self, position: Vec3) -> bool {
        match self.shape {
            AoiShape::Circle => {
                let distance = (position - self.center).length();
                distance <= self.radius
            }
            AoiShape::Rectangle { width, height } => {
                let dx = (position.x - self.center.x).abs();
                let dz = (position.z - self.center.z).abs();

                dx <= width / 2.0 && dz <= height / 2.0
            }
        }
    }

    /// Update center position
    pub fn set_center(&mut self, new_center: Vec3) {
        self.center = new_center;
    }

    /// Get bounding box (for rect queries)
    pub fn bounding_box(&self) -> (Vec3, Vec3) {
        match self.shape {
            AoiShape::Circle => {
                let min = self.center - Vec3::new(self.radius, 0.0, self.radius);
                let max = self.center + Vec3::new(self.radius, 0.0, self.radius);
                (min, max)
            }
            AoiShape::Rectangle { width, height } => {
                let half_width = width / 2.0;
                let half_height = height / 2.0;

                let min = self.center - Vec3::new(half_width, 0.0, half_height);
                let max = self.center + Vec3::new(half_width, 0.0, half_height);
                (min, max)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_aoi() {
        let aoi = AreaOfInterest::circle(Vec3::ZERO, 10.0);

        assert!(aoi.contains(Vec3::new(5.0, 0.0, 0.0)));
        assert!(aoi.contains(Vec3::new(0.0, 0.0, 9.0)));
        assert!(!aoi.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_rectangle_aoi() {
        let aoi = AreaOfInterest::rectangle(Vec3::ZERO, 20.0, 10.0);

        assert!(aoi.contains(Vec3::new(5.0, 0.0, 2.0)));
        assert!(aoi.contains(Vec3::new(-9.0, 0.0, 4.0)));
        assert!(!aoi.contains(Vec3::new(15.0, 0.0, 0.0)));
        assert!(!aoi.contains(Vec3::new(0.0, 0.0, 10.0)));
    }
}
```

---

### **3. Interest Manager** (Day 2-3)

**File:** `engine/networking/src/interest/manager.rs`

```rust
use super::{SpatialGrid, AreaOfInterest};
use std::collections::{HashMap, HashSet};
use glam::Vec3;

/// Interest manager for server
pub struct InterestManager {
    /// Spatial grid for entities
    grid: SpatialGrid,

    /// Client AOIs (client_id -> AOI)
    client_aois: HashMap<u64, AreaOfInterest>,

    /// Cached relevant entities per client (client_id -> entity set)
    client_relevant_entities: HashMap<u64, HashSet<u32>>,

    /// Entity positions (for distance checks)
    entity_positions: HashMap<u32, Vec3>,

    /// Configuration
    config: InterestConfig,
}

#[derive(Debug, Clone)]
pub struct InterestConfig {
    /// Default AOI radius
    pub default_radius: f32,

    /// Grid cell size
    pub grid_cell_size: f32,

    /// Update interval (how often to recalculate interest)
    pub update_interval_ticks: u64,
}

impl Default for InterestConfig {
    fn default() -> Self {
        Self {
            default_radius: 100.0,
            grid_cell_size: 50.0,
            update_interval_ticks: 5, // Every 5 ticks (~83ms at 60 TPS)
        }
    }
}

impl InterestManager {
    pub fn new(config: InterestConfig) -> Self {
        Self {
            grid: SpatialGrid::new(config.grid_cell_size),
            client_aois: HashMap::new(),
            client_relevant_entities: HashMap::new(),
            entity_positions: HashMap::new(),
            config,
        }
    }

    /// Register client
    pub fn register_client(&mut self, client_id: u64, position: Vec3) {
        let aoi = AreaOfInterest::circle(position, self.config.default_radius);
        self.client_aois.insert(client_id, aoi);
        self.client_relevant_entities.insert(client_id, HashSet::new());
    }

    /// Unregister client
    pub fn unregister_client(&mut self, client_id: u64) {
        self.client_aois.remove(&client_id);
        self.client_relevant_entities.remove(&client_id);
    }

    /// Update client position
    pub fn update_client_position(&mut self, client_id: u64, position: Vec3) {
        if let Some(aoi) = self.client_aois.get_mut(&client_id) {
            aoi.set_center(position);
        }
    }

    /// Add entity to interest management
    pub fn add_entity(&mut self, entity_id: u32, position: Vec3) {
        self.grid.insert(entity_id, position);
        self.entity_positions.insert(entity_id, position);
    }

    /// Update entity position
    pub fn update_entity_position(&mut self, entity_id: u32, position: Vec3) {
        self.grid.update(entity_id, position);
        self.entity_positions.insert(entity_id, position);
    }

    /// Remove entity
    pub fn remove_entity(&mut self, entity_id: u32) {
        self.grid.remove(entity_id);
        self.entity_positions.remove(&entity_id);

        // Remove from all client relevance sets
        for relevant_set in self.client_relevant_entities.values_mut() {
            relevant_set.remove(&entity_id);
        }
    }

    /// Calculate relevant entities for client
    pub fn calculate_relevance(&mut self, client_id: u64) -> Option<RelevanceUpdate> {
        let aoi = self.client_aois.get(&client_id)?.clone();

        // Query entities in AOI using spatial grid
        let candidate_entities = self.grid.query_radius(aoi.center, aoi.radius);

        // Filter by precise AOI shape
        let mut relevant_entities = HashSet::new();
        for entity_id in candidate_entities {
            if let Some(&position) = self.entity_positions.get(&entity_id) {
                if aoi.contains(position) {
                    relevant_entities.insert(entity_id);
                }
            }
        }

        // Calculate changes from last update
        let old_relevant = self
            .client_relevant_entities
            .get(&client_id)
            .cloned()
            .unwrap_or_default();

        let entered: Vec<u32> = relevant_entities
            .difference(&old_relevant)
            .copied()
            .collect();

        let exited: Vec<u32> = old_relevant
            .difference(&relevant_entities)
            .copied()
            .collect();

        let still_relevant: Vec<u32> = relevant_entities
            .intersection(&old_relevant)
            .copied()
            .collect();

        // Update cached relevance
        self.client_relevant_entities.insert(client_id, relevant_entities);

        Some(RelevanceUpdate {
            entered,
            exited,
            still_relevant,
        })
    }

    /// Get currently relevant entities for client
    pub fn get_relevant_entities(&self, client_id: u64) -> Option<&HashSet<u32>> {
        self.client_relevant_entities.get(&client_id)
    }

    /// Calculate relevance for all clients
    pub fn calculate_all_relevance(&mut self) -> HashMap<u64, RelevanceUpdate> {
        let client_ids: Vec<_> = self.client_aois.keys().copied().collect();

        let mut updates = HashMap::new();

        for client_id in client_ids {
            if let Some(update) = self.calculate_relevance(client_id) {
                updates.insert(client_id, update);
            }
        }

        updates
    }

    /// Get statistics
    pub fn stats(&self) -> InterestStats {
        let total_entities = self.grid.entity_count();
        let total_clients = self.client_aois.len();

        let avg_relevant_entities = if total_clients > 0 {
            let sum: usize = self
                .client_relevant_entities
                .values()
                .map(|set| set.len())
                .sum();
            sum as f32 / total_clients as f32
        } else {
            0.0
        };

        InterestStats {
            total_entities,
            total_clients,
            avg_relevant_entities,
            grid_cells: self.grid.cell_count(),
        }
    }
}

/// Relevance update for a client
#[derive(Debug, Clone)]
pub struct RelevanceUpdate {
    /// Entities that entered AOI
    pub entered: Vec<u32>,

    /// Entities that exited AOI
    pub exited: Vec<u32>,

    /// Entities still in AOI
    pub still_relevant: Vec<u32>,
}

/// Interest management statistics
#[derive(Debug, Clone)]
pub struct InterestStats {
    pub total_entities: usize,
    pub total_clients: usize,
    pub avg_relevant_entities: f32,
    pub grid_cells: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interest_manager() {
        let mut manager = InterestManager::new(InterestConfig::default());

        // Register client
        manager.register_client(1, Vec3::ZERO);

        // Add entities
        manager.add_entity(10, Vec3::new(5.0, 0.0, 0.0)); // Inside AOI
        manager.add_entity(11, Vec3::new(200.0, 0.0, 0.0)); // Outside AOI

        // Calculate relevance
        let update = manager.calculate_relevance(1).unwrap();

        assert_eq!(update.entered.len(), 1);
        assert!(update.entered.contains(&10));
        assert!(!update.entered.contains(&11));
    }

    #[test]
    fn test_relevance_changes() {
        let mut manager = InterestManager::new(InterestConfig::default());

        manager.register_client(1, Vec3::ZERO);
        manager.add_entity(10, Vec3::new(5.0, 0.0, 0.0));

        // First calculation
        let update1 = manager.calculate_relevance(1).unwrap();
        assert_eq!(update1.entered.len(), 1);

        // Move entity out of range
        manager.update_entity_position(10, Vec3::new(200.0, 0.0, 0.0));

        // Second calculation
        let update2 = manager.calculate_relevance(1).unwrap();
        assert_eq!(update2.exited.len(), 1);
        assert!(update2.exited.contains(&10));
    }
}
```

---

### **4. Integration with State Sync** (Day 3-4)

**File:** `engine/networking/src/sync/interest_filtering.rs`

```rust
use super::*;
use crate::interest::InterestManager;

/// Extended state synchronizer with interest management
pub struct InterestAwareStateSynchronizer {
    /// Base state sync
    base_sync: ServerStateSynchronizer,

    /// Interest manager
    interest_manager: InterestManager,

    /// Current tick
    current_tick: u64,

    /// Update interval
    update_interval: u64,
}

impl InterestAwareStateSynchronizer {
    pub fn new(
        base_sync: ServerStateSynchronizer,
        interest_config: InterestConfig,
    ) -> Self {
        Self {
            base_sync,
            interest_manager: InterestManager::new(interest_config.clone()),
            current_tick: 0,
            update_interval: interest_config.update_interval_ticks,
        }
    }

    /// Add client with position
    pub fn add_client(&mut self, client_id: u64, position: Vec3) {
        self.base_sync.add_client(client_id);
        self.interest_manager.register_client(client_id, position);
    }

    /// Update client position (player movement)
    pub fn update_client_position(&mut self, client_id: u64, position: Vec3) {
        self.interest_manager.update_client_position(client_id, position);
    }

    /// Tick and generate filtered updates
    pub fn tick(&mut self, world: &World) -> HashMap<u64, Vec<u8>> {
        self.current_tick += 1;

        // Update interest management periodically
        if self.current_tick % self.update_interval == 0 {
            self.update_interest_management(world);
        }

        // Generate base state updates
        let base_updates = self.base_sync.tick(self.current_tick);

        // Filter updates based on interest
        let mut filtered_updates = HashMap::new();

        for (client_id, update_data) in base_updates {
            // Get relevant entities for this client
            if let Some(relevant_entities) = self.interest_manager.get_relevant_entities(client_id) {
                // Filter update to only include relevant entities
                let filtered_data = self.filter_update(update_data, relevant_entities);
                filtered_updates.insert(client_id, filtered_data);
            } else {
                // No filtering, send all
                filtered_updates.insert(client_id, update_data);
            }
        }

        filtered_updates
    }

    /// Update interest management from world state
    fn update_interest_management(&mut self, world: &World) {
        // Update entity positions in interest manager
        for (entity, transform) in world.query::<&Transform>() {
            let entity_id = entity.id();
            self.interest_manager.update_entity_position(entity_id, transform.position);
        }

        // Recalculate relevance for all clients
        let relevance_updates = self.interest_manager.calculate_all_relevance();

        // Log statistics
        let stats = self.interest_manager.stats();
        tracing::debug!(
            "Interest stats: {} entities, {} clients, {:.1} avg relevant/client",
            stats.total_entities,
            stats.total_clients,
            stats.avg_relevant_entities
        );

        // Send enter/exit notifications to clients
        for (client_id, update) in relevance_updates {
            self.notify_relevance_changes(client_id, update);
        }
    }

    /// Filter update data to only include relevant entities
    fn filter_update(&self, update_data: Vec<u8>, relevant_entities: &HashSet<u32>) -> Vec<u8> {
        // Decode update
        let packet = match Protocol::decode_server_packet(&update_data) {
            Ok(p) => p,
            Err(_) => return update_data, // Can't decode, return as-is
        };

        match packet.message_type() {
            ServerMessage::WorldSnapshot => {
                // Filter snapshot entities
                let snapshot = packet.message_as_world_snapshot().unwrap();
                self.filter_snapshot(snapshot, relevant_entities)
            }
            ServerMessage::WorldDelta => {
                // Filter delta entities
                let delta = packet.message_as_world_delta().unwrap();
                self.filter_delta(delta, relevant_entities)
            }
            _ => update_data, // Other messages, no filtering
        }
    }

    fn filter_snapshot(&self, snapshot: WorldSnapshot, relevant: &HashSet<u32>) -> Vec<u8> {
        // Rebuild snapshot with only relevant entities
        // (Implementation would rebuild FlatBuffers message)
        // For now, return empty vec as placeholder
        vec![]
    }

    fn filter_delta(&self, delta: WorldDelta, relevant: &HashSet<u32>) -> Vec<u8> {
        // Rebuild delta with only relevant entities
        vec![]
    }

    fn notify_relevance_changes(&mut self, client_id: u64, update: RelevanceUpdate) {
        // Send EntitySpawned for entered entities
        for entity_id in update.entered {
            // Build and send EntitySpawned message
            tracing::trace!("Entity {} entered AOI for client {}", entity_id, client_id);
        }

        // Send EntityDespawned for exited entities
        for entity_id in update.exited {
            // Build and send EntityDespawned message
            tracing::trace!("Entity {} exited AOI for client {}", entity_id, client_id);
        }
    }

    pub fn remove_client(&mut self, client_id: u64) {
        self.base_sync.remove_client(client_id);
        self.interest_manager.unregister_client(client_id);
    }

    pub fn stats(&self) -> InterestStats {
        self.interest_manager.stats()
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Spatial grid efficiently partitions world
- [ ] Grid updates entities in O(1) amortized time
- [ ] AOI calculations are accurate
- [ ] Interest management filters entity updates
- [ ] Only relevant entities sent to each client
- [ ] Enter/exit notifications sent correctly
- [ ] Interest calculation < 2ms for 1000 entities
- [ ] Grid query < 1ms
- [ ] 10x bandwidth reduction with 1000+ entities
- [ ] Works with moving clients and entities

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Grid insert | < 0.05ms | < 0.1ms |
| Grid update | < 0.1ms | < 0.5ms |
| Grid query (radius) | < 0.5ms | < 1ms |
| Interest calculation (per client) | < 1ms | < 2ms |
| Interest calculation (all clients, 100 clients) | < 50ms | < 100ms |
| Total interest overhead (per tick) | < 2ms | < 5ms |

**Scalability Targets:**
- Without interest: 100 clients × 1000 entities = 100,000 updates
- With interest (100 unit radius): 100 clients × ~50 entities = 5,000 updates
- Bandwidth reduction: 95%

---

## 🧪 **Tests**

```rust
#[test]
fn test_spatial_grid_performance() {
    let mut grid = SpatialGrid::new(50.0);

    // Insert 1000 entities
    for i in 0..1000 {
        let x = (i % 100) as f32 * 10.0;
        let z = (i / 100) as f32 * 10.0;
        grid.insert(i, Vec3::new(x, 0.0, z));
    }

    // Query should be fast
    let start = Instant::now();
    let results = grid.query_radius(Vec3::new(500.0, 0.0, 500.0), 100.0);
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 1, "Query too slow: {:?}", elapsed);
    assert!(results.len() > 0, "Should find entities");
}

#[test]
fn test_interest_bandwidth_reduction() {
    let mut manager = InterestManager::new(InterestConfig {
        default_radius: 100.0,
        grid_cell_size: 50.0,
        update_interval_ticks: 1,
    });

    // Add 10 clients spread out
    for i in 0..10 {
        let pos = Vec3::new(i as f32 * 500.0, 0.0, 0.0);
        manager.register_client(i, pos);
    }

    // Add 1000 entities across world
    for i in 0..1000 {
        let x = (i % 100) as f32 * 50.0;
        let z = (i / 100) as f32 * 50.0;
        manager.add_entity(i, Vec3::new(x, 0.0, z));
    }

    // Calculate relevance
    let updates = manager.calculate_all_relevance();

    // Each client should only see ~20-50 entities (not all 1000)
    for (client_id, update) in updates {
        let total_relevant = update.entered.len() + update.still_relevant.len();
        assert!(
            total_relevant < 100,
            "Client {} sees too many entities: {}",
            client_id,
            total_relevant
        );
    }

    let stats = manager.stats();
    assert!(
        stats.avg_relevant_entities < 100.0,
        "Average too high: {:.1}",
        stats.avg_relevant_entities
    );
}

#[test]
fn test_enter_exit_detection() {
    let mut manager = InterestManager::new(InterestConfig::default());

    manager.register_client(1, Vec3::ZERO);
    manager.add_entity(10, Vec3::new(50.0, 0.0, 0.0)); // Inside

    // Initial calculation
    let update1 = manager.calculate_relevance(1).unwrap();
    assert_eq!(update1.entered.len(), 1);
    assert_eq!(update1.exited.len(), 0);

    // Move entity out
    manager.update_entity_position(10, Vec3::new(200.0, 0.0, 0.0));

    // Should detect exit
    let update2 = manager.calculate_relevance(1).unwrap();
    assert_eq!(update2.entered.len(), 0);
    assert_eq!(update2.exited.len(), 1);

    // Move entity back in
    manager.update_entity_position(10, Vec3::new(50.0, 0.0, 0.0));

    // Should detect enter again
    let update3 = manager.calculate_relevance(1).unwrap();
    assert_eq!(update3.entered.len(), 1);
    assert_eq!(update3.exited.len(), 0);
}
```

---

## 📊 **Bandwidth Analysis**

### **Without Interest Management:**
- 100 clients, 1000 entities
- Each tick: 100 × 1000 = 100,000 entity updates
- At 19 bytes per update: ~1.9 MB per tick
- At 20 ticks/sec: ~38 MB/sec total

### **With Interest Management (100 unit radius):**
- 100 clients, ~50 relevant entities each
- Each tick: 100 × 50 = 5,000 entity updates
- At 19 bytes per update: ~95 KB per tick
- At 20 ticks/sec: ~1.9 MB/sec total

**Reduction: 95% bandwidth savings**

### **Scalability:**
- Without: O(N × M) - N clients, M entities
- With: O(N × K) - K entities in AOI (K << M)
- 1000 clients, 10,000 entities:
  - Without: 10,000,000 updates
  - With: 50,000 updates (200x reduction)

---

## 💡 **Future Enhancements (Phase 3+)**

- Hierarchical spatial hashing for massive worlds
- Priority-based relevance (closer = higher priority)
- View frustum culling
- Occlusion-based interest
- Network LOD (different detail levels by distance)

---

**Dependencies:** [phase2-network-protocol.md](phase2-network-protocol.md), [phase2-state-sync.md](phase2-state-sync.md), [phase2-udp-packets.md](phase2-udp-packets.md)
**Next:** Phase 3 (Physics, Audio, Advanced Features)
