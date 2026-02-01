# Phase 3.5: Advanced Interest Management

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Critical (scalability optimization)

---

## 🎯 **Objective**

Implement advanced interest management with VALORANT-style occlusion culling, fog of war, and optimized interest sets. Ensures clients only receive updates for entities they can see or interact with, dramatically improving scalability.

**Features:**
- Advanced occlusion culling (visibility testing)
- Fog of war system (team-based visibility)
- Interest set optimization with spatial partitioning
- Predictive interest (pre-load entities about to be visible)
- Performance: <2% server CPU overhead
- Support for 10,000+ entities per server

---

## 📋 **Detailed Tasks**

### **1. Spatial Partitioning** (Day 1)

**File:** `engine/spatial/src/grid.rs`

```rust
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// Spatial hash grid for fast entity lookups
pub struct SpatialGrid {
    /// Cell size (meters)
    cell_size: f32,

    /// Grid cells (cell_key -> entity_ids)
    cells: HashMap<(i32, i32, i32), HashSet<u64>>,

    /// Entity positions (entity_id -> cell_key)
    entity_cells: HashMap<u64, (i32, i32, i32)>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Get cell key for position
    fn get_cell_key(&self, position: Vec3) -> (i32, i32, i32) {
        (
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }

    /// Insert entity
    pub fn insert(&mut self, entity_id: u64, position: Vec3) {
        let cell_key = self.get_cell_key(position);

        // Remove from old cell if exists
        if let Some(old_key) = self.entity_cells.get(&entity_id) {
            if let Some(cell) = self.cells.get_mut(old_key) {
                cell.remove(&entity_id);
            }
        }

        // Insert into new cell
        self.cells
            .entry(cell_key)
            .or_insert_with(HashSet::new)
            .insert(entity_id);

        self.entity_cells.insert(entity_id, cell_key);
    }

    /// Remove entity
    pub fn remove(&mut self, entity_id: u64) {
        if let Some(cell_key) = self.entity_cells.remove(&entity_id) {
            if let Some(cell) = self.cells.get_mut(&cell_key) {
                cell.remove(&entity_id);

                // Clean up empty cells
                if cell.is_empty() {
                    self.cells.remove(&cell_key);
                }
            }
        }
    }

    /// Query entities in radius
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u64> {
        let mut result = Vec::new();

        // Calculate cell range to check
        let cell_radius = (radius / self.cell_size).ceil() as i32;
        let center_key = self.get_cell_key(center);

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -cell_radius..=cell_radius {
                    let cell_key = (
                        center_key.0 + dx,
                        center_key.1 + dy,
                        center_key.2 + dz,
                    );

                    if let Some(cell) = self.cells.get(&cell_key) {
                        for &entity_id in cell {
                            // Would need to check actual distance here
                            // For now, just add all entities in cells
                            result.push(entity_id);
                        }
                    }
                }
            }
        }

        result
    }

    /// Query entities in box
    pub fn query_box(&self, min: Vec3, max: Vec3) -> Vec<u64> {
        let mut result = Vec::new();

        let min_key = self.get_cell_key(min);
        let max_key = self.get_cell_key(max);

        for x in min_key.0..=max_key.0 {
            for y in min_key.1..=max_key.1 {
                for z in min_key.2..=max_key.2 {
                    if let Some(cell) = self.cells.get(&(x, y, z)) {
                        result.extend(cell.iter());
                    }
                }
            }
        }

        result
    }

    /// Get all entities
    pub fn all_entities(&self) -> Vec<u64> {
        self.entity_cells.keys().copied().collect()
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    /// Get cell count
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}
```

---

### **2. Occlusion Culling** (Day 1-2)

**File:** `engine/spatial/src/occlusion.rs`

