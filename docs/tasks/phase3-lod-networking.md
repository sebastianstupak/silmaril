# Phase 3.4: Network Update Rate LOD

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (bandwidth optimization)

---

## 🎯 **Objective**

Implement network-level LOD system that reduces update rates and filters components based on distance to players. Dramatically reduces bandwidth usage by sending fewer/smaller updates for distant entities.

**Features:**
- Distance-based update rate LOD
- Component filtering (only send relevant components)
- Per-client interest sets with LOD
- Bandwidth optimization (80%+ reduction)
- Adaptive update rates
- Priority-based updates

---

## 📋 **Detailed Tasks**

### **1. Network LOD Configuration** (Day 1)

**File:** `engine/networking/src/lod/config.rs`

```rust
use serde::{Deserialize, Serialize};

/// Network LOD configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLodConfig {
    /// LOD levels for network updates
    pub levels: Vec<NetworkLodLevel>,

    /// Component filtering rules
    pub component_filters: ComponentFilterRules,

    /// Minimum update rate (updates per second)
    pub min_update_rate: u32,

    /// Maximum update rate (updates per second)
    pub max_update_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLodLevel {
    /// LOD level (0 = highest priority)
    pub level: u32,

    /// Distance threshold (meters)
    pub distance: f32,

    /// Update rate (updates per second)
    pub update_rate: u32,

    /// Component filter
    pub component_filter: ComponentFilter,

    /// Position quantization (reduces precision to save bandwidth)
    pub position_quantization: f32,

    /// Rotation quantization (degrees)
    pub rotation_quantization: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentFilter {
    /// Send all components
    All,

    /// Send only these components
    Include(Vec<String>),

    /// Send all except these components
    Exclude(Vec<String>),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentFilterRules {
    /// Always send these components (regardless of LOD)
    pub always_send: Vec<String>,

    /// Never send these components (server-only)
    pub never_send: Vec<String>,

    /// Component importance (higher = more important)
    pub importance: std::collections::HashMap<String, u32>,
}

impl Default for NetworkLodConfig {
    fn default() -> Self {
        Self {
            levels: vec![
                // LOD 0: Very close (< 10m)
                NetworkLodLevel {
                    level: 0,
                    distance: 10.0,
                    update_rate: 60, // 60 Hz
                    component_filter: ComponentFilter::All,
                    position_quantization: 0.01,  // 1cm precision
                    rotation_quantization: 1.0,   // 1 degree
                },
                // LOD 1: Close (10-30m)
                NetworkLodLevel {
                    level: 1,
                    distance: 30.0,
                    update_rate: 30, // 30 Hz
                    component_filter: ComponentFilter::Exclude(vec![
                        "ParticleEmitter".to_string(),
                        "AudioSource".to_string(),
                    ]),
                    position_quantization: 0.05,  // 5cm precision
                    rotation_quantization: 5.0,   // 5 degrees
                },
                // LOD 2: Medium (30-60m)
                NetworkLodLevel {
                    level: 2,
                    distance: 60.0,
                    update_rate: 10, // 10 Hz
                    component_filter: ComponentFilter::Include(vec![
                        "Transform".to_string(),
                        "Rigidbody".to_string(),
                        "Health".to_string(),
                    ]),
                    position_quantization: 0.1,   // 10cm precision
                    rotation_quantization: 10.0,  // 10 degrees
                },
                // LOD 3: Far (60-120m)
                NetworkLodLevel {
                    level: 3,
                    distance: 120.0,
                    update_rate: 5, // 5 Hz
                    component_filter: ComponentFilter::Include(vec![
                        "Transform".to_string(),
                    ]),
                    position_quantization: 0.5,   // 50cm precision
                    rotation_quantization: 45.0,  // 45 degrees
                },
                // LOD 4: Very far (> 120m)
                NetworkLodLevel {
                    level: 4,
                    distance: f32::INFINITY,
                    update_rate: 1, // 1 Hz (minimal updates)
                    component_filter: ComponentFilter::Include(vec![
                        "Transform".to_string(),
                    ]),
                    position_quantization: 1.0,   // 1m precision
                    rotation_quantization: 90.0,  // 90 degrees
                },
            ],
            component_filters: ComponentFilterRules {
                always_send: vec![
                    "Transform".to_string(),
                ],
                never_send: vec![
                    "PhysicsInternal".to_string(),
                    "ServerOnly".to_string(),
                ],
                importance: [
                    ("Transform".to_string(), 100),
                    ("Rigidbody".to_string(), 80),
                    ("Health".to_string(), 70),
                    ("Weapon".to_string(), 60),
                    ("ParticleEmitter".to_string(), 30),
                    ("AudioSource".to_string(), 20),
                ]
                .iter()
                .cloned()
                .collect(),
            },
            min_update_rate: 1,
            max_update_rate: 60,
        }
    }
}

impl NetworkLodConfig {
    /// Get LOD level for distance
    pub fn get_lod_level(&self, distance: f32) -> &NetworkLodLevel {
        for level in &self.levels {
            if distance < level.distance {
                return level;
            }
        }

        // Return last level (furthest)
        self.levels.last().unwrap()
    }

    /// Check if component should be sent at this LOD level
    pub fn should_send_component(
        &self,
        component_name: &str,
        lod_level: &NetworkLodLevel,
    ) -> bool {
        // Never send blacklisted components
        if self.component_filters.never_send.contains(&component_name.to_string()) {
            return false;
        }

        // Always send whitelisted components
        if self.component_filters.always_send.contains(&component_name.to_string()) {
            return true;
        }

        // Apply LOD-specific filter
        match &lod_level.component_filter {
            ComponentFilter::All => true,
            ComponentFilter::Include(list) => list.contains(&component_name.to_string()),
            ComponentFilter::Exclude(list) => !list.contains(&component_name.to_string()),
        }
    }
}
```

