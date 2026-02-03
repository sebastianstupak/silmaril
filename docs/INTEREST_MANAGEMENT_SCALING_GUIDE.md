# Interest Management - Scaling Guide

## Overview

This guide covers when and how to scale interest management for production MMO deployments.

**Topics Covered:**
- When to scale (triggers and thresholds)
- Horizontal vs vertical scaling
- Sharding strategies
- Cross-server visibility
- Cost optimization

---

## When to Scale

### Performance-Based Triggers

#### Trigger 1: High CPU Utilization

**Indicator:** Server CPU >70% sustained for 10+ minutes

**Diagnosis:**
```promql
# Prometheus query
avg(rate(process_cpu_seconds_total[5m])) * 100 > 70
```

**Action:** Add horizontal capacity (new servers)

**Why 70%?**
- Leaves headroom for spikes
- Allows graceful degradation before 100%
- Prevents cascading failures

---

#### Trigger 2: Slow Visibility Calculations

**Indicator:** p95 visibility calc >2ms sustained

**Diagnosis:**
```promql
interest_visibility_calculation_duration_p95 > 2000
```

**Action:** Either:
1. Vertical scale (more CPU cores)
2. Horizontal scale (distribute load)
3. Optimize (tune grid cell size, reduce AOI radius)

**Why 2ms?**
- Target: <1ms normally
- 2ms = warning threshold
- 5ms = critical threshold

---

#### Trigger 3: High Player Density

**Indicator:** >80% of target capacity

**Diagnosis:**
```bash
# Check current vs target capacity
current_players=$(curl -s http://server:9090/metrics | grep interest_total_clients | awk '{print $2}')
target_capacity=1000
utilization=$((current_players * 100 / target_capacity))

if [ $utilization -gt 80 ]; then
  echo "Scale up needed: ${utilization}% capacity"
fi
```

**Action:** Provision new server, enable load balancing

**Why 80%?**
- Prevents hitting hard capacity limits
- Allows time to provision new servers (10-15 min)
- Buffer for sudden influx (login storms, events)

---

### Capacity-Based Triggers

#### Player Count Thresholds

| Players | Status | Action |
|---------|--------|--------|
| 0-500 | ✅ Green | Normal operation |
| 501-800 | ⚠️ Yellow | Monitor closely |
| 801-1000 | 🟠 Orange | Provision new server |
| 1001+ | 🔴 Red | Reject new connections |

**Implementation:**
```rust
match player_count {
    0..=500 => ServerStatus::Green,
    501..=800 => {
        tracing::warn!("Approaching capacity: {} players", player_count);
        ServerStatus::Yellow
    }
    801..=1000 => {
        tracing::error!("At capacity threshold, scaling needed");
        provision_new_server();
        ServerStatus::Orange
    }
    _ => {
        tracing::error!("Over capacity, rejecting connections");
        reject_new_connections();
        ServerStatus::Red
    }
}
```

---

#### Entity Count Thresholds

| Entities | Status | Action |
|----------|--------|--------|
| 0-5K | ✅ Green | Normal operation |
| 5K-10K | ⚠️ Yellow | Monitor performance |
| 10K-15K | 🟠 Orange | Consider vertical scale |
| 15K+ | 🔴 Red | Must scale or optimize |

**Why these numbers?**
- Tested up to 100K entities (stress test)
- 10K = comfortable production capacity
- 15K = approaching performance cliff
- 20K+ = requires careful tuning

---

## Horizontal Scaling

### Load Balancing Strategy

#### Round-Robin (Simple)

**Best for:** Even player distribution across zones

```nginx
upstream game_servers {
    server game1.example.com:7777;
    server game2.example.com:7777;
    server game3.example.com:7777;
}

server {
    listen 7777;
    proxy_pass game_servers;
}
```

**Pros:**
- Simple to configure
- Even load distribution
- No session affinity needed (if stateless)

**Cons:**
- Doesn't consider server load
- No geographic routing

---

#### Least Connections (Load-Aware)

**Best for:** Variable player session length

```nginx
upstream game_servers {
    least_conn;
    server game1.example.com:7777;
    server game2.example.com:7777;
    server game3.example.com:7777;
}
```

**Pros:**
- Balances by active connections
- Adapts to server load
- Prevents overloading slow servers

