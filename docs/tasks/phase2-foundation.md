# Phase 2.1: Foundation & Infrastructure

**Time Estimate:** 5-7 days
**Complexity:** Medium-High
**Prerequisites:** Phase 1.4 complete (error handling, platform abstraction)

---

## 📋 **Overview**

This is the critical foundation for all of Phase 2. We establish:
1. **Code Separation**: Macros that enforce client/server boundaries at compile-time
2. **Build System**: Separate binaries with feature flags
3. **Container Infrastructure**: Docker for development and production
4. **Observability**: Metrics and admin console for debugging

**Key Principle:** Everything must be testable and measurable from day one.

---

## 🎯 **Goals**

### **Primary Goals**
- ✅ Compile-time enforcement of client/server code separation
- ✅ One-command development environment
- ✅ Production-ready container images
- ✅ Comprehensive metrics for AI-assisted debugging

### **Non-Goals (Deferred)**
- ❌ Kubernetes deployment (Phase 4)
- ❌ Web-based admin dashboard (Phase 4)
- ❌ Authentication for admin console (Phase 4)
- ❌ Database/persistence (Phase 3-4)

---

## 🏗️ **Part A: Proc Macros (Days 1-2)**

### **Architecture**

#### **Macro Patterns**

```rust
// 1. Client-only code (rendering, audio, input)
#[client_only]
fn render_health_bar(health: &Health, renderer: &mut Renderer) {
    // Only compiled in client builds
    // Server builds: compile error if called
}

// 2. Server-only code (anti-cheat, authoritative logic)
#[server_only]
fn validate_damage(attacker: Entity, target: Entity, amount: f32) -> bool {
    // Only compiled in server builds
    // Client builds: compile error if called
}

// 3. Shared code (both run the same code)
#[shared]
fn calculate_distance(a: Vec3, b: Vec3) -> f32 {
    // Both client and server execute
    (a - b).magnitude()
}

// 4. Server-authoritative (different implementations)
#[server_authoritative]
fn apply_damage(target: Entity, amount: f32) {
    // Client sees one implementation (prediction)
    // Server sees different implementation (authoritative)
}
```

### **Macro Expansion**

```rust
// Input:
#[client_only]
fn foo() { }

// Expands to:
#[cfg(feature = "client")]
fn foo() { }
```

```rust
// Input:
#[server_authoritative]
fn calculate_damage(target: Entity, amount: f32) -> f32 {
    // Implementation
}

// Expands to:
#[cfg(feature = "server")]
fn calculate_damage(target: Entity, amount: f32) -> f32 {
    // Server implementation (authoritative)
}

#[cfg(feature = "client")]
fn calculate_damage(target: Entity, amount: f32) -> f32 {
    // Client implementation (prediction)
    // Macro generates skeleton, user fills in
}
```

### **Implementation**

**File:** `engine/macros/src/client_server.rs`

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Marks function as client-only
///
/// # Examples
/// ```
/// #[client_only]
/// fn render() {
///     // Only compiled in client builds
/// }
/// ```
#[proc_macro_attribute]
pub fn client_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let output = quote! {
        #[cfg(feature = "client")]
        #input
    };
    TokenStream::from(output)
}

/// Marks function as server-only
#[proc_macro_attribute]
pub fn server_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let output = quote! {
        #[cfg(feature = "server")]
        #input
    };
    TokenStream::from(output)
}

/// Marks function as shared (client and server)
#[proc_macro_attribute]
pub fn shared(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let output = quote! {
        #[cfg(any(feature = "client", feature = "server"))]
        #input
    };
    TokenStream::from(output)
}

