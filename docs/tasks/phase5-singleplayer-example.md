# Phase 5.1: Singleplayer Example Game

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (demonstrates engine capabilities)

---

## 🎯 **Objective**

Create a complete singleplayer game example that showcases the engine's core features including ECS, rendering, input handling, and game logic. This serves as both a demo and a reference implementation for developers learning the engine.

**Game Concept:**
- **Genre:** Top-down action/exploration
- **Player:** Character with movement, shooting, health
- **Enemies:** Simple AI with pathfinding and attack patterns
- **Collectibles:** Health packs, score items, powerups
- **Win Condition:** Defeat all enemies or reach the goal
- **Lose Condition:** Player health reaches zero

---

## 📋 **Detailed Tasks**

### **1. Project Setup** (Day 1 Morning)

**File:** `examples/singleplayer/Cargo.toml`

```toml
[package]
name = "singleplayer-example"
version = "0.1.0"
edition = "2021"

[dependencies]
silmaril-core = { path = "../../engine/core" }
silmaril-macros = { path = "../../engine/macros" }
silmaril-platform = { path = "../../engine/platform" }
silmaril-rendering = { path = "../../engine/rendering" }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
glam = "0.24"
rand = "0.8"

[dev-dependencies]
criterion = "0.5"
```

**Directory Structure:**
```
examples/singleplayer/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── components.rs
│   ├── systems/
│   │   ├── mod.rs
│   │   ├── player.rs
│   │   ├── enemy.rs
│   │   ├── combat.rs
│   │   └── collectibles.rs
│   ├── assets/
│   │   ├── sprites/
│   │   └── sounds/
│   └── ui/
│       ├── mod.rs
│       ├── hud.rs
│       └── menu.rs
└── README.md
```

---

### **2. Game Components** (Day 1 Afternoon)

**File:** `examples/singleplayer/src/components.rs`

```rust
use silmaril_core::prelude::*;
use glam::{Vec2, Vec3};

/// Player-controlled character
#[derive(Component, Debug, Clone)]
pub struct Player {
    pub score: i32,
    pub lives: i32,
}

/// AI-controlled enemy
#[derive(Component, Debug, Clone)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub aggro_range: f32,
    pub attack_cooldown: f32,
    pub current_cooldown: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    Chaser,   // Follows player
    Shooter,  // Ranged attacks
    Turret,   // Stationary
}

/// Movement and physics
#[derive(Component, Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: Vec2::ZERO,
            angular: 0.0,
        }
    }
}

/// Health and damage
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn take_damage(&mut self, damage: f32) {
        self.current = (self.current - damage).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Damage dealer
#[derive(Component, Debug, Clone, Copy)]
pub struct Damage {
    pub amount: f32,
    pub damage_type: DamageType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Melee,
    Ranged,
    Explosion,
}

/// Collision detection
#[derive(Component, Debug, Clone, Copy)]
pub struct Collider {
    pub radius: f32,
    pub layer: CollisionLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Player,
    Enemy,
    Projectile,
    Collectible,
    Wall,
}

/// Collectible item
#[derive(Component, Debug, Clone)]
pub struct Collectible {
    pub item_type: CollectibleType,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectibleType {
    HealthPack,
    ScoreItem,
    Powerup,
}

/// Projectile (bullets, missiles)
#[derive(Component, Debug, Clone, Copy)]
pub struct Projectile {
    pub lifetime: f32,
    pub owner: Entity,
}

/// Visual representation
#[derive(Component, Debug, Clone)]
pub struct Sprite {
    pub texture_path: String,
    pub color: [f32; 4],
    pub layer: i32,
}

/// Lifetime timer (auto-despawn)
#[derive(Component, Debug, Clone, Copy)]
pub struct Lifetime {
    pub remaining: f32,
}
```

---

### **3. Player System** (Day 1-2)

**File:** `examples/singleplayer/src/systems/player.rs`