**Cons:**
- Requires connection tracking
- More complex than round-robin

---

#### Geographic Routing (Latency-Optimized)

**Best for:** Global playerbase

```yaml
# AWS Global Accelerator / Cloudflare Load Balancer
regions:
  us-east:
    servers:
      - game-us-east-1.example.com
      - game-us-east-2.example.com
  eu-west:
    servers:
      - game-eu-west-1.example.com
      - game-eu-west-2.example.com
  ap-southeast:
    servers:
      - game-ap-southeast-1.example.com
      - game-ap-southeast-2.example.com

routing:
  policy: lowest_latency
```

**Pros:**
- Minimizes player latency
- Region isolation (data residency)
- Scales per region independently

**Cons:**
- More complex infrastructure
- Higher cost (multi-region)
- Cross-region visibility harder

---

### Sharding Strategies

#### Strategy 1: Zone-Based Sharding

**Best for:** Open world MMOs with distinct zones

```rust
pub struct ZoneShard {
    zone_id: u32,
    server_address: String,
    player_capacity: usize,
    current_players: usize,
}

impl GameWorld {
    fn get_shard_for_zone(&self, zone_id: u32) -> &ZoneShard {
        // Each zone mapped to specific server
        self.zone_to_shard.get(&zone_id).unwrap()
    }

    fn transition_player_to_zone(&mut self, player: PlayerId, target_zone: u32) {
        let target_shard = self.get_shard_for_zone(target_zone);

        // Transfer player to new server
        self.transfer_player(player, target_shard);
    }
}
```

**Pros:**
- Natural boundaries (zones)
- No cross-server visibility needed
- Easy to reason about

**Cons:**
- Uneven load if zones have different popularity
- Hotspot zones need sub-sharding

**Example:**
```
Server 1: Starting Zone (always busy)
Server 2: Mid-game Zones 1-5
Server 3: Mid-game Zones 6-10
Server 4: Endgame Zones
Server 5: Capital City (always busy)
```

---

#### Strategy 2: Dynamic Instancing

**Best for:** Cities, dungeons, raids

```rust
pub struct InstanceManager {
    instances: HashMap<InstanceId, ServerAddress>,
    capacity_per_instance: usize,
}

impl InstanceManager {
    fn get_or_create_instance(&mut self, zone: ZoneId) -> InstanceId {
        // Find instance with room
        for (instance_id, server) in &self.instances {
            if server.current_players < self.capacity_per_instance {
                return *instance_id;
            }
        }

        // All full, create new instance
        let new_instance = self.create_instance(zone);
        new_instance
    }
}
```

**Pros:**
- Automatically scales with demand
- Prevents hotspot overcrowding
- Natural load distribution

**Cons:**
- Players can't see friends in other instances
- Requires instance merge/split logic
- More complex to implement

**Example:**
```
Capital City Instance 1: 500/500 players (full)
Capital City Instance 2: 300/500 players (available)
Capital City Instance 3: 100/500 players (available)

New player → Capital City Instance 2
```

---

#### Strategy 3: Player-Based Sharding

**Best for:** Faction-based games, friend groups

```rust
pub struct PlayerShard {
    shard_id: u32,
    players: HashSet<PlayerId>,
    server_address: String,
}

impl ShardManager {
    fn assign_shard(&self, player: PlayerId) -> u32 {
        // Assign based on:
        // - Faction
        // - Guild
        // - Friends list
        // - Geographic region

        if let Some(guild) = player.guild() {
            return guild.preferred_shard;
        }

        // Default: least loaded shard
        self.least_loaded_shard()
    }
}
```

**Pros:**
- Social groups stay together
- Factions naturally separated
- Reduces cross-shard communication

**Cons:**
- Load imbalance if one faction dominates
- Complex shard assignment logic
- Shard migration tricky (guild moves)

---

### Cross-Server Visibility

#### Option 1: No Cross-Server Visibility (Simplest)

**Implementation:**
```rust
// Each server is independent
// Players on Server 1 never see players on Server 2

manager.set_client_interest(player_id, aoi); // Only sees local entities
```

**Pros:**
- Simplest to implement
- No network overhead
- Scales linearly

**Cons:**
- World feels empty if poorly balanced
- Can't have "megaservers"

**Best for:** Zone-based sharding, instanced content

