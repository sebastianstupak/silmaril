# Architecture Overview

> **Complete system architecture for agent-game-engine**
>
> ⚠️ **MUST READ** when working on any system-level code

---

## 🎯 **Architectural Principles**

### **1. Data-Driven Everything**
- Scenes defined in YAML/binary, not code
- ECS architecture (data separate from logic)
- Components are pure data structures
- Systems operate on component queries

### **2. Agent-First Design**
- Complete feedback loop (render → capture → analyze)
- Introspectable state (export to YAML at any time)
- Programmatic control (no UI required)
- Deterministic behavior (reproducible results)

### **3. Server-Authoritative**
- Server owns game state truth
- Client predicts, server corrects
- Critical logic only on server
- Anti-cheat by design

### **4. Cross-Platform from Day One**
- Platform abstraction layers (traits)
- No `#[cfg]` in business logic
- CI tests all platforms on every commit

### **5. Performance-First**
- Industry-standard targets (60 FPS client, 60 TPS server)
- Profile early and often
- Zero-cost abstractions where possible
- Batch operations, minimize allocations

---

## 🏗️ **System Architecture**

### **High-Level Diagram**

```
┌─────────────────────────────────────────────────────────────┐
│                     Game Client                              │
│                                                              │
│  ┌──────────┐   ┌──────────┐   ┌────────────┐             │
│  │  Input   │──▶│Prediction│──▶│  Renderer  │             │
│  │  System  │   │  World   │   │  (Vulkan)  │             │
│  └──────────┘   └────┬─────┘   └────────────┘             │
│                      │                                       │
│  ┌──────────────────▼──────────────────┐                   │
│  │      Network Client                 │                   │
│  │  - TCP (critical data)              │                   │
│  │  - UDP (positions/updates)          │                   │
│  │  - FlatBuffers serialization        │                   │
│  └──────────────────┬──────────────────┘                   │
└─────────────────────┼──────────────────────────────────────┘
                      │
            ┌─────────▼─────────┐
            │   Network Layer   │
            │ - Compression     │
            │ - Delta encoding  │
            └─────────┬─────────┘
                      │
┌─────────────────────▼──────────────────────────────────────┐
│                   Game Server                               │
│                                                             │
│  ┌──────────┐   ┌──────────┐   ┌────────────┐            │
│  │ Network  │──▶│   ECS    │──▶│Game Logic  │            │
│  │  Server  │   │  World   │   │  Systems   │            │
│  └──────────┘   └────┬─────┘   └────┬───────┘            │
│                      │              │                      │
│  ┌──────────────────▼──────────────▼──────────┐          │
│  │         Interest Management                │          │
│  │  - Spatial culling                         │          │
│  │  - LOD filtering                           │          │
│  │  - Fog of war                              │          │
│  └────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

---

## 🧩 **Core Systems**

### **ECS (Entity Component System)**

**Location:** `engine/core/ecs/`

**Responsibilities:**
- Entity lifecycle (spawn, despawn)
- Component storage (sparse sets)
- Query system (compile-time checked)
- World serialization

**Key Types:**
```rust
pub struct World { /* ... */ }
pub struct Entity(u32, u32); // ID + generation
pub trait Component: 'static + Send + Sync {}
pub trait Query { /* ... */ }
```

**Performance Targets:**
- Spawn 10k entities: < 1ms
- Query 10k entities (single component): < 0.5ms
- Query 10k entities (3 components): < 1ms

**See:** [docs/ecs.md](docs/ecs.md)

---

### **Renderer**

**Location:** `engine/renderer/`

**Responsibilities:**
- Vulkan initialization (cross-platform)
- Graphics pipeline management
- Mesh/texture loading
- Frame rendering + capture
- LOD rendering

**Key Types:**
```rust
pub struct VulkanRenderer { /* ... */ }
pub struct RenderResult {
    pub color: Option<Image>,
    pub depth: Option<Image>,
    pub metrics: PerformanceMetrics,
}
```

**Performance Targets:**
- Frame time (1080p, 10k triangles): < 16.67ms (60 FPS)
- Frame capture overhead: < 2ms

**See:** [docs/rendering.md](docs/rendering.md)

---

### **Networking**

**Location:** `engine/networking/`

**Responsibilities:**
- Client/server communication
- TCP + UDP channels
- State synchronization (full + delta)
- Client prediction + server reconciliation

**Key Types:**
```rust
pub struct NetworkClient { /* ... */ }
pub struct NetworkServer { /* ... */ }
pub enum StateUpdate {
    Full { snapshot_id: u64, state: WorldState },
    Delta { snapshot_id: u64, baseline_id: u64, changes: Vec<StateChange> },
}
```

**Performance Targets:**
- Latency overhead: < 5ms
- Bandwidth (delta vs full): 80%+ reduction
- Server tick rate: 60 TPS (1000 concurrent players)

**See:** [docs/networking.md](docs/networking.md)

---

### **Physics**

**Location:** `engine/physics/`

**Responsibilities:**
- Physics simulation (via backend trait)
- Collision detection
- Transform synchronization (ECS ↔ Physics)
- Async physics thread

**Key Types:**
```rust
pub trait PhysicsBackend { /* ... */ }
pub struct RapierBackend { /* ... */ }
pub struct RigidBody { /* ... */ }
```

**Performance Targets:**
- Physics step (1000 bodies): < 10ms

**See:** [docs/physics.md](docs/physics.md)

---

### **Audio**

**Location:** `engine/audio/`

**Responsibilities:**
- 3D spatial audio
- Audio asset loading
- Adaptive music

**Key Types:**
```rust
pub struct AudioEngine { /* ... */ }
pub struct AudioSource { /* ... */ }
```

**See:** [docs/audio.md](docs/audio.md)

---

### **LOD (Level of Detail)**

**Location:** `engine/lod/`

**Responsibilities:**
- Distance-based LOD switching (rendering)
- Network update rate reduction (bandwidth)
- Component filtering (send less data)

**Key Types:**
```rust
pub struct LodLevels {
    pub levels: Vec<LodLevel>,
}