```rust
use crate::components::*;
use silmaril_core::prelude::*;
use silmaril_platform::input::Input;
use glam::Vec2;

/// Player input and movement
pub fn player_movement_system(
    world: &mut World,
    input: &Input,
    dt: f32,
) {
    const MOVE_SPEED: f32 = 5.0;
    const ROTATION_SPEED: f32 = 3.0;

    for (entity, (player, transform, velocity)) in
        world.query::<(&Player, &mut Transform, &mut Velocity)>()
    {
        // Get input direction
        let mut movement = Vec2::ZERO;

        if input.is_key_pressed("W") || input.is_key_pressed("Up") {
            movement.y += 1.0;
        }
        if input.is_key_pressed("S") || input.is_key_pressed("Down") {
            movement.y -= 1.0;
        }
        if input.is_key_pressed("A") || input.is_key_pressed("Left") {
            movement.x -= 1.0;
        }
        if input.is_key_pressed("D") || input.is_key_pressed("Right") {
            movement.x += 1.0;
        }

        // Normalize and apply speed
        if movement.length_squared() > 0.0 {
            movement = movement.normalize() * MOVE_SPEED;
        }

        velocity.linear = movement;

        // Rotate to face movement direction
        if movement.length_squared() > 0.0 {
            let target_rotation = movement.y.atan2(movement.x);
            let rotation_diff = target_rotation - transform.rotation;

            // Smooth rotation
            transform.rotation += rotation_diff * ROTATION_SPEED * dt;
        }
    }
}

/// Player shooting
pub fn player_shooting_system(
    world: &mut World,
    input: &Input,
    dt: f32,
) {
    const FIRE_RATE: f32 = 0.2; // Seconds between shots
    static mut FIRE_COOLDOWN: f32 = 0.0;

    unsafe {
        FIRE_COOLDOWN -= dt;

        if input.is_key_pressed("Space") && FIRE_COOLDOWN <= 0.0 {
            for (entity, (player, transform)) in world.query::<(&Player, &Transform)>() {
                // Spawn projectile
                let projectile = world.spawn();

                let direction = Vec2::new(
                    transform.rotation.cos(),
                    transform.rotation.sin(),
                );

                world.add(projectile, Transform {
                    position: transform.position,
                    rotation: transform.rotation,
                    scale: Vec2::splat(0.3),
                });

                world.add(projectile, Velocity {
                    linear: direction * 10.0,
                    angular: 0.0,
                });

                world.add(projectile, Projectile {
                    lifetime: 2.0,
                    owner: entity,
                });

                world.add(projectile, Damage {
                    amount: 25.0,
                    damage_type: DamageType::Ranged,
                });

                world.add(projectile, Collider {
                    radius: 0.2,
                    layer: CollisionLayer::Projectile,
                });

                world.add(projectile, Sprite {
                    texture_path: "bullet.png".to_string(),
                    color: [1.0, 1.0, 0.0, 1.0],
                    layer: 1,
                });

                world.add(projectile, Lifetime { remaining: 2.0 });

                FIRE_COOLDOWN = FIRE_RATE;

                tracing::debug!("Player fired projectile");
                break; // Only one player
            }
        }
    }
}
```

---

### **4. Enemy AI System** (Day 2)

**File:** `examples/singleplayer/src/systems/enemy.rs`

