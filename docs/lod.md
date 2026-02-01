# Level of Detail (LOD) System

> **LOD system for agent-game-engine**
>
> Adaptive quality management for rendering and networking optimization

---

## Overview

The LOD system dynamically adjusts entity detail based on:
- **Distance from viewer** - Closer objects get more detail
- **Screen space size** - Smaller objects use simpler representations
- **Performance budget** - Degrade gracefully under load
- **Network bandwidth** - Reduce update frequency for distant entities

## Architecture

### LOD Levels

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LODLevel {
    High,      // Full detail (< 20m)
    Medium,    // Reduced detail (20-50m)
    Low,       // Minimal detail (50-100m)
    VeryLow,   // Impostor/billboard (100-200m)
    Culled,    // Not rendered (> 200m)
}

impl LODLevel {
    pub fn from_distance(distance: f32) -> Self {
        match distance {
            d if d < 20.0 => Self::High,
            d if d < 50.0 => Self::Medium,
            d if d < 100.0 => Self::Low,
            d if d < 200.0 => Self::VeryLow,
            _ => Self::Culled,
        }
    }

    pub fn mesh_quality(&self) -> f32 {
        match self {
            Self::High => 1.0,
            Self::Medium => 0.5,
            Self::Low => 0.25,
            Self::VeryLow => 0.1,
            Self::Culled => 0.0,
        }
    }
}
```

---

## Rendering LOD

### Mesh LOD Component

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MeshLOD {
    pub meshes: [Option<AssetHandle<Mesh>>; 4], // High, Medium, Low, VeryLow
    pub current_level: LODLevel,
    pub distances: [f32; 4], // Distance thresholds
}

impl MeshLOD {
    pub fn new(high_mesh: AssetHandle<Mesh>) -> Self {
        Self {
            meshes: [Some(high_mesh), None, None, None],
            current_level: LODLevel::High,
            distances: [20.0, 50.0, 100.0, 200.0],
        }
    }

    pub fn with_levels(
        high: AssetHandle<Mesh>,
        medium: AssetHandle<Mesh>,
        low: AssetHandle<Mesh>,
        impostor: AssetHandle<Mesh>,
    ) -> Self {
        Self {
            meshes: [Some(high), Some(medium), Some(low), Some(impostor)],
            current_level: LODLevel::High,
            distances: [20.0, 50.0, 100.0, 200.0],
        }
    }

    pub fn get_mesh(&self) -> Option<&AssetHandle<Mesh>> {
        let index = match self.current_level {
            LODLevel::High => 0,
            LODLevel::Medium => 1,
            LODLevel::Low => 2,
            LODLevel::VeryLow => 3,
            LODLevel::Culled => return None,
        };

        self.meshes[index].as_ref()
    }
}
```

### LOD Update System

```rust
use engine_profiling::profile_scope;

#[profile(category = "Rendering")]
pub fn mesh_lod_system(world: &mut World) {
    profile_scope!("mesh_lod");

    // Find camera position
    let camera_pos = world
        .query::<(&Transform, &Camera)>()
        .next()
        .map(|(transform, _)| transform.position)
        .unwrap_or(Vec3::ZERO);

    // Update LOD levels based on distance
    for (transform, mesh_lod) in world.query::<(&Transform, &mut MeshLOD)>() {
        let distance = (transform.position - camera_pos).length();

        let new_level = if distance < mesh_lod.distances[0] {
            LODLevel::High
        } else if distance < mesh_lod.distances[1] {
            LODLevel::Medium
        } else if distance < mesh_lod.distances[2] {
            LODLevel::Low
        } else if distance < mesh_lod.distances[3] {
            LODLevel::VeryLow
        } else {
            LODLevel::Culled
        };

        if mesh_lod.current_level != new_level {
            mesh_lod.current_level = new_level;
        }
    }
}
```

### Hysteresis

Prevent LOD thrashing with hysteresis:

```rust
pub struct LODHysteresis {
    pub switch_distance: f32,
    pub hysteresis_margin: f32,
}

impl LODHysteresis {
    pub fn should_upgrade(&self, current_distance: f32) -> bool {
        current_distance < self.switch_distance - self.hysteresis_margin
    }

    pub fn should_downgrade(&self, current_distance: f32) -> bool {
        current_distance > self.switch_distance + self.hysteresis_margin
    }
}

// Usage
const MEDIUM_TO_HIGH: LODHysteresis = LODHysteresis {
    switch_distance: 20.0,
    hysteresis_margin: 2.0, // 18m upgrade, 22m downgrade
};
```

---

## Network LOD

### Update Frequency

Reduce network updates for distant entities:

```rust
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkLOD {
    pub current_level: LODLevel,
    pub update_rate: UpdateRate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateRate {
    EveryFrame,    // 60 Hz
    Every2Frames,  // 30 Hz
    Every5Frames,  // 12 Hz
    Every10Frames, // 6 Hz
    Every30Frames, // 2 Hz
}

impl UpdateRate {
    pub fn from_lod_level(level: LODLevel) -> Self {
        match level {
            LODLevel::High => Self::EveryFrame,
            LODLevel::Medium => Self::Every2Frames,
            LODLevel::Low => Self::Every5Frames,
            LODLevel::VeryLow => Self::Every10Frames,
            LODLevel::Culled => Self::Every30Frames,
        }
    }

    pub fn should_update(&self, frame: u64) -> bool {
        match self {
            Self::EveryFrame => true,
            Self::Every2Frames => frame % 2 == 0,
            Self::Every5Frames => frame % 5 == 0,
            Self::Every10Frames => frame % 10 == 0,
            Self::Every30Frames => frame % 30 == 0,
        }
    }
}
```

### Network LOD System

```rust
#[server_only]
#[profile(category = "Networking")]
pub fn network_lod_system(
    world: &mut World,
    client_positions: &HashMap<ClientId, Vec3>,
    frame: u64,
) {
    profile_scope!("network_lod");

    for (entity, transform, network_lod) in world.query::<(
        &Entity,
        &Transform,
        &mut NetworkLOD,
    )>() {
        // Find closest client
        let min_distance = client_positions
            .values()
            .map(|pos| (transform.position - *pos).length())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f32::MAX);

        // Update LOD level
        let new_level = LODLevel::from_distance(min_distance);
        if network_lod.current_level != new_level {
            network_lod.current_level = new_level;
            network_lod.update_rate = UpdateRate::from_lod_level(new_level);
        }
    }
}
```

### Selective Replication

Only send entity state when needed:

```rust
#[server_only]
pub fn should_replicate_entity(
    entity: Entity,
    world: &World,
    frame: u64,
) -> bool {
    if let Some(network_lod) = world.get::<NetworkLOD>(entity) {
        if network_lod.current_level == LODLevel::Culled {
            return false; // Don't replicate culled entities
        }

        return network_lod.update_rate.should_update(frame);
    }

    true // Replicate by default
}
```

---

## Animation LOD

### Animation Quality