/// Marks function as server-authoritative
/// Generates skeleton for client implementation
#[proc_macro_attribute]
pub fn server_authoritative(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_sig = &input.sig;
    let fn_block = &input.block;

    // Parse attribute for client implementation hint
    let client_impl = if !attr.is_empty() {
        // User can provide custom client implementation
        parse_macro_input!(attr as syn::Block)
    } else {
        // Default: simplified implementation
        syn::parse_quote! {{
            // TODO: Implement client prediction
            // This is authoritative on server, estimated on client
            unimplemented!("Client prediction not implemented")
        }}
    };

    let output = quote! {
        // Server gets the full implementation
        #[cfg(feature = "server")]
        #fn_sig #fn_block

        // Client gets simplified/predicted implementation
        #[cfg(feature = "client")]
        #fn_sig #client_impl
    };

    TokenStream::from(output)
}
```

### **Testing Strategy**

#### **Test 1: Compile-Time Enforcement**

**File:** `engine/macros/tests/compile_fail/client_server_separation.rs`

```rust
// This test MUST fail to compile
#[test]
#[cfg(feature = "server")]
fn test_server_cannot_call_client_code() {
    #[client_only]
    fn client_function() {}

    // This should fail: server calling client-only function
    client_function();
}
```

Use `trybuild` crate to verify these fail:

```toml
[dev-dependencies]
trybuild = "1.0"
```

```rust
#[test]
fn test_compile_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
```

#### **Test 2: Property-Based Validation**

```rust
#[cfg(all(test, feature = "client", feature = "server"))]
mod tests {
    use proptest::prelude::*;

    // Example: Damage calculation
    #[server_authoritative]
    fn calculate_damage(base: f32, armor: f32) -> f32 {
        // Server implementation
    }

    proptest! {
        #[test]
        fn client_server_damage_parity(
            base_damage in 1.0f32..100.0,
            armor in 0.0f32..50.0,
        ) {
            // Both implementations should be called here
            // (requires building with --all-features)
            let client_result = calculate_damage_client(base_damage, armor);
            let server_result = calculate_damage_server(base_damage, armor);

            // Properties to verify:
            // 1. Server result is never negative
            prop_assert!(server_result >= 0.0);

            // 2. Client prediction within reasonable bounds
            let diff_percent = (client_result - server_result).abs() / server_result;
            prop_assert!(diff_percent < 0.5, "Client prediction >50% off");

            // 3. Both scale similarly (correlation)
            // If base damage doubles, both should increase
        }
    }
}
```

### **Documentation**

**File:** `engine/macros/CLAUDE.md`

```markdown
# Client/Server Macros

## Usage Patterns

### 1. Pure Client Code
Use for: Graphics, audio, input handling
- Code ONLY runs on client
- Server builds: function doesn't exist

### 2. Pure Server Code
Use for: Anti-cheat, economy, loot tables
- Code ONLY runs on server
- Client builds: function doesn't exist
- Prevents: Hacking, exploitation

### 3. Shared Code
Use for: Utility functions, math, physics
- Both client and server execute
- Example: Distance calculation, movement

### 4. Server Authoritative
Use for: Gameplay logic with prediction
- Server: Full authoritative implementation
- Client: Simplified prediction for UX
- Example: Damage calculation, movement validation
```

### **Acceptance Criteria**

- [ ] All 4 macros implemented and tested
- [ ] Compile-fail tests verify separation
- [ ] Property tests validate parity
- [ ] Documentation with examples
- [ ] CI fails if client calls server-only code

---

## 🔧 **Part B: Build Infrastructure (Days 2-3)**

### **Directory Structure**

```
engine/
├── core/
│   └── Cargo.toml
│       [features]
│       client = ["rendering", "audio", "input"]
│       server = ["headless", "admin-tools"]
│       networking = ["tokio", "quinn"]
│       all = ["client", "server"]  # For testing both
│
├── binaries/
│   ├── client/
│   │   ├── Cargo.toml
│   │   │   [dependencies]
│   │   │   engine-core = { path = "../../core", features = ["client"] }
│   │   └── src/
│   │       └── main.rs  # Client entry point
│   │
│   └── server/
│       ├── Cargo.toml
│       │   [dependencies]
│       │   engine-core = { path = "../../core", features = ["server"] }
│       └── src/
│           └── main.rs  # Server entry point
```

### **Build Profiles**

**File:** `Cargo.toml` (workspace root)

```toml
[workspace]
members = [
    "engine/core",
    "engine/macros",
    "engine/binaries/client",
    "engine/binaries/server",
]