---

### **2. Network LOD Manager** (Day 1-2)

**File:** `engine/networking/src/lod/manager.rs`

```rust
use crate::lod::config::{NetworkLodConfig, NetworkLodLevel};
use std::collections::HashMap;
use glam::Vec3;

/// Network LOD manager (per client)
pub struct NetworkLodManager {
    /// Client ID
    client_id: u64,

    /// Client position (for distance calculation)
    client_position: Vec3,

    /// Entity LOD states
    entity_states: HashMap<u64, EntityNetworkLod>,

    /// Configuration
    config: NetworkLodConfig,

    /// Current tick
    current_tick: u64,

    /// Statistics
    stats: NetworkLodStats,
}

#[derive(Debug, Clone)]
struct EntityNetworkLod {
    /// Entity ID
    entity_id: u64,

    /// Current LOD level
    lod_level: u32,

    /// Last update tick
    last_update_tick: u64,

    /// Distance to client
    distance: f32,

    /// Update rate (ticks between updates)
    update_interval: u64,
}

#[derive(Debug, Default, Clone)]
pub struct NetworkLodStats {
    pub total_entities: usize,
    pub entities_per_lod: [u32; 8],
    pub updates_this_tick: u32,
    pub bytes_saved: u64,
    pub bandwidth_reduction_percent: f32,
}

impl NetworkLodManager {
    pub fn new(client_id: u64, config: NetworkLodConfig) -> Self {
        Self {
            client_id,
            client_position: Vec3::ZERO,
            entity_states: HashMap::new(),
            config,
            current_tick: 0,
            stats: NetworkLodStats::default(),
        }
    }

    /// Update client position
    pub fn set_client_position(&mut self, position: Vec3) {
        self.client_position = position;
    }

    /// Update tick
    pub fn tick(&mut self, tick: u64) {
        self.current_tick = tick;
        self.stats = NetworkLodStats::default();
    }

    /// Update entity LOD state
    pub fn update_entity(
        &mut self,
        entity_id: u64,
        entity_position: Vec3,
    ) {
        // Calculate distance
        let distance = (entity_position - self.client_position).length();

        // Get appropriate LOD level
        let lod_config = self.config.get_lod_level(distance);

        // Calculate update interval from update rate
        let server_tick_rate = 60; // Assume 60 TPS
        let update_interval = (server_tick_rate / lod_config.update_rate).max(1) as u64;

        // Get or create entity state
        let state = self.entity_states.entry(entity_id).or_insert_with(|| {
            EntityNetworkLod {
                entity_id,
                lod_level: lod_config.level,
                last_update_tick: 0,
                distance,
                update_interval,
            }
        });

        // Update state
        state.distance = distance;
        state.lod_level = lod_config.level;
        state.update_interval = update_interval;

        // Update stats
        self.stats.total_entities += 1;
        if (state.lod_level as usize) < self.stats.entities_per_lod.len() {
            self.stats.entities_per_lod[state.lod_level as usize] += 1;
        }
    }

    /// Check if entity should be updated this tick
    pub fn should_update_entity(&mut self, entity_id: u64) -> bool {
        if let Some(state) = self.entity_states.get_mut(&entity_id) {
            let ticks_since_update = self.current_tick - state.last_update_tick;

            if ticks_since_update >= state.update_interval {
                state.last_update_tick = self.current_tick;
                self.stats.updates_this_tick += 1;
                return true;
            }
        }

        false
    }

    /// Get LOD level for entity
    pub fn get_entity_lod_level(&self, entity_id: u64) -> Option<u32> {
        self.entity_states.get(&entity_id).map(|s| s.lod_level)
    }

    /// Get component filter for entity
    pub fn filter_components(
        &self,
        entity_id: u64,
        components: &[String],
    ) -> Vec<String> {
        if let Some(state) = self.entity_states.get(&entity_id) {
            let lod_config = &self.config.levels[state.lod_level as usize];

            components
                .iter()
                .filter(|comp| {
                    self.config.should_send_component(comp, lod_config)
                })
                .cloned()
                .collect()
        } else {
            components.to_vec()
        }
    }

    /// Quantize position based on LOD level
    pub fn quantize_position(&self, entity_id: u64, position: Vec3) -> Vec3 {
        if let Some(state) = self.entity_states.get(&entity_id) {
            let lod_config = &self.config.levels[state.lod_level as usize];
            let q = lod_config.position_quantization;

            Vec3::new(
                (position.x / q).round() * q,
                (position.y / q).round() * q,
                (position.z / q).round() * q,
            )
        } else {
            position
        }
    }

    /// Quantize rotation based on LOD level
    pub fn quantize_rotation(&self, entity_id: u64, rotation: glam::Quat) -> glam::Quat {
        if let Some(state) = self.entity_states.get(&entity_id) {
            let lod_config = &self.config.levels[state.lod_level as usize];
            let q = lod_config.rotation_quantization;

            // Convert to euler, quantize, convert back
            let (x, y, z) = rotation.to_euler(glam::EulerRot::XYZ);
            let x = (x.to_degrees() / q).round() * q;
            let y = (y.to_degrees() / q).round() * q;
            let z = (z.to_degrees() / q).round() * q;

            glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                x.to_radians(),
                y.to_radians(),
                z.to_radians(),
            )
        } else {
            rotation
        }
    }

    /// Remove entity
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entity_states.remove(&entity_id);
    }

    /// Get statistics
    pub fn stats(&self) -> &NetworkLodStats {
        &self.stats
    }

    /// Calculate bandwidth reduction
    pub fn calculate_bandwidth_reduction(&mut self) {
        // Assume all entities at LOD 0 (baseline)
        let baseline_updates = self.stats.total_entities as u32;

        // Actual updates with LOD
        let actual_updates = self.stats.updates_this_tick;

        if baseline_updates > 0 {
            self.stats.bandwidth_reduction_percent =
                (1.0 - (actual_updates as f32 / baseline_updates as f32)) * 100.0;
        }
    }
}
```

