# Interest Management

> **Interest management system for silmaril**
>
> Network optimization through spatial awareness and visibility culling

---

## Overview

Interest management reduces network bandwidth by only sending data about entities that are "interesting" to each client:
- **Spatial partitioning** - Grid-based proximity queries
- **Visibility culling** - Only replicate visible entities
- **Interest areas** - Define zones of relevance per client
- **Priority-based updates** - Send important entities first

**Use Cases:**
- **MMORPGs** - Players only see nearby entities
- **Battle royales** - Fog of war and shrinking play zones
- **Simulation games** - Partial world visibility

---

## Architecture

### Interest Manager

```rust
pub struct InterestManager {
    pub spatial_grid: SpatialGrid,
    pub client_interests: HashMap<ClientId, InterestArea>,
    pub visibility_range: f32,
}

pub struct InterestArea {
    pub center: Vec3,
    pub radius: f32,
    pub visible_entities: HashSet<Entity>,
}

impl InterestManager {
    pub fn new(cell_size: f32, visibility_range: f32) -> Self {
        Self {
            spatial_grid: SpatialGrid::new(cell_size),
            client_interests: HashMap::new(),
            visibility_range,
        }
    }

    pub fn register_client(&mut self, client_id: ClientId, position: Vec3) {
        self.client_interests.insert(
            client_id,
            InterestArea {
                center: position,
                radius: self.visibility_range,
                visible_entities: HashSet::new(),
            },
        );
    }

    pub fn update_client_position(&mut self, client_id: ClientId, position: Vec3) {
        if let Some(interest) = self.client_interests.get_mut(&client_id) {
            interest.center = position;
        }
    }

    pub fn query_visible_entities(&self, client_id: ClientId) -> Vec<Entity> {
        let interest = match self.client_interests.get(&client_id) {
            Some(i) => i,
            None => return Vec::new(),
        };

        self.spatial_grid.query_radius(interest.center, interest.radius)
    }
}
```

**Implementation:** TBD `engine/interest/src/manager.rs`

---

## Spatial Grid

### Grid-Based Partitioning

Divide world into cells for fast proximity queries:

```rust
use std::collections::HashMap;

pub struct SpatialGrid {
    pub cell_size: f32,
    pub cells: HashMap<IVec2, GridCell>,
}

pub struct GridCell {
    pub entities: HashSet<Entity>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn world_to_cell(&self, position: Vec3) -> IVec2 {
        IVec2::new(
            (position.x / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }

    pub fn insert(&mut self, entity: Entity, position: Vec3) {
        let cell_pos = self.world_to_cell(position);
        self.cells
            .entry(cell_pos)
            .or_insert_with(|| GridCell {
                entities: HashSet::new(),
            })
            .entities
            .insert(entity);
    }

    pub fn remove(&mut self, entity: Entity, position: Vec3) {
        let cell_pos = self.world_to_cell(position);
        if let Some(cell) = self.cells.get_mut(&cell_pos) {
            cell.entities.remove(&entity);
        }
    }

    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        let center_cell = self.world_to_cell(center);
        let cell_radius = (radius / self.cell_size).ceil() as i32;

        let mut results = Vec::new();

        for x in -cell_radius..=cell_radius {
            for z in -cell_radius..=cell_radius {
                let cell_pos = center_cell + IVec2::new(x, z);

                if let Some(cell) = self.cells.get(&cell_pos) {
                    for entity in &cell.entities {
                        results.push(*entity);
                    }
                }
            }
        }

        results
    }
}
```

**Implementation:** `engine/core/src/spatial/grid.rs` ✅ Complete

### Spatial Grid Update System

Keep spatial grid synchronized with entity positions:

```rust
use engine_profiling::profile_scope;

#[server_only]
#[profile(category = "Networking")]
pub fn spatial_grid_update_system(
    world: &World,
    spatial_grid: &mut SpatialGrid,
) {
    profile_scope!("spatial_grid_update");

    // Clear grid
    spatial_grid.cells.clear();

    // Rebuild from current entity positions
    for (entity, transform) in world.query::<(&Entity, &Transform)>() {
        spatial_grid.insert(*entity, transform.position);
    }
}
```