pub struct LodLevel {
    pub distance: f32,
    pub mesh: Option<MeshHandle>,      // Rendering LOD
    pub update_rate: UpdateRate,       // Network LOD
    pub component_mask: ComponentMask, // Data LOD
}
```

**Performance Targets:**
- Network bandwidth reduction: 80%+
- Rendering performance gain: 50%+

**See:** [docs/lod.md](docs/lod.md)

---

### **Interest Management**

**Location:** `engine/interest/`

**Responsibilities:**
- Spatial culling (send only nearby entities)
- Fog of war (team-based visibility)
- Occlusion culling (line-of-sight)

**Key Types:**
```rust
pub struct InterestManager {
    grid: SpatialGrid,
    visibility_cache: HashMap<ClientId, VisibilitySet>,
}
```

**Performance Targets:**
- Per-client visibility computation: < 1ms
- Server CPU overhead: < 2% (like VALORANT)

**See:** [docs/interest-management.md](docs/interest-management.md)

---

## 🔀 **Data Flow**

### **Client Rendering Loop**

```
┌──────────────┐
│ Capture Input│
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Send to Server│ (UDP: movement, TCP: actions)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Apply Locally│ (Client prediction)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Receive Update│ (From server)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Reconcile    │ (Replay unacknowledged inputs)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Render     │ (Vulkan)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Capture Frame │ (Optional, for agent)
└──────────────┘
```

### **Server Tick Loop**

```
┌──────────────┐
│Receive Inputs│ (From all clients)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Validate Input│ (Anti-cheat)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Run Game Logic│ (Systems: physics, combat, etc.)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Interest Mgmt │ (Per-client visibility)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Generate Delta│ (Or full state if needed)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Send Updates │ (TCP or UDP per message type)
└──────────────┘
```

---

## 🔌 **Plugin Architecture**

Plugins are **separate crates** that extend the engine:

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn build(&self, app: &mut App);
    fn systems(&self) -> Vec<Box<dyn System>>;
    fn components(&self) -> Vec<ComponentDescriptor>;
}
```

### **Built-in Plugins**
- `renderer` - Vulkan rendering
- `physics` - Physics simulation (optional)
- `networking` - Client + server (optional)
- `audio` - Audio engine (optional)
- `lod` - LOD system (optional)
- `interest` - Interest management (optional)

### **Optional Plugins**
- `auto-update` - Auto-updater
- `server-scaling` - Database, Kubernetes
- `observability` - Metrics, tracing

### **Game Plugins**
Games can create their own plugins for custom logic.

---

## 📦 **Module Dependencies**

```
core
 ├─ ecs
 ├─ math
 ├─ assets
 └─ error

renderer ──▶ core
         ├─ platform (abstraction)
         └─ ash (Vulkan)

networking ──▶ core
           ├─ tokio (async)
           └─ flatbuffers

physics ──▶ core
        └─ rapier

audio ──▶ core
      └─ kira

lod ──▶ core
    └─ renderer (for mesh LOD)

interest ──▶ core
         └─ networking

client ──▶ core + renderer + networking(client) + physics(optional) + audio
server ──▶ core + networking(server) + physics + game-logic
```

**Rule:** No circular dependencies. Core is lowest level.

---

## 🎮 **Configuration System**

### **Hybrid Approach**

**Option 1: TOML/YAML files**
```toml
# game_config.toml
[renderer]
backend = "vulkan"
vsync = true
resolution = [1920, 1080]

[networking]
server_url = "ws://localhost:8080"
protocol = "tcp+udp"
```

**Option 2: Rust builder**
```rust
let config = EngineConfig::builder()
    .renderer(RendererConfig::vulkan()
        .vsync(true)
        .resolution(1920, 1080))
    .networking(NetworkingConfig::client()
        .server_url("ws://localhost:8080"))
    .build()?;
```

**Option 3: Env var overrides**
```bash
SERVER_URL=ws://prod.com:8080 cargo run
```