Reduce animation quality for distant characters:

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AnimationLOD {
    pub current_level: LODLevel,
    pub skeleton_quality: SkeletonQuality,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SkeletonQuality {
    Full,      // All bones (200+)
    Reduced,   // Important bones only (50)
    Minimal,   // Root + limbs (10)
    RootOnly,  // Root bone only (1)
}

impl SkeletonQuality {
    pub fn from_lod_level(level: LODLevel) -> Self {
        match level {
            LODLevel::High => Self::Full,
            LODLevel::Medium => Self::Reduced,
            LODLevel::Low => Self::Minimal,
            LODLevel::VeryLow | LODLevel::Culled => Self::RootOnly,
        }
    }

    pub fn bone_count(&self) -> usize {
        match self {
            Self::Full => 200,
            Self::Reduced => 50,
            Self::Minimal => 10,
            Self::RootOnly => 1,
        }
    }
}
```

### Animation Update Rate

Skip animation frames for distant characters:

```rust
#[profile(category = "Animation")]
pub fn animation_lod_system(world: &mut World, frame: u64) {
    profile_scope!("animation_lod");

    for (animation, animation_lod) in world.query::<(&mut Animation, &AnimationLOD)>() {
        let should_update = match animation_lod.current_level {
            LODLevel::High => true,
            LODLevel::Medium => frame % 2 == 0,
            LODLevel::Low => frame % 4 == 0,
            LODLevel::VeryLow => frame % 8 == 0,
            LODLevel::Culled => false,
        };

        if should_update {
            animation.update(0.016); // 60 FPS delta
        }
    }
}
```

---

## Physics LOD

### Collision Complexity

Simplify collision shapes for distant objects:

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsLOD {
    pub current_level: LODLevel,
    pub colliders: [Option<ColliderShape>; 4],
}

impl PhysicsLOD {
    pub fn get_collider(&self) -> Option<&ColliderShape> {
        let index = match self.current_level {
            LODLevel::High => 0,
            LODLevel::Medium => 1,
            LODLevel::Low => 2,
            LODLevel::VeryLow | LODLevel::Culled => 3,
        };

        self.colliders[index].as_ref()
    }
}

// Example collider progression:
// High: Convex mesh (1000 vertices)
// Medium: Simplified convex mesh (100 vertices)
// Low: Box collider
// VeryLow: No collider (non-physical)
```

### Physics Update Rate

Reduce physics update frequency:

```rust
#[server_only]
#[profile(category = "Physics")]
pub fn physics_lod_system(world: &mut World, frame: u64) {
    profile_scope!("physics_lod");

    for (rigid_body, physics_lod) in world.query::<(&mut RigidBody, &PhysicsLOD)>() {
        let should_simulate = match physics_lod.current_level {
            LODLevel::High => true,
            LODLevel::Medium => frame % 2 == 0,
            LODLevel::Low => frame % 5 == 0,
            LODLevel::VeryLow => frame % 10 == 0,
            LODLevel::Culled => false,
        };

        if !should_simulate {
            // Put body to sleep to skip physics
            rigid_body.sleeping = true;
        }
    }
}
```

---

## Performance Budget

### Frame Time Budget

Dynamically adjust LOD to maintain target frame rate:

```rust
pub struct LODBudget {
    pub target_frame_time: Duration,
    pub current_frame_time: Duration,
    pub budget_remaining: Duration,
}

impl LODBudget {
    pub fn new(target_fps: u32) -> Self {
        let target_frame_time = Duration::from_secs_f32(1.0 / target_fps as f32);
        Self {
            target_frame_time,
            current_frame_time: Duration::ZERO,
            budget_remaining: target_frame_time,
        }
    }

    pub fn update(&mut self, frame_time: Duration) {
        self.current_frame_time = frame_time;
        self.budget_remaining = self.target_frame_time.saturating_sub(frame_time);
    }

    pub fn is_over_budget(&self) -> bool {
        self.current_frame_time > self.target_frame_time
    }

    pub fn stress_factor(&self) -> f32 {
        self.current_frame_time.as_secs_f32() / self.target_frame_time.as_secs_f32()
    }
}
```

### Adaptive LOD

Adjust LOD distances based on performance:

```rust
pub struct AdaptiveLODSettings {
    pub base_distances: [f32; 4],
    pub current_distances: [f32; 4],
    pub budget: LODBudget,
}

impl AdaptiveLODSettings {
    pub fn adjust_for_performance(&mut self) {
        let stress = self.budget.stress_factor();

        if stress > 1.2 {
            // Over budget - reduce LOD distances
            for i in 0..4 {
                self.current_distances[i] = self.base_distances[i] * 0.8;
            }
        } else if stress < 0.8 {
            // Under budget - increase LOD distances
            for i in 0..4 {
                self.current_distances[i] = (self.base_distances[i] * 1.2)
                    .min(self.base_distances[i] * 2.0); // Cap at 2x
            }
        }
    }
}
```

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| LOD system overhead | < 0.5ms per frame | < 2ms |
| LOD transitions | Imperceptible | < 100ms |
| Network bandwidth savings | > 50% (distant) | > 30% |
| Rendering performance gain | > 2x (crowded) | > 1.5x |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_level_from_distance() {
        assert_eq!(LODLevel::from_distance(10.0), LODLevel::High);
        assert_eq!(LODLevel::from_distance(30.0), LODLevel::Medium);
        assert_eq!(LODLevel::from_distance(75.0), LODLevel::Low);
        assert_eq!(LODLevel::from_distance(150.0), LODLevel::VeryLow);
        assert_eq!(LODLevel::from_distance(250.0), LODLevel::Culled);
    }

    #[test]
    fn test_update_rate() {
        let rate = UpdateRate::Every2Frames;
        assert!(rate.should_update(0));
        assert!(!rate.should_update(1));
        assert!(rate.should_update(2));
    }

    #[test]
    fn test_lod_budget() {
        let mut budget = LODBudget::new(60);
        budget.update(Duration::from_millis(20));
        assert!(budget.is_over_budget());
        assert!(budget.stress_factor() > 1.0);
    }
}
```

---

## Best Practices

### DO

- ✅ Use hysteresis to prevent LOD thrashing
- ✅ Profile LOD system overhead
- ✅ Author multiple mesh LODs in content pipeline
- ✅ Test with worst-case entity counts
- ✅ Adjust LOD distances per entity type

### DON'T

- ❌ Switch LOD every frame (use hysteresis)
- ❌ Use too many LOD levels (3-4 is enough)
- ❌ Forget to LOD animations and physics
- ❌ Make LOD transitions too abrupt (pop-in)
- ❌ Use same LOD distances for all entities

---

## Advanced Topics

### Screen Space LOD

Calculate LOD based on screen coverage:

```rust
pub fn calculate_screen_space_lod(
    world_bounds: f32,      // Object size in meters
    distance: f32,          // Distance from camera
    fov: f32,               // Camera field of view (radians)
    screen_height: f32,     // Screen height in pixels
) -> LODLevel {
    // Project world size to screen space
    let screen_size = (world_bounds * screen_height) / (2.0 * distance * (fov / 2.0).tan());

    match screen_size {
        s if s > 100.0 => LODLevel::High,
        s if s > 50.0 => LODLevel::Medium,
        s if s > 20.0 => LODLevel::Low,
        s if s > 5.0 => LODLevel::VeryLow,
        _ => LODLevel::Culled,
    }
}
```

### Temporal LOD

Smooth LOD transitions over multiple frames:

```rust
pub struct TemporalLOD {
    pub current: LODLevel,
    pub target: LODLevel,
    pub transition_progress: f32,
    pub transition_duration: f32,
}

impl TemporalLOD {
    pub fn update(&mut self, dt: f32) {
        if self.current != self.target {
            self.transition_progress += dt / self.transition_duration;

            if self.transition_progress >= 1.0 {
                self.current = self.target;
                self.transition_progress = 0.0;
            }
        }
    }
}
```

---

## References

- **Implementation:** TBD `engine/lod/src/`
- **Rendering Integration:** `engine/renderer/src/`
- **Network Integration:** `engine/networking/src/`

**Related Documentation:**
- [Rendering](rendering.md)
- [Networking](networking.md)
- [Interest Management](interest-management.md)
- [Performance Targets](performance-targets.md)