---

## Interest Update System

### Server-Side Interest Management

Calculate which entities each client can see:

```rust
#[server_only]
#[profile(category = "Networking")]
pub fn interest_management_system(
    world: &World,
    interest_manager: &mut InterestManager,
    client_positions: &HashMap<ClientId, Vec3>,
) {
    profile_scope!("interest_management");

    // Update client positions
    for (client_id, position) in client_positions {
        interest_manager.update_client_position(*client_id, *position);
    }

    // Update visible entities for each client
    for (client_id, interest) in interest_manager.client_interests.iter_mut() {
        let old_visible = std::mem::take(&mut interest.visible_entities);
        let new_visible: HashSet<Entity> = interest_manager
            .spatial_grid
            .query_radius(interest.center, interest.radius)
            .into_iter()
            .collect();

        // Calculate diffs
        let entered: Vec<_> = new_visible.difference(&old_visible).copied().collect();
        let exited: Vec<_> = old_visible.difference(&new_visible).copied().collect();

        // Queue enter/exit events
        for entity in entered {
            queue_entity_enter_event(*client_id, entity);
        }

        for entity in exited {
            queue_entity_exit_event(*client_id, entity);
        }

        interest.visible_entities = new_visible;
    }
}

fn queue_entity_enter_event(client_id: ClientId, entity: Entity) {
    // Send full entity state to client
}

fn queue_entity_exit_event(client_id: ClientId, entity: Entity) {
    // Send despawn message to client
}
```

---

## Priority System

### Update Priorities

