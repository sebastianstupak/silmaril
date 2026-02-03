# Phase 2 Game Loop Design

> **Purpose:** Complete architecture for Client and Server game loops
> **Target:** Integrate all Phase 2 networking systems into working game loops
> **Status:** Design Document - Ready for Implementation

---

## 🎯 **Design Goals**

### **Client Loop:**
- 60 FPS target (16.67ms per frame)
- Responsive input (<16ms latency)
- Smooth rendering with prediction
- Network I/O without blocking rendering

### **Server Loop:**
- 60 TPS target (16.67ms per tick)
- Authoritative simulation
- Fair processing (all clients get equal time)
- Efficient state synchronization

---

## 🔄 **CLIENT GAME LOOP ARCHITECTURE**

### **High-Level Flow**

```
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT MAIN LOOP                        │
│                     (Target: 60 FPS)                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  1. Poll Window Events (winit)        │
        │     - Input (keyboard, mouse)         │
        │     - Window resize, close            │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  2. Process Network Messages          │
        │     - Receive from server (TCP/UDP)   │
        │     - Apply state updates             │
        │     - Reconcile predictions           │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  3. Send Input to Server              │
        │     - Buffered inputs from step 1     │
        │     - Send via UDP                    │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  4. Run Client Prediction             │
        │     - Apply input locally             │
        │     - Physics/movement prediction     │
        │     - Store predicted state           │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  5. Update ECS Systems                │
        │     - Animation                       │
        │     - Audio (3D spatial)              │
        │     - Particles, VFX                  │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  6. Render Frame                      │
        │     - Query renderable entities       │
        │     - Submit draw calls               │
        │     - Present to screen               │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  7. Frame Time Management             │
        │     - Measure delta time              │
        │     - Sleep if under budget           │
        │     - Detect frame drops              │
        └───────────────────────────────────────┘
                            │
                            │
                            └──────────┐
                                       │ Loop back
                                       └─────────┐
                                                 ▼
```

---

### **Detailed Client Loop Implementation**

```rust
// engine/binaries/client/src/main.rs

use engine_core::{World, Time};
use engine_renderer::Renderer;
use engine_networking::Client;
use engine_audio::AudioEngine;
use winit::event_loop::{EventLoop, ControlFlow};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize subsystems
    let event_loop = EventLoop::new()?;
    let mut world = World::new();
    let mut renderer = Renderer::new(&event_loop)?;
    let mut client = Client::new()?;
    let mut audio = AudioEngine::new()?;
    let mut time = Time::new();

    // 2. Connect to server
    client.connect("127.0.0.1:7777")?;
    info!("Connected to server");

    // 3. Input buffer for prediction
    let mut input_buffer = InputBuffer::new();

    // 4. Main loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    // Step 1: Capture input
                    WindowEvent::KeyboardInput { input, .. } => {
                        let action = map_input_to_action(input);
                        input_buffer.push(action);
                    }

                    WindowEvent::CloseRequested => {
                        client.disconnect();
                        elwt.exit();
                    }

                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width, size.height);
                    }

                    _ => {}
                }
            }

            Event::AboutToWait => {
                // Step 2: Process network messages
                while let Some(message) = client.poll_message() {
                    match message {
                        ServerMessage::StateUpdate(state) => {
                            // Apply server state
                            world.apply_state_update(state);

                            // Reconcile predictions
                            input_buffer.reconcile(state.server_tick);
                        }

                        ServerMessage::EntitySpawned(entity) => {
                            world.spawn_from_network(entity);
                        }

                        ServerMessage::EntityDespawned(id) => {
                            world.despawn(id);
                        }

                        _ => {}
                    }
                }

                // Step 3: Send input to server
                if let Some(input) = input_buffer.pop_pending() {
                    client.send_input(input);
                }

                // Step 4: Client-side prediction
                let local_player_id = client.local_player_id();
                if let Ok(prediction) = world.get::<ClientPrediction>(local_player_id) {
                    // Apply local input for immediate feedback
                    for input in input_buffer.unconfirmed_inputs() {
                        prediction.apply_input(input, time.delta());
                    }
                }

                // Step 5: Update ECS systems
                time.update();
                run_client_systems(&mut world, time.delta());
                audio.update(&world);

                // Step 6: Render
                renderer.begin_frame();
                render_world(&renderer, &world);
                renderer.end_frame();

                // Step 7: Frame time management
                let frame_time = time.frame_time();
                if frame_time < TARGET_FRAME_TIME {
                    // Optionally yield if frame completed early
                    // (winit will handle this automatically)
                }

                if frame_time > TARGET_FRAME_TIME * 1.5 {
                    warn!("Frame drop: {:?}", frame_time);
                }
            }

            _ => {}
        }
    })?;

    Ok(())
}

// Client-specific systems (no physics, rendering only)
fn run_client_systems(world: &mut World, dt: f32) {
    // Animation system
    animation_system(world, dt);

    // Audio system (3D spatial)
    audio_system(world);

    // Particle systems
    particle_system(world, dt);

    // VFX systems
    vfx_system(world, dt);
}
```

