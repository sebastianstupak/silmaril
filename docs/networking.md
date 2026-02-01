# Networking Architecture

> **Client-server networking for agent-game-engine**
>
> Server-authoritative multiplayer with client-side prediction, optimized for AI agents

---

## Overview

The agent-game-engine uses a hybrid TCP+UDP networking architecture:
- **TCP** - Reliable critical data (login, chat, commands)
- **UDP** - Unreliable fast data (positions, rotations, actions)
- **Server-authoritative** - Anti-cheat and consistency
- **Client prediction** - Responsive local input
- **Delta compression** - Bandwidth optimization

## Architecture

### Client/Server Separation

Code is separated at compile-time using feature flags:

```rust
use agent_game_engine_macros::{client_only, server_only, shared, server_authoritative};

#[client_only]
fn render_health_bars(world: &World, renderer: &mut Renderer) {
    // Only compiled in client binary
}

#[server_only]
fn validate_movement(input: &PlayerInput, world: &World) -> bool {
    // Only compiled in server binary
    // Anti-cheat logic here
}

#[shared]
fn calculate_damage(weapon: &Weapon, distance: f32) -> f32 {
    // Compiled in both client and server
}

#[server_authoritative]
fn apply_damage(target: Entity, damage: f32, world: &mut World) {
    // Server implementation (authoritative)
    let health = world.get_mut::<Health>(target).unwrap();
    health.current -= damage;
}

#[server_authoritative]
fn apply_damage(target: Entity, damage: f32, world: &mut World) {
    // Client implementation (prediction only)
    // Visual feedback, no actual state change
}
```

**Build Commands:**
```bash
cargo build --bin client --features client    # Client binary
cargo build --bin server --features server    # Server binary
```

**Implementation:** `engine/macros/src/client_server.rs` ✅ Complete

---

## Network Protocol

### Message Types

```rust
pub enum ClientMessage {
    Login { username: String, password: String },
    Input { sequence: u32, input: PlayerInput },
    Chat { message: String },
    Disconnect,
}

pub enum ServerMessage {
    LoginResponse { success: bool, player_id: u32 },
    StateUpdate { update: StateUpdate },
    Chat { sender: String, message: String },
    Kicked { reason: String },
}
```

### Serialization

Messages use FlatBuffers for zero-copy serialization:

```fbs
// network.fbs
namespace Network;

table PlayerInput {
    sequence: uint32;
    forward: float;
    right: float;
    jump: bool;
    fire: bool;
}

table StateUpdate {
    full_state: [ubyte];  // Full world state (rare)
    delta: [ubyte];       // Delta from last ack (common)
}
```

**Build Integration:**
```rust
// build.rs
fn main() {
    flatbuffers::compile_schemas(&["schemas/network.fbs"]);
}
```

**Status:** ⚪ Not implemented (Phase 2.2)

---

## TCP Channel

### Connection Management

```rust
use tokio::net::{TcpListener, TcpStream};

pub struct TcpServer {
    listener: TcpListener,
    clients: HashMap<u32, TcpClient>,
}

impl TcpServer {
    pub async fn bind(addr: &str) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            clients: HashMap::new(),
        })
    }

    pub async fn accept(&mut self) -> Result<TcpClient, NetworkError> {
        let (stream, addr) = self.listener.accept().await?;
        Ok(TcpClient::new(stream, addr))
    }

    pub async fn send_reliable(&mut self, client_id: u32, message: ServerMessage)
        -> Result<(), NetworkError>
    {
        let client = self.clients.get_mut(&client_id)
            .ok_or(NetworkError::ClientNotFound)?;
        client.send(message).await
    }
}
```

### Message Framing

Use length-prefix framing to handle partial reads:

```
+--------+------------------+
| Length | Message Payload  |
| 4 bytes| N bytes          |
+--------+------------------+
```

```rust
async fn send_framed(stream: &mut TcpStream, message: &[u8]) -> io::Result<()> {
    let len = message.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(message).await?;
    Ok(())
}

async fn recv_framed(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).await?;
    Ok(buffer)
}
```

### Heartbeat

Keep connections alive with periodic heartbeats:

```rust
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn heartbeat_task(client: Arc<Mutex<TcpClient>>) {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        interval.tick().await;
        let mut client = client.lock().await;
        if client.send(ServerMessage::Heartbeat).await.is_err() {
            break; // Connection lost
        }
    }
}
```

