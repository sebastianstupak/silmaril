# Phase 3.3: Rendering Level of Detail (LOD)

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (performance optimization)

---

## 🎯 **Objective**

Implement mesh-based Level of Detail (LOD) system for rendering optimization. Automatically selects mesh detail based on distance to camera, reducing vertex processing and improving frame rate.

**Features:**
- Distance-based LOD selection
- Multiple LOD levels per mesh
- Automatic LOD transitions
- LOD generation/loading pipeline
- Hysteresis to prevent LOD popping
- Performance metrics and tuning

---

## 📋 **Detailed Tasks**

### **1. LOD Data Structures** (Day 1)

**File:** `engine/rendering/src/lod/mod.rs`

```rust
use glam::Vec3;
use std::sync::Arc;

/// LOD level for a mesh
#[derive(Debug, Clone)]
pub struct MeshLod {
    /// LOD level (0 = highest detail)
    pub level: u32,

    /// Mesh data for this LOD
    pub mesh: Arc<MeshData>,

    /// Distance threshold (switch to this LOD beyond this distance)
    pub distance: f32,

    /// Screen coverage threshold (% of screen)
    pub screen_coverage: f32,

    /// Triangle count
    pub triangle_count: u32,
}

/// Mesh with multiple LOD levels
#[derive(Debug, Clone)]
pub struct LodMesh {
    /// All LOD levels (sorted by level, 0 = highest)
    pub levels: Vec<MeshLod>,

    /// Current active LOD level
    pub active_level: u32,

    /// Bounds for distance calculation
    pub bounds: BoundingSphere,

    /// LOD bias (multiplier for distance thresholds)
    pub lod_bias: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl LodMesh {
    pub fn new(levels: Vec<MeshLod>, bounds: BoundingSphere) -> Self {
        Self {
            levels,
            active_level: 0,
            bounds,
            lod_bias: 1.0,
        }
    }

    /// Get current active LOD
    pub fn active_lod(&self) -> Option<&MeshLod> {
        self.levels.iter().find(|lod| lod.level == self.active_level)
    }

    /// Get mesh for current LOD
    pub fn active_mesh(&self) -> Option<&Arc<MeshData>> {
        self.active_lod().map(|lod| &lod.mesh)
    }

    /// Update LOD based on distance to camera
    pub fn update_lod(
        &mut self,
        camera_position: Vec3,
        hysteresis_factor: f32,
    ) -> Option<u32> {
        if self.levels.is_empty() {
            return None;
        }

        // Calculate distance from camera to mesh bounds
        let distance = (camera_position - self.bounds.center).length() - self.bounds.radius;
        let distance = distance.max(0.0);

        // Apply LOD bias
        let effective_distance = distance / self.lod_bias;

        // Find appropriate LOD level
        let mut new_level = self.levels.len() - 1; // Default to lowest detail

        for (i, lod) in self.levels.iter().enumerate() {
            let threshold = lod.distance;

            // Apply hysteresis to prevent flickering
            let threshold = if lod.level > self.active_level {
                // Switching to lower detail: increase threshold
                threshold * (1.0 + hysteresis_factor)
            } else if lod.level < self.active_level {
                // Switching to higher detail: decrease threshold
                threshold * (1.0 - hysteresis_factor)
            } else {
                threshold
            };

            if effective_distance < threshold {
                new_level = i;
                break;
            }
        }

        let new_level = self.levels[new_level].level;

        if new_level != self.active_level {
            let old_level = self.active_level;
            self.active_level = new_level;

            tracing::debug!(
                "LOD changed: {} -> {} (distance: {:.2}m)",
                old_level,
                new_level,
                distance
            );

            Some(new_level)
        } else {
            None
        }
    }
}

/// LOD configuration
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Hysteresis factor (0.0 - 1.0, prevents flickering)
    pub hysteresis: f32,

    /// Global LOD bias (higher = use lower LODs more aggressively)
    pub global_bias: f32,

    /// Update frequency (update LOD every N frames)
    pub update_frequency: u32,

    /// Distance thresholds for auto-generated LODs
    pub auto_distances: Vec<f32>,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            hysteresis: 0.1, // 10% hysteresis
            global_bias: 1.0,
            update_frequency: 10, // Update every 10 frames
            auto_distances: vec![10.0, 30.0, 60.0, 100.0], // LOD 0, 1, 2, 3
        }
    }
}
```