---

### **Client Threading Model**

```
┌─────────────────────────────────────────────────────────────┐
│                        MAIN THREAD                           │
│  - Window events (winit)                                     │
│  - Rendering (Vulkan - main thread required)                │
│  - ECS systems                                               │
│  - Input processing                                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ├─ Channels ─┐
                            │             │
┌───────────────────────────▼─────────────▼───────────────────┐
│                    NETWORK THREAD                            │
│  - TCP receive loop (tokio)                                  │
│  - UDP receive loop (tokio)                                  │
│  - Message deserialization                                   │
│  - Send queue processing                                     │
└──────────────────────────────────────────────────────────────┘
                            │
                            ├─ Channels ─┐
                            │             │
┌───────────────────────────▼─────────────▼───────────────────┐
│                     AUDIO THREAD                             │
│  - Kira audio engine                                         │
│  - 3D spatial audio processing                               │
│  - Effects processing                                        │
└──────────────────────────────────────────────────────────────┘
```

**Communication:**
- **Main → Network:** `mpsc::channel` for sending inputs
- **Network → Main:** `mpsc::channel` for state updates
- **Main → Audio:** Direct API calls (Kira is thread-safe)

---

## 🖥️ **SERVER GAME LOOP ARCHITECTURE**

### **High-Level Flow**

```
┌─────────────────────────────────────────────────────────────┐
│                      SERVER MAIN LOOP                        │
│                     (Target: 60 TPS)                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  1. Process Network Events            │
        │     - New connections                 │
        │     - Disconnections                  │
        │     - TCP messages (reliable)         │
        │     - UDP packets (positions)         │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  2. Process Client Inputs             │
        │     - Per-client input queue          │
        │     - Validate inputs (anti-cheat)    │
        │     - Apply to entities               │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  3. Run Game Simulation               │
        │     - Physics (Rapier)                │
        │     - Game logic systems              │
        │     - AI/NPCs                         │
        │     - Collision resolution            │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  4. Update Interest Management        │
        │     - Spatial grid update             │
        │     - Per-client visibility           │
        │     - Adaptive interest filtering     │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  5. Generate State Updates            │
        │     - Full snapshot (1Hz)             │
        │     - Delta updates (20Hz)            │
        │     - Per-client filtering            │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  6. Send Updates to Clients           │
        │     - TCP for critical data           │
        │     - UDP for positions               │
        │     - Rate limit per client           │
        └───────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │  7. Tick Time Management              │
        │     - Measure tick duration           │
        │     - Sleep for remaining time        │
        │     - Detect tick overruns            │
        └───────────────────────────────────────┘
                            │
                            │
                            └──────────┐
                                       │ Loop back
                                       └─────────┐
                                                 ▼
```

---

### **Detailed Server Loop Implementation**

