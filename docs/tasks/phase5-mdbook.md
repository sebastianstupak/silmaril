# Phase 5.5: mdBook Documentation

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 days
**Priority:** High (critical for adoption)

---

## 🎯 **Objective**

Create comprehensive documentation using mdBook that covers user guides, API references, tutorials, and best practices. This makes the engine accessible to developers and showcases its capabilities.

**Documentation Goals:**
- **User-Friendly:** Clear, concise, example-driven
- **Comprehensive:** Covers all major features
- **Searchable:** Easy to navigate and find information
- **Living:** Updated alongside code changes
- **Beautiful:** Professional appearance with code highlighting

---

## 📋 **Detailed Tasks**

### **1. mdBook Setup** (Day 1 Morning)

**File:** `docs/book/book.toml`

```toml
[book]
title = "Silmaril Documentation"
authors = ["Silmaril Contributors"]
language = "en"
multilingual = false
src = "src"
description = "Comprehensive documentation for the Silmaril"

[build]
build-dir = "book"
create-missing = true

[output.html]
default-theme = "navy"
preferred-dark-theme = "navy"
git-repository-url = "https://github.com/yourusername/silmaril"
edit-url-template = "https://github.com/yourusername/silmaril/edit/main/docs/book/{path}"

[output.html.search]
enable = true
limit-results = 30
teaser-word-count = 30
use-boolean-and = true
boost-title = 2
boost-hierarchy = 1
boost-paragraph = 1
expand = true
heading-split-level = 3

[output.html.playground]
editable = true
copyable = true
copy-js = true
line-numbers = false

[preprocessor.mermaid]
command = "mdbook-mermaid"

[preprocessor.admonish]
command = "mdbook-admonish"
assets_version = "2.0.0"
```

**File:** `docs/book/src/SUMMARY.md`

```markdown
# Summary

[Introduction](./introduction.md)

# Getting Started

- [Installation](./getting-started/installation.md)
- [Quick Start](./getting-started/quick-start.md)
- [Your First Game](./getting-started/first-game.md)
- [Project Structure](./getting-started/project-structure.md)

# Core Concepts

- [Entity Component System](./core-concepts/ecs.md)
  - [Entities](./core-concepts/entities.md)
  - [Components](./core-concepts/components.md)
  - [Systems](./core-concepts/systems.md)
  - [Queries](./core-concepts/queries.md)
  - [Resources](./core-concepts/resources.md)
- [Game Loop](./core-concepts/game-loop.md)
- [Serialization](./core-concepts/serialization.md)

# Rendering

- [Rendering Overview](./rendering/overview.md)
- [Vulkan Setup](./rendering/vulkan-setup.md)
- [Mesh Rendering](./rendering/meshes.md)
- [Materials & Shaders](./rendering/materials.md)
- [Camera System](./rendering/camera.md)
- [Frame Capture](./rendering/frame-capture.md)

# Networking

- [Networking Overview](./networking/overview.md)
- [Client-Server Architecture](./networking/client-server.md)
- [Protocol Design](./networking/protocol.md)
- [State Synchronization](./networking/state-sync.md)
- [Client Prediction](./networking/prediction.md)
- [Server Reconciliation](./networking/reconciliation.md)

# Physics

- [Physics Overview](./physics/overview.md)
- [Rigid Bodies](./physics/rigid-bodies.md)
- [Collision Detection](./physics/collision.md)
- [Constraints](./physics/constraints.md)

# Audio

- [Audio System](./audio/overview.md)
- [Playing Sounds](./audio/sounds.md)
- [Spatial Audio](./audio/spatial.md)

# Scripting

- [Scripting with Lua](./scripting/lua.md)
- [Scripting with Rhai](./scripting/rhai.md)
- [Script Integration](./scripting/integration.md)

# Advanced Topics

- [Performance Optimization](./advanced/performance.md)
- [Memory Management](./advanced/memory.md)
- [Parallel Systems](./advanced/parallel.md)
- [Custom Allocators](./advanced/allocators.md)
- [Profiling](./advanced/profiling.md)

# Platform Support

- [Windows](./platforms/windows.md)
- [Linux](./platforms/linux.md)
- [macOS](./platforms/macos.md)
- [WebAssembly](./platforms/wasm.md)
- [Android](./platforms/android.md)
- [iOS](./platforms/ios.md)

# Tutorials

- [Singleplayer Game](./tutorials/singleplayer.md)
- [Multiplayer Game](./tutorials/multiplayer.md)
- [Turn-Based Strategy](./tutorials/turnbased.md)
- [MOBA Example](./tutorials/moba.md)

# API Reference

- [Core API](./api/core.md)
- [Rendering API](./api/rendering.md)
- [Networking API](./api/networking.md)
- [Physics API](./api/physics.md)
- [Audio API](./api/audio.md)

# Contributing

- [Contributing Guide](./contributing/guide.md)
- [Code Style](./contributing/code-style.md)
- [Testing](./contributing/testing.md)
- [Documentation](./contributing/documentation.md)

[Changelog](./changelog.md)
[FAQ](./faq.md)
```