```rust
use crate::components::*;
use silmaril_core::prelude::*;
use glam::Vec2;

/// Simple AI for enemies
pub fn enemy_ai_system(world: &mut World, dt: f32) {
    // Find player position
    let player_pos = world
        .query::<(&Player, &Transform)>()
        .next()
        .map(|(_, (_, transform))| transform.position);

    let Some(player_pos) = player_pos else {
        return; // No player
    };

    // Update each enemy
    for (entity, (enemy, transform, velocity)) in
        world.query::<(&mut Enemy, &Transform, &mut Velocity)>()
    {
        let to_player = Vec2::new(
            player_pos.x - transform.position.x,
            player_pos.y - transform.position.y,
        );
        let distance = to_player.length();

        match enemy.enemy_type {
            EnemyType::Chaser => {
                // Chase player if in range
                if distance < enemy.aggro_range {
                    let direction = to_player.normalize();
                    velocity.linear = direction * 3.0;
                } else {
                    velocity.linear = Vec2::ZERO;
                }
            }

            EnemyType::Shooter => {
                // Keep distance and shoot
                if distance < enemy.aggro_range {
                    if distance < 3.0 {
                        // Back away
                        let direction = -to_player.normalize();
                        velocity.linear = direction * 2.0;
                    } else if distance > 5.0 {
                        // Move closer
                        let direction = to_player.normalize();
                        velocity.linear = direction * 1.5;
                    } else {
                        // Stay in range
                        velocity.linear = Vec2::ZERO;
                    }

                    // Shoot cooldown
                    enemy.current_cooldown -= dt;
                    if enemy.current_cooldown <= 0.0 {
                        // Shoot (handled in combat system)
                        enemy.current_cooldown = enemy.attack_cooldown;
                    }
                }
            }

            EnemyType::Turret => {
                // Stationary, just rotate to face player
                velocity.linear = Vec2::ZERO;

                if distance < enemy.aggro_range {
                    enemy.current_cooldown -= dt;
                }
            }
        }

        // Face movement direction or player
        if velocity.linear.length_squared() > 0.0 {
            transform.rotation = velocity.linear.y.atan2(velocity.linear.x);
        } else if distance < enemy.aggro_range {
            transform.rotation = to_player.y.atan2(to_player.x);
        }
    }
}

/// Enemy shooting
pub fn enemy_shooting_system(world: &mut World) {
    let mut projectiles_to_spawn = Vec::new();

    for (entity, (enemy, transform)) in world.query::<(&Enemy, &Transform)>() {
        if enemy.current_cooldown <= 0.0 {
            if matches!(enemy.enemy_type, EnemyType::Shooter | EnemyType::Turret) {
                let direction = Vec2::new(
                    transform.rotation.cos(),
                    transform.rotation.sin(),
                );

                projectiles_to_spawn.push((entity, transform.position, direction));
            }
        }
    }

    // Spawn projectiles (avoid borrow issues)
    for (owner, position, direction) in projectiles_to_spawn {
        let projectile = world.spawn();

        world.add(projectile, Transform {
            position,
            rotation: direction.y.atan2(direction.x),
            scale: Vec2::splat(0.25),
        });

        world.add(projectile, Velocity {
            linear: direction * 8.0,
            angular: 0.0,
        });

        world.add(projectile, Projectile {
            lifetime: 3.0,
            owner,
        });

        world.add(projectile, Damage {
            amount: 10.0,
            damage_type: DamageType::Ranged,
        });

        world.add(projectile, Collider {
            radius: 0.15,
            layer: CollisionLayer::Projectile,
        });

        world.add(projectile, Sprite {
            texture_path: "enemy_bullet.png".to_string(),
            color: [1.0, 0.2, 0.2, 1.0],
            layer: 1,
        });

        world.add(projectile, Lifetime { remaining: 3.0 });
    }
}
```

---

### **5. Combat & Collision System** (Day 2-3)

**File:** `examples/singleplayer/src/systems/combat.rs`

