# Phase 2 Quick Wins Implementation Guide

> **Purpose:** 5 high-impact, low-effort improvements to boost Phase 2 from 75% → 85%
> **Time:** ~1 day total (8 hours)
> **Difficulty:** Low to Medium

---

## 🎯 **Quick Wins Overview**

These 5 tasks provide immediate value with minimal risk:

| # | Task | Impact | Time | Difficulty |
|---|------|--------|------|------------|
| 1 | Uncomment game loops | High | 30 min | Easy |
| 2 | Add Prometheus endpoint | Medium | 2 hours | Easy |
| 3 | Write 1 basic E2E test | High | 4 hours | Medium |
| 4 | Add protocol version check | Medium | 2 hours | Easy |
| 5 | Add connection timeout | Medium | 3 hours | Easy |

**Total:** ~8 hours → **+10% Phase 2 completion**

---

## ✅ **QUICK WIN #1: Uncomment Game Loops**

**Current State:** Client and server binaries have stubbed/commented game loops

**Goal:** Enable basic game loops so binaries actually run game logic

**Impact:**
- ✅ Client can connect and render
- ✅ Server can accept connections and tick
- ✅ Foundation for all other work

**Time:** 30 minutes

---

### **Implementation Steps**

#### **Step 1.1: Update Client Main Loop** (15 min)

**File:** `engine/binaries/client/src/main.rs`

**Changes:**
```rust
// BEFORE (stubbed):
fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Client starting...");
    // TODO: Implement game loop
    Ok(())
}

// AFTER (basic loop):
fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Client starting...");

    // Initialize subsystems
    let event_loop = EventLoop::new()?;
    let mut world = World::new();
    let mut renderer = Renderer::new(&event_loop)?;

    // Basic game loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                info!("Closing client");
                elwt.exit();
            }

            Event::AboutToWait => {
                // Update world (stubbed for now)
                // Render
                renderer.begin_frame();
                // TODO: render_world(&renderer, &world);
                renderer.end_frame();
            }

            _ => {}
        }
    })?;

    Ok(())
}
```

**Test:**
```bash
cargo run --bin client
# Should open window (blank screen OK)
# Should close without panic
```

---

#### **Step 1.2: Update Server Main Loop** (15 min)

**File:** `engine/binaries/server/src/main.rs`

**Changes:**
```rust
// BEFORE (stubbed):
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Server starting...");
    // TODO: Implement tick loop
    Ok(())
}

// AFTER (basic tick loop):
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Server starting on 0.0.0.0:7777");

    let mut world = World::new();
    let mut server = Server::bind("0.0.0.0:7777").await?;

    let mut tick_count = 0u64;
    let target_tick_time = Duration::from_micros(16_667); // 60 TPS

    loop {
        let tick_start = Instant::now();

        // Poll network events (non-blocking)
        while let Some(event) = server.try_recv_event() {
            match event {
                NetworkEvent::ClientConnected { client_id, .. } => {
                    info!("Client {} connected", client_id);
                }
                NetworkEvent::ClientDisconnected { client_id } => {
                    info!("Client {} disconnected", client_id);
                }
                _ => {}
            }
        }

        // Update world (stubbed for now)
        // TODO: run_server_systems(&mut world);

        // Sleep for remaining tick time
        let tick_duration = tick_start.elapsed();
        if tick_duration < target_tick_time {
            tokio::time::sleep(target_tick_time - tick_duration).await;
        }

        tick_count += 1;

        // Log TPS every 60 ticks
        if tick_count % 60 == 0 {
            let tps = 1.0 / tick_duration.as_secs_f32();
            info!("Tick {}: {:.1} TPS, {} clients", tick_count, tps, server.client_count());
        }
    }
}
```

**Test:**
```bash
cargo run --bin server
# Should print "Server starting on 0.0.0.0:7777"
# Should print tick logs every second
# Should accept connections (test with telnet 127.0.0.1 7777)
```

---

## ✅ **QUICK WIN #2: Add Prometheus Metrics Endpoint**

**Current State:** Metrics framework exists but no HTTP endpoint

**Goal:** Expose server metrics via Prometheus endpoint for monitoring

**Impact:**
- ✅ Real-time server monitoring
- ✅ Performance tracking
- ✅ Production-ready observability

**Time:** 2 hours

---

### **Implementation Steps**

#### **Step 2.1: Add Dependencies** (5 min)

**File:** `engine/binaries/server/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
prometheus = "0.13"
axum = "0.7"  # Lightweight HTTP server
tokio = { version = "1.35", features = ["full"] }
```

#### **Step 2.2: Create Metrics Module** (30 min)