---

#### Option 2: Cross-Server Queries (Moderate Complexity)

**Implementation:**
```rust
pub struct CrossServerInterestManager {
    local_manager: InterestManager,
    remote_servers: Vec<ServerAddress>,
}

impl CrossServerInterestManager {
    fn calculate_visibility(&self, player_id: PlayerId) -> Vec<Entity> {
        let mut visible = self.local_manager.calculate_visibility(player_id);

        // Query nearby servers for cross-server entities
        for server in &self.remote_servers {
            if self.is_server_nearby(player.position, server) {
                let remote_entities = server.query_entities(player.position, aoi_radius);
                visible.extend(remote_entities);
            }
        }

        visible
    }

    fn is_server_nearby(&self, pos: Vec3, server: &ServerAddress) -> bool {
        // Check if server's zone borders player's position
        server.zone_bounds.intersects(pos, aoi_radius)
    }
}
```

**Pros:**
- Players see entities across server boundaries
- Seamless zone transitions
- Enables "megaservers"

**Cons:**
- Network latency for cross-server queries
- More complex synchronization
- Harder to test

**Best for:** Zone-based sharding with seamless borders

---

#### Option 3: Distributed Spatial Grid (Complex)

**Implementation:**
```rust
pub struct DistributedSpatialGrid {
    local_grid: SpatialGrid,
    remote_grids: HashMap<ServerId, RemoteGridProxy>,
}

// Each server maintains a subset of the world grid
// Queries automatically span local + remote grids
```

**Pros:**
- True "megaserver" experience
- No visible server boundaries
- Transparent to gameplay

**Cons:**
- Very complex to implement
- High network overhead
- Consistency challenges (CAP theorem)

**Best for:** AAA studios with dedicated infrastructure team

---

## Vertical Scaling

### CPU Scaling

#### When to Vertically Scale

**Indicators:**
- High CPU utilization (>80%)
- All cores saturated
- Visibility calc time increasing linearly with players

**Before:**
```
Server: 8 cores
Players: 1000
Visibility calc p95: 1.5ms
CPU utilization: 85%
```

**After:**
```
Server: 16 cores (2x)
Players: 1800 (1.8x)
Visibility calc p95: 1.2ms (improved)
CPU utilization: 75%
```

**Cost-Benefit:**
- Doubling cores ≠ doubling capacity
- Diminishing returns after 16-32 cores
- Horizontal scaling usually more cost-effective

---

### Memory Scaling

#### When to Vertically Scale Memory

**Indicators:**
- Memory usage >80%
- Swapping detected
- Out-of-memory warnings

**Memory Requirements:**

| Component | Memory per Player |
|-----------|-------------------|
| Player state | ~1 KB |
| Visibility cache | ~0.5 KB |
| Spatial grid entry | ~64 bytes |
| Network buffers | ~10 KB |
| **Total** | **~11.5 KB/player** |

**Capacity Planning:**
```
1000 players × 11.5 KB = 11.5 MB (player data)
+ 100 MB (spatial grid overhead)
+ 500 MB (OS + engine)
= ~600 MB minimum

Recommended: 2 GB for 1000 players (3x safety factor)
```

---

## Cost Optimization

### Cloud Provider Comparison

#### AWS (us-east-1)

**c6i.2xlarge** (8 vCPU, 16 GB RAM)
- Price: $0.34/hour = $244.80/month
- Capacity: ~1000 players
- Cost per 1000 players: $244.80/month

**c6i.4xlarge** (16 vCPU, 32 GB RAM)
- Price: $0.68/hour = $489.60/month
- Capacity: ~1800 players
- Cost per 1000 players: $272/month (higher!)

**Verdict:** Horizontal scaling cheaper at this tier

---

#### GCP (us-central1)

**c2-standard-8** (8 vCPU, 32 GB RAM)
- Price: $0.35/hour = $252/month
- Capacity: ~1000 players
- Cost per 1000 players: $252/month

**c2-standard-16** (16 vCPU, 64 GB RAM)
- Price: $0.70/hour = $504/month
- Capacity: ~1800 players
- Cost per 1000 players: $280/month

---

#### Bare Metal (Hetzner Dedicated)