[profile.release]
# Client profile: Optimize for performance
lto = "thin"
codegen-units = 16
opt-level = 3
strip = false  # Keep symbols for profiling

[profile.release-server]
# Server profile: Optimize for size + throughput
inherits = "release"
lto = "fat"
codegen-units = 1
opt-level = "z"  # Optimize for size
strip = true     # Remove symbols

[workspace.lints.clippy]
# From Phase 1.4
correctness = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
perf = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
```

### **Build Commands**

```bash
# Client (development)
cargo build --bin client

# Client (release, optimized for performance)
cargo build --bin client --release

# Server (development)
cargo build --bin server

# Server (release, optimized for size)
cargo build --bin server --profile release-server

# Both (for testing)
cargo build --workspace --all-features

# Cross-compile (future)
cargo build --bin client --target x86_64-pc-windows-msvc
cargo build --bin server --target x86_64-unknown-linux-gnu
```

### **CI Configuration**

**File:** `.github/workflows/client-server.yml`

```yaml
name: Client/Server Build Matrix

on: [push, pull_request]

jobs:
  test-client:
    name: Test Client (${{ matrix.os }})
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Build client
        run: cargo build --bin client --release

      - name: Test client code
        run: cargo test --features client

      - name: Verify client binary size
        run: |
          SIZE=$(stat -c%s "target/release/client")
          echo "Client binary: $SIZE bytes"
          # Fail if >100MB (something wrong)
          test $SIZE -lt 104857600

  test-server:
    name: Test Server (${{ matrix.os }})
    strategy:
      matrix:
        os: [ubuntu-latest]  # Server is Linux-only
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Build server
        run: cargo build --bin server --profile release-server

      - name: Test server code
        run: cargo test --features server

      - name: Verify server binary is smaller
        run: |
          CLIENT_SIZE=$(stat -c%s "target/release/client")
          SERVER_SIZE=$(stat -c%s "target/release-server/server")
          echo "Client: $CLIENT_SIZE bytes"
          echo "Server: $SERVER_SIZE bytes"
          # Server should be smaller (stripped)
          test $SERVER_SIZE -lt $CLIENT_SIZE

  test-separation:
    name: Verify Client/Server Separation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Test compile-fail tests
        run: cargo test --package engine-macros compile_fail

      - name: Build with all features (should work)
        run: cargo build --all-features
```

### **Acceptance Criteria**

- [ ] Client binary compiles with `--features client`
- [ ] Server binary compiles with `--features server`
- [ ] Both binaries can be built simultaneously with `--all-features`
- [ ] CI matrix tests on all platforms
- [ ] Server binary is smaller (stripped, optimized for size)
- [ ] Compile-fail tests verify separation

---

## 🐳 **Part C: Docker Infrastructure (Days 3-4)**

### **Development Environment**

**File:** `docker-compose.dev.yml`

```yaml
version: '3.8'