```rust
use glam::Vec3;
use std::collections::HashSet;

/// Simple occlusion culling using raycasting
pub struct OcclusionCuller {
    /// Static occluders (walls, buildings)
    occluders: Vec<Box<dyn Occluder>>,

    /// Ray cast cache (from_pos + to_pos -> visible)
    ray_cache: HashMap<(u64, u64), bool>,

    /// Max cache size
    max_cache_size: usize,
}

/// Trait for occluders
pub trait Occluder: Send + Sync {
    /// Check if ray intersects this occluder
    fn intersects_ray(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool;

    /// Get bounds
    fn bounds(&self) -> AABB;
}

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub fn intersects_ray(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
        // AABB ray intersection (slab method)
        let inv_dir = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);

        let t1 = (self.min.x - origin.x) * inv_dir.x;
        let t2 = (self.max.x - origin.x) * inv_dir.x;
        let t3 = (self.min.y - origin.y) * inv_dir.y;
        let t4 = (self.max.y - origin.y) * inv_dir.y;
        let t5 = (self.min.z - origin.z) * inv_dir.z;
        let t6 = (self.max.z - origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax || tmin > max_distance {
            false
        } else {
            true
        }
    }
}

/// Box occluder (wall, building)
pub struct BoxOccluder {
    pub bounds: AABB,
}

impl Occluder for BoxOccluder {
    fn intersects_ray(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
        self.bounds.intersects_ray(origin, direction, max_distance)
    }

    fn bounds(&self) -> AABB {
        self.bounds
    }
}

impl OcclusionCuller {
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            occluders: Vec::new(),
            ray_cache: HashMap::new(),
            max_cache_size,
        }
    }

    /// Add occluder
    pub fn add_occluder(&mut self, occluder: Box<dyn Occluder>) {
        self.occluders.push(occluder);
    }

    /// Check if target is visible from origin
    pub fn is_visible(
        &mut self,
        origin: Vec3,
        target: Vec3,
        entity_id_from: u64,
        entity_id_to: u64,
    ) -> bool {
        // Check cache
        let cache_key = (entity_id_from, entity_id_to);
        if let Some(&cached) = self.ray_cache.get(&cache_key) {
            return cached;
        }

        // Compute visibility
        let direction = (target - origin).normalize();
        let distance = origin.distance(target);

        let mut visible = true;

        for occluder in &self.occluders {
            if occluder.intersects_ray(origin, direction, distance) {
                visible = false;
                break;
            }
        }

        // Cache result
        if self.ray_cache.len() < self.max_cache_size {
            self.ray_cache.insert(cache_key, visible);
        }

        visible
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.ray_cache.clear();
    }

    /// Prune cache (remove old entries)
    pub fn prune_cache(&mut self) {
        if self.ray_cache.len() > self.max_cache_size {
            // Remove oldest half
            let keys_to_remove: Vec<_> = self
                .ray_cache
                .keys()
                .take(self.ray_cache.len() / 2)
                .copied()
                .collect();

            for key in keys_to_remove {
                self.ray_cache.remove(&key);
            }
        }
    }
}
```

---

### **3. Fog of War** (Day 2-3)

**File:** `engine/spatial/src/fog_of_war.rs`

