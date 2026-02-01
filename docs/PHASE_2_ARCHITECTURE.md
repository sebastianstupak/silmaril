# Phase 2: Networking & Client/Server - Architecture & Decisions

**Date:** 2026-02-01
**Status:** 📋 Planned - Ready for Implementation
**Total Estimated Time:** 4-5 weeks

---

## 📚 **Document Index**

### **Planning Documents** (Read These First)
1. **[ROADMAP.md](../ROADMAP.md)** - Phase 2 overview and timeline
2. **THIS FILE** - Architecture decisions and design patterns
3. **[docs/tasks/phase2-foundation.md](tasks/phase2-foundation.md)** - Part 1: Infrastructure & macros

### **Task Files** (Detailed Implementation Guides)
- **[phase2-foundation.md](tasks/phase2-foundation.md)** - Macros, build system, Docker, metrics
- **[phase2-network-protocol.md](tasks/phase2-network-protocol.md)** - FlatBuffers, message definitions
- **[phase2-tcp-connection.md](tasks/phase2-tcp-connection.md)** - TCP reliable channel
- **[phase2-udp-packets.md](tasks/phase2-udp-packets.md)** - UDP fast channel
- **[phase2-state-sync.md](tasks/phase2-state-sync.md)** - Full state + delta compression
- **[phase2-client-prediction.md](tasks/phase2-client-prediction.md)** - Client-side prediction
- **[phase2-server-tick.md](tasks/phase2-server-tick.md)** - Server authoritative logic
- **[phase2-interest-basic.md](tasks/phase2-interest-basic.md)** - Basic interest management

---

## 🎯 **Architecture Decisions Summary**

This document captures all architectural decisions made during Phase 2 planning. These decisions are **final** and should be followed during implementation.

---

## 1. Feature Flags & Code Splitting

### **Decision:** Flexible Multi-Pattern System

**Patterns:**
```rust
#[client_only]     // Only compiled in client builds
#[server_only]     // Only compiled in server builds
#[shared]          // Compiled in both
#[server_authoritative]  // Different implementations for client/server
```

**Rationale:**
- Not all code is purely client OR server
- Some systems need prediction (client) + validation (server)
- Server-authoritative pattern allows different implementations with same signature

**Example Use Cases:**

| Pattern | Use Case | Example |
|---------|----------|---------|
| `#[client_only]` | Graphics, audio, input | `render_health_bar()` |
| `#[server_only]` | Anti-cheat, loot tables | `validate_damage()` |
| `#[shared]` | Utility, physics | `calculate_distance()` |
| `#[server_authoritative]` | Gameplay with prediction | `apply_damage()` |

**Feature Flags:**
```toml
[features]
client = ["rendering", "audio", "input"]
server = ["headless", "admin-tools"]
networking = ["tokio", "quinn"]
all = ["client", "server"]  # For testing
```

**Build Commands:**
```bash
cargo build --bin client     # Client only
cargo build --bin server     # Server only
cargo build --all-features   # Both (testing)
```

---

## 2. Module Structure (Future Phases)

### **Decision:** Three Separate Optional Modules

#### **A. `engine/persistence`** (Phase 3-4)

**Responsibility:** Data storage and retrieval

**Contents:**
- Database backends (SQLite, PostgreSQL)
- Cache backends (Redis, in-memory)
- Session management
- Player data schemas
- World state persistence

**When to implement:**
- Small game (<100 CCU): Phase 4 (SQLite only)
- Medium game (100-1K CCU): Phase 3 (Redis + PostgreSQL)
- Large game (MMORPG): Phase 3 (full implementation)

#### **B. `engine/infrastructure`** (Phase 3-4)

**Responsibility:** Cross-cutting services

**Contents:**
- Configuration management (YAML, env vars, remote config)
- Service discovery (Consul, etcd, static)
- Secret management (Vault, AWS Secrets)
- Health checks & readiness probes
- Distributed tracing (OpenTelemetry)
- Event messaging (pub/sub, Kafka, NATS)
- Feature flags

**When to implement:**
- Single server: Phase 4 (basic config only)
- Multi-server: Phase 3 (service discovery needed)
- Production: Phase 3 (health checks, secrets)

#### **C. `engine/scaling`** (Phase 4-5)

**Responsibility:** Horizontal scaling

**Contents:**
- Sharding (spatial, consistent hash)
- Load balancing (round-robin, least-loaded, geo-aware)
- Replication (state sync, consensus)
- Player migration (cross-server transfer)
- Dynamic scaling (add/remove servers)
- Capacity planning

**When to implement:**
- <1K CCU: Never (single server sufficient)
- 1K-10K CCU: Phase 4 (basic sharding)
- 10K+ CCU (MMORPG): Phase 3 (full implementation)