```rust
// engine/binaries/server/src/main.rs

use engine_core::{World, Time};
use engine_networking::{Server, ServerConfig};
use engine_physics::PhysicsWorld;
use std::time::{Duration, Instant};

const TARGET_TPS: u32 = 60;
const TARGET_TICK_TIME: Duration = Duration::from_micros(16_667); // 60Hz

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize subsystems
    let mut world = World::new();
    let mut physics = PhysicsWorld::new();
    let mut server = Server::bind("0.0.0.0:7777").await?;
    let mut time = Time::new();

    info!("Server listening on 0.0.0.0:7777");

    // 2. Server state
    let mut tick_count: u64 = 0;
    let mut snapshot_timer = 0.0f32; // Full snapshot every 1 second

    // 3. Main tick loop
    loop {
        let tick_start = Instant::now();

        // Step 1: Process network events
        // Note: Non-blocking poll of network events
        while let Some(event) = server.poll_event() {
            match event {
                NetworkEvent::ClientConnected { client_id, addr } => {
                    info!("Client {} connected from {}", client_id, addr);

                    // Spawn player entity
                    let player_entity = world.spawn();
                    world.add(player_entity, Transform::default());
                    world.add(player_entity, Velocity::default());
                    world.add(player_entity, PlayerComponent { client_id });

                    // Send initial world state to new client
                    let world_state = world.serialize_full();
                    server.send_to(client_id, ServerMessage::InitialState(world_state));
                }

                NetworkEvent::ClientDisconnected { client_id } => {
                    info!("Client {} disconnected", client_id);

                    // Remove player entity
                    if let Some(entity) = find_player_entity(&world, client_id) {
                        world.despawn(entity);
                    }
                }

                NetworkEvent::Message { client_id, message } => {
                    match message {
                        ClientMessage::Input(input) => {
                            // Queue input for processing
                            server.queue_input(client_id, input);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Step 2: Process client inputs
        for (client_id, inputs) in server.drain_input_queues() {
            if let Some(player_entity) = find_player_entity(&world, client_id) {
                for input in inputs {
                    // Validate input (anti-cheat)
                    if validate_input(&input) {
                        // Apply to entity
                        apply_input_to_entity(&mut world, player_entity, input);
                    } else {
                        warn!("Invalid input from client {}", client_id);
                    }
                }
            }
        }

        // Step 3: Run game simulation (SERVER-AUTHORITATIVE)
        time.update();
        let dt = time.delta();

        // Physics simulation
        physics.step(dt);
        physics.sync_to_world(&mut world);

        // Game logic systems
        run_server_systems(&mut world, dt);

        // Step 4: Update interest management
        let interest_updates = update_interest_management(&world, &server);

        // Step 5: Generate state updates
        snapshot_timer += dt;

        if snapshot_timer >= 1.0 {
            // Full snapshot every 1 second
            let snapshot = world.serialize_full();
            server.broadcast(ServerMessage::Snapshot(snapshot));
            snapshot_timer = 0.0;
        } else {
            // Delta updates at 20Hz (every 3 ticks at 60 TPS)
            if tick_count % 3 == 0 {
                let delta = world.serialize_delta();

                // Per-client filtering based on interest
                for (client_id, interest) in interest_updates {
                    let filtered_delta = delta.filter_by_interest(&interest);
                    server.send_to(client_id, ServerMessage::DeltaUpdate(filtered_delta));
                }
            }
        }

        // Step 6: Send updates (already done in step 5)
        // Network thread handles actual sending

        // Step 7: Tick time management
        let tick_duration = tick_start.elapsed();

        if tick_duration < TARGET_TICK_TIME {
            // Sleep for remaining time
            let sleep_time = TARGET_TICK_TIME - tick_duration;
            tokio::time::sleep(sleep_time).await;
        } else {
            // Tick overrun!
            warn!(
                "Tick overrun: {:?} (target: {:?})",
                tick_duration,
                TARGET_TICK_TIME
            );
        }

        // Metrics
        if tick_count % 60 == 0 {
            let tps = 1.0 / tick_duration.as_secs_f32();
            info!(
                "Tick {}: {:.1} TPS, {} clients",
                tick_count,
                tps,
                server.client_count()
            );
        }

        tick_count += 1;
    }
}

// Server-authoritative systems
fn run_server_systems(world: &mut World, dt: f32) {
    // Movement system (applies velocities)
    movement_system(world, dt);

    // Combat system (damage, health)
    combat_system(world, dt);

    // AI system (NPCs)
    ai_system(world, dt);

    // Spawner system (item spawns, etc.)
    spawner_system(world, dt);
}
```

---

### **Server Threading Model**

```
┌─────────────────────────────────────────────────────────────┐
│                        MAIN THREAD                           │
│  - Game simulation (ECS systems)                             │
│  - Physics (Rapier)                                          │
│  - State generation                                          │
│  - Interest management                                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ├─ Channels ─┐
                            │             │
┌───────────────────────────▼─────────────▼───────────────────┐
│                  NETWORK THREAD (Tokio)                      │
│  - TCP listener (accept connections)                         │
│  - TCP receive loops (per client)                            │
│  - UDP socket (shared)                                       │
│  - Message serialization/deserialization                     │
│  - Send queue processing                                     │
└──────────────────────────────────────────────────────────────┘
                            │
                            ├─ Optional ─┐
                            │             │
┌───────────────────────────▼─────────────▼───────────────────┐
│                    PHYSICS THREAD                            │
│  (Optional: Async physics)                                   │
│  - Rapier simulation in background                           │
│  - Sync results to main thread                               │
└──────────────────────────────────────────────────────────────┘
```

**Communication:**
- **Main → Network:** `mpsc::channel` for outgoing messages
- **Network → Main:** `mpsc::channel` for incoming events
- **Main → Physics:** Direct API calls (or channel for async)

---

## ⚖️ **Key Design Decisions**