```rust
use crate::components::*;
use silmaril_core::prelude::*;
use glam::Vec2;

/// Simple circle-circle collision detection
pub fn collision_system(world: &mut World) {
    let mut collisions = Vec::new();

    // Collect all entities with colliders
    let entities: Vec<_> = world
        .query::<(Entity, &Transform, &Collider)>()
        .collect();

    // Check all pairs
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (e1, t1, c1) = entities[i];
            let (e2, t2, c2) = entities[j];

            let distance = Vec2::new(
                t2.position.x - t1.position.x,
                t2.position.y - t1.position.y,
            ).length();

            let combined_radius = c1.radius + c2.radius;

            if distance < combined_radius {
                collisions.push((e1, e2, c1.layer, c2.layer));
            }
        }
    }

    // Handle collisions
    for (e1, e2, layer1, layer2) in collisions {
        handle_collision(world, e1, e2, layer1, layer2);
    }
}

fn handle_collision(
    world: &mut World,
    e1: Entity,
    e2: Entity,
    layer1: CollisionLayer,
    layer2: CollisionLayer,
) {
    use CollisionLayer::*;

    match (layer1, layer2) {
        // Projectile hits player
        (Projectile, Player) | (Player, Projectile) => {
            let (projectile, player) = if layer1 == Projectile {
                (e1, e2)
            } else {
                (e2, e1)
            };

            // Apply damage
            if let Some(damage) = world.get::<Damage>(projectile) {
                if let Some(health) = world.get_mut::<Health>(player) {
                    health.take_damage(damage.amount);
                    tracing::info!("Player took {} damage, health: {}",
                        damage.amount, health.current);
                }
            }

            // Destroy projectile
            world.despawn(projectile);
        }

        // Projectile hits enemy
        (Projectile, Enemy) | (Enemy, Projectile) => {
            let (projectile, enemy) = if layer1 == Projectile {
                (e1, e2)
            } else {
                (e2, e1)
            };

            // Apply damage
            if let Some(damage) = world.get::<Damage>(projectile) {
                if let Some(health) = world.get_mut::<Health>(enemy) {
                    health.take_damage(damage.amount);
                    tracing::info!("Enemy took {} damage, health: {}",
                        damage.amount, health.current);
                }
            }

            // Destroy projectile
            world.despawn(projectile);
        }

        // Player picks up collectible
        (Player, Collectible) | (Collectible, Player) => {
            let (player, collectible) = if layer1 == Player {
                (e1, e2)
            } else {
                (e2, e1)
            };

            if let Some(item) = world.get::<Collectible>(collectible) {
                match item.item_type {
                    CollectibleType::HealthPack => {
                        if let Some(health) = world.get_mut::<Health>(player) {
                            health.heal(item.value);
                            tracing::info!("Player collected health pack, health: {}",
                                health.current);
                        }
                    }
                    CollectibleType::ScoreItem => {
                        if let Some(player_comp) = world.get_mut::<Player>(player) {
                            player_comp.score += item.value as i32;
                            tracing::info!("Player collected score item, score: {}",
                                player_comp.score);
                        }
                    }
                    CollectibleType::Powerup => {
                        tracing::info!("Player collected powerup!");
                        // Apply powerup effect
                    }
                }
            }

            // Remove collectible
            world.despawn(collectible);
        }

        _ => {}
    }
}

/// Remove dead entities
pub fn death_system(world: &mut World) {
    let dead_entities: Vec<_> = world
        .query::<(Entity, &Health)>()
        .filter(|(_, health)| health.is_dead())
        .map(|(entity, _)| entity)
        .collect();

    for entity in dead_entities {
        tracing::info!("Entity {:?} died", entity);
        world.despawn(entity);
    }
}
```

---

### **6. Main Game Loop** (Day 3)

**File:** `examples/singleplayer/src/main.rs`