**Key Principle:** These modules are **independent** and **optional**. Games can use networking without persistence/scaling.

---

## 3. Validation Approach for Server-Authoritative Code

### **Decision:** Hybrid (Shared Core + Property Tests)

**Pattern:**
```rust
// Shared core logic
#[shared]
fn calculate_damage_core(base: f32, modifiers: &Modifiers) -> f32 {
    base * modifiers.total_multiplier()
}

// Server-authoritative implementations
#[cfg(feature = "server")]
fn calculate_damage(attacker: &Stats, target: &Stats) -> f32 {
    let modifiers = Modifiers {
        strength: attacker.strength,
        crit: roll_critical(attacker.crit_chance),
        armor: calculate_armor_reduction(target.armor),
        resistance: target.get_resistance(weapon.damage_type),
    };
    calculate_damage_core(weapon.base_damage, &modifiers)
}

#[cfg(feature = "client")]
fn calculate_damage(attacker: &Stats, target: &Stats) -> f32 {
    let modifiers = Modifiers {
        strength: attacker.strength,
        crit: 1.0,    // Assume no crit
        armor: 0.8,   // Estimate 20% reduction
        resistance: 1.0,
    };
    calculate_damage_core(weapon.base_damage, &modifiers)
}
```

**Property-Based Validation:**
```rust
proptest! {
    #[test]
    fn client_server_damage_parity(/* random inputs */) {
        let client_result = calculate_damage_client(...);
        let server_result = calculate_damage_server(...);

        // Properties:
        prop_assert!(server_result >= 0.0);
        prop_assert!((client_result - server_result).abs() / server_result < 0.5);
    }
}
```

**Runtime Metrics:**
```rust
metrics::histogram!("prediction.damage.error_percent", diff_percent);
if diff_percent > 0.5 {
    warn!("Large prediction error - possible cheat");
}
```

**Rationale:**
- Shared core reduces duplication
- Property tests catch divergence automatically
- Runtime metrics help AI debugging
- CI fails if implementations drift apart

---

## 4. Testing Strategy

### **Decision:** Comprehensive AAA-Quality Testing

**Test Pyramid:**

```
           /\
          /  \    E2E Tests (10%)
         /----\   Integration Tests (30%)
        /------\  Property Tests (20%)
       /--------\ Unit Tests (40%)
```

**Requirements:**
- >80% code coverage
- All critical paths have property tests
- All network operations benchmarked (<1ms target)
- Integration tests for client/server interaction

**Test Types:**

| Type | Purpose | Example |
|------|---------|---------|
| **Unit** | Individual functions | Test damage calculation |
| **Property** | Invariants hold | Roundtrip serialization |
| **Integration** | Components work together | Client connects to server |
| **Benchmark** | Performance targets | Message serialization <1µs |
| **E2E** | Full system | 2 clients play together |

**Property Test Focus:**
- Message serialization roundtrips
- Client/server prediction parity
- State sync correctness (delta == full)
- Network packet ordering

---

## 5. Metrics & Monitoring

### **Decision:** Comprehensive Metrics from Day One (Optional)

**Built-in Metrics:**

```rust
// Configuration
metrics:
  enabled: true  # Can disable for benchmarking
  prometheus:
    port: 8080
  sample_rate: 1.0  # 100% (can reduce if overhead too high)
```

**Core Metrics Tracked:**

| Category | Metrics | Purpose |
|----------|---------|---------|
| **Server Health** | TPS, memory, CPU | Overall server status |
| **Performance** | Tick duration, per-system timing | Find bottlenecks |
| **Network** | Latency, bandwidth, packet loss | Network health |
| **Errors** | Error counts, rates, types | Catch issues |
| **Queries** | Query duration, match counts | ECS optimization |
| **Memory** | Component memory, allocations | Memory leaks |
| **Predictions** | Client/server sync quality | UX quality |

**Why Comprehensive:**
- AI can only debug what it can see
- Can't predict which metrics needed
- Overhead is ~1-2% (acceptable for AAA)
- Can disable specific metrics if needed

**Prometheus Endpoint:**
```bash
curl http://localhost:8080/metrics

# server_tps 59.8
# player_count 5
# entity_count 1234
# tick_duration_ms_bucket{le="16"} 950
# network_latency_ms{client="42"} 45
```

---

## 6. Admin Console

### **Decision:** Basic Console Now, Robust Later

**Phase 2:** Basic Telnet Console
- Localhost only (127.0.0.1)
- No authentication (dev builds only)
- Text-based commands
- TODO: Add auth in Phase 4

**Commands:**
```
> status
Server: OK, TPS: 59.8, Players: 5/100

> spawn entity Goblin 100 200 300
Spawned: Entity(12345) at (100, 200, 300)

> kick player "Cheater123"
Player kicked

> help
Available commands: status, spawn, kick, players, set, help
```