---

### **2. LOD Component** (Day 1)

**File:** `engine/ecs/src/components/lod.rs`

```rust
use serde::{Deserialize, Serialize};

/// LOD component for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodComponent {
    /// LOD mesh reference
    pub mesh_id: String,

    /// Current active LOD level
    #[serde(skip)]
    pub active_level: u32,

    /// LOD bias for this entity (1.0 = normal)
    pub lod_bias: f32,

    /// Last update frame
    #[serde(skip)]
    pub last_update_frame: u32,

    /// Force LOD level (None = automatic)
    pub force_level: Option<u32>,
}

impl Default for LodComponent {
    fn default() -> Self {
        Self {
            mesh_id: String::new(),
            active_level: 0,
            lod_bias: 1.0,
            last_update_frame: 0,
            force_level: None,
        }
    }
}

impl LodComponent {
    pub fn new(mesh_id: impl Into<String>) -> Self {
        Self {
            mesh_id: mesh_id.into(),
            ..Default::default()
        }
    }

    pub fn with_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }

    pub fn force_lod(mut self, level: u32) -> Self {
        self.force_level = Some(level);
        self
    }
}
```

---

### **3. LOD Selection System** (Day 2-3)

**File:** `engine/rendering/src/lod/system.rs`

```rust
use crate::lod::{LodMesh, LodConfig, MeshLod};
use engine_ecs::prelude::*;
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;

/// LOD selection system
pub struct LodSystem {
    /// Loaded LOD meshes
    lod_meshes: HashMap<String, LodMesh>,

    /// Configuration
    config: LodConfig,

    /// Current frame counter
    frame_counter: u32,

    /// Statistics
    stats: LodStats,
}

#[derive(Debug, Default)]
pub struct LodStats {
    pub total_entities: usize,
    pub lod_transitions: u32,
    pub total_triangles: u64,
    pub lod_distribution: [u32; 8], // Count per LOD level
}

impl LodSystem {
    pub fn new(config: LodConfig) -> Self {
        Self {
            lod_meshes: HashMap::new(),
            config,
            frame_counter: 0,
            stats: LodStats::default(),
        }
    }

    /// Register LOD mesh
    pub fn register_lod_mesh(&mut self, id: String, lod_mesh: LodMesh) {
        self.lod_meshes.insert(id, lod_mesh);
    }

    /// Update LOD selections
    pub fn update(&mut self, world: &mut World, camera_position: Vec3) {
        self.frame_counter += 1;

        // Reset stats
        self.stats = LodStats::default();

        let update_this_frame = self.frame_counter % self.config.update_frequency == 0;

        for (entity, (transform, lod_component)) in world
            .query::<(&Transform, &mut LodComponent)>()
            .iter()
        {
            self.stats.total_entities += 1;

            // Check if forced LOD
            if let Some(forced_level) = lod_component.force_level {
                if lod_component.active_level != forced_level {
                    lod_component.active_level = forced_level;
                    self.stats.lod_transitions += 1;
                }
                continue;
            }

            // Update based on frequency
            if !update_this_frame && lod_component.last_update_frame != 0 {
                continue;
            }

            lod_component.last_update_frame = self.frame_counter;

            // Get LOD mesh
            if let Some(lod_mesh) = self.lod_meshes.get_mut(&lod_component.mesh_id) {
                // Calculate world position
                let world_position = transform.position;

                // Apply entity-specific LOD bias
                let original_bias = lod_mesh.lod_bias;
                lod_mesh.lod_bias = self.config.global_bias * lod_component.lod_bias;

                // Update LOD
                if let Some(new_level) = lod_mesh.update_lod(
                    camera_position,
                    self.config.hysteresis,
                ) {
                    lod_component.active_level = new_level;
                    self.stats.lod_transitions += 1;
                }

                // Restore bias
                lod_mesh.lod_bias = original_bias;

                // Update stats
                if let Some(active_lod) = lod_mesh.active_lod() {
                    self.stats.total_triangles += active_lod.triangle_count as u64;
                    if (active_lod.level as usize) < self.stats.lod_distribution.len() {
                        self.stats.lod_distribution[active_lod.level as usize] += 1;
                    }
                }
            }
        }

        if update_this_frame {
            tracing::trace!(
                "LOD update: {} entities, {} transitions, {} triangles",
                self.stats.total_entities,
                self.stats.lod_transitions,
                self.stats.total_triangles
            );
        }
    }

    /// Get LOD mesh
    pub fn get_lod_mesh(&self, id: &str) -> Option<&LodMesh> {
        self.lod_meshes.get(id)
    }

    /// Get statistics
    pub fn stats(&self) -> &LodStats {
        &self.stats
    }

    /// Set global LOD bias
    pub fn set_global_bias(&mut self, bias: f32) {
        self.config.global_bias = bias;
    }
}
```