---

### **3. Server Integration** (Day 2-3)

**File:** `server/src/network_lod.rs`

```rust
use engine_networking::lod::{NetworkLodManager, NetworkLodConfig};
use engine_ecs::{World, Entity, Transform};
use std::collections::HashMap;
use glam::Vec3;

/// Server-side network LOD system
pub struct ServerNetworkLod {
    /// LOD manager per client
    client_lod_managers: HashMap<u64, NetworkLodManager>,

    /// Configuration
    config: NetworkLodConfig,

    /// Statistics (aggregated across all clients)
    total_updates_saved: u64,
    total_bandwidth_saved: u64,
}

impl ServerNetworkLod {
    pub fn new(config: NetworkLodConfig) -> Self {
        Self {
            client_lod_managers: HashMap::new(),
            config,
            total_updates_saved: 0,
            total_bandwidth_saved: 0,
        }
    }

    /// Add client
    pub fn add_client(&mut self, client_id: u64, position: Vec3) {
        let mut manager = NetworkLodManager::new(client_id, self.config.clone());
        manager.set_client_position(position);
        self.client_lod_managers.insert(client_id, manager);

        tracing::info!("Added network LOD manager for client {}", client_id);
    }

    /// Remove client
    pub fn remove_client(&mut self, client_id: u64) {
        self.client_lod_managers.remove(&client_id);
        tracing::info!("Removed network LOD manager for client {}", client_id);
    }

    /// Update client position
    pub fn update_client_position(&mut self, client_id: u64, position: Vec3) {
        if let Some(manager) = self.client_lod_managers.get_mut(&client_id) {
            manager.set_client_position(position);
        }
    }

    /// Update all entities for all clients
    pub fn update(&mut self, world: &World, tick: u64) {
        for manager in self.client_lod_managers.values_mut() {
            manager.tick(tick);

            // Update all entities
            for (entity, transform) in world.query::<&Transform>().iter() {
                manager.update_entity(entity.id(), transform.position);
            }

            manager.calculate_bandwidth_reduction();
        }
    }

    /// Get entities to update for client
    pub fn get_entities_to_update(
        &mut self,
        client_id: u64,
        world: &World,
    ) -> Vec<(u64, Vec<String>)> {
        if let Some(manager) = self.client_lod_managers.get_mut(&client_id) {
            let mut entities_to_update = Vec::new();

            for (entity, _) in world.query::<&Transform>().iter() {
                if manager.should_update_entity(entity.id()) {
                    // Get component names for this entity
                    let component_names = get_entity_component_names(world, entity);

                    // Filter components based on LOD
                    let filtered_components = manager.filter_components(
                        entity.id(),
                        &component_names,
                    );

                    entities_to_update.push((entity.id(), filtered_components));
                }
            }

            entities_to_update
        } else {
            Vec::new()
        }
    }

    /// Quantize transform for client
    pub fn quantize_transform(
        &self,
        client_id: u64,
        entity_id: u64,
        transform: &Transform,
    ) -> Transform {
        if let Some(manager) = self.client_lod_managers.get(&client_id) {
            Transform {
                position: manager.quantize_position(entity_id, transform.position),
                rotation: manager.quantize_rotation(entity_id, transform.rotation),
                scale: transform.scale, // Don't quantize scale
            }
        } else {
            *transform
        }
    }

    /// Get statistics
    pub fn print_stats(&self) {
        println!("=== Network LOD Statistics ===");

        for (client_id, manager) in &self.client_lod_managers {
            let stats = manager.stats();
            println!("Client {}: {} entities", client_id, stats.total_entities);
            println!("  Updates this tick: {}", stats.updates_this_tick);
            println!("  Bandwidth reduction: {:.1}%", stats.bandwidth_reduction_percent);
            println!("  LOD distribution: {:?}", stats.entities_per_lod);
        }

        println!("==============================");
    }
}

/// Helper: Get component names for entity
fn get_entity_component_names(world: &World, entity: Entity) -> Vec<String> {
    // This would need to query the world's component registry
    // For now, return common components
    vec![
        "Transform".to_string(),
        "Rigidbody".to_string(),
        "Health".to_string(),
        "Weapon".to_string(),
    ]
}
```