**Status:** ⚪ Not implemented (Phase 2.3)

---

## UDP Channel

### Unreliable Messaging

```rust
use tokio::net::UdpSocket;

pub struct UdpServer {
    socket: UdpSocket,
    sequence: AtomicU32,
}

impl UdpServer {
    pub async fn bind(addr: &str) -> Result<Self, NetworkError> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket,
            sequence: AtomicU32::new(0),
        })
    }

    pub async fn send_unreliable(&self, addr: SocketAddr, message: &[u8])
        -> Result<(), NetworkError>
    {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut packet = Vec::with_capacity(4 + message.len());
        packet.extend_from_slice(&seq.to_le_bytes());
        packet.extend_from_slice(message);

        self.socket.send_to(&packet, addr).await?;
        Ok(())
    }

    pub async fn recv(&self) -> Result<(SocketAddr, u32, Vec<u8>), NetworkError> {
        let mut buffer = vec![0u8; 1500]; // MTU size
        let (len, addr) = self.socket.recv_from(&mut buffer).await?;
        buffer.truncate(len);

        let seq = u32::from_le_bytes(buffer[0..4].try_into()?);
        let data = buffer[4..].to_vec();

        Ok((addr, seq, data))
    }
}
```

### Sequence Numbers

Track sequence numbers to detect:
- **Duplicates** - Discard packets with old sequences
- **Out-of-order** - Reorder if needed (or accept disorder)
- **Packet loss** - Detect gaps in sequence

```rust
pub struct PacketTracker {
    last_sequence: u32,
    received: BitSet, // Track last 256 packets
}

impl PacketTracker {
    pub fn should_accept(&mut self, sequence: u32) -> bool {
        if sequence <= self.last_sequence.saturating_sub(256) {
            return false; // Too old
        }

        if sequence > self.last_sequence {
            self.last_sequence = sequence;
            true
        } else {
            // Out of order - check if already received
            let offset = (self.last_sequence - sequence) as usize;
            !self.received.test_and_set(offset)
        }
    }
}
```

**Status:** ⚪ Not implemented (Phase 2.4)

---

## State Synchronization

### Full State Updates

Send complete world state (rare, fallback):

```rust
pub struct FullStateUpdate {
    pub tick: u32,
    pub entities: Vec<EntityState>,
}

pub struct EntityState {
    pub entity: Entity,
    pub components: HashMap<TypeId, ComponentData>,
}
```

### Delta Compression

Send only changes since last acknowledged state (common):

```rust
pub struct DeltaUpdate {
    pub base_tick: u32,   // Tick this delta is based on
    pub current_tick: u32,
    pub spawned: Vec<Entity>,
    pub despawned: Vec<Entity>,
    pub modified: Vec<EntityDelta>,
}

pub struct EntityDelta {
    pub entity: Entity,
    pub components: HashMap<TypeId, ComponentData>,
}
```

### Snapshot History

Server maintains history for delta calculation:

```rust
pub struct SnapshotHistory {
    snapshots: VecDeque<WorldSnapshot>,
    max_history: usize, // 60 ticks = 1 second at 60 TPS
}

impl SnapshotHistory {
    pub fn add(&mut self, snapshot: WorldSnapshot) {
        self.snapshots.push_back(snapshot);
        if self.snapshots.len() > self.max_history {
            self.snapshots.pop_front();
        }
    }

    pub fn get(&self, tick: u32) -> Option<&WorldSnapshot> {
        self.snapshots.iter().find(|s| s.tick == tick)
    }

    pub fn compute_delta(&self, from_tick: u32, to_tick: u32)
        -> Option<DeltaUpdate>
    {
        let from = self.get(from_tick)?;
        let to = self.get(to_tick)?;
        Some(from.diff(to))
    }
}
```

### Adaptive Strategy

Switch between full/delta based on delta size:

```rust
const DELTA_SIZE_THRESHOLD: usize = 1024; // bytes

fn create_state_update(history: &SnapshotHistory, client_ack: u32)
    -> StateUpdate
{
    if let Some(delta) = history.compute_delta(client_ack, current_tick) {
        let delta_bytes = delta.to_bytes();
        if delta_bytes.len() < DELTA_SIZE_THRESHOLD {
            return StateUpdate::Delta(delta);
        }
    }

    // Delta too large or base snapshot not available
    StateUpdate::Full(history.latest())
}
```