```rust
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// Fog of war system (team-based visibility)
pub struct FogOfWar {
    /// Team visibility data
    team_visibility: HashMap<u32, TeamVisibility>,

    /// Entity teams
    entity_teams: HashMap<u64, u32>,

    /// Reveal radius per entity type
    reveal_radius: HashMap<String, f32>,

    /// Default reveal radius
    default_reveal_radius: f32,
}

struct TeamVisibility {
    team_id: u32,

    /// Revealed areas (entities this team can see)
    revealed_entities: HashSet<u64>,

    /// Vision sources (entities on this team that reveal)
    vision_sources: HashSet<u64>,
}

impl FogOfWar {
    pub fn new(default_reveal_radius: f32) -> Self {
        Self {
            team_visibility: HashMap::new(),
            entity_teams: HashMap::new(),
            reveal_radius: HashMap::new(),
            default_reveal_radius,
        }
    }

    /// Add team
    pub fn add_team(&mut self, team_id: u32) {
        self.team_visibility.insert(
            team_id,
            TeamVisibility {
                team_id,
                revealed_entities: HashSet::new(),
                vision_sources: HashSet::new(),
            },
        );
    }

    /// Set entity team
    pub fn set_entity_team(&mut self, entity_id: u64, team_id: u32) {
        self.entity_teams.insert(entity_id, team_id);

        // Add to team's vision sources
        if let Some(team) = self.team_visibility.get_mut(&team_id) {
            team.vision_sources.insert(entity_id);
        }
    }

    /// Remove entity
    pub fn remove_entity(&mut self, entity_id: u64) {
        if let Some(team_id) = self.entity_teams.remove(&entity_id) {
            if let Some(team) = self.team_visibility.get_mut(&team_id) {
                team.vision_sources.remove(&entity_id);
            }
        }
    }

    /// Set reveal radius for entity type
    pub fn set_reveal_radius(&mut self, entity_type: String, radius: f32) {
        self.reveal_radius.insert(entity_type, radius);
    }

    /// Update fog of war
    pub fn update(
        &mut self,
        entity_positions: &HashMap<u64, Vec3>,
        entity_types: &HashMap<u64, String>,
    ) {
        // Clear all revealed entities
        for team in self.team_visibility.values_mut() {
            team.revealed_entities.clear();
        }

        // For each team
        for (team_id, team) in &self.team_visibility {
            // For each vision source on this team
            for &source_id in &team.vision_sources {
                if let Some(&source_pos) = entity_positions.get(&source_id) {
                    // Get reveal radius
                    let radius = if let Some(entity_type) = entity_types.get(&source_id) {
                        *self.reveal_radius.get(entity_type).unwrap_or(&self.default_reveal_radius)
                    } else {
                        self.default_reveal_radius
                    };

                    // Reveal all entities in radius
                    for (target_id, target_pos) in entity_positions {
                        // Don't reveal entities on same team (they're always visible)
                        if let Some(&target_team) = self.entity_teams.get(target_id) {
                            if target_team == *team_id {
                                continue;
                            }
                        }

                        // Check distance
                        if source_pos.distance(*target_pos) <= radius {
                            // Would also check occlusion here
                            self.team_visibility
                                .get_mut(team_id)
                                .unwrap()
                                .revealed_entities
                                .insert(*target_id);
                        }
                    }
                }
            }
        }
    }

    /// Check if entity is visible to team
    pub fn is_visible_to_team(&self, entity_id: u64, team_id: u32) -> bool {
        // Same team = always visible
        if let Some(&entity_team) = self.entity_teams.get(&entity_id) {
            if entity_team == team_id {
                return true;
            }
        }

        // Check revealed entities
        if let Some(team) = self.team_visibility.get(&team_id) {
            team.revealed_entities.contains(&entity_id)
        } else {
            false
        }
    }

    /// Get visible entities for team
    pub fn get_visible_entities(&self, team_id: u32) -> Vec<u64> {
        let mut result = Vec::new();

        if let Some(team) = self.team_visibility.get(&team_id) {
            // Add own team entities
            for (entity_id, entity_team) in &self.entity_teams {
                if *entity_team == team_id {
                    result.push(*entity_id);
                }
            }

            // Add revealed entities
            result.extend(team.revealed_entities.iter());
        }

        result
    }
}
```

---

### **4. Advanced Interest Manager** (Day 3-4)

**File:** `engine/networking/src/interest/advanced.rs`