---

### **4. LOD Mesh Builder** (Day 3-4)

**File:** `engine/rendering/src/lod/builder.rs`

```rust
use crate::lod::{LodMesh, MeshLod, BoundingSphere};
use std::sync::Arc;

/// Build LOD mesh from multiple mesh files
pub struct LodMeshBuilder {
    levels: Vec<MeshLod>,
    bounds: Option<BoundingSphere>,
}

impl LodMeshBuilder {
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            bounds: None,
        }
    }

    /// Add LOD level
    pub fn add_level(
        mut self,
        level: u32,
        mesh: Arc<MeshData>,
        distance: f32,
    ) -> Self {
        let triangle_count = mesh.indices.len() as u32 / 3;

        self.levels.push(MeshLod {
            level,
            mesh,
            distance,
            screen_coverage: 0.0, // Computed later
            triangle_count,
        });

        self
    }

    /// Set bounding sphere
    pub fn with_bounds(mut self, center: Vec3, radius: f32) -> Self {
        self.bounds = Some(BoundingSphere { center, radius });
        self
    }

    /// Auto-compute bounds from LOD 0 mesh
    pub fn auto_bounds(mut self) -> Self {
        if let Some(lod0) = self.levels.iter().find(|l| l.level == 0) {
            let bounds = compute_bounding_sphere(&lod0.mesh);
            self.bounds = Some(bounds);
        }
        self
    }

    /// Build LOD mesh
    pub fn build(mut self) -> Result<LodMesh, String> {
        if self.levels.is_empty() {
            return Err("No LOD levels provided".to_string());
        }

        let bounds = self.bounds
            .ok_or_else(|| "Bounding sphere not set".to_string())?;

        // Sort levels by level number
        self.levels.sort_by_key(|l| l.level);

        Ok(LodMesh::new(self.levels, bounds))
    }
}

/// Compute bounding sphere from mesh
fn compute_bounding_sphere(mesh: &MeshData) -> BoundingSphere {
    if mesh.positions.is_empty() {
        return BoundingSphere {
            center: Vec3::ZERO,
            radius: 0.0,
        };
    }

    // Compute center (average of all vertices)
    let mut center = Vec3::ZERO;
    for pos in &mesh.positions {
        center += *pos;
    }
    center /= mesh.positions.len() as f32;

    // Compute radius (max distance from center)
    let mut max_dist_sq = 0.0f32;
    for pos in &mesh.positions {
        let dist_sq = (*pos - center).length_squared();
        max_dist_sq = max_dist_sq.max(dist_sq);
    }

    BoundingSphere {
        center,
        radius: max_dist_sq.sqrt(),
    }
}

/// Auto-generate LOD distances based on triangle count reduction
pub fn auto_generate_lod_distances(
    levels: &[MeshLod],
    base_distance: f32,
) -> Vec<f32> {
    let mut distances = Vec::new();

    if let Some(lod0) = levels.first() {
        let base_triangles = lod0.triangle_count as f32;

        for lod in levels {
            let reduction_ratio = lod.triangle_count as f32 / base_triangles;

            // Distance inversely proportional to triangle count
            // LOD with 25% triangles -> 2x distance
            // LOD with 10% triangles -> 3.16x distance
            let distance_multiplier = 1.0 / reduction_ratio.sqrt();
            let distance = base_distance * distance_multiplier;

            distances.push(distance);
        }
    }

    distances
}
```