**Status:** ⚪ Not implemented (Phase 2.5)

---

## Client-Side Prediction

### Input Buffering

Client buffers inputs with sequence numbers:

```rust
pub struct InputBuffer {
    inputs: VecDeque<SequencedInput>,
    next_sequence: u32,
}

pub struct SequencedInput {
    pub sequence: u32,
    pub input: PlayerInput,
    pub timestamp: Instant,
}

impl InputBuffer {
    pub fn add(&mut self, input: PlayerInput) -> u32 {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        self.inputs.push_back(SequencedInput {
            sequence,
            input,
            timestamp: Instant::now(),
        });

        sequence
    }

    pub fn ack(&mut self, sequence: u32) {
        self.inputs.retain(|i| i.sequence > sequence);
    }
}
```

### Prediction

Client predicts movement immediately:

```rust
#[client_only]
fn client_update(state: &mut ClientState, input: PlayerInput) {
    // Buffer input for server
    let sequence = state.input_buffer.add(input.clone());

    // Send to server (unreliable)
    state.send_input(sequence, input.clone());

    // Apply locally for immediate feedback
    apply_input_to_world(&mut state.predicted_world, input);
}
```

### Server Reconciliation

When server state arrives, rewind and replay:

```rust
#[client_only]
fn on_server_state(state: &mut ClientState, server_state: WorldState, acked_sequence: u32) {
    // Discard acknowledged inputs
    state.input_buffer.ack(acked_sequence);

    // Reset to server state
    state.predicted_world = server_state;

    // Replay unacknowledged inputs
    for input in &state.input_buffer.inputs {
        apply_input_to_world(&mut state.predicted_world, input.input.clone());
    }
}
```

### Interpolation

Smooth out prediction errors:

```rust
#[client_only]
fn interpolate_position(
    current: Vec3,
    server: Vec3,
    dt: f32,
) -> Vec3 {
    const LERP_SPEED: f32 = 10.0; // units per second
    current.lerp(server, (LERP_SPEED * dt).min(1.0))
}
```

**Status:** ⚪ Not implemented (Phase 2.6)

---

## Server Tick

### Tick Loop

Server runs at fixed 60 TPS (ticks per second):

```rust
#[server_only]
pub async fn server_tick_loop(mut state: ServerState) {
    const TICK_RATE: u32 = 60;
    const TICK_DURATION: Duration = Duration::from_millis(1000 / TICK_RATE as u64);

    let mut interval = tokio::time::interval(TICK_DURATION);

    loop {
        interval.tick().await;

        // Process inputs
        for (client_id, inputs) in state.pending_inputs.drain() {
            for input in inputs {
                if validate_input(&input, &state.world) {
                    apply_input(&mut state.world, client_id, input);
                }
            }
        }

        // Run game logic systems
        movement_system(&mut state.world, TICK_DURATION.as_secs_f32());
        physics_system(&mut state.world, TICK_DURATION.as_secs_f32());
        combat_system(&mut state.world);

        // Take snapshot
        state.tick += 1;
        let snapshot = WorldSnapshot::from_world(&state.world, state.tick);
        state.snapshot_history.add(snapshot);

        // Broadcast state to clients
        broadcast_state(&state).await;
    }
}
```

### Input Validation

Prevent cheating with server-side validation:

```rust
#[server_only]
fn validate_input(input: &PlayerInput, world: &World) -> bool {
    // Check magnitude
    let movement = Vec2::new(input.forward, input.right);
    if movement.length() > 1.0 {
        return false; // Speed hack
    }

    // Check cooldowns
    if input.fire {
        // Verify weapon cooldown
    }

    // Check physics constraints
    // ...

    true
}
```

**Status:** ⚪ Not implemented (Phase 2.7)

---

## Interest Management

### Spatial Grid

Partition world into grid cells for proximity queries:

```rust
pub struct SpatialGrid {
    cell_size: f32,
    cells: HashMap<IVec2, HashSet<Entity>>,
}

impl SpatialGrid {
    pub fn insert(&mut self, entity: Entity, position: Vec3) {
        let cell = self.world_to_cell(position);
        self.cells.entry(cell).or_default().insert(entity);
    }

    pub fn query_radius(&self, position: Vec3, radius: f32) -> Vec<Entity> {
        let cell = self.world_to_cell(position);
        let cell_radius = (radius / self.cell_size).ceil() as i32;

        let mut results = Vec::new();
        for x in -cell_radius..=cell_radius {
            for y in -cell_radius..=cell_radius {
                let check_cell = cell + IVec2::new(x, y);
                if let Some(entities) = self.cells.get(&check_cell) {
                    results.extend(entities);
                }
            }
        }
        results
    }
}
```

### Per-Client Visibility

Track what each client can see:

```rust
pub struct ClientVisibility {
    pub client_id: u32,
    pub visible_entities: HashSet<Entity>,
}

pub fn update_visibility(
    state: &mut ServerState,
    client_id: u32,
) {
    let player_entity = state.clients[&client_id].entity;
    let player_pos = state.world.get::<Transform>(player_entity)
        .unwrap()
        .position;

    const VISIBILITY_RADIUS: f32 = 100.0;
    let visible = state.spatial_grid.query_radius(player_pos, VISIBILITY_RADIUS);

    state.client_visibility.get_mut(&client_id)
        .unwrap()
        .visible_entities = visible.into_iter().collect();
}
```

### Filtered State Updates

Only send visible entities to each client:

```rust
pub fn create_filtered_update(
    state: &ServerState,
    client_id: u32,
) -> StateUpdate {
    let visibility = &state.client_visibility[&client_id];
    let full_state = state.snapshot_history.latest();

    // Filter entities
    let filtered_entities: Vec<_> = full_state.entities
        .iter()
        .filter(|e| visibility.visible_entities.contains(&e.entity))
        .cloned()
        .collect();

    StateUpdate {
        tick: full_state.tick,
        entities: filtered_entities,
    }
}
```

**Performance Target:** < 1ms per client

**Status:** ⚪ Not implemented (Phase 2.8)

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Server tick time | < 16ms (60 TPS) | < 33ms |
| Network latency overhead | < 5ms | < 10ms |
| State update size | < 5KB (delta) | < 50KB (full) |
| Bandwidth per client | < 100KB/s | < 500KB/s |
| Interest management | < 1ms per client | < 5ms |

---

## Testing

### Property Tests

Verify client/server consistency:

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_delta_equals_full_state(
            inputs in prop::collection::vec(any::<PlayerInput>(), 0..100)
        ) {
            let mut server_world = World::new();
            let mut client_world = World::new();

            // Apply inputs on server
            for input in &inputs {
                apply_input(&mut server_world, input.clone());
            }

            // Delta update
            let delta = create_delta_update(&server_world);
            apply_delta(&mut client_world, delta);

            // Should match full state
            assert_eq!(serialize_world(&server_world), serialize_world(&client_world));
        }
    }
}
```

### Integration Tests

Test full client-server interaction:

```rust
#[tokio::test]
async fn test_client_server_connection() {
    let server = TcpServer::bind("127.0.0.1:7777").await.unwrap();
    let client = TcpClient::connect("127.0.0.1:7777").await.unwrap();

    // Login
    client.send(ClientMessage::Login {
        username: "player1".into(),
        password: "pass".into(),
    }).await.unwrap();

    let response = server.recv().await.unwrap();
    assert!(matches!(response, ServerMessage::LoginResponse { success: true, .. }));
}
```

---

## Observability

### Metrics

Track network performance:

```rust
pub struct NetworkMetrics {
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub packets_lost: AtomicU64,
    pub rtt_ms: AtomicU32, // Round-trip time
}
```

### Prometheus Export

```
# HELP network_bytes_sent Total bytes sent
# TYPE network_bytes_sent counter
network_bytes_sent 1234567

# HELP network_rtt_ms Round-trip time in milliseconds
# TYPE network_rtt_ms gauge
network_rtt_ms 45
```

**Status:** 🟡 Framework ready (engine/observability/), endpoint not exposed

---

## References

- **Implementation:** `engine/networking/src/` (placeholder)
- **Macros:** `engine/macros/src/client_server.rs` ✅ Complete
- **Observability:** `engine/observability/src/` ✅ Complete
- **Binaries:** `engine/binaries/client/`, `engine/binaries/server/` (stubs)

**Related Documentation:**
- [ECS](ecs.md)
- [Profiling](profiling.md)
- [Error Handling](error-handling.md)