### **1. Fixed Timestep vs Variable Timestep**

**Decision:** Fixed timestep (60 Hz) for both client and server

**Rationale:**
- Deterministic simulation (critical for prediction/reconciliation)
- Predictable network behavior
- Easier to debug desyncs

**Implementation:**
```rust
const TICK_RATE: f32 = 60.0;
const FIXED_DELTA: f32 = 1.0 / TICK_RATE;

// Always use FIXED_DELTA for simulation
physics.step(FIXED_DELTA);
```

---

### **2. Prediction Strategy**

**Decision:** Client predicts own entity only (not other players)

**Rationale:**
- Simpler to implement
- Avoids complex rollback of other entities
- Most responsive for local player (what matters most)

**Implementation:**
```rust
// Only predict local player
if entity_id == local_player_id {
    apply_prediction(entity, input);
} else {
    // Other entities: interpolate between updates
    interpolate_entity(entity, server_state);
}
```

---

### **3. State Update Strategy**

**Decision:** Hybrid approach
- Full snapshots: 1 Hz (every 60 ticks)
- Delta updates: 20 Hz (every 3 ticks)

**Rationale:**
- Full snapshots prevent drift
- Deltas reduce bandwidth
- 20 Hz is sufficient for smooth motion

---

### **4. Interest Management Integration**

**Decision:** Filter deltas per-client before sending

**Implementation:**
```rust
// Generate delta once
let delta = world.serialize_delta();

// Send filtered version to each client
for client_id in server.connected_clients() {
    let visible_entities = interest_manager.get_visible(client_id);
    let filtered = delta.filter(visible_entities);
    server.send_to(client_id, filtered);
}
```

---

## 📊 **Performance Budgets**

### **Client (per frame, 16.67ms budget):**

| Task | Budget | Priority |
|------|--------|----------|
| Input processing | 0.5ms | High |
| Network receive | 1.0ms | High |
| Client prediction | 1.0ms | High |
| ECS systems | 3.0ms | Medium |
| Rendering | 10.0ms | High |
| Frame sync | 1.0ms | Low |
| **TOTAL** | **16.5ms** | - |

---

### **Server (per tick, 16.67ms budget):**

| Task | Budget | Priority |
|------|--------|----------|
| Network events | 2.0ms | High |
| Input processing | 1.0ms | High |
| Physics | 5.0ms | High |
| ECS systems | 4.0ms | Medium |
| Interest management | 2.0ms | Medium |
| State generation | 2.0ms | High |
| **TOTAL** | **16.0ms** | - |

---

## 🧪 **Testing Strategy**

### **Client Loop Testing:**
1. **Headless mode** (no rendering)
2. **Input recording/playback**
3. **Network stub** (simulated server)
4. **Frame time profiling**

### **Server Loop Testing:**
1. **Synthetic clients** (bots)
2. **Load testing** (100+ clients)
3. **Tick time profiling**
4. **Memory leak detection**

---

## 📁 **File Changes Required**

### **Client Binary:**
```
engine/binaries/client/src/
├── main.rs                 # Main loop (update from stub)
├── input.rs                # Input mapping
├── prediction.rs           # Client prediction logic
└── rendering.rs            # Render world helper
```

### **Server Binary:**
```
engine/binaries/server/src/
├── main.rs                 # Main loop (update from stub)
├── systems.rs              # Server systems
├── validation.rs           # Input validation (anti-cheat)
└── interest.rs             # Interest management integration
```

---

## ✅ **Implementation Checklist**

### **Client Implementation (3-4 days):**
- [ ] Update main.rs with full loop
- [ ] Integrate input system
- [ ] Wire up network receive
- [ ] Implement client prediction
- [ ] Connect renderer
- [ ] Add frame time management
- [ ] Test with real server

### **Server Implementation (3-4 days):**
- [ ] Update main.rs with full loop
- [ ] Integrate network events
- [ ] Wire up input processing
- [ ] Connect physics
- [ ] Implement state generation
- [ ] Add interest filtering
- [ ] Add tick time management
- [ ] Test with real clients

### **Integration Testing (2 days):**
- [ ] Test client + server together
- [ ] Verify prediction/reconciliation
- [ ] Test with multiple clients
- [ ] Profile performance
- [ ] Fix bugs

**Total: 8-10 days**

---

## 🚀 **Next Steps**

1. **Review this design** with team
2. **Start with client loop** (more visible progress)
3. **Then implement server loop**
4. **Test together**
5. **Iterate and optimize**

---

**Status:** ✅ Design Complete - Ready for Implementation
**Author:** Claude Sonnet 4.5
**Date:** 2026-02-03