services:
  server:
    build:
      context: .
      dockerfile: docker/Dockerfile.server.dev
    container_name: game-server-dev
    volumes:
      # Mount source for hot-reload
      - ./:/workspace
      # Cache Cargo dependencies
      - cargo-cache:/usr/local/cargo/registry
      - target-cache:/workspace/target
    ports:
      - "7777:7777/tcp"  # Game port (TCP)
      - "7777:7777/udp"  # Game port (UDP)
      - "8080:8080"      # Metrics/admin
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
      - SERVER_PORT=7777
      - METRICS_PORT=8080
    command: cargo watch -x 'run --bin server'
    networks:
      - game-network

  client:
    build:
      context: .
      dockerfile: docker/Dockerfile.client.dev
    container_name: game-client-dev
    volumes:
      - ./:/workspace
      - cargo-cache:/usr/local/cargo/registry
      - target-cache:/workspace/target
      # X11 for rendering (Linux only)
      - /tmp/.X11-unix:/tmp/.X11-unix
    environment:
      - DISPLAY=${DISPLAY}
      - SERVER_ADDRESS=server:7777
      - RUST_LOG=debug
    depends_on:
      - server
    command: cargo watch -x 'run --bin client'
    networks:
      - game-network

  # Optional: Prometheus for metrics
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus-dev
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
    networks:
      - game-network

networks:
  game-network:
    driver: bridge

volumes:
  cargo-cache:
  target-cache:
```

**File:** `monitoring/prometheus.yml`

```yaml
global:
  scrape_interval: 5s

scrape_configs:
  - job_name: 'game-server'
    static_configs:
      - targets: ['server:8080']
```

### **Development Dockerfiles**

**File:** `docker/Dockerfile.server.dev`

```dockerfile
FROM rust:1.75

WORKDIR /workspace

# Install cargo-watch for hot-reload
RUN cargo install cargo-watch

# Expose ports
EXPOSE 7777 8080

# Development mode: cargo-watch handles rebuilds
CMD ["cargo", "watch", "-x", "run --bin server"]
```

**File:** `docker/Dockerfile.client.dev`

```dockerfile
FROM rust:1.75

WORKDIR /workspace

# Install dependencies for Vulkan/rendering
RUN apt-get update && apt-get install -y \
    libvulkan-dev \
    vulkan-utils \
    libx11-dev \
    libxcb1-dev \
    libxrandr-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch
RUN cargo install cargo-watch

CMD ["cargo", "watch", "-x", "run --bin client"]
```

### **Production Dockerfiles**

**File:** `docker/Dockerfile.server`

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /build

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY engine/ ./engine/

# Build server in release mode
RUN cargo build --bin server --profile release-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 gameserver && \
    mkdir -p /app/data && \
    chown -R gameserver:gameserver /app

USER gameserver
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release-server/server /app/server

# Expose ports
EXPOSE 7777/tcp
EXPOSE 7777/udp
EXPOSE 8080/tcp

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run server
CMD ["/app/server"]
```

**Binary size target:** < 50MB (stripped)

### **Development Scripts**

**File:** `scripts/dev.sh`

```bash
#!/bin/bash
set -e

echo "🚀 Starting Silmaril Development Environment"
echo ""

# Check Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker Desktop."
    exit 1
fi

# Check if containers are already running
if docker-compose -f docker-compose.dev.yml ps | grep -q "Up"; then
    echo "⚠️  Containers already running. Stopping..."
    docker-compose -f docker-compose.dev.yml down
fi

# Build and start containers
echo "📦 Building containers..."
docker-compose -f docker-compose.dev.yml build

echo "🏃 Starting services..."
docker-compose -f docker-compose.dev.yml up -d

# Wait for server to be healthy
echo "⏳ Waiting for server to be ready..."
sleep 5

# Check server health
if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Server ready at localhost:7777"
    echo "✅ Metrics at http://localhost:8080/metrics"
    echo "✅ Prometheus at http://localhost:9090"
else
    echo "⚠️  Server not responding yet, check logs:"
    echo "   docker-compose -f docker-compose.dev.yml logs server"
fi

echo ""
echo "📊 View logs:"
echo "   docker-compose -f docker-compose.dev.yml logs -f server"
echo "   docker-compose -f docker-compose.dev.yml logs -f client"
echo ""
echo "🛑 Stop environment:"
echo "   docker-compose -f docker-compose.dev.yml down"
```

Make executable:
```bash
chmod +x scripts/dev.sh
```

