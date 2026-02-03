# Prometheus Metrics Guide

Complete guide for monitoring Silmaril with Prometheus and Grafana.

## Quick Start

### 1. Enable Metrics Feature

```toml
# In your Cargo.toml
[dependencies]
engine-observability = { path = "engine/observability", features = ["metrics"] }
```

### 2. Initialize Metrics in Your Server

```rust
use engine_observability::metrics::{MetricsRegistry, start_metrics_server};

#[tokio::main]
async fn main() {
    // Create metrics registry
    let metrics = MetricsRegistry::new();

    // Start HTTP server for Prometheus (in background)
    tokio::spawn(async {
        start_metrics_server("0.0.0.0:9090").await.unwrap();
    });

    // Your game loop
    loop {
        let tick_start = Instant::now();

        // Game logic...

        // Record metrics
        metrics.record_tick_duration(tick_start.elapsed().as_secs_f64() * 1000.0);
        metrics.set_entity_count(world.entity_count() as i64);
    }
}
```

### 3. Start Prometheus + Grafana

```bash
# Using cargo xtask
cargo xtask docker prod

# Or manually
docker-compose up -d

# Development environment
just dev
```

### 4. Access Dashboards

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (admin/changeme)
- **Raw Metrics**: http://localhost:9090/metrics

## Available Metrics

### Frame/Rendering Metrics (Client)

| Metric | Type | Description |
|--------|------|-------------|
| `engine_frame_time_seconds` | Histogram | Frame rendering time |
| `engine_fps` | Gauge | Current frames per second |

**Usage:**
```rust
metrics.record_frame_time(16.7); // 60 FPS
metrics.set_fps(60.0);
```

### Server Tick Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `engine_tick_duration_seconds` | Histogram | Server tick processing time |
| `engine_tick_rate_tps` | Gauge | Current tick rate |
| `engine_tick_count_total` | Counter | Total ticks processed |

**Usage:**
```rust
let start = Instant::now();
// Process game tick...
metrics.record_tick_duration(start.elapsed().as_secs_f64() * 1000.0);
```

**Prometheus Queries:**
```promql
# Average tick rate over 1 minute
rate(engine_tick_count_total[1m]) * 60

# p95 tick duration
histogram_quantile(0.95, sum(rate(engine_tick_duration_seconds_bucket[1m])) by (le))

# Ticks exceeding 16ms budget
sum(rate(engine_tick_duration_seconds_bucket{le="0.016"}[1m])) / sum(rate(engine_tick_duration_seconds_count[1m]))
```

### ECS Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `engine_entity_count` | Gauge | Current entities in world |
| `engine_entities_spawned_total` | Counter | Total entities spawned |
| `engine_entities_despawned_total` | Counter | Total entities despawned |
| `engine_query_time_seconds` | Histogram | ECS query execution time |

**Usage:**
```rust
// Set entity count
metrics.set_entity_count(world.entity_count() as i64);

// Increment/decrement
metrics.increment_entity_count(10);  // Spawned 10
metrics.increment_entity_count(-5);  // Despawned 5

// Query timing
let start = Instant::now();
let results = world.query::<(&Transform, &Velocity)>();
metrics.record_query_time(start.elapsed().as_secs_f64() * 1000.0);
```

**Prometheus Queries:**
```promql
# Entity churn rate (spawns + despawns per second)
rate(engine_entities_spawned_total[1m]) + rate(engine_entities_despawned_total[1m])

# Average query time
rate(engine_query_time_seconds_sum[1m]) / rate(engine_query_time_seconds_count[1m])
```

### Network Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `engine_connected_clients` | Gauge | Currently connected clients |
| `engine_network_bytes_sent_total` | Counter | Total bytes sent |
| `engine_network_bytes_received_total` | Counter | Total bytes received |
| `engine_network_packets_sent_total` | Counter | Total packets sent |
| `engine_network_packets_received_total` | Counter | Total packets received |
| `engine_network_latency_seconds` | Histogram | Network round-trip time |

**Usage:**
```rust
// Client connections
metrics.set_connected_clients(server.client_count() as i64);

// Network traffic
metrics.record_bytes_sent(packet.len() as u64);
metrics.record_packet_sent();

// Latency (measured via ping/pong)
metrics.record_network_latency(rtt_ms);
```

**Prometheus Queries:**
```promql
# Bandwidth usage (bytes/sec)
rate(engine_network_bytes_sent_total[1m])

# Packets per second
rate(engine_network_packets_sent_total[1m])

# p95 network latency
histogram_quantile(0.95, sum(rate(engine_network_latency_seconds_bucket[1m])) by (le))

# Average bandwidth per client
rate(engine_network_bytes_sent_total[1m]) / engine_connected_clients
```