---

### **4. Example & Testing** (Day 3-4)

**File:** `examples/network_lod_demo.rs`

```rust
use engine_ecs::prelude::*;
use engine_networking::lod::{NetworkLodConfig, NetworkLodManager};
use glam::{Vec3, Quat};

fn main() {
    tracing_subscriber::fmt::init();

    // Create world with many entities
    let mut world = World::new();

    // Spawn entities in grid
    for x in -50..50 {
        for z in -50..50 {
            let entity = world.spawn();
            world.add_component(entity, Transform {
                position: Vec3::new(x as f32 * 10.0, 0.0, z as f32 * 10.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            });
        }
    }

    println!("Created {} entities", world.entity_count());

    // Create network LOD manager (simulating one client)
    let mut lod_manager = NetworkLodManager::new(1, NetworkLodConfig::default());

    // Simulate client at origin
    lod_manager.set_client_position(Vec3::ZERO);

    // Simulate 60 ticks (1 second at 60 TPS)
    let mut total_updates = 0;
    let mut total_entities = 0;

    for tick in 0..60 {
        lod_manager.tick(tick);

        // Update all entities
        for (entity, transform) in world.query::<&Transform>().iter() {
            lod_manager.update_entity(entity.id(), transform.position);
            total_entities += 1;

            if lod_manager.should_update_entity(entity.id()) {
                total_updates += 1;
            }
        }

        lod_manager.calculate_bandwidth_reduction();
    }

    // Calculate statistics
    let stats = lod_manager.stats();

    println!("\n=== Network LOD Performance ===");
    println!("Total entities: {}", total_entities);
    println!("Total updates (1 second): {}", total_updates);
    println!("Baseline updates (no LOD): {}", total_entities);
    println!(
        "Bandwidth reduction: {:.1}%",
        (1.0 - (total_updates as f32 / total_entities as f32)) * 100.0
    );
    println!("LOD distribution: {:?}", stats.entities_per_lod);

    // Verify bandwidth reduction
    let reduction_percent = (1.0 - (total_updates as f32 / total_entities as f32)) * 100.0;
    assert!(
        reduction_percent > 80.0,
        "Bandwidth reduction too low: {:.1}%",
        reduction_percent
    );
}
```

---

## ✅ **Acceptance Criteria**

- [ ] NetworkLodConfig defines LOD levels
- [ ] NetworkLodManager tracks per-client LOD
- [ ] Distance-based update rate LOD works
- [ ] Component filtering works
- [ ] Position/rotation quantization works
- [ ] Per-client interest sets with LOD
- [ ] Bandwidth reduction > 80%
- [ ] LOD manager < 1ms for 1000 entities
- [ ] Statistics tracking works
- [ ] Example demonstrates bandwidth savings

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| LOD update (1000 entities/client) | < 1ms | < 3ms |
| Should update check | < 1μs | < 5μs |
| Component filtering | < 10μs | < 50μs |
| Quantization | < 1μs | < 5μs |
| Bandwidth reduction | > 80% | > 60% |
| Update rate reduction | > 90% | > 70% |

---

**Dependencies:** [phase2-state-sync.md](phase2-state-sync.md), [phase2-server-tick.md](phase2-server-tick.md)
**Next:** [phase3-interest-advanced.md](phase3-interest-advanced.md)