Prioritize important entities:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityPriority {
    Critical,   // Always send (player, important NPCs)
    High,       // Send frequently (nearby enemies)
    Medium,     // Send occasionally (distant NPCs)
    Low,        // Send rarely (environmental objects)
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkPriority {
    pub priority: EntityPriority,
}

impl NetworkPriority {
    pub fn calculate_priority(
        distance: f32,
        is_player: bool,
        is_moving: bool,
    ) -> EntityPriority {
        if is_player {
            return EntityPriority::Critical;
        }

        match distance {
            d if d < 10.0 => EntityPriority::High,
            d if d < 50.0 && is_moving => EntityPriority::High,
            d if d < 50.0 => EntityPriority::Medium,
            _ => EntityPriority::Low,
        }
    }
}
```

### Priority-Based Bandwidth Allocation

Send high-priority entities first:

```rust
pub struct BandwidthBudget {
    pub max_bytes_per_tick: usize,
    pub used_bytes: usize,
}

impl BandwidthBudget {
    pub fn new(max_kbps: f32, tick_rate: f32) -> Self {
        let max_bytes_per_tick = ((max_kbps * 1024.0) / tick_rate) as usize;
        Self {
            max_bytes_per_tick,
            used_bytes: 0,
        }
    }

    pub fn can_send(&self, message_size: usize) -> bool {
        self.used_bytes + message_size <= self.max_bytes_per_tick
    }

    pub fn record_send(&mut self, message_size: usize) {
        self.used_bytes += message_size;
    }

    pub fn reset(&mut self) {
        self.used_bytes = 0;
    }
}

#[server_only]
pub fn send_state_updates_prioritized(
    client_id: ClientId,
    visible_entities: &[Entity],
    world: &World,
    budget: &mut BandwidthBudget,
) {
    // Sort by priority
    let mut prioritized: Vec<_> = visible_entities
        .iter()
        .filter_map(|entity| {
            let priority = world.get::<NetworkPriority>(*entity)?;
            Some((*entity, priority.priority))
        })
        .collect();

    prioritized.sort_by_key(|(_, priority)| std::cmp::Reverse(*priority));

    // Send entities until budget exhausted
    for (entity, _) in prioritized {
        let message = create_entity_state_message(world, entity);
        let size = message.size_bytes();

        if budget.can_send(size) {
            send_to_client(client_id, message);
            budget.record_send(size);
        } else {
            break; // Budget exhausted
        }
    }
}
```

---

## Fog of War

### Visibility System

Track which areas each client has explored:

```rust
pub struct FogOfWar {
    pub grid: Vec<Vec<FogState>>,
    pub grid_size: IVec2,
    pub cell_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogState {
    Unexplored,
    Explored,
    Visible,
}

impl FogOfWar {
    pub fn new(world_size: Vec2, cell_size: f32) -> Self {
        let grid_size = IVec2::new(
            (world_size.x / cell_size).ceil() as i32,
            (world_size.y / cell_size).ceil() as i32,
        );

        let grid = vec![vec![FogState::Unexplored; grid_size.x as usize]; grid_size.y as usize];

        Self {
            grid,
            grid_size,
            cell_size,
        }
    }

    pub fn update_visibility(&mut self, center: Vec3, vision_radius: f32) {
        let center_cell = self.world_to_cell(center);
        let cell_radius = (vision_radius / self.cell_size).ceil() as i32;

        // Clear current visibility
        for row in &mut self.grid {
            for cell in row {
                if *cell == FogState::Visible {
                    *cell = FogState::Explored;
                }
            }
        }

        // Set new visibility
        for x in -cell_radius..=cell_radius {
            for z in -cell_radius..=cell_radius {
                let cell_pos = center_cell + IVec2::new(x, z);

                if self.is_in_bounds(cell_pos) {
                    let distance = (cell_pos - center_cell).as_vec2().length();
                    if distance <= cell_radius as f32 {
                        self.grid[cell_pos.y as usize][cell_pos.x as usize] = FogState::Visible;
                    }
                }
            }
        }
    }

    pub fn is_visible(&self, position: Vec3) -> bool {
        let cell = self.world_to_cell(position);
        if !self.is_in_bounds(cell) {
            return false;
        }

        matches!(
            self.grid[cell.y as usize][cell.x as usize],
            FogState::Visible
        )
    }

    fn world_to_cell(&self, position: Vec3) -> IVec2 {
        IVec2::new(
            (position.x / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }

    fn is_in_bounds(&self, cell: IVec2) -> bool {
        cell.x >= 0 && cell.x < self.grid_size.x && cell.y >= 0 && cell.y < self.grid_size.y
    }
}
```

### Fog of War System

```rust
#[server_only]
#[profile(category = "Networking")]
pub fn fog_of_war_system(
    world: &World,
    client_fow: &mut HashMap<ClientId, FogOfWar>,
    client_positions: &HashMap<ClientId, Vec3>,
) {
    profile_scope!("fog_of_war");

    for (client_id, position) in client_positions {
        if let Some(fow) = client_fow.get_mut(client_id) {
            fow.update_visibility(*position, 50.0); // 50m vision radius
        }
    }
}
```

---

## Area of Interest (AOI)

### Dynamic Interest Zones

Define custom interest zones per client:

```rust
#[derive(Debug, Clone)]
pub struct AreaOfInterest {
    pub zones: Vec<InterestZone>,
}

#[derive(Debug, Clone)]
pub struct InterestZone {
    pub shape: ZoneShape,
    pub priority: EntityPriority,
}

#[derive(Debug, Clone)]
pub enum ZoneShape {
    Circle { center: Vec3, radius: f32 },
    Rectangle { min: Vec3, max: Vec3 },
    Cone { origin: Vec3, direction: Vec3, angle: f32, range: f32 },
}

impl ZoneShape {
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            Self::Circle { center, radius } => {
                (point - *center).length() <= *radius
            }
            Self::Rectangle { min, max } => {
                point.x >= min.x
                    && point.x <= max.x
                    && point.y >= min.y
                    && point.y <= max.y
                    && point.z >= min.z
                    && point.z <= max.z
            }
            Self::Cone { origin, direction, angle, range } => {
                let to_point = (point - *origin).normalize();
                let dot = to_point.dot(*direction);
                let dist = (point - *origin).length();

                dot >= angle.cos() && dist <= *range
            }
        }
    }
}
```

### Multi-Zone Interest

Assign priorities based on zone containment:

```rust
pub fn calculate_entity_priority_with_zones(
    entity_pos: Vec3,
    aoi: &AreaOfInterest,
) -> Option<EntityPriority> {
    let mut best_priority = None;

    for zone in &aoi.zones {
        if zone.shape.contains(entity_pos) {
            best_priority = Some(match best_priority {
                Some(existing) => existing.max(zone.priority),
                None => zone.priority,
            });
        }
    }

    best_priority
}
```

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Spatial query (1000 entities) | < 100μs | < 500μs |
| Interest update (100 clients) | < 5ms | < 10ms |
| Bandwidth reduction | > 70% (sparse) | > 50% |
| Memory overhead per client | < 100KB | < 1MB |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_grid_insert_query() {
        let mut grid = SpatialGrid::new(10.0);

        let entity = Entity::new(0, 0);
        grid.insert(entity, Vec3::new(5.0, 0.0, 5.0));

        let results = grid.query_radius(Vec3::new(5.0, 0.0, 5.0), 10.0);
        assert!(results.contains(&entity));
    }

    #[test]
    fn test_fog_of_war_visibility() {
        let mut fow = FogOfWar::new(Vec2::new(100.0, 100.0), 1.0);

        fow.update_visibility(Vec3::new(50.0, 0.0, 50.0), 10.0);

        assert!(fow.is_visible(Vec3::new(50.0, 0.0, 50.0)));
        assert!(!fow.is_visible(Vec3::new(0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_zone_containment() {
        let zone = ZoneShape::Circle {
            center: Vec3::ZERO,
            radius: 10.0,
        };

        assert!(zone.contains(Vec3::new(5.0, 0.0, 0.0)));
        assert!(!zone.contains(Vec3::new(15.0, 0.0, 0.0)));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_interest_manager_update() {
    let mut manager = InterestManager::new(10.0, 50.0);

    let client_id = ClientId(1);
    manager.register_client(client_id, Vec3::ZERO);

    let entity = Entity::new(0, 0);
    manager.spatial_grid.insert(entity, Vec3::new(10.0, 0.0, 10.0));

    let visible = manager.query_visible_entities(client_id);
    assert!(visible.contains(&entity));
}
```

---

## Best Practices

### DO

- ✅ Use spatial partitioning for large worlds
- ✅ Implement priority-based updates
- ✅ Profile interest queries regularly
- ✅ Tune cell size to game scale
- ✅ Combine with LOD for max savings

### DON'T

- ❌ Query entire world for every client
- ❌ Use too small cell sizes (overhead)
- ❌ Forget to update spatial grid
- ❌ Send all entities to all clients
- ❌ Ignore bandwidth budgets

---

## Advanced Topics

### Hierarchical Grids

Multi-resolution grids for varying densities:

```rust
pub struct HierarchicalGrid {
    pub levels: Vec<SpatialGrid>,
    pub level_sizes: Vec<f32>,
}

impl HierarchicalGrid {
    pub fn query_adaptive(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        // Use finer grid for small radius, coarser for large
        let level = self.select_level(radius);
        self.levels[level].query_radius(center, radius)
    }

    fn select_level(&self, radius: f32) -> usize {
        for (i, size) in self.level_sizes.iter().enumerate() {
            if radius < size * 2.0 {
                return i;
            }
        }
        self.levels.len() - 1
    }
}
```

### Predictive Interest

Predict where clients will move:

```rust
pub fn predict_future_interest(
    position: Vec3,
    velocity: Vec3,
    prediction_time: f32,
) -> Vec3 {
    position + velocity * prediction_time
}

pub fn prefetch_entities_for_predicted_position(
    manager: &InterestManager,
    client_id: ClientId,
    velocity: Vec3,
) -> Vec<Entity> {
    let current_pos = manager.client_interests[&client_id].center;
    let predicted_pos = predict_future_interest(current_pos, velocity, 1.0);

    manager.spatial_grid.query_radius(predicted_pos, 50.0)
}
```

---

## References

- **Implementation:** TBD `engine/interest/src/`
- **Spatial Grid:** `engine/core/src/spatial/grid.rs` ✅
- **Networking Integration:** `engine/networking/src/`

**Related Documentation:**
- [Networking](networking.md)
- [LOD System](lod.md)
- [ECS](ecs.md)
- [Performance Targets](performance-targets.md)
