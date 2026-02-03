# Phase 2 End-to-End Test Plan

> **Purpose:** Comprehensive testing strategy to validate Phase 2 networking completion
> **Target:** Reach 100% Phase 2 completion with confidence
> **Status:** Planning Document - Ready for Implementation

---

## 🎯 **Test Objectives**

Validate that the networking system works end-to-end with:
- Multiple clients connecting to a server
- Real-time state synchronization
- Client-side prediction and reconciliation
- Network resilience (lag, packet loss, disconnects)
- Performance under load (100+ concurrent clients)

---

## 📊 **Test Pyramid**

```
                    /\
                   /  \
                  /E2E \         ← 10 tests (this document)
                 /------\
                / Integr.\       ← 50+ tests (already exist)
               /----------\
              /  Unit Tests \    ← 419+ tests (already exist)
             /--------------\
```

**Focus:** The E2E layer (missing ~10 critical tests)

---

## 🧪 **Test Categories**

### **Category 1: Basic Connectivity** (3 tests)

#### **Test 1.1: Single Client Connection**
**Scenario:** One client connects to server and disconnects cleanly

**Steps:**
1. Start server on port 7777
2. Start client, connect to localhost:7777
3. Verify TCP connection established
4. Verify UDP socket bound
5. Verify client receives welcome message
6. Disconnect client
7. Verify server detects disconnect

**Success Criteria:**
- Connection established within 1 second
- No error messages in logs
- Clean disconnect without panics

**Implementation:**
```rust
#[test]
fn test_single_client_connection() {
    let server = spawn_server("127.0.0.1:7777");
    let client = spawn_client("127.0.0.1:7777");

    assert!(client.is_connected());
    assert_eq!(server.client_count(), 1);

    client.disconnect();
    sleep(100ms);

    assert_eq!(server.client_count(), 0);
}
```

---

#### **Test 1.2: Multiple Client Connection**
**Scenario:** 10 clients connect simultaneously

**Steps:**
1. Start server
2. Spawn 10 clients in parallel
3. Verify all connections established
4. Verify server tracks all 10 clients
5. Disconnect all clients
6. Verify server cleans up properly

**Success Criteria:**
- All 10 clients connect within 2 seconds
- Server memory stable after disconnects
- No connection refused errors

**Implementation:**
```rust
#[test]
fn test_multiple_client_connection() {
    let server = spawn_server("127.0.0.1:7777");
    let clients: Vec<_> = (0..10)
        .map(|_| spawn_client("127.0.0.1:7777"))
        .collect();

    sleep(1000ms);
    assert_eq!(server.client_count(), 10);

    for client in clients {
        client.disconnect();
    }

    sleep(500ms);
    assert_eq!(server.client_count(), 0);
}
```

---

#### **Test 1.3: Reconnection After Disconnect**
**Scenario:** Client disconnects and reconnects

**Steps:**
1. Start server
2. Client connects
3. Client disconnects (network error simulated)
4. Client reconnects with same session ID
5. Verify state restored

**Success Criteria:**
- Reconnection successful within 2 seconds
- Session state preserved
- No duplicate entities

---

### **Category 2: State Synchronization** (3 tests)

#### **Test 2.1: Entity Spawn Synchronization**
**Scenario:** Server spawns entity, all clients see it

**Steps:**
1. Start server + 3 clients
2. Server spawns entity at position (10, 0, 10)
3. Wait 100ms for network propagation
4. Verify all 3 clients have the entity
5. Verify entity position matches on all clients

**Success Criteria:**
- Entity visible to all clients within 100ms
- Position accuracy within 0.01 units
- Component data identical across clients

**Implementation:**
```rust
#[test]
fn test_entity_spawn_sync() {
    let server = spawn_server("127.0.0.1:7777");
    let clients = spawn_n_clients(3, "127.0.0.1:7777");

    let entity_id = server.spawn_entity(Transform::new(10.0, 0.0, 10.0));
    sleep(100ms);

    for client in &clients {
        assert!(client.has_entity(entity_id));
        let pos = client.get_entity_position(entity_id);
        assert_approx_eq!(pos, Vec3::new(10.0, 0.0, 10.0), 0.01);
    }
}
```

---

#### **Test 2.2: Player Movement Synchronization**
**Scenario:** One player moves, other players see movement

**Steps:**
1. Start server + 2 clients (Alice, Bob)
2. Alice sends movement input (forward)
3. Server processes movement
4. Verify Bob sees Alice's new position
5. Verify position updates at 20Hz

**Success Criteria:**
- Bob sees Alice's movement within 100ms
- Position updates smooth (no teleporting)
- Update rate: 15-25 updates/second