**Phase 4:** Robust Console
- Web-based dashboard (React/Svelte)
- Authentication (token-based)
- Real-time graphs
- Entity inspector
- Log streaming

**Rationale:**
- Basic console sufficient for Phase 2 debugging
- Web dashboard requires significant effort
- Focus on core networking first

---

## 7. Container Infrastructure

### **Decision:** Docker from Day One

**Development:**
```yaml
# docker-compose.dev.yml
services:
  server:
    build: ./docker/Dockerfile.server.dev
    ports:
      - "7777:7777"  # Game
      - "8080:8080"  # Metrics
    volumes:
      - ./:/workspace  # Hot-reload
    command: cargo watch -x 'run --bin server'

  client:
    build: ./docker/Dockerfile.client.dev
    environment:
      - SERVER_ADDRESS=server:7777
    depends_on:
      - server

  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
```

**Production:**
```dockerfile
# Multi-stage build
FROM rust:1.75 as builder
COPY . .
RUN cargo build --bin server --profile release-server

FROM debian:bookworm-slim
COPY --from=builder /build/target/release-server/server /app/server
EXPOSE 7777 8080
CMD ["/app/server"]
```

**Targets:**
- Production image: <50MB (stripped, optimized)
- Hot-reload: <3s for code changes
- Build cache: Efficient dependency caching

**Rationale:**
- Docker is industry standard
- Ensures consistent environment
- Simplifies deployment
- Enables easy multi-server testing

---

## 8. Network Protocol Design

### **Decision:** FlatBuffers for Zero-Copy Performance