---

### **2. Introduction & Getting Started** (Day 1)

**File:** `docs/book/src/introduction.md`

```markdown
# Introduction

Welcome to the **Silmaril** documentation!

The Silmaril is a high-performance, data-oriented game engine written in Rust. It's designed for building both singleplayer and multiplayer games with a focus on performance, scalability, and developer productivity.

## Why Silmaril?

- **Blazing Fast**: Built with Rust and data-oriented design for maximum performance
- **Multiplayer First**: Client-server networking with state synchronization built-in
- **Modern Rendering**: Vulkan-based renderer with advanced features
- **ECS Architecture**: Entity Component System for flexible game logic
- **Cross-Platform**: Supports Windows, Linux, macOS, Web, iOS, and Android
- **Open Source**: MIT licensed, free to use for any project

## Key Features

### Entity Component System (ECS)

The engine uses a powerful ECS architecture that separates data from logic:

```rust
// Define a component
#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

// Create a system
fn health_regen_system(world: &mut World, dt: f32) {
    for (entity, health) in world.query::<&mut Health>() {
        if health.current < health.max {
            health.current += 10.0 * dt;
        }
    }
}
```

### Networking

Built-in client-server architecture with state synchronization:

```rust
// Server
let mut server = GameServer::new(config).await?;
server.run().await?;

// Client
let mut client = GameClient::connect("127.0.0.1:7777").await?;
client.update(input, dt)?;
```

### High-Performance Rendering

Vulkan-based renderer with modern features:

- Physically-based rendering (PBR)
- Dynamic shadows
- Post-processing effects
- Instanced rendering
- GPU culling

### Cross-Platform

Write once, deploy everywhere:

- **Desktop**: Windows, Linux, macOS
- **Web**: WebAssembly with WebGL/WebGPU
- **Mobile**: iOS, Android
- **Console**: Support planned

## What Can You Build?

The Silmaril is suitable for:

- **Action Games**: Fast-paced combat with responsive controls
- **RPGs**: Complex character systems and inventory management
- **Strategy Games**: Turn-based or real-time tactical gameplay
- **Multiplayer Games**: MMORPGs, MOBAs, battle royales
- **Simulations**: Physics-based simulations and sandboxes

## Getting Help

- **Discord**: Join our [Discord server](https://discord.gg/silmaril)
- **GitHub**: Report issues on [GitHub](https://github.com/yourusername/silmaril)
- **Forum**: Ask questions on our [community forum](https://forum.silmaril.dev)

## Next Steps

Ready to get started? Head over to the [Installation](./getting-started/installation.md) guide!
```

**File:** `docs/book/src/getting-started/quick-start.md`

```markdown
# Quick Start

This guide will get you up and running with the Silmaril in 10 minutes.

## Prerequisites

- Rust 1.70 or later
- Vulkan SDK (for graphics)
- Git

## Create a New Project

```bash
cargo new my_game
cd my_game
```

## Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
silmaril = "0.1"
```

## Write Your First Game

Edit `src/main.rs`:

```rust
use silmaril::prelude::*;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Position(Vec3);

fn main() -> Result<()> {
    // Create app
    let mut app = App::new()?;

    // Setup world
    app.world_mut().spawn((
        Player,
        Position(Vec3::ZERO),
    ));

    // Add systems
    app.add_system(player_movement_system);
    app.add_system(render_system);

    // Run
    app.run()
}

fn player_movement_system(
    world: &mut World,
    input: Res<Input>,
    time: Res<Time>,
) {
    for (_, (_, position)) in world.query::<(&Player, &mut Position)>() {
        if input.is_key_pressed("W") {
            position.0.z += 5.0 * time.delta_seconds();
        }
        // ... other movement
    }
}

fn render_system(
    world: &World,
    renderer: ResMut<Renderer>,
) {
    for (_, position) in world.query::<&Position>() {
        renderer.draw_cube(position.0, Vec3::ONE);
    }
}
```