```rust
use crate::spatial::{SpatialGrid, OcclusionCuller, FogOfWar};
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// Advanced interest manager
pub struct AdvancedInterestManager {
    /// Spatial grid for fast queries
    spatial_grid: SpatialGrid,

    /// Occlusion culler
    occlusion_culler: OcclusionCuller,

    /// Fog of war
    fog_of_war: FogOfWar,

    /// Client interest sets
    client_interests: HashMap<u64, ClientInterestSet>,

    /// Configuration
    config: InterestConfig,
}

#[derive(Debug, Clone)]
pub struct InterestConfig {
    /// Max interest distance
    pub max_distance: f32,

    /// Predictive interest distance (pre-load)
    pub predictive_distance: f32,

    /// Update frequency (ticks)
    pub update_frequency: u32,

    /// Max entities per client
    pub max_entities_per_client: usize,

    /// Enable occlusion culling
    pub enable_occlusion: bool,

    /// Enable fog of war
    pub enable_fog_of_war: bool,
}

impl Default for InterestConfig {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            predictive_distance: 120.0,
            update_frequency: 10,
            max_entities_per_client: 1000,
            enable_occlusion: true,
            enable_fog_of_war: true,
        }
    }
}

struct ClientInterestSet {
    client_id: u64,
    client_position: Vec3,
    client_team: u32,
    interested_entities: HashSet<u64>,
    last_update_tick: u64,
}

impl AdvancedInterestManager {
    pub fn new(config: InterestConfig) -> Self {
        Self {
            spatial_grid: SpatialGrid::new(50.0), // 50m cell size
            occlusion_culler: OcclusionCuller::new(10000),
            fog_of_war: FogOfWar::new(50.0), // 50m default vision
            client_interests: HashMap::new(),
            config,
        }
    }

    /// Add client
    pub fn add_client(&mut self, client_id: u64, position: Vec3, team: u32) {
        self.client_interests.insert(
            client_id,
            ClientInterestSet {
                client_id,
                client_position: position,
                client_team: team,
                interested_entities: HashSet::new(),
                last_update_tick: 0,
            },
        );

        tracing::info!("Added client {} to interest manager", client_id);
    }

    /// Remove client
    pub fn remove_client(&mut self, client_id: u64) {
        self.client_interests.remove(&client_id);
        tracing::info!("Removed client {} from interest manager", client_id);
    }

    /// Update client position
    pub fn update_client_position(&mut self, client_id: u64, position: Vec3) {
        if let Some(interest) = self.client_interests.get_mut(&client_id) {
            interest.client_position = position;
        }
    }

    /// Update entity position
    pub fn update_entity_position(&mut self, entity_id: u64, position: Vec3) {
        self.spatial_grid.insert(entity_id, position);
    }

    /// Remove entity
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.spatial_grid.remove(entity_id);
        self.fog_of_war.remove_entity(entity_id);
    }

    /// Update interest sets
    pub fn update(
        &mut self,
        tick: u64,
        entity_positions: &HashMap<u64, Vec3>,
        entity_types: &HashMap<u64, String>,
    ) {
        // Update fog of war
        if self.config.enable_fog_of_war {
            self.fog_of_war.update(entity_positions, entity_types);
        }

        // Update each client's interest set
        for interest in self.client_interests.values_mut() {
            // Check update frequency
            if tick - interest.last_update_tick < self.config.update_frequency as u64 {
                continue;
            }

            interest.last_update_tick = tick;

            // Query nearby entities
            let nearby_entities = self.spatial_grid.query_radius(
                interest.client_position,
                self.config.max_distance,
            );

            // Filter by visibility
            let mut new_interest_set = HashSet::new();

            for entity_id in nearby_entities {
                if new_interest_set.len() >= self.config.max_entities_per_client {
                    break;
                }

                // Skip if entity doesn't exist
                if !entity_positions.contains_key(&entity_id) {
                    continue;
                }

                let entity_pos = entity_positions[&entity_id];

                // Check distance
                let distance = interest.client_position.distance(entity_pos);
                if distance > self.config.max_distance {
                    continue;
                }

                // Check fog of war
                if self.config.enable_fog_of_war {
                    if !self.fog_of_war.is_visible_to_team(entity_id, interest.client_team) {
                        continue;
                    }
                }

                // Check occlusion
                if self.config.enable_occlusion {
                    if !self.occlusion_culler.is_visible(
                        interest.client_position,
                        entity_pos,
                        interest.client_id,
                        entity_id,
                    ) {
                        continue;
                    }
                }

                new_interest_set.insert(entity_id);
            }

            // Update interest set
            let added: Vec<_> = new_interest_set
                .difference(&interest.interested_entities)
                .copied()
                .collect();

            let removed: Vec<_> = interest
                .interested_entities
                .difference(&new_interest_set)
                .copied()
                .collect();

            if !added.is_empty() || !removed.is_empty() {
                tracing::debug!(
                    "Client {} interest updated: +{} -{} (total: {})",
                    interest.client_id,
                    added.len(),
                    removed.len(),
                    new_interest_set.len()
                );
            }

            interest.interested_entities = new_interest_set;
        }

        // Prune occlusion cache
        self.occlusion_culler.prune_cache();
    }

    /// Get interested entities for client
    pub fn get_interested_entities(&self, client_id: u64) -> Vec<u64> {
        if let Some(interest) = self.client_interests.get(&client_id) {
            interest.interested_entities.iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    /// Check if client is interested in entity
    pub fn is_interested(&self, client_id: u64, entity_id: u64) -> bool {
        if let Some(interest) = self.client_interests.get(&client_id) {
            interest.interested_entities.contains(&entity_id)
        } else {
            false
        }
    }

    /// Get statistics
    pub fn stats(&self) -> InterestStats {
        let total_clients = self.client_interests.len();
        let total_entities = self.spatial_grid.entity_count();

        let mut total_interested = 0;
        for interest in self.client_interests.values() {
            total_interested += interest.interested_entities.len();
        }

        let avg_interested = if total_clients > 0 {
            total_interested / total_clients
        } else {
            0
        };

        InterestStats {
            total_clients,
            total_entities,
            total_interested,
            avg_interested,
            spatial_cells: self.spatial_grid.cell_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterestStats {
    pub total_clients: usize,
    pub total_entities: usize,
    pub total_interested: usize,
    pub avg_interested: usize,
    pub spatial_cells: usize,
}
```