### **Acceptance Criteria**

- [ ] `./scripts/dev.sh` starts complete environment
- [ ] Server accessible at localhost:7777
- [ ] Metrics accessible at localhost:8080/metrics
- [ ] Hot-reload works (change code, auto-rebuilds)
- [ ] Production Dockerfile builds < 50MB image
- [ ] Multi-stage build caches dependencies efficiently

---

## 📊 **Part D: Metrics & Observability (Days 4-5)**

### **Metrics Architecture**

```rust
// engine/observability/src/metrics.rs

use prometheus::{Registry, Histogram, Gauge, Counter, Opts};
use std::sync::Arc;

pub struct MetricsCollector {
    registry: Arc<Registry>,

    // Core server metrics
    pub server_tps: Gauge,
    pub player_count: Gauge,
    pub entity_count: Gauge,

    // Performance metrics
    pub tick_duration: Histogram,
    pub physics_duration: Histogram,
    pub network_duration: Histogram,
    pub queries_duration: Histogram,

    // Network metrics
    pub bytes_sent: Counter,
    pub bytes_received: Counter,
    pub packets_dropped: Counter,

    // Error metrics
    pub error_count: Counter,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, PrometheusError> {
        let registry = Arc::new(Registry::new());

        Ok(Self {
            registry: registry.clone(),

            server_tps: Gauge::with_opts(Opts::new(
                "server_tps",
                "Server ticks per second"
            ))?,

            player_count: Gauge::with_opts(Opts::new(
                "player_count",
                "Number of connected players"
            ))?,

            entity_count: Gauge::with_opts(Opts::new(
                "entity_count",
                "Total entities in world"
            ))?,

            tick_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "server_tick_duration_ms",
                    "Server tick duration in milliseconds"
                ).buckets(vec![1.0, 5.0, 10.0, 16.0, 25.0, 50.0, 100.0])
            )?,

            // ... register all metrics
        })
    }

    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}
```

### **Prometheus Endpoint**

```rust
// engine/observability/src/endpoint.rs

use axum::{Router, routing::get, extract::State};
use prometheus::{Registry, TextEncoder, Encoder};
use std::sync::Arc;

pub async fn serve_metrics(
    port: u16,
    registry: Arc<Registry>,
) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(registry);

    let addr = format!("0.0.0.0:{}", port);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
}

async fn metrics_handler(
    State(registry): State<Arc<Registry>>,
) -> Result<String, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(String::from_utf8(buffer).unwrap())
}

async fn health_handler() -> &'static str {
    "OK"
}
```

### **Admin Console**

```rust
// engine/admin/src/console.rs

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::collections::HashMap;

pub struct AdminConsole {
    port: u16,
    commands: HashMap<String, Box<dyn CommandHandler>>,
}

pub trait CommandHandler: Send + Sync {
    fn execute(&self, args: &[&str]) -> Result<String, String>;
}

impl AdminConsole {
    pub fn new(port: u16) -> Self {
        let mut console = Self {
            port,
            commands: HashMap::new(),
        };

        // Register built-in commands
        console.register("status", Box::new(StatusCommand));
        console.register("players", Box::new(PlayersCommand));
        console.register("spawn", Box::new(SpawnCommand));
        console.register("kick", Box::new(KickCommand));
        console.register("help", Box::new(HelpCommand));

        console
    }

    pub async fn serve(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).await?;

        warn!(
            "Admin console started on localhost:{} - NO AUTHENTICATION",
            self.port
        );

        loop {
            let (socket, addr) = listener.accept().await?;
            info!("Admin console connection from: {}", addr);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket).await {
                    error!("Console error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(socket: TcpStream) -> Result<(), std::io::Error> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    writer.write_all(b"Silmaril Admin Console\n").await?;
    writer.write_all(b"Type 'help' for commands\n\n").await?;

    loop {
        writer.write_all(b"> ").await?;
        writer.flush().await?;

        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // Connection closed
        }

        let response = process_command(&line);
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }

    Ok(())
}
```

