# Server Binary

## Purpose

The server binary is the authoritative game server that manages game state, processes player input, runs game logic, and synchronizes state to connected clients. It ensures fair gameplay by validating all actions server-side and preventing cheating.

## Architecture

The server is built on top of the engine's core systems:

- **ECS Core**: Entity management and component storage (authoritative)
- **Networking Server**: TCP/UDP server for client connections
- **Game Logic Systems**: Physics, combat, AI, etc.
- **State Synchronization**: Sends full snapshots and delta updates to clients
- **Interest Management**: Optimizes bandwidth by sending only relevant entities
- **Persistence**: Saves/loads game state to database

### Data Flow

```
Receive Client Input (TCP/UDP)
    |
    v
Validate Input (anti-cheat)
    |
    v
Queue Input for Processing
    |
    v
Run Game Tick (60 TPS)
    |
    +---> Run Game Logic Systems
    |
    +---> Update Physics (authoritative)
    |
    +---> Interest Management (per-client visibility)
    |
    v
Generate State Updates (full/delta)
    |
    v
Send to Clients (TCP/UDP)
```

## Architecture

The server runs at a fixed tick rate (60 TPS) and processes:
1. Network events (connections, disconnections, messages)
2. Player input (movement, actions)
3. Game logic systems (physics, combat, AI)
4. State synchronization (broadcasting updates to clients)

## Feature Flags

The server binary is compiled with the `server` feature flag enabled:

```toml
[features]
default = ["server"]
server = []
```

This ensures that:
- Server-only code (physics, AI, etc.) is included
- Client-only code (rendering, UI, etc.) is excluded
- Shared code (ECS, math, etc.) is available

### Component Availability

With `#[server_only]` macro:
- `ServerAuthority` - Available
- `PlayerConnection` - Available
- `AIController` - Available
- `PhysicsBackend` - Available

With `#[client_only]` macro:
- `MeshRenderer` - NOT available
- `Camera` - NOT available
- `AudioListener` - NOT available

With `#[shared]` macro:
- `Transform` - Available
- `Velocity` - Available
- `Health` - Available

## Build Instructions

### Development Build

```bash
# Build server with debug symbols
cargo build --bin server --features server

# Run server
cargo run --bin server --features server
```

### Release Build

```bash
# Build optimized server
cargo build --bin server --features server --release

# Strip symbols for smaller binary
cargo build --bin server --features server --release --config strip=symbols
```

### Build Scripts

Use the provided build scripts:

```bash
# Build server only
./scripts/build-server.sh

# Build both client and server
./scripts/build-both.sh
```

### Docker Build

```bash
# Build Docker image
docker build -f engine/binaries/server/Dockerfile -t silmaril-server .

# Run in container
docker run -it --rm \
  -p 7777:7777 \
  -p 7778:7778/udp \
  silmaril-server
```

## Platform-Specific Considerations

### Linux (Primary Platform)

- Recommended for production deployments
- Best performance for server workloads
- Easy containerization with Docker/Kubernetes
- Native support for systemd services

### Windows

- Supported for development
- May have slightly higher overhead than Linux
- Good for testing server on Windows dev machines

### macOS

- Supported for development
- ARM64 (Apple Silicon) has excellent performance
- Good for local testing

### Containerization

The server is designed to run in containers:

```yaml
# docker-compose.yml
version: '3.8'
services:
  server:
    image: silmaril-server
    ports:
      - "7777:7777"
      - "7778:7778/udp"
    environment:
      - RUST_LOG=info
      - TCP_BIND_ADDR=0.0.0.0:7777
      - UDP_BIND_ADDR=0.0.0.0:7778
    restart: always
```

### Kubernetes Deployment

```yaml
# server-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: game-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: game-server
  template:
    metadata:
      labels:
        app: game-server
    spec:
      containers:
      - name: server
        image: silmaril-server:latest
        ports:
        - containerPort: 7777
          protocol: TCP
        - containerPort: 7778
          protocol: UDP
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "2000m"
```

## Configuration

The server can be configured via:

1. **Config file** (`server_config.toml`):
```toml
[network]
tcp_bind_addr = "0.0.0.0:7777"
udp_bind_addr = "0.0.0.0:7778"
max_clients = 1000

[game]
ticks_per_second = 60
max_entities = 100000

[sync]
snapshot_interval = 60  # Every 1 second
update_rate = 20        # 20 updates/sec per client
```

2. **Environment variables**:
```bash
TCP_BIND_ADDR=0.0.0.0:7777 cargo run --bin server
UDP_BIND_ADDR=0.0.0.0:7778 cargo run --bin server
RUST_LOG=debug cargo run --bin server
```

3. **Command-line arguments**:
```bash
cargo run --bin server -- --tcp 0.0.0.0:7777 --udp 0.0.0.0:7778 --tps 60
```

## Performance Targets

- **Tick Rate**: 60 TPS ± 1 tick
- **Tick Duration**: < 10ms average, < 16ms max
- **Concurrent Players**: 1000+ (per server instance)
- **Input Latency**: < 50ms (processing time)
- **Memory Usage**: < 500 MB (1000 entities, 100 clients)
- **CPU Usage**: < 80% (full load)

## Monitoring

The server provides metrics for monitoring:

```rust
// Exposed metrics (Prometheus format)
server_tick_duration_seconds
server_tick_rate_tps
server_connected_clients
server_entity_count
server_bandwidth_out_bytes_per_second
server_input_queue_size
```

### Logging

Structured logging with tracing:

```bash
# Development (pretty console output)
RUST_LOG=debug cargo run --bin server

# Production (JSON for log aggregation)
RUST_LOG=info,silmaril=debug cargo run --bin server
```

### Health Checks

The server exposes health endpoints:

```bash
# Liveness check (is server running?)
curl http://localhost:8080/health/live

# Readiness check (is server ready to accept connections?)
curl http://localhost:8080/health/ready
```

## Server Authority

The server is authoritative for all game state:

- **Physics**: Server runs physics simulation, clients predict locally
- **Combat**: Server validates damage calculations
- **Movement**: Server validates player positions (anti-cheat)
- **Spawning**: Server controls entity lifecycle
- **Items**: Server manages inventory and pickups

Clients send **input**, server sends **state updates**.

## Anti-Cheat

The server implements several anti-cheat measures:

1. **Input Validation**: Check physical constraints (speed, distance)
2. **Rate Limiting**: Prevent input spam
3. **Fog of War**: Don't send invisible entities to clients
4. **Replay Detection**: Sequence numbers prevent replay attacks
5. **Server Authority**: All critical logic runs server-side

## Scaling

### Vertical Scaling

Single server instance can handle:
- 1000+ concurrent players
- 100,000+ entities
- 60 TPS tick rate

Optimize by:
- Increasing CPU cores (multi-threaded systems)
- Adding RAM (more entities)
- Using faster storage (database operations)

### Horizontal Scaling

Multiple server instances with:
- Load balancer (distribute players across servers)
- Shared database (PostgreSQL, Redis)
- Region-based sharding (US-East, EU-West, etc.)

```
                 Load Balancer
                      |
        +-------------+-------------+
        |             |             |
    Server 1      Server 2      Server 3
        |             |             |
        +-------------+-------------+
                      |
                  Database
```

## Related Documentation

- [D:\dev\silmaril\docs\architecture.md](../../docs/architecture.md) - Overall system architecture
- [D:\dev\silmaril\docs\tasks\phase2-proc-macros.md](../../docs/tasks/phase2-proc-macros.md) - Server/client code splitting with macros
- [D:\dev\silmaril\docs\tasks\phase2-server-tick.md](../../docs/tasks/phase2-server-tick.md) - Server tick loop implementation
- [D:\dev\silmaril\docs\tasks\phase2-state-sync.md](../../docs/tasks/phase2-state-sync.md) - State synchronization
- [D:\dev\silmaril\docs\tasks\phase2-tcp-connection.md](../../docs/tasks/phase2-tcp-connection.md) - TCP networking
- [D:\dev\silmaril\docs\tasks\phase2-udp-packets.md](../../docs/tasks/phase2-udp-packets.md) - UDP networking
- [D:\dev\silmaril\docs\performance-targets.md](../../docs/performance-targets.md) - Performance benchmarks