**All three work together:**
```rust
let config = EngineConfig::from_file("config.toml")?
    .with_env_overrides()?  // SERVER_URL, RUST_LOG
    .with_overrides(|c| {
        c.renderer.vsync = false;  // Force off
    });
```

---

## 🔒 **Security Architecture**

### **Server-Authoritative Design**

```
Client                          Server
------                          ------
Input: "Move forward"   ──▶     Validate input
                                ├─ Check physics (can move?)
                                ├─ Check cooldown (too soon?)
                                └─ Check state (alive? stunned?)

Apply locally (predict) ◀──     Apply to authoritative state
                                Send update

Reconcile with server
```

**Anti-Cheat Principles:**
1. **Never trust client** - All critical data on server
2. **Validate inputs** - Check physical constraints
3. **Fog of war** - Don't send invisible entities
4. **Rate limiting** - Prevent input spam
5. **Replay detection** - Sequence numbers

---

## 📊 **State Management**

### **World State**

```rust
#[derive(Serialize, Deserialize)]
pub struct WorldState {
    pub version: u64,
    pub entities: Vec<EntityState>,
}

#[derive(Serialize, Deserialize)]
pub struct EntityState {
    pub id: EntityId,
    pub components: Vec<ComponentData>,
}
```

**Serialization Formats:**
- **YAML**: Debug, agent inspection (human-readable)
- **Bincode**: Local IPC (fast)
- **FlatBuffers**: Network (zero-copy)

**Use Cases:**
- Save/load games
- Network synchronization
- Replay recording
- Agent state inspection

---

## 🧪 **Testing Strategy**

### **Unit Tests**
- Each component, system tested independently
- Mock dependencies
- Fast (< 1s total)

### **Integration Tests**
- Test crate boundaries
- Real dependencies (no mocks)
- Headless rendering tests

### **E2E Tests**
- Full client + server
- Docker Compose orchestration
- Visual verification (screenshot comparison)

### **Property Tests**
- Serialization roundtrips
- Math operations
- State synchronization correctness

**See:** [docs/testing-strategy.md](docs/testing-strategy.md)

---

## 🚀 **Deployment Architecture**

### **Singleplayer Game**
```
┌─────────────┐
│   Client    │
│  (no server)│
└─────────────┘
```

**Deployment:** Single executable

---

### **Multiplayer Game**

```
┌─────────────┐        ┌─────────────┐
│   Client    │◀──────▶│   Server    │
│             │        │ (Container) │
└─────────────┘        └─────┬───────┘
                             │
                       ┌─────▼───────┐
                       │  Database   │
                       │ (PostgreSQL)│
                       └─────────────┘
```

**Deployment:**
- Client: Downloadable executable (with auto-update)
- Server: Docker container in Kubernetes
- Database: Managed service (AWS RDS, etc.)

---

### **Production Stack (Example)**

```
                    ┌──────────────┐
                    │ Load Balancer│
                    │  (AWS ALB)   │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐        ┌────▼────┐       ┌────▼────┐
   │ Server  │        │ Server  │       │ Server  │
   │  Pod 1  │        │  Pod 2  │       │  Pod 3  │
   └────┬────┘        └────┬────┘       └────┬────┘
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
                    ┌──────▼───────┐
                    │   Redis      │
                    │  (Cache)     │
                    └──────────────┘
                           │
                    ┌──────▼───────┐
                    │  PostgreSQL  │
                    │ (Persistent) │
                    └──────────────┘
```

---

## 📈 **Scalability Considerations**

### **Horizontal Scaling**
- Stateless game servers (state in DB)
- Consistent hashing (player → server mapping)
- Database sharding (by region, guild, etc.)

### **Vertical Scaling**
- Multi-threaded physics
- Async networking (tokio)
- GPU compute for complex operations

### **Caching**
- Redis for frequently accessed data
- In-memory caches with TTL
- CDN for asset delivery

**Target:** 10,000+ concurrent players per region

---

## 🔧 **Development vs Production**

| Feature | Development | Production |
|---------|-------------|------------|
| Logging | Pretty, verbose (TRACE) | JSON, structured (INFO) |
| Validation | Vulkan validation layers | Disabled |
| Assets | Loose files (hot-reload) | Bundled .pak |
| Networking | Localhost | TLS + compression |
| Database | SQLite local | PostgreSQL managed |
| Profiling | Tracy enabled | Disabled |

**Feature flags control the split:**
```rust
#[cfg(debug_assertions)]
fn init_logging() {
    // Pretty console output
}

#[cfg(not(debug_assertions))]
fn init_logging() {
    // JSON for log aggregation
}
```

---

## 📚 **Related Documentation**

- [docs/ecs.md](docs/ecs.md) - ECS implementation details
- [docs/networking.md](docs/networking.md) - Network architecture
- [docs/rendering.md](docs/rendering.md) - Vulkan renderer
- [docs/platform-abstraction.md](docs/platform-abstraction.md) - Cross-platform layers
- [docs/performance-targets.md](docs/performance-targets.md) - Performance goals

---

**Last Updated:** 2026-01-31