**Implementation:**
```rust
#[test]
fn test_player_movement_sync() {
    let server = spawn_server("127.0.0.1:7777");
    let alice = spawn_client("127.0.0.1:7777");
    let bob = spawn_client("127.0.0.1:7777");

    let alice_id = alice.player_entity_id();

    // Alice moves forward for 1 second
    alice.send_input(InputAction::MoveForward);
    sleep(1000ms);
    alice.send_input(InputAction::None);

    // Verify Bob sees Alice's movement
    let alice_pos_from_bob = bob.get_entity_position(alice_id);
    assert!(alice_pos_from_bob.z > 5.0); // Moved forward
}
```

---

#### **Test 2.3: Delta Synchronization Under Load**
**Scenario:** 50 entities moving simultaneously, deltas compress efficiently

**Steps:**
1. Start server + 1 client
2. Spawn 50 moving entities
3. Track network bandwidth for 10 seconds
4. Verify delta compression active
5. Verify bandwidth < 50 KB/s

**Success Criteria:**
- All 50 entities synchronized correctly
- Bandwidth reduction: >50% vs full snapshots
- No missed updates (all entities moving)

---

### **Category 3: Client-Side Prediction** (2 tests)

#### **Test 3.1: Local Player Prediction**
**Scenario:** Player input feels responsive despite 100ms latency

**Steps:**
1. Start server + client with 100ms artificial delay
2. Client sends movement input
3. Verify immediate local movement (prediction)
4. Wait for server confirmation
5. Verify no visible "rubber-banding"

**Success Criteria:**
- Local movement appears within 16ms (1 frame)
- Server reconciliation smooth (< 0.1 unit correction)
- No visual stuttering

**Implementation:**
```rust
#[test]
fn test_client_prediction_responsiveness() {
    let server = spawn_server("127.0.0.1:7777");
    let client = spawn_client_with_latency("127.0.0.1:7777", 100ms);

    let initial_pos = client.local_player_position();

    client.send_input(InputAction::MoveForward);
    sleep(16ms); // 1 frame

    let predicted_pos = client.local_player_position();
    assert!(predicted_pos.z > initial_pos.z); // Moved immediately

    sleep(200ms); // Wait for server response

    let final_pos = client.local_player_position();
    assert_approx_eq!(predicted_pos, final_pos, 0.1); // Minimal correction
}
```

---

#### **Test 3.2: Prediction Error Correction**
**Scenario:** Server corrects client prediction error smoothly

**Steps:**
1. Start server + client
2. Client predicts movement (hitting a wall)
3. Server rejects movement (collision)
4. Verify client smoothly corrects position
5. Verify no jarring teleportation

**Success Criteria:**
- Correction animation < 200ms
- Position converges to server state
- No repeated corrections (stable)

---

### **Category 4: Network Resilience** (2 tests)

#### **Test 4.1: Packet Loss Tolerance**
**Scenario:** 10% UDP packet loss, game remains playable

**Steps:**
1. Start server + client with 10% packet loss simulator
2. Player moves continuously
3. Verify movement still synchronized
4. Verify no visual glitches
5. Measure update rate degradation

**Success Criteria:**
- Game remains playable (subjective: smooth motion)
- Update rate: >10 Hz (down from 20 Hz is acceptable)
- Latency increase: <20%

**Implementation:**
```rust
#[test]
fn test_packet_loss_tolerance() {
    let server = spawn_server("127.0.0.1:7777");
    let client = spawn_client_with_packet_loss("127.0.0.1:7777", 0.1);

    // Move for 5 seconds
    for _ in 0..5 {
        client.send_input(InputAction::MoveForward);
        sleep(1000ms);
    }

    let final_pos = client.local_player_position();
    assert!(final_pos.z > 20.0); // Should have moved ~5 units/sec

    let update_rate = client.get_update_rate_hz();
    assert!(update_rate >= 10.0);
}
```

---

#### **Test 4.2: High Latency Compensation**
**Scenario:** 300ms latency, lag compensation keeps game playable

**Steps:**
1. Start server + client with 300ms latency
2. Test various actions (move, shoot, interact)
3. Verify prediction maintains responsiveness
4. Verify server reconciliation works
5. Measure perceived responsiveness

**Success Criteria:**
- Local actions feel instant (<50ms)
- Server corrections smooth
- Gameplay still functional (not unplayable)

---

## 📈 **Performance Tests**

### **Test P.1: 100 Concurrent Clients**
**Scenario:** Server handles 100 connected clients

**Steps:**
1. Start server
2. Spawn 100 clients sequentially
3. All clients send movement input
4. Monitor server performance

**Success Criteria:**
- Server tick rate: 58-60 TPS (target: 60 TPS)
- Server CPU: <80% on single core
- Server memory: <500 MB
- All clients receive updates

---

### **Test P.2: 1000 Entity Synchronization**
**Scenario:** 1000 moving entities across 10 clients

**Steps:**
1. Start server + 10 clients
2. Spawn 1000 entities with velocities
3. Run for 60 seconds
4. Verify all entities synchronized