```rust
mod components;
mod systems;

use silmaril_core::prelude::*;
use silmaril_platform::{Platform, WindowConfig, Input};
use silmaril_rendering::Renderer;
use components::*;
use systems::*;
use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Singleplayer Example");

    // Create platform and window
    let mut platform = Platform::new()?;
    let window = platform.create_window(WindowConfig {
        title: "Singleplayer Example".to_string(),
        width: 1280,
        height: 720,
        resizable: true,
        ..Default::default()
    })?;

    // Create renderer
    let mut renderer = Renderer::new(&window)?;

    // Create world and register components
    let mut world = World::new();
    register_components(&mut world);

    // Spawn game entities
    spawn_game(&mut world);

    // Game loop
    let mut last_frame = Instant::now();
    let mut running = true;

    while running {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;

        // Process input
        let input = platform.poll_events();
        if input.should_quit() {
            running = false;
        }

        // Update systems
        player::player_movement_system(&mut world, &input, dt);
        player::player_shooting_system(&mut world, &input, dt);
        enemy::enemy_ai_system(&mut world, dt);
        enemy::enemy_shooting_system(&mut world);
        combat::collision_system(&mut world);
        combat::death_system(&mut world);
        physics_system(&mut world, dt);
        lifetime_system(&mut world, dt);

        // Render
        renderer.begin_frame()?;
        render_world(&mut world, &mut renderer)?;
        renderer.end_frame()?;

        // Check win/lose conditions
        if check_game_over(&world) {
            tracing::info!("Game Over!");
            running = false;
        }
    }

    tracing::info!("Shutting down");
    Ok(())
}

fn register_components(world: &mut World) {
    world.register::<Player>();
    world.register::<Enemy>();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Damage>();
    world.register::<Collider>();
    world.register::<Collectible>();
    world.register::<Projectile>();
    world.register::<Sprite>();
    world.register::<Lifetime>();
}

fn spawn_game(world: &mut World) {
    // Spawn player
    let player = world.spawn();
    world.add(player, Player { score: 0, lives: 3 });
    world.add(player, Transform::default());
    world.add(player, Velocity::default());
    world.add(player, Health::new(100.0));
    world.add(player, Collider {
        radius: 0.5,
        layer: CollisionLayer::Player,
    });
    world.add(player, Sprite {
        texture_path: "player.png".to_string(),
        color: [0.2, 0.8, 0.2, 1.0],
        layer: 2,
    });

    // Spawn enemies
    for i in 0..5 {
        spawn_enemy(world, EnemyType::Chaser, i as f32 * 3.0, 5.0);
    }

    for i in 0..3 {
        spawn_enemy(world, EnemyType::Shooter, -5.0, i as f32 * 3.0);
    }

    // Spawn collectibles
    for i in 0..10 {
        spawn_collectible(world, CollectibleType::ScoreItem,
            (i as f32 - 5.0) * 2.0, (i % 3) as f32 * 2.0);
    }
}

fn spawn_enemy(world: &mut World, enemy_type: EnemyType, x: f32, y: f32) {
    let enemy = world.spawn();

    world.add(enemy, Enemy {
        enemy_type,
        aggro_range: 8.0,
        attack_cooldown: 1.5,
        current_cooldown: 0.0,
    });

    world.add(enemy, Transform {
        position: glam::Vec3::new(x, y, 0.0),
        ..Default::default()
    });

    world.add(enemy, Velocity::default());
    world.add(enemy, Health::new(50.0));
    world.add(enemy, Collider {
        radius: 0.4,
        layer: CollisionLayer::Enemy,
    });

    let color = match enemy_type {
        EnemyType::Chaser => [1.0, 0.2, 0.2, 1.0],
        EnemyType::Shooter => [1.0, 0.5, 0.2, 1.0],
        EnemyType::Turret => [0.8, 0.2, 0.8, 1.0],
    };

    world.add(enemy, Sprite {
        texture_path: "enemy.png".to_string(),
        color,
        layer: 2,
    });
}

fn spawn_collectible(
    world: &mut World,
    item_type: CollectibleType,
    x: f32,
    y: f32,
) {
    let collectible = world.spawn();

    let value = match item_type {
        CollectibleType::HealthPack => 25.0,
        CollectibleType::ScoreItem => 10.0,
        CollectibleType::Powerup => 1.0,
    };

    world.add(collectible, Collectible { item_type, value });
    world.add(collectible, Transform {
        position: glam::Vec3::new(x, y, 0.0),
        scale: glam::Vec2::splat(0.3),
        ..Default::default()
    });
    world.add(collectible, Collider {
        radius: 0.3,
        layer: CollisionLayer::Collectible,
    });

    let color = match item_type {
        CollectibleType::HealthPack => [0.2, 1.0, 0.2, 1.0],
        CollectibleType::ScoreItem => [1.0, 1.0, 0.2, 1.0],
        CollectibleType::Powerup => [0.2, 0.5, 1.0, 1.0],
    };

    world.add(collectible, Sprite {
        texture_path: "collectible.png".to_string(),
        color,
        layer: 1,
    });
}

fn physics_system(world: &mut World, dt: f32) {
    for (_, (transform, velocity)) in world.query::<(&mut Transform, &Velocity)>() {
        transform.position.x += velocity.linear.x * dt;
        transform.position.y += velocity.linear.y * dt;
        transform.rotation += velocity.angular * dt;
    }
}

fn lifetime_system(world: &mut World, dt: f32) {
    let expired: Vec<_> = world
        .query::<(Entity, &mut Lifetime)>()
        .filter_map(|(entity, lifetime)| {
            lifetime.remaining -= dt;
            if lifetime.remaining <= 0.0 {
                Some(entity)
            } else {
                None
            }
        })
        .collect();

    for entity in expired {
        world.despawn(entity);
    }
}

fn render_world(world: &mut World, renderer: &mut Renderer) -> Result<()> {
    for (_, (transform, sprite)) in world.query::<(&Transform, &Sprite)>() {
        renderer.draw_sprite(
            &sprite.texture_path,
            transform.position,
            transform.scale,
            transform.rotation,
            sprite.color,
        )?;
    }
    Ok(())
}

fn check_game_over(world: &World) -> bool {
    // Check if player is dead
    for (_, (player, health)) in world.query::<(&Player, &Health)>() {
        if health.is_dead() {
            return true;
        }
    }

    // Check if all enemies are dead
    let enemy_count = world.query::<&Enemy>().count();
    if enemy_count == 0 {
        tracing::info!("Victory! All enemies defeated!");
        return true;
    }

    false
}
```