## Run Your Game

```bash
cargo run --release
```

You should see a window with a cube that you can move with WASD!

## What's Next?

- Learn about the [ECS architecture](../core-concepts/ecs.md)
- Follow the [First Game tutorial](./first-game.md)
- Explore [example games](../tutorials/singleplayer.md)

## Common Issues

### Vulkan Not Found

Make sure the Vulkan SDK is installed:

- **Windows**: Download from [LunarG](https://vulkan.lunarg.com/)
- **Linux**: `sudo apt install vulkan-tools libvulkan-dev`
- **macOS**: `brew install molten-vk`

### Slow Compilation

Use the `--release` flag for optimized builds:

```bash
cargo run --release
```

### GPU Errors

Check that your GPU supports Vulkan 1.2 or later:

```bash
vulkaninfo
```
```

---

### **3. Core Concepts Documentation** (Day 1-2)

**File:** `docs/book/src/core-concepts/ecs.md`

```markdown
# Entity Component System

The Silmaril uses an **Entity Component System (ECS)** architecture. This is a data-oriented design pattern that separates data (components) from logic (systems).

## Why ECS?

Traditional object-oriented hierarchies can be limiting for games:

```rust
// ❌ Traditional approach
class Entity {
    Position position;
    Renderer renderer;
    Physics physics;
    // What if we want an entity without physics?
}
```

ECS solves this with composition:

```rust
// ✅ ECS approach
world.spawn((
    Position::default(),
    Renderer::default(),
    // Add only what you need!
));
```

## Core Principles

### 1. Entities

Entities are just unique IDs. They have no data or behavior themselves.

```rust
let entity = world.spawn();
println!("Created entity: {:?}", entity); // Entity(0, generation: 0)
```

### 2. Components

Components are pure data structures:

```rust
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}
```

**Rules for components:**
- Must implement `Component` trait
- Should be data-only (no methods)
- Should be small and focused
- Use composition over large structs

### 3. Systems

Systems contain game logic and operate on components:

```rust
fn movement_system(world: &mut World, dt: f32) {
    // Query entities with both Position and Velocity
    for (entity, (position, velocity)) in world.query::<(&mut Position, &Velocity)>() {
        position.0 += velocity.0 * dt;
    }
}
```

## Example: Player Movement

Let's build a complete player movement system:

```rust
use silmaril::prelude::*;

// 1. Define components
#[derive(Component)]
struct Player {
    speed: f32,
}

#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Velocity(Vec3);

// 2. Spawn player
fn spawn_player(world: &mut World) {
    world.spawn((
        Player { speed: 5.0 },
        Position(Vec3::ZERO),
        Velocity(Vec3::ZERO),
    ));
}

// 3. Input system
fn player_input_system(
    world: &mut World,
    input: &Input,
) {
    for (_, (player, velocity)) in world.query::<(&Player, &mut Velocity)>() {
        let mut direction = Vec3::ZERO;

        if input.is_key_pressed("W") { direction.z += 1.0; }
        if input.is_key_pressed("S") { direction.z -= 1.0; }
        if input.is_key_pressed("A") { direction.x -= 1.0; }
        if input.is_key_pressed("D") { direction.x += 1.0; }

        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
        }

        velocity.0 = direction * player.speed;
    }
}

// 4. Movement system
fn movement_system(world: &mut World, dt: f32) {
    for (_, (position, velocity)) in world.query::<(&mut Position, &Velocity)>() {
        position.0 += velocity.0 * dt;
    }
}

// 5. Wire it all up
fn main() -> Result<()> {
    let mut app = App::new()?;

    // Spawn entities
    spawn_player(app.world_mut());

    // Add systems
    app.add_system(player_input_system);
    app.add_system(movement_system);

    app.run()
}
```

## Performance Benefits

ECS enables several optimizations:

### Cache-Friendly Iteration

Components are stored in contiguous arrays:

```rust
// ✅ Fast: Linear memory access
for position in positions.iter_mut() {
    position.0.y += 1.0;
}

// ❌ Slow: Random memory access
for entity in entities {
    entity.position.y += 1.0; // Cache miss!
}
```

### Parallel Systems

Systems that don't share mutable data can run in parallel:

```rust
// These can run simultaneously:
app.add_parallel_system(physics_system);
app.add_parallel_system(ai_system);
app.add_parallel_system(animation_system);
```

### Efficient Queries

Only iterate entities that have the required components:

```rust
// Only entities with Health AND Damage components
for (entity, (health, damage)) in world.query::<(&mut Health, &Damage)>() {
    health.current -= damage.amount;
}
```

## Best Practices

### Keep Components Small

```rust
// ✅ Good: Focused components
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Rotation(Quat);

// ❌ Bad: Mega component
#[derive(Component)]
struct Everything {
    position: Vec3,
    rotation: Quat,
    health: f32,
    inventory: Vec<Item>,
    // ... 50 more fields
}
```

### Use Marker Components

```rust
#[derive(Component)]
struct Player; // Empty marker component

#[derive(Component)]
struct Enemy;

// Query only player entities
for (entity, position) in world.query::<(&Player, &Position)>() {
    // ...
}
```

### Prefer Composition

```rust
// ✅ Good: Compose capabilities
world.spawn((
    Flying,      // Can fly
    Burning,     // Is on fire
    Frozen,      // Is frozen
));

// ❌ Bad: Inheritance-style
enum EntityState {
    FlyingAndBurning,
    FlyingAndFrozen,
    BurningAndFrozen,
    FlyingBurningAndFrozen, // Combinatorial explosion!
}
```

## Next Steps

- Learn about [Queries](./queries.md) for complex entity filtering
- Understand [Systems](./systems.md) execution order
- Explore [Resources](./resources.md) for global data
```

---

### **4. Tutorial Pages** (Day 2)

**File:** `docs/book/src/tutorials/singleplayer.md`

```markdown
# Tutorial: Singleplayer Game

In this tutorial, we'll build a complete singleplayer action game with:

- Player character with movement and combat
- Enemies with AI behavior
- Collectibles and scoring
- Health system
- Win/lose conditions

**Time:** ~2 hours
**Difficulty:** Beginner
**Code:** [examples/singleplayer](https://github.com/yourusername/silmaril/tree/main/examples/singleplayer)

## Prerequisites

- Basic Rust knowledge
- Completed [Quick Start](../getting-started/quick-start.md)
- Read [ECS Concepts](../core-concepts/ecs.md)

## Step 1: Project Setup

Create a new project:

```bash
cargo new singleplayer-game
cd singleplayer-game
```

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
silmaril = "0.1"
glam = "0.24"
rand = "0.8"
```

## Step 2: Define Components

Create `src/components.rs`:

```rust
use silmaril::prelude::*;
use glam::Vec3;

#[derive(Component)]
pub struct Player {
    pub score: i32,
}

#[derive(Component)]
pub struct Enemy {
    pub chase_speed: f32,
}

#[derive(Component)]
pub struct Position(pub Vec3);

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct Collider {
    pub radius: f32,
}
```

## Step 3: Player System

Create `src/systems/player.rs`:

```rust
use crate::components::*;
use silmaril::prelude::*;

pub fn player_movement_system(
    world: &mut World,
    input: &Input,
    dt: f32,
) {
    const SPEED: f32 = 5.0;

    for (_, (_, velocity)) in world.query::<(&Player, &mut Velocity)>() {
        let mut direction = Vec3::ZERO;

        if input.is_key_pressed("W") { direction.z += 1.0; }
        if input.is_key_pressed("S") { direction.z -= 1.0; }
        if input.is_key_pressed("A") { direction.x -= 1.0; }
        if input.is_key_pressed("D") { direction.x += 1.0; }

        if direction.length_squared() > 0.0 {
            velocity.0 = direction.normalize() * SPEED;
        } else {
            velocity.0 = Vec3::ZERO;
        }
    }
}
```

*[Continue with full tutorial...]*

## Step 4: Enemy AI

## Step 5: Combat System

## Step 6: Collectibles

## Step 7: Game Over

## Conclusion

You've built a complete singleplayer game! Here's what you learned:

- Component-based entity design
- System architecture
- Input handling
- Collision detection
- AI behaviors
- Game state management

## Next Steps

- Add more enemy types
- Implement powerups
- Create multiple levels
- Add sound effects
- Try the [Multiplayer Tutorial](./multiplayer.md)
```

---

### **5. API Reference** (Day 2-3)

**File:** `docs/book/src/api/core.md`

```markdown
# Core API Reference

## World

The `World` contains all entities and components.

### Methods

#### `spawn() -> Entity`

Creates a new entity.

```rust
let entity = world.spawn();
```

#### `add<T: Component>(&mut self, entity: Entity, component: T)`

Adds a component to an entity.

```rust
world.add(entity, Position(Vec3::ZERO));
```

#### `get<T: Component>(&self, entity: Entity) -> Option<&T>`

Gets a component reference.

```rust
if let Some(position) = world.get::<Position>(entity) {
    println!("Position: {:?}", position.0);
}
```

#### `get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T>`

Gets a mutable component reference.

```rust
if let Some(health) = world.get_mut::<Health>(entity) {
    health.current -= 10.0;
}
```

#### `remove<T: Component>(&mut self, entity: Entity) -> Option<T>`

Removes a component from an entity.

```rust
let removed = world.remove::<Velocity>(entity);
```

#### `despawn(&mut self, entity: Entity) -> bool`

Destroys an entity and all its components.

```rust
world.despawn(entity);
```

#### `query<Q: Query>(&self) -> QueryIter<Q>`

Queries entities with specific components.

```rust
for (entity, (position, velocity)) in world.query::<(&Position, &Velocity)>() {
    // ...
}
```

## Entity

An opaque handle to an entity.

```rust
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Entity {
    id: u32,
    generation: u32,
}
```

Entities use generational indices to prevent use-after-free bugs.

## Component

Trait for component types.

```rust
pub trait Component: 'static + Send + Sync {}
```

Implement with the derive macro:

```rust
#[derive(Component)]
struct MyComponent {
    data: i32,
}
```

## Query

Query trait for filtering entities.

### Supported Queries

- `&T` - Immutable component
- `&mut T` - Mutable component
- `Option<&T>` - Optional component
- `(A, B, C)` - Multiple components (up to 16)

### Examples

```rust
// Single component
for (entity, position) in world.query::<&Position>() { }

// Multiple components
for (entity, (pos, vel)) in world.query::<(&mut Position, &Velocity)>() { }

// Optional components
for (entity, (pos, vel)) in world.query:<(&Position, Option<&Velocity>)>() {
    if let Some(vel) = vel {
        // Has velocity
    }
}
```

## Resources

Global singleton data.

```rust
// Insert resource
world.insert_resource(Time::default());

// Get resource
let time = world.resource::<Time>();

// Get mutable resource
let mut time = world.resource_mut::<Time>();
```

*[Continue with complete API documentation...]*
```

---

### **6. Build & Deploy** (Day 3)

**File:** `docs/book/.github/workflows/deploy.yml`

```yaml
name: Deploy Documentation

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup mdBook
        uses: peaceiris/actions-mdbook@v1
        with:
          mdbook-version: 'latest'

      - name: Install preprocessors
        run: |
          cargo install mdbook-mermaid
          cargo install mdbook-admonish

      - name: Build book
        run: |
          cd docs/book
          mdbook build

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book/book
```

---

## ✅ **Acceptance Criteria**

- [ ] mdBook configured and building
- [ ] Introduction and getting started guides complete
- [ ] Core concepts documented with examples
- [ ] All major features have documentation
- [ ] API reference generated or written
- [ ] At least 3 complete tutorials
- [ ] Code examples tested and working
- [ ] Search functionality works
- [ ] Responsive design (mobile-friendly)
- [ ] CI/CD deploys to GitHub Pages
- [ ] Internal links all work
- [ ] Images and diagrams included
- [ ] Table of contents complete

---

## 🎯 **Quality Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Page count | 50+ | 30+ |
| Code examples | 100+ | 50+ |
| Tutorials | 5+ | 3+ |
| Build time | < 30s | < 60s |
| Coverage | 80%+ | 60%+ |

---

## 💡 **Best Practices**

### Writing Style

- Use clear, concise language
- Explain *why*, not just *how*
- Provide code examples for concepts
- Include common pitfalls and solutions
- Use diagrams where helpful

### Code Examples

- Test all code examples
- Keep examples focused and minimal
- Show complete, runnable code
- Highlight important lines
- Include expected output

### Structure

- Organize by user journey
- Start with simple concepts
- Build complexity gradually
- Cross-reference related topics
- Provide search keywords

---

**Dependencies:** Phase 1-4 (All engine features)
**Next:** [phase5-benchmarks.md](phase5-benchmarks.md)