**Success Criteria:**
- Server tick rate stable: 58-60 TPS
- Client frame rate: >30 FPS
- Network bandwidth per client: <100 KB/s
- No memory leaks (memory stable over time)

---

## 🔧 **Test Infrastructure**

### **Test Harness Components**

#### **1. Server Spawner**
```rust
fn spawn_server(addr: &str) -> TestServer {
    TestServer::new()
        .bind(addr)
        .tick_rate(60)
        .spawn_background()
}
```

#### **2. Client Spawner**
```rust
fn spawn_client(server_addr: &str) -> TestClient {
    TestClient::new()
        .connect(server_addr)
        .wait_for_connection(2000ms)
}

fn spawn_client_with_latency(server_addr: &str, latency: Duration) -> TestClient {
    TestClient::new()
        .connect(server_addr)
        .simulate_latency(latency)
        .wait_for_connection(2000ms)
}
```

#### **3. Network Simulator**
```rust
struct NetworkSimulator {
    latency: Duration,
    jitter: Duration,
    packet_loss: f32,
}

impl NetworkSimulator {
    fn apply_to_client(&self, client: &mut TestClient);
}
```

#### **4. Assertion Helpers**
```rust
fn assert_approx_eq!(a: Vec3, b: Vec3, epsilon: f32);
fn assert_within_duration!(action: impl Fn(), max_duration: Duration);
fn assert_eventually!(condition: impl Fn() -> bool, timeout: Duration);
```

---

## 📁 **Test File Organization**

```
engine/shared/tests/e2e/
├── mod.rs                          # Test harness
├── connectivity_test.rs            # Category 1 (3 tests)
├── state_sync_test.rs              # Category 2 (3 tests)
├── prediction_test.rs              # Category 3 (2 tests)
├── resilience_test.rs              # Category 4 (2 tests)
├── performance_test.rs             # Performance (2 tests)
└── helpers/
    ├── server_spawner.rs
    ├── client_spawner.rs
    └── network_simulator.rs
```

---

## ⏱️ **Test Execution Times**

| Test Category | Count | Time Each | Total Time |
|---------------|-------|-----------|------------|
| Connectivity | 3 | 2-5s | ~10s |
| State Sync | 3 | 2-5s | ~10s |
| Prediction | 2 | 3-8s | ~15s |
| Resilience | 2 | 5-10s | ~15s |
| Performance | 2 | 30-60s | ~90s |
| **TOTAL** | **12** | - | **~140s (2.3 min)** |

---

## 🎯 **Success Metrics**

**To consider Phase 2 E2E testing COMPLETE:**

- [ ] All 10 functional tests passing
- [ ] All 2 performance tests passing
- [ ] Tests run in CI on all platforms
- [ ] Tests documented and maintainable
- [ ] Test failures are actionable (clear error messages)

**Confidence Level After Tests:**
- ✅ Networking works end-to-end
- ✅ System handles real-world conditions
- ✅ Performance targets met
- ✅ Ready for production use

---

## 📝 **Implementation Checklist**

### **Phase 1: Test Infrastructure (2 days)**
- [ ] Create test harness (`TestServer`, `TestClient`)
- [ ] Implement network simulator (latency, packet loss)
- [ ] Add assertion helpers
- [ ] Write example test (connectivity test 1.1)

### **Phase 2: Connectivity Tests (1 day)**
- [ ] Test 1.1: Single client connection
- [ ] Test 1.2: Multiple clients
- [ ] Test 1.3: Reconnection

### **Phase 3: State Sync Tests (1 day)**
- [ ] Test 2.1: Entity spawn sync
- [ ] Test 2.2: Player movement sync
- [ ] Test 2.3: Delta synchronization

### **Phase 4: Prediction Tests (1 day)**
- [ ] Test 3.1: Local prediction
- [ ] Test 3.2: Error correction

### **Phase 5: Resilience Tests (1 day)**
- [ ] Test 4.1: Packet loss tolerance
- [ ] Test 4.2: High latency compensation

### **Phase 6: Performance Tests (2 days)**
- [ ] Test P.1: 100 concurrent clients
- [ ] Test P.2: 1000 entity sync

### **Phase 7: CI Integration (1 day)**
- [ ] Add E2E tests to GitHub Actions
- [ ] Configure timeout (5 minutes)
- [ ] Add to PR checks

**Total Estimate: 9-10 days**

---

## 🚀 **Next Steps**

1. **Review this plan** with team
2. **Implement test harness** (highest priority)
3. **Write first test** (1.1 Single Client Connection)
4. **Iterate**: Add tests one category at a time
5. **Integrate into CI** once stable

---

**Status:** ✅ Planning Complete - Ready for Implementation
**Author:** Claude Sonnet 4.5
**Date:** 2026-02-03