**File:** `engine/binaries/server/src/metrics.rs`

```rust
use prometheus::{Encoder, TextEncoder, Registry, IntCounter, Gauge, HistogramVec};
use axum::{routing::get, Router, response::Response};
use std::sync::Arc;

pub struct ServerMetrics {
    registry: Registry,
    pub tick_duration: HistogramVec,
    pub connected_clients: Gauge,
    pub messages_received: IntCounter,
    pub messages_sent: IntCounter,
    pub entity_count: Gauge,
}

impl ServerMetrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();

        let tick_duration = HistogramVec::new(
            prometheus::histogram_opts!("server_tick_duration_seconds", "Server tick duration"),
            &["status"]
        ).unwrap();
        registry.register(Box::new(tick_duration.clone())).unwrap();

        let connected_clients = Gauge::new(
            "server_connected_clients",
            "Number of connected clients"
        ).unwrap();
        registry.register(Box::new(connected_clients.clone())).unwrap();

        let messages_received = IntCounter::new(
            "server_messages_received_total",
            "Total messages received"
        ).unwrap();
        registry.register(Box::new(messages_received.clone())).unwrap();

        let messages_sent = IntCounter::new(
            "server_messages_sent_total",
            "Total messages sent"
        ).unwrap();
        registry.register(Box::new(messages_sent.clone())).unwrap();

        let entity_count = Gauge::new(
            "server_entity_count",
            "Number of entities in world"
        ).unwrap();
        registry.register(Box::new(entity_count.clone())).unwrap();

        Arc::new(Self {
            registry,
            tick_duration,
            connected_clients,
            messages_received,
            messages_sent,
            entity_count,
        })
    }

    pub fn to_prometheus_string(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

pub async fn metrics_handler(
    axum::extract::State(metrics): axum::extract::State<Arc<ServerMetrics>>
) -> Response {
    let body = metrics.to_prometheus_string();
    Response::builder()
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(body.into())
        .unwrap()
}

pub async fn start_metrics_server(metrics: Arc<ServerMetrics>, port: u16) {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("Metrics server listening on http://{}/metrics", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

#### **Step 2.3: Integrate into Main Loop** (30 min)

**File:** `engine/binaries/server/src/main.rs`

```rust
mod metrics;
use metrics::{ServerMetrics, start_metrics_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize metrics
    let metrics = ServerMetrics::new();

    // Start metrics server in background
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        start_metrics_server(metrics_clone, 9090).await;
    });

    // ... existing server setup ...

    loop {
        let tick_start = Instant::now();

        // ... game tick logic ...

        // Update metrics
        let tick_duration = tick_start.elapsed();
        metrics.tick_duration
            .with_label_values(&["ok"])
            .observe(tick_duration.as_secs_f64());

        metrics.connected_clients.set(server.client_count() as f64);
        metrics.entity_count.set(world.entity_count() as f64);

        // ... sleep logic ...
    }
}
```

#### **Step 2.4: Test** (5 min)

```bash
# Start server
cargo run --bin server

# In another terminal, check metrics
curl http://localhost:9090/metrics

# Should see:
# server_tick_duration_seconds{status="ok"} ...
# server_connected_clients 0
# server_entity_count 0
```

---

## ✅ **QUICK WIN #3: Write 1 Basic E2E Test**

**Current State:** No E2E tests, only unit/integration tests

**Goal:** Prove networking works end-to-end with one simple test

**Impact:**
- ✅ Confidence networking actually works
- ✅ Foundation for more E2E tests
- ✅ Catches integration bugs

**Time:** 4 hours

---

### **Implementation Steps**

#### **Step 3.1: Create Test Infrastructure** (2 hours)

**File:** `engine/shared/tests/e2e/mod.rs`

```rust
use engine_networking::{Server, Client};
use std::time::Duration;
use tokio::time::sleep;

pub struct TestServer {
    handle: tokio::task::JoinHandle<()>,
    addr: String,
}