**Why FlatBuffers?**
- Zero-copy deserialization (vs Bincode)
- Schema evolution support
- Cross-language compatibility (future: C#, Python clients)
- Battle-tested (used by Google, Facebook)

**Message Categories:**
```rust
// Client → Server (every frame)
- Input { sequence, forward, right, jump, attack }

// Server → Client (60 TPS)
- FullState { tick, entities[] }
- DeltaState { tick, base_tick, changes[] }

// Connection
- Connect, Disconnect, ConnectionAccepted, ConnectionRejected
```

**Framing:**
```
[4 bytes: length][N bytes: FlatBuffer data]
```

**Versioning:**
```rust
const PROTOCOL_VERSION: (u32, u32, u32) = (1, 0, 0);

// Major version must match
// Minor version client <= server
// Patch version doesn't affect compatibility
```

---

## 9. TCP + UDP Dual Channel

### **Decision:** TCP for Reliable, UDP for Fast

| Channel | Use Case | Delivery | Order | Examples |
|---------|----------|----------|-------|----------|
| **TCP** | Critical data | Guaranteed | In-order | Chat, inventory, state sync |
| **UDP** | Fast updates | Best-effort | Unordered | Position, input, cosmetics |

**Architecture:**
```rust
pub struct NetworkClient {
    tcp: TcpStream,      // Reliable channel
    udp: UdpSocket,      // Fast channel
    server_addr: SocketAddr,
}

impl NetworkClient {
    pub async fn send_reliable(&mut self, msg: ClientMessage) {
        // Send over TCP
    }

    pub async fn send_unreliable(&mut self, msg: ClientMessage) {
        // Send over UDP
    }
}
```

**Why Both?**
- TCP: Automatic retransmission, ordering (but high latency)
- UDP: Low latency (but no guarantees)
- Industry standard (used by Source Engine, Unreal, Unity)

---

## 10. State Synchronization

### **Decision:** Adaptive Full + Delta

**Strategy:**
```rust
// Server tracks last ack from each client
client.last_acked_tick: u32

// Send delta if client has recent state
if current_tick - client.last_acked_tick < 30 {
    send_delta(base: client.last_acked_tick, current: current_tick)
} else {
    send_full_state(current_tick)
}
```

**Delta Encoding:**
```rust
pub struct StateDelta {
    base_tick: u32,
    target_tick: u32,
    changes: Vec<EntityChange>,
}

pub enum EntityChange {
    Added { entity: Entity, components: Vec<Component> },
    Modified { entity: Entity, component: Component },
    Removed { entity: Entity },
}
```

**Benefits:**
- Full state: Simple, always works
- Delta: 80-90% bandwidth reduction
- Adaptive: Falls back if client too far behind

**Property Test:**
```rust
proptest! {
    #[test]
    fn delta_produces_same_result_as_full(/* random world states */) {
        let result_full = apply_full_state(target_state);
        let result_delta = apply_delta(base_state, delta);

        prop_assert_eq!(result_full, result_delta);
    }
}
```

---

## 11. Client-Side Prediction

### **Decision:** Predict + Reconcile Pattern

**Flow:**
```
Client Frame N:
1. Get input
2. Assign sequence number
3. Apply input locally (prediction)
4. Send input to server

Client Frame N+5 (server response arrives):
5. Receive server state for tick N
6. Compare server state to predicted state
7. If mismatch:
   - Snap to server state
   - Replay inputs N+1 to N+5
8. Continue from corrected state
```

**Implementation:**
```rust
pub struct PredictionSystem {
    // Buffer of unconfirmed inputs
    pending_inputs: VecDeque<(u32, PlayerInput)>,

    // Last confirmed server state
    last_server_tick: u32,
}

impl PredictionSystem {
    pub fn reconcile(&mut self, server_state: &WorldState, server_tick: u32) {
        // 1. Snap to server state
        self.world = server_state.clone();

        // 2. Replay all inputs since server tick
        for (seq, input) in &self.pending_inputs {
            if *seq > server_tick {
                apply_input(&mut self.world, input);
            }
        }

        // 3. Remove confirmed inputs
        self.pending_inputs.retain(|(seq, _)| *seq > server_tick);
    }
}
```

**Target:** <10% prediction error rate

---

## 12. Performance Targets

### **Network Operations**
- Message serialization: <1µs
- Message deserialization: <500ns
- Framing overhead: <100ns
- TCP send: <100µs
- UDP send: <50µs

### **Server Tick**
- Total tick time: <16ms (60 TPS)
- Physics: <8ms
- Queries: <3ms
- Networking: <3ms
- Other: <2ms

### **Client**
- Frame time: <16.67ms (60 FPS)
- Network update: <1ms
- Prediction reconciliation: <500µs

### **Bandwidth** (per client, 60 TPS)
- Outgoing (client → server): <5 KB/s
- Incoming (server → client): <20 KB/s
- Peak (full state): <50 KB/s

### **Latency**
- Server overhead: <5ms
- Total round-trip: <50ms (on good connection)

---

## 🚀 **Implementation Order**

### **Week 1: Foundation** (Phase 2.1)
1. Proc macros (#[client_only], etc.)
2. Build infrastructure (separate binaries)
3. Docker development environment
4. Metrics & admin console

**Deliverable:** `./scripts/dev.sh` starts working environment

### **Week 2: Protocol** (Phase 2.2-2.3)
1. FlatBuffers schema
2. Message serialization
3. TCP connection
4. Handshake + versioning

**Deliverable:** Client connects to server, exchanges messages

### **Week 3: Channels & Sync** (Phase 2.4-2.5)
1. UDP channel
2. Full state sync
3. Delta compression
4. Acknowledgment system

**Deliverable:** World state syncs from server to client

### **Week 4: Prediction & Polish** (Phase 2.6-2.8)
1. Client-side prediction
2. Input reconciliation
3. Server tick loop
4. Basic interest management

**Deliverable:** Playable multiplayer demo (2+ clients)

### **Week 5: Testing & Optimization**
1. Comprehensive test suite
2. Benchmarking
3. Bug fixes
4. Documentation

**Deliverable:** Production-ready networking

---

## ✅ **Success Criteria (Phase 2 Complete)**

### **Functionality**
- [ ] Client connects to server reliably
- [ ] TCP + UDP channels working
- [ ] State syncs from server to client
- [ ] Client prediction working (<10% error)
- [ ] 2+ clients can play together
- [ ] Interest management culls distant entities

### **Performance**
- [ ] Server maintains 60 TPS with 10 clients
- [ ] Network latency overhead <5ms
- [ ] Bandwidth <20 KB/s per client
- [ ] All benchmarks meet targets

### **Quality**
- [ ] >80% test coverage
- [ ] All property tests pass
- [ ] CI green on all platforms
- [ ] Zero clippy warnings

### **Infrastructure**
- [ ] `./scripts/dev.sh` works
- [ ] Production Docker images <50MB
- [ ] Metrics endpoint functional
- [ ] Admin console operational

### **Documentation**
- [ ] All task files complete
- [ ] API docs for public types
- [ ] Examples working
- [ ] Architecture decisions recorded

---

## 📚 **References**

### **Industry Standards**
- **Source Engine:** TCP + UDP, client prediction
- **Unreal:** Replication, delta compression
- **VALORANT:** 128 tick server, <2% server CPU for interest management

### **Technical Resources**
- [Gaffer on Games - Networked Physics](https://gafferongames.com)
- [Valve Developer Wiki - Source Multiplayer](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)
- [Fast-Paced Multiplayer](https://www.gabrielgambetta.com/client-server-game-architecture.html)

### **FlatBuffers**
- [FlatBuffers Documentation](https://google.github.io/flatbuffers/)
- [FlatBuffers vs Protobuf](https://google.github.io/flatbuffers/flatbuffers_benchmarks.html)

---

**Last Updated:** 2026-02-01
**Status:** Ready for Implementation
**Next Action:** Start Phase 2.1 (Foundation)