### **Metrics Integration**

```rust
// Example: Server tick with metrics

use engine_observability::MetricsCollector;
use std::time::Instant;

pub struct GameServer {
    world: World,
    metrics: Arc<MetricsCollector>,
}

impl GameServer {
    pub fn tick(&mut self, dt: f32) {
        let tick_start = Instant::now();

        // Update entity count
        self.metrics.entity_count.set(self.world.entity_count() as f64);

        // Physics
        let physics_start = Instant::now();
        self.update_physics(dt);
        self.metrics.physics_duration.observe(
            physics_start.elapsed().as_millis() as f64
        );

        // Networking
        let network_start = Instant::now();
        self.update_networking();
        self.metrics.network_duration.observe(
            network_start.elapsed().as_millis() as f64
        );

        // Record total tick time
        self.metrics.tick_duration.observe(
            tick_start.elapsed().as_millis() as f64
        );
    }
}
```

### **Acceptance Criteria**

- [ ] Prometheus endpoint serves metrics at :8080/metrics
- [ ] Health check endpoint at :8080/health
- [ ] Admin console accessible via telnet localhost:9000
- [ ] Core metrics tracked (TPS, entities, memory)
- [ ] Performance metrics tracked (tick duration, per-system)
- [ ] Network metrics tracked (bytes, latency, packet loss)
- [ ] Error metrics tracked (counts, types)
- [ ] Metrics configurable (can disable for benchmarking)

---

## 🧪 **Testing Strategy**

### **Unit Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_macro_compiles() {
        #[client_only]
        fn test_fn() {}

        // Should compile with client feature
        #[cfg(feature = "client")]
        test_fn();
    }

    #[test]
    fn test_metrics_collector() {
        let metrics = MetricsCollector::new().unwrap();

        metrics.server_tps.set(60.0);
        metrics.entity_count.set(1000.0);

        let families = metrics.registry().gather();
        assert!(!families.is_empty());
    }
}
```

### **Integration Tests**

```rust
// tests/foundation_integration.rs

#[tokio::test]
async fn test_complete_environment() {
    // 1. Start metrics server
    let metrics = Arc::new(MetricsCollector::new().unwrap());
    let metrics_handle = tokio::spawn(async move {
        serve_metrics(8080, metrics.registry()).await
    });

    // 2. Start admin console
    let console = AdminConsole::new(9000);
    let console_handle = tokio::spawn(async move {
        console.serve().await
    });

    // 3. Verify endpoints
    tokio::time::sleep(Duration::from_millis(100)).await;

    let health_response = reqwest::get("http://localhost:8080/health")
        .await
        .unwrap();
    assert_eq!(health_response.status(), StatusCode::OK);

    let metrics_response = reqwest::get("http://localhost:8080/metrics")
        .await
        .unwrap();
    assert!(metrics_response.text().await.unwrap().contains("server_tps"));

    // Cleanup
    metrics_handle.abort();
    console_handle.abort();
}
```

### **Benchmark Tests**

```rust
// benches/foundation_benches.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_metrics_update(c: &mut Criterion) {
    let metrics = MetricsCollector::new().unwrap();

    c.bench_function("metrics_update_10_gauges", |b| {
        b.iter(|| {
            for i in 0..10 {
                metrics.entity_count.set(black_box(i as f64));
            }
        });
    });
}

fn bench_histogram_observe(c: &mut Criterion) {
    let metrics = MetricsCollector::new().unwrap();

    c.bench_function("histogram_observe", |b| {
        b.iter(|| {
            metrics.tick_duration.observe(black_box(16.0));
        });
    });
}