---

### **7. README & Documentation** (Day 4)

**File:** `examples/singleplayer/README.md`

```markdown
# Singleplayer Example Game

A complete singleplayer game demonstrating the Silmaril's core features.

## Features

- **Player Control**: WASD movement, Space to shoot
- **Enemy AI**: Three enemy types with different behaviors
  - Chasers: Pursue the player
  - Shooters: Keep distance and fire projectiles
  - Turrets: Stationary defense
- **Combat System**: Health, damage, projectiles
- **Collectibles**: Health packs, score items, powerups
- **Win/Lose Conditions**: Defeat all enemies or die trying

## Running

```bash
cargo run --example singleplayer --release
```

## Controls

- **WASD / Arrow Keys**: Move
- **Space**: Shoot
- **ESC**: Quit

## Architecture

This example demonstrates:

- ECS component design
- System-based game logic
- Input handling
- Collision detection
- AI behaviors
- Game state management

## Code Overview

- `components.rs`: All game components
- `systems/player.rs`: Player movement and shooting
- `systems/enemy.rs`: Enemy AI and behavior
- `systems/combat.rs`: Collision and damage handling
- `main.rs`: Game loop and initialization

## Extending

Add new features by:

1. Creating new components in `components.rs`
2. Implementing systems in `systems/`
3. Registering components in `main.rs`
4. Adding systems to the game loop
```

---

## ✅ **Acceptance Criteria**

- [ ] Project builds and runs without errors
- [ ] Player can move and shoot smoothly
- [ ] Enemies exhibit correct AI behaviors
- [ ] Collision detection works accurately
- [ ] Health and damage system functions
- [ ] Collectibles can be picked up
- [ ] Game over conditions trigger correctly
- [ ] Code is well-documented with comments
- [ ] README provides clear instructions
- [ ] Performance: 60 FPS with 50+ entities
- [ ] No memory leaks during gameplay
- [ ] Clean shutdown on exit

---

## 🎯 **Performance Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Frame rate | 60 FPS | 30 FPS |
| Entity count | 100+ | 50+ |
| Input latency | < 16ms | < 33ms |
| Memory usage | < 100 MB | < 200 MB |
| Load time | < 2s | < 5s |

---

## 🧪 **Tests**

```rust
#[test]
fn test_player_spawns() {
    let mut world = World::new();
    register_components(&mut world);
    spawn_game(&mut world);

    let player_count = world.query::<&Player>().count();
    assert_eq!(player_count, 1);
}

#[test]
fn test_collision_detection() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Collider>();

    let e1 = world.spawn();
    world.add(e1, Transform::default());
    world.add(e1, Collider {
        radius: 1.0,
        layer: CollisionLayer::Player,
    });

    let e2 = world.spawn();
    world.add(e2, Transform {
        position: glam::Vec3::new(1.5, 0.0, 0.0),
        ..Default::default()
    });
    world.add(e2, Collider {
        radius: 1.0,
        layer: CollisionLayer::Enemy,
    });

    // Should collide (distance 1.5 < combined radius 2.0)
    // Test collision system
}

#[test]
fn test_enemy_ai() {
    // Test that chasers move toward player
    // Test that shooters maintain distance
    // Test that turrets stay stationary
}
```

---

## 💡 **Future Enhancements**

- Save/load game state
- Level progression
- More enemy types
- Boss fights
- Particle effects
- Sound effects and music
- Gamepad support
- Difficulty settings
- Achievements

---

**Dependencies:** Phase 1-4 (ECS, Rendering, Platform, Input)
**Next:** [phase5-mmorpg-example.md](phase5-mmorpg-example.md)