impl TestServer {
    pub async fn spawn(addr: &str) -> Self {
        let addr_clone = addr.to_string();
        let handle = tokio::spawn(async move {
            let mut server = Server::bind(&addr_clone).await.unwrap();
            // Run server until dropped
            loop {
                server.poll_events().await;
                sleep(Duration::from_millis(16)).await;
            }
        });

        // Wait for server to start
        sleep(Duration::from_millis(100)).await;

        Self {
            handle,
            addr: addr.to_string(),
        }
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub struct TestClient {
    client: Client,
}

impl TestClient {
    pub async fn connect(server_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = Client::new()?;
        client.connect(server_addr).await?;

        // Wait for connection
        sleep(Duration::from_millis(100)).await;

        Ok(Self { client })
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn disconnect(&mut self) {
        self.client.disconnect();
    }
}
```

#### **Step 3.2: Write First E2E Test** (1 hour)

**File:** `engine/shared/tests/e2e/connectivity_test.rs`

```rust
use super::*;

#[tokio::test]
async fn test_single_client_connection() {
    // Spawn server
    let server = TestServer::spawn("127.0.0.1:17777").await;

    // Connect client
    let mut client = TestClient::connect(server.addr())
        .await
        .expect("Failed to connect client");

    // Verify connection
    assert!(client.is_connected(), "Client should be connected");

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Disconnect
    client.disconnect();

    // Wait for disconnect to propagate
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test passes if no panics occurred
}
```

#### **Step 3.3: Run Test** (30 min)

```bash
# Run the test
cargo test --test e2e --package engine-shared -- test_single_client_connection

# Should see:
# test e2e::connectivity_test::test_single_client_connection ... ok
```

---

## ✅ **QUICK WIN #4: Add Protocol Version Check**

**Current State:** No version negotiation; clients/servers of different versions can connect

**Goal:** Reject incompatible client/server versions

**Impact:**
- ✅ Prevents hard-to-debug desyncs
- ✅ Production-ready versioning
- ✅ Safe deployments

**Time:** 2 hours

---

### **Implementation Steps**

#### **Step 4.1: Define Protocol Version** (15 min)

**File:** `engine/networking/src/protocol.rs`

```rust
// Add to top of file
pub const PROTOCOL_VERSION: u32 = 1;

// Add new message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // ... existing messages ...

    /// First message sent by client
    Handshake {
        protocol_version: u32,
        client_version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // ... existing messages ...

    /// Response to handshake
    HandshakeAccepted {
        server_version: String,
    },

    /// Rejection due to version mismatch
    HandshakeRejected {
        reason: String,
        server_version: u32,
        client_version: u32,
    },
}
```

#### **Step 4.2: Implement Server-Side Check** (45 min)

**File:** `engine/networking/src/server.rs`

```rust
impl Server {
    async fn handle_new_connection(&mut self, client_id: ClientId, stream: TcpStream) {
        // Wait for handshake (with timeout)
        let handshake = match tokio::time::timeout(
            Duration::from_secs(5),
            self.receive_message(client_id)
        ).await {
            Ok(Some(ClientMessage::Handshake { protocol_version, client_version })) => {
                (protocol_version, client_version)
            }
            _ => {
                warn!("Client {} failed to send handshake", client_id);
                self.disconnect_client(client_id);
                return;
            }
        };

        // Check protocol version
        if handshake.0 != PROTOCOL_VERSION {
            error!(
                "Client {} protocol mismatch: server={}, client={}",
                client_id, PROTOCOL_VERSION, handshake.0
            );

            // Send rejection
            self.send_to(
                client_id,
                ServerMessage::HandshakeRejected {
                    reason: format!(
                        "Protocol version mismatch. Server requires version {}",
                        PROTOCOL_VERSION
                    ),
                    server_version: PROTOCOL_VERSION,
                    client_version: handshake.0,
                }
            );

            // Disconnect
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.disconnect_client(client_id);
            return;
        }

        // Accept handshake
        info!(
            "Client {} handshake accepted (version: {})",
            client_id, handshake.1
        );

        self.send_to(
            client_id,
            ServerMessage::HandshakeAccepted {
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            }
        );
    }
}
```

#### **Step 4.3: Implement Client-Side Check** (30 min)

**File:** `engine/networking/src/client.rs`

```rust
impl Client {
    pub async fn connect(&mut self, addr: &str) -> Result<(), NetworkError> {
        // ... existing TCP connect ...

        // Send handshake
        self.send(ClientMessage::Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        })?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            self.receive_message()
        ).await??;

        match response {
            ServerMessage::HandshakeAccepted { server_version } => {
                info!("Connected to server (version: {})", server_version);
                Ok(())
            }

            ServerMessage::HandshakeRejected { reason, server_version, client_version } => {
                error!(
                    "Connection rejected: {} (server: v{}, client: v{})",
                    reason, server_version, client_version
                );
                Err(NetworkError::VersionMismatch {
                    server: server_version,
                    client: client_version,
                })
            }

            _ => {
                error!("Unexpected response to handshake");
                Err(NetworkError::ProtocolError)
            }
        }
    }
}
```

#### **Step 4.4: Test** (30 min)

```rust
#[test]
fn test_version_mismatch_rejected() {
    let server = spawn_server();

    // Create client with wrong version
    let mut client = Client::new();
    // Manually set wrong version (for testing)
    client.protocol_version = 999;

    let result = client.connect("127.0.0.1:7777");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), NetworkError::VersionMismatch { .. }));
}
```

---

## ✅ **QUICK WIN #5: Add Connection Timeout Handling**

**Current State:** Connections stay open forever; dead connections not detected

**Goal:** Automatically detect and close dead connections

**Impact:**
- ✅ Server doesn't leak connections
- ✅ Clients detect disconnects
- ✅ Better user experience

**Time:** 3 hours

---

### **Implementation Steps**

#### **Step 5.1: Add Heartbeat Messages** (1 hour)

**File:** `engine/networking/src/protocol.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // ... existing ...
    Heartbeat { timestamp: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // ... existing ...
    Heartbeat { timestamp: u64 },
}
```

**File:** `engine/networking/src/client.rs`

```rust
impl Client {
    pub fn update(&mut self) {
        // Send heartbeat every 5 seconds
        if self.last_heartbeat.elapsed() > Duration::from_secs(5) {
            self.send(ClientMessage::Heartbeat {
                timestamp: Instant::now().elapsed().as_millis() as u64,
            });
            self.last_heartbeat = Instant::now();
        }

        // Check for server heartbeat timeout
        if self.last_server_heartbeat.elapsed() > Duration::from_secs(15) {
            warn!("Server heartbeat timeout - disconnecting");
            self.disconnect();
        }
    }
}
```

#### **Step 5.2: Server-Side Timeout Detection** (1 hour)

**File:** `engine/networking/src/server.rs`

```rust
struct ClientConnection {
    // ... existing fields ...
    last_activity: Instant,
}