### Memory Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `engine_memory_allocated_bytes` | Gauge | Total allocated memory |
| `engine_ecs_memory_bytes` | Gauge | Memory used by ECS storage |

**Usage:**
```rust
// Update periodically (e.g., every 60 ticks)
if tick_count % 60 == 0 {
    metrics.set_memory_allocated(allocator.total_allocated() as i64);
    metrics.set_ecs_memory(world.memory_usage() as i64);
}
```

**Prometheus Queries:**
```promql
# Memory usage in MB
engine_memory_allocated_bytes / 1024 / 1024

# ECS memory as percentage of total
engine_ecs_memory_bytes / engine_memory_allocated_bytes * 100
```

## Grafana Dashboards

### Pre-configured Dashboard

The repository includes a pre-configured Grafana dashboard at:
`engine/observability/grafana-dashboards/game-server.json`

**Panels:**
1. Server Tick Rate (TPS)
2. Server Tick Duration (p50, p95, p99)
3. Entity Count
4. Connected Clients
5. Network Bandwidth (sent/received)
6. Network Latency (p50, p95, p99)
7. Memory Usage (total, ECS)
8. ECS Query Performance (p50, p95)

### Importing Dashboard

1. Open Grafana: http://localhost:3000
2. Login (admin/changeme)
3. Click "+" → "Import"
4. Upload `game-server.json`
5. Select Prometheus data source
6. Click "Import"

### Creating Custom Panels

**Example: Alert when tick rate drops below 55 TPS**

```promql
engine_tick_rate_tps < 55
```

**Example: Entity spawn rate per second**

```promql
rate(engine_entities_spawned_total[1m])
```

**Example: Network bandwidth (MB/sec)**

```promql
rate(engine_network_bytes_sent_total[1m]) / 1024 / 1024
```

## Performance Impact

The metrics system is designed for minimal overhead:

**With metrics feature disabled:**
- Zero overhead (stub implementation)
- No dependencies compiled

**With metrics feature enabled:**
- ~1-2 microseconds per metric operation
- ~0.1ms per HTTP scrape (Prometheus)
- Memory: ~1MB for metric storage

**Recommendation:**
- Enable in development and production
- Disable only for benchmarking raw performance

## Production Deployment

### 1. Configure Retention

In `prometheus.yml`:
```yaml
global:
  scrape_interval: 15s     # How often to scrape
  evaluation_interval: 15s # How often to evaluate rules

# Command in docker-compose.yml
command:
  - '--storage.tsdb.retention.time=30d'  # Keep 30 days
```

### 2. Set Up Alerts

Create `alerts.yml`:
```yaml
groups:
  - name: game_server
    rules:
      # Alert if tick rate drops below 55 TPS
      - alert: LowTickRate
        expr: engine_tick_rate_tps < 55
        for: 1m
        annotations:
          summary: "Server tick rate is low"
          description: "Tick rate is {{ $value }} TPS"

      # Alert if tick duration exceeds 16ms
      - alert: SlowTicks
        expr: histogram_quantile(0.95, rate(engine_tick_duration_seconds_bucket[1m])) > 0.016
        for: 1m
        annotations:
          summary: "Server ticks are slow"
          description: "p95 tick duration is {{ $value }}s"

      # Alert if too many entities
      - alert: HighEntityCount
        expr: engine_entity_count > 100000
        for: 5m
        annotations:
          summary: "Entity count is very high"
```

Reference in `prometheus.yml`:
```yaml
rule_files:
  - "alerts.yml"
```

### 3. External Monitoring

**Push to remote Prometheus:**
```yaml
remote_write:
  - url: "https://prometheus.example.com/api/v1/write"
    basic_auth:
      username: "user"
      password: "pass"
```

**Export to other systems:**
- Datadog: Use Prometheus integration
- CloudWatch: Use CloudWatch exporter
- New Relic: Use remote write

## Troubleshooting

### Metrics endpoint not accessible

```bash
# Check if server is running
curl http://localhost:9090/metrics

# Check Docker logs
docker logs agent-game-server

# Verify port binding
docker port agent-game-server
```

### No data in Grafana

1. Check Prometheus targets: http://localhost:9090/targets
2. Verify service name matches prometheus.yml
3. Check network connectivity: `docker network inspect game-network`
4. Verify time range in Grafana (default: last 15 minutes)

### High cardinality warnings

If you see "high cardinality" warnings:

```yaml
# In prometheus.yml
metric_relabel_configs:
  # Drop problematic metrics
  - source_labels: [__name__]
    regex: 'engine_problematic_metric_.*'
    action: drop
```

## Examples

See:
- `engine/observability/examples/server_metrics.rs` - Complete server example
- Run: `cargo run --example server_metrics --features metrics`

## Further Reading

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
- [Docker Monitoring](https://docs.docker.com/config/daemon/prometheus/)