---

### **5. Performance Testing** (Day 4-5)

**File:** `examples/interest_advanced_demo.rs`

```rust
use engine_networking::interest::AdvancedInterestManager;
use glam::Vec3;
use std::collections::HashMap;

fn main() {
    tracing_subscriber::fmt::init();

    // Create interest manager
    let mut interest_manager = AdvancedInterestManager::new(Default::default());

    // Create entity positions
    let mut entity_positions = HashMap::new();
    let mut entity_types = HashMap::new();

    // Spawn 10,000 entities
    for i in 0..10000 {
        let x = (i % 100) as f32 * 10.0 - 500.0;
        let z = (i / 100) as f32 * 10.0 - 500.0;

        entity_positions.insert(i, Vec3::new(x, 0.0, z));
        entity_types.insert(i, "soldier".to_string());

        interest_manager.update_entity_position(i, Vec3::new(x, 0.0, z));
    }

    // Add 100 clients
    for i in 0..100 {
        let x = (i % 10) as f32 * 50.0 - 250.0;
        let z = (i / 10) as f32 * 50.0 - 250.0;

        interest_manager.add_client(i, Vec3::new(x, 0.0, z), i % 4); // 4 teams
    }

    // Benchmark interest updates
    let iterations = 100;
    let mut total_time = std::time::Duration::ZERO;

    for tick in 0..iterations {
        let start = std::time::Instant::now();
        interest_manager.update(tick, &entity_positions, &entity_types);
        let elapsed = start.elapsed();
        total_time += elapsed;
    }

    let avg_time = total_time / iterations;

    let stats = interest_manager.stats();

    println!("=== Advanced Interest Management ===");
    println!("Total entities: {}", stats.total_entities);
    println!("Total clients: {}", stats.total_clients);
    println!("Average entities per client: {}", stats.avg_interested);
    println!("Spatial cells: {}", stats.spatial_cells);
    println!("Average update time: {:.3}ms", avg_time.as_secs_f64() * 1000.0);

    // Calculate overhead
    let server_tick_time = 16.67; // 60 TPS = 16.67ms per tick
    let overhead_percent = (avg_time.as_secs_f64() * 1000.0 / server_tick_time) * 100.0;

    println!("Server overhead: {:.2}%", overhead_percent);

    assert!(overhead_percent < 2.0, "Interest management overhead too high!");
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Spatial grid partitioning works
- [ ] Occlusion culling with raycasting
- [ ] Fog of war system (team-based)
- [ ] Advanced interest manager integrates all systems
- [ ] Per-client interest sets accurate
- [ ] Interest updates < 2% server overhead
- [ ] Supports 10,000+ entities
- [ ] Occlusion cache working
- [ ] Statistics tracking
- [ ] Example demonstrates scalability

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Interest update (10k entities, 100 clients) | < 5ms | < 10ms |
| Spatial query (100m radius) | < 0.5ms | < 2ms |
| Occlusion check (cached) | < 1μs | < 5μs |
| Occlusion check (uncached) | < 50μs | < 200μs |
| Fog of war update | < 2ms | < 5ms |
| Server overhead | < 2% | < 5% |

---

**Dependencies:** [phase2-state-sync.md](phase2-state-sync.md), [phase3-lod-networking.md](phase3-lod-networking.md)
**Next:** [phase3-cross-platform-verify.md](phase3-cross-platform-verify.md)