impl Server {
    pub fn update(&mut self) {
        let now = Instant::now();

        // Check for client timeouts
        let mut timed_out_clients = Vec::new();

        for (client_id, conn) in &self.connections {
            if now.duration_since(conn.last_activity) > Duration::from_secs(30) {
                warn!("Client {} timed out (no activity for 30s)", client_id);
                timed_out_clients.push(*client_id);
            }
        }

        // Disconnect timed out clients
        for client_id in timed_out_clients {
            self.disconnect_client(client_id);
        }

        // Send heartbeats every 5 seconds
        if self.last_heartbeat.elapsed() > Duration::from_secs(5) {
            self.broadcast(ServerMessage::Heartbeat {
                timestamp: now.elapsed().as_millis() as u64,
            });
            self.last_heartbeat = now;
        }
    }

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        // Update last activity
        if let Some(conn) = self.connections.get_mut(&client_id) {
            conn.last_activity = Instant::now();
        }

        match message {
            ClientMessage::Heartbeat { .. } => {
                // Just updates last_activity, no response needed
            }
            // ... handle other messages ...
        }
    }
}
```

#### **Step 5.3: Test** (1 hour)

```rust
#[tokio::test]
async fn test_connection_timeout() {
    let server = spawn_server();
    let client = spawn_client();

    // Simulate network failure (stop sending heartbeats)
    client.stop_heartbeats();

    // Wait for timeout
    tokio::time::sleep(Duration::from_secs(35)).await;

    // Verify client disconnected
    assert!(!client.is_connected());
}
```

---

## 📊 **Quick Wins Summary**

### **Before Quick Wins:**
- Phase 2: 75% complete
- No working binaries
- No monitoring
- No E2E tests
- No version safety
- Connections leak

### **After Quick Wins:**
- Phase 2: **85% complete** (+10%)
- ✅ Client and server run
- ✅ Prometheus metrics available
- ✅ 1 E2E test passing
- ✅ Version checking enforced
- ✅ Connections timeout properly

---

## ✅ **Validation Checklist**

After completing all quick wins, verify:

- [ ] `cargo run --bin server` starts and accepts connections
- [ ] `cargo run --bin client` opens window and connects
- [ ] `curl http://localhost:9090/metrics` returns Prometheus metrics
- [ ] `cargo test test_single_client_connection` passes
- [ ] Connecting with wrong protocol version is rejected
- [ ] Dead connections are detected and closed within 30s

---

## 🚀 **Next Steps After Quick Wins**

With 85% completion, you're ready for:

1. **Complete game loop implementation** (Week 1 of Phase 2 completion)
2. **Full E2E test suite** (Week 2)
3. **Production hardening** (Week 3)

---

**Status:** ✅ Guide Complete - Ready for Implementation
**Estimated Impact:** +10% Phase 2 completion in 1 day
**Author:** Claude Sonnet 4.5
**Date:** 2026-02-03