---

### **5. Example & Performance Test** (Day 4)

**File:** `examples/lod_demo.rs`

```rust
use engine_ecs::prelude::*;
use engine_rendering::lod::*;
use glam::{Vec3, Quat};

fn main() {
    tracing_subscriber::fmt::init();

    // Create world
    let mut world = World::new();

    // Create camera
    let camera_pos = Vec3::new(0.0, 10.0, 50.0);

    // Create LOD system
    let mut lod_system = LodSystem::new(LodConfig::default());

    // Create test LOD mesh (simulated)
    let lod_mesh = create_test_lod_mesh();
    lod_system.register_lod_mesh("test_mesh".to_string(), lod_mesh);

    // Spawn entities at various distances
    for i in 0..1000 {
        let distance = (i as f32 / 10.0).powf(1.5); // Exponential distribution
        let angle = i as f32 * 0.1;

        let entity = world.spawn();
        world.add_component(entity, Transform {
            position: Vec3::new(
                angle.cos() * distance,
                0.0,
                angle.sin() * distance,
            ),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        world.add_component(entity, LodComponent::new("test_mesh"));
    }

    // Benchmark LOD selection
    let iterations = 100;
    let mut total_time = std::time::Duration::ZERO;

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        lod_system.update(&mut world, camera_pos);
        let elapsed = start.elapsed();
        total_time += elapsed;
    }

    let avg_time = total_time / iterations;

    println!("=== LOD Performance Test ===");
    println!("Entities: 1000");
    println!("Average LOD update: {:.3}ms", avg_time.as_secs_f64() * 1000.0);
    println!("Total triangles: {}", lod_system.stats().total_triangles);
    println!("LOD distribution: {:?}", lod_system.stats().lod_distribution);

    assert!(avg_time.as_millis() < 1, "LOD selection too slow!");
}

fn create_test_lod_mesh() -> LodMesh {
    // Create mock mesh data for different LOD levels
    let lod0 = create_mock_mesh(10000); // High detail
    let lod1 = create_mock_mesh(5000);  // Medium detail
    let lod2 = create_mock_mesh(1000);  // Low detail
    let lod3 = create_mock_mesh(100);   // Very low detail

    LodMeshBuilder::new()
        .add_level(0, Arc::new(lod0), 0.0)
        .add_level(1, Arc::new(lod1), 10.0)
        .add_level(2, Arc::new(lod2), 30.0)
        .add_level(3, Arc::new(lod3), 60.0)
        .auto_bounds()
        .build()
        .unwrap()
}

fn create_mock_mesh(triangle_count: usize) -> MeshData {
    MeshData {
        positions: vec![Vec3::ZERO; triangle_count * 3],
        normals: vec![Vec3::Y; triangle_count * 3],
        uvs: vec![Vec2::ZERO; triangle_count * 3],
        indices: (0..triangle_count * 3).map(|i| i as u32).collect(),
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] LodMesh supports multiple LOD levels
- [ ] Distance-based LOD selection works
- [ ] Hysteresis prevents LOD flickering
- [ ] LOD component integrates with ECS
- [ ] LodSystem updates efficiently
- [ ] Bounding sphere calculation correct
- [ ] LOD transitions logged
- [ ] Statistics tracking works
- [ ] LOD selection < 1ms for 1000 entities
- [ ] Example demonstrates LOD system

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| LOD selection (1000 entities) | < 1ms | < 3ms |
| Register LOD mesh | < 1ms | < 5ms |
| Bounding sphere computation | < 0.5ms | < 2ms |
| Per-entity LOD update | < 1μs | < 5μs |
| Triangle reduction | 70%+ | 50%+ |
| Memory overhead | < 20% | < 50% |

---

**Dependencies:** [phase1-ecs-core.md](phase1-ecs-core.md), [phase1-mesh-rendering.md](phase1-mesh-rendering.md)
**Next:** [phase3-lod-networking.md](phase3-lod-networking.md)