criterion_group!(benches, bench_metrics_update, bench_histogram_observe);
criterion_main!(benches);
```

**Performance Targets:**
- Metrics update: < 100ns per gauge/counter
- Histogram observe: < 500ns per observation
- Metrics gathering (Prometheus scrape): < 10ms

---

## 📚 **Documentation**

### **Developer Guide**

**File:** `docs/development-workflow.md` (update)

```markdown
## Phase 2 Development Environment

### Quick Start

1. **Install Docker Desktop**
   - Windows/Mac: Download from docker.com
   - Linux: Install docker + docker-compose

2. **Start Development Environment**
   ```bash
   ./scripts/dev.sh
   ```

   This starts:
   - Game server (localhost:7777)
   - Metrics endpoint (localhost:8080)
   - Prometheus (localhost:9090)
   - Hot-reload enabled

3. **Verify Environment**
   ```bash
   # Check server health
   curl http://localhost:8080/health

   # View metrics
   curl http://localhost:8080/metrics

   # Connect to admin console
   telnet localhost 9000
   ```

### Building Binaries

```bash
# Client (Windows, Linux, macOS)
cargo build --bin client --release

# Server (Linux only for production)
cargo build --bin server --profile release-server

# Both (for testing)
cargo build --workspace --all-features
```

### Running Tests

```bash
# Test client code
cargo test --features client

# Test server code
cargo test --features server

# Test both
cargo test --all-features

# Integration tests
cargo test --test foundation_integration
```

### Debugging

- **Server logs:** `docker-compose -f docker-compose.dev.yml logs -f server`
- **Metrics:** http://localhost:8080/metrics
- **Admin console:** `telnet localhost 9000`
- **Prometheus:** http://localhost:9090
```

---

## ✅ **Acceptance Criteria (Phase 2.1 Complete)**

### **Macros**
- [ ] All 4 macros implemented (#[client_only], #[server_only], #[shared], #[server_authoritative])
- [ ] Compile-fail tests verify separation
- [ ] Property tests validate client/server parity
- [ ] Documentation with examples

### **Build System**
- [ ] Client binary builds with `cargo build --bin client`
- [ ] Server binary builds with `cargo build --bin server`
- [ ] Separate build profiles (performance vs size)
- [ ] CI matrix tests both builds on all platforms
- [ ] Server binary < 50MB (stripped)

### **Docker**
- [ ] `./scripts/dev.sh` starts complete environment
- [ ] Hot-reload works (change code → auto-rebuild)
- [ ] Production Dockerfiles build < 50MB images
- [ ] Multi-stage builds cache efficiently
- [ ] docker-compose networking allows client ↔ server

### **Metrics**
- [ ] Prometheus endpoint at :8080/metrics
- [ ] Health check at :8080/health
- [ ] Core metrics tracked (TPS, entities, memory)
- [ ] Performance metrics (tick duration, per-system)
- [ ] Network metrics (bytes, latency, packet loss)
- [ ] Configurable (can disable for benchmarking)

### **Admin Console**
- [ ] Accessible via `telnet localhost 9000`
- [ ] Basic commands (status, players, spawn, kick)
- [ ] Localhost-only (not exposed externally)
- [ ] TODO added for Phase 4 authentication

### **Quality**
- [ ] >80% test coverage
- [ ] All tests pass (unit + integration + property)
- [ ] Benchmarks meet targets (<100ns metrics, <10ms gather)
- [ ] CI passes on all platforms
- [ ] Documentation complete

---

## 🚀 **Next Steps**

After Phase 2.1 complete, proceed to:
- **Phase 2.2:** Network Protocol (FlatBuffers, message definitions)
- **Phase 2.3:** TCP Channel (reliable connection)
- **Phase 2.4:** UDP Channel (fast unreliable packets)

With foundation solid, networking implementation will be straightforward.

---

**Estimated Timeline:** 5-7 days
**Critical Path:** Macros → Build → Docker → Metrics
**Parallelization:** Can work on Docker while testing macros
**Risk Level:** Medium (Docker networking can be tricky)