**AX41-NVME** (8 cores, 64 GB RAM, AMD Ryzen)
- Price: €39/month = ~$45/month
- Capacity: ~1200 players (better CPU than cloud)
- Cost per 1000 players: $37.50/month

**Verdict:** 6x cheaper than AWS/GCP!

**Trade-offs:**
- No auto-scaling
- No instant provisioning
- Manual infrastructure management
- Great for established games with predictable load

---

### Bandwidth Costs

#### Without Interest Management

```
1000 players × 1000 entities = 1M updates/tick
60 ticks/sec × 100 bytes/update = 6 GB/sec
6 GB/sec × 3600 sec/hour × 24 hours = ~500 TB/day

AWS bandwidth: $0.09/GB = $45,000/day = $1.35M/month 💸
```

#### With Interest Management (98.6% reduction)

```
1M updates × 1.4% = 14K updates/tick
60 ticks/sec × 100 bytes/update = 84 MB/sec
84 MB/sec × 3600 × 24 = ~7 TB/day

AWS bandwidth: $0.09/GB = $630/day = $18,900/month

Savings: $1.33M/month! 🎉
```

**ROI:** Interest management pays for itself 70x over!

---

## Auto-Scaling Policies

### AWS Auto Scaling Group

```yaml
autoscaling_group:
  name: game-servers
  min_size: 2
  max_size: 20
  desired_capacity: 5

  scaling_policies:
    - name: scale-up-cpu
      metric: CPUUtilization
      threshold: 70
      adjustment: +2 instances
      cooldown: 300 # 5 min

    - name: scale-down-cpu
      metric: CPUUtilization
      threshold: 30
      adjustment: -1 instance
      cooldown: 600 # 10 min (slower scale-down)

    - name: scale-up-players
      metric: InterestTotalClients
      threshold: 800 # 80% of 1000
      adjustment: +1 instance
      cooldown: 300
```

---

### Kubernetes HPA (Horizontal Pod Autoscaler)

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: game-server
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: game-server
  minReplicas: 3
  maxReplicas: 50
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70

    - type: Pods
      pods:
        metric:
          name: interest_total_clients
        target:
          type: AverageValue
          averageValue: "800"
```

---

## Best Practices

### DO

✅ **Start small, scale gradually** - Don't over-provision
✅ **Monitor before scaling** - Use data, not guesses
✅ **Test auto-scaling before launch** - Simulate load spikes
✅ **Set upper bounds** - Prevent runaway scaling costs
✅ **Use spot/preemptible instances** - Save 60-80% on compute
✅ **Consider bare metal** - 6x cheaper for stable load

### DON'T

❌ **Don't scale too late** - Players notice lag before metrics
❌ **Don't forget cooldowns** - Prevent flapping (scale up/down/up)
❌ **Don't scale based on averages** - Use p95/p99 instead
❌ **Don't ignore bandwidth costs** - Can exceed compute costs
❌ **Don't over-shard** - Too many small servers = empty world feel

---

## Scaling Roadmap

### Phase 1: Single Server (0-1000 players)

**Infrastructure:**
- 1 game server (8 cores, 16 GB RAM)
- 1 database
- Simple monitoring

**Cost:** ~$300/month

---

### Phase 2: Horizontal Scaling (1000-5000 players)

**Infrastructure:**
- 5 game servers (load balanced)
- 1 database (read replicas)
- Prometheus + Grafana
- Auto-scaling enabled

**Cost:** ~$1500/month

---

### Phase 3: Regional Expansion (5000-20000 players)

**Infrastructure:**
- 3 regions (US, EU, Asia)
- 5-10 servers per region
- CDN for assets
- Global load balancing

**Cost:** ~$6000/month

---

### Phase 4: Megaserver (20000+ players)

**Infrastructure:**
- Cross-server visibility
- Distributed spatial grid
- Multiple data centers
- Dedicated ops team

**Cost:** ~$20,000+/month (+ headcount)

---

## References

- [AWS Auto Scaling Guide](https://docs.aws.amazon.com/autoscaling/)
- [Kubernetes HPA](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/)
- [MMO Server Architecture (GDC)](https://gdcvault.com/)
- [silmaril Benchmarks](./INTEREST_MANAGEMENT_AAA_COMPARISON.md)

---

**Last Updated:** 2026-02-02
**Version:** 1.0
**Maintained By:** silmaril team
