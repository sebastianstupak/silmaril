# Phase 5.6: Comprehensive Benchmark Suite

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 days
**Priority:** High (ensures performance standards)

---

## 🎯 **Objective**

Create a comprehensive benchmark suite to measure and track performance across all engine systems. This ensures the engine meets performance targets and prevents regressions.

**Benchmark Goals:**
- **Comprehensive:** Cover all major systems
- **Reproducible:** Consistent results across runs
- **Automated:** Run in CI/CD pipeline
- **Historical:** Track performance over time
- **Actionable:** Identify bottlenecks and regressions

---

## 📋 **Detailed Tasks**

### **1. Benchmark Infrastructure** (Day 1 Morning)

**File:** `benches/Cargo.toml`

```toml
[package]
name = "silmaril-benchmarks"
version = "0.1.0"
edition = "2021"

[[bench]]
name = "ecs"
harness = false

[[bench]]
name = "rendering"
harness = false

[[bench]]
name = "networking"
harness = false

[[bench]]
name = "physics"
harness = false

[[bench]]
name = "serialization"
harness = false

[dependencies]
silmaril-core = { path = "../engine/core" }
silmaril-networking = { path = "../engine/networking" }
silmaril-rendering = { path = "../engine/rendering" }
criterion = { version = "0.5", features = ["html_reports"] }
rand = "0.8"
glam = "0.24"

[profile.bench]
opt-level = 3
lto = true
codegen-units = 1
```

**Directory Structure:**
```
benches/
├── Cargo.toml
├── ecs.rs
├── rendering.rs
├── networking.rs
├── physics.rs
├── serialization.rs
└── utils/
    ├── mod.rs
    └── fixtures.rs
```

---

### **2. ECS Benchmarks** (Day 1)

**File:** `benches/ecs.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use silmaril_core::prelude::*;
use glam::Vec3;
use rand::Rng;

#[derive(Component, Clone, Copy)]
struct Position(Vec3);

#[derive(Component, Clone, Copy)]
struct Velocity(Vec3);

#[derive(Component, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component, Clone, Copy)]
struct Damage(f32);

/// Benchmark entity allocation
fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/spawn");

    for count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || World::new(),
                    |mut world| {
                        for _ in 0..count {
                            black_box(world.spawn());
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark component addition
fn bench_component_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/add_component");

    for count in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        world.register::<Position>();
                        let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for entity in entities {
                            world.add(entity, Position(Vec3::ZERO));
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark simple queries
fn bench_query_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/query_simple");

    for count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut world = setup_world_with_positions(count);

                b.iter(|| {
                    let mut sum = Vec3::ZERO;
                    for (_, position) in world.query::<&Position>() {
                        sum += position.0;
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complex queries
fn bench_query_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/query_complex");

    for count in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut world = setup_complex_world(count);

                b.iter(|| {
                    for (_, (position, velocity, health)) in
                        world.query::<(&mut Position, &Velocity, &Health)>()
                    {
                        if health.current > 0.0 {
                            position.0 += velocity.0;
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark component removal
fn bench_component_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/remove_component");

    for count in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || setup_world_with_positions(count),
                    |mut world| {
                        for (entity, _) in world.query::<&Position>() {
                            world.remove::<Position>(entity);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark entity despawn
fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/despawn");

    for count in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || setup_complex_world(count),
                    |mut world| {
                        let entities: Vec<_> = world.query::<&Position>()
                            .map(|(entity, _)| entity)
                            .collect();

                        for entity in entities {
                            world.despawn(entity);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark fragmented world (entities with different components)
fn bench_query_fragmented(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs/query_fragmented");

    for count in [1_000, 10_000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut world = setup_fragmented_world(count);

                b.iter(|| {
                    // Query will only match ~25% of entities
                    for (_, (position, velocity)) in world.query::<(&mut Position, &Velocity)>() {
                        position.0 += velocity.0;
                    }
                });
            },
        );
    }

    group.finish();
}

// Helper functions

fn setup_world_with_positions(count: u64) -> World {
    let mut world = World::new();
    world.register::<Position>();

    for _ in 0..count {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(
            rand::random(),
            rand::random(),
            rand::random(),
        )));
    }

    world
}

fn setup_complex_world(count: u64) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();

    for _ in 0..count {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::ZERO));
        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
        world.add(entity, Health {
            current: 100.0,
            max: 100.0,
        });
    }

    world
}

fn setup_fragmented_world(count: u64) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Damage>();

    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let entity = world.spawn();

        // Randomly add components (creates fragmentation)
        if rng.gen_bool(0.5) {
            world.add(entity, Position(Vec3::ZERO));
        }
        if rng.gen_bool(0.5) {
            world.add(entity, Velocity(Vec3::ZERO));
        }
        if rng.gen_bool(0.5) {
            world.add(entity, Health { current: 100.0, max: 100.0 });
        }
        if rng.gen_bool(0.5) {
            world.add(entity, Damage(10.0));
        }
    }

    world
}

criterion_group!(
    benches,
    bench_entity_spawn,
    bench_component_add,
    bench_query_simple,
    bench_query_complex,
    bench_component_remove,
    bench_entity_despawn,
    bench_query_fragmented,
);
criterion_main!(benches);
```

---

### **3. Networking Benchmarks** (Day 1-2)

**File:** `benches/networking.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use silmaril_networking::*;

/// Benchmark packet serialization
fn bench_packet_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("networking/serialize");

    for size in [64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                let data = vec![0u8; size];
                let packet = Packet {
                    sequence: 123,
                    ack: 456,
                    ack_bits: 0xFFFF,
                    data,
                };

                b.iter(|| {
                    black_box(packet.serialize());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark packet deserialization
fn bench_packet_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("networking/deserialize");

    for size in [64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                let data = vec![0u8; size];
                let packet = Packet {
                    sequence: 123,
                    ack: 456,
                    ack_bits: 0xFFFF,
                    data,
                };
                let serialized = packet.serialize();

                b.iter(|| {
                    black_box(Packet::deserialize(&serialized));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark state delta compression
fn bench_state_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("networking/state_delta");

    for entity_count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(entity_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &entity_count| {
                let old_state = create_world_state(entity_count);
                let new_state = create_world_state(entity_count);

                b.iter(|| {
                    black_box(compute_delta(&old_state, &new_state));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark input buffer management
fn bench_input_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("networking/input_buffer");

    group.bench_function("push", |b| {
        let mut buffer = InputBuffer::new(60);

        b.iter(|| {
            buffer.push(PlayerInput {
                sequence: 1,
                movement: Vec3::ZERO,
                look: Vec2::ZERO,
            });
        });
    });

    group.bench_function("reconcile", |b| {
        let mut buffer = InputBuffer::new(60);

        // Fill buffer
        for i in 0..60 {
            buffer.push(PlayerInput {
                sequence: i,
                movement: Vec3::ZERO,
                look: Vec2::ZERO,
            });
        }

        b.iter(|| {
            black_box(buffer.reconcile(30));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_packet_serialize,
    bench_packet_deserialize,
    bench_state_delta,
    bench_input_buffer,
);
criterion_main!(benches);
```

---

### **4. Serialization Benchmarks** (Day 2)

**File:** `benches/serialization.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use silmaril_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone)]
struct GameEntity {
    position: Vec3,
    velocity: Vec3,
    health: f32,
    armor: f32,
    inventory: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Item {
    id: u32,
    name: String,
    quantity: u32,
}

/// Benchmark world serialization
fn bench_world_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/world");

    for entity_count in [10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &entity_count| {
                let world = create_game_world(entity_count);

                b.iter(|| {
                    black_box(world.serialize());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark world deserialization
fn bench_world_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/world_deserialize");

    for entity_count in [10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &entity_count| {
                let world = create_game_world(entity_count);
                let serialized = world.serialize();

                b.iter(|| {
                    black_box(World::deserialize(&serialized));
                });
            },
        );
    }

    group.finish();
}

/// Compare serialization formats
fn bench_serialization_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/formats");

    let entity = GameEntity {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(0.5, 0.0, -0.5),
        health: 100.0,
        armor: 50.0,
        inventory: vec![
            Item { id: 1, name: "Sword".to_string(), quantity: 1 },
            Item { id: 2, name: "Potion".to_string(), quantity: 5 },
        ],
    };

    group.bench_function("bincode", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&entity).unwrap());
        });
    });

    group.bench_function("json", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&entity).unwrap());
        });
    });

    group.bench_function("messagepack", |b| {
        b.iter(|| {
            black_box(rmp_serde::to_vec(&entity).unwrap());
        });
    });

    group.finish();
}

fn create_game_world(entity_count: u64) -> World {
    let mut world = World::new();
    world.register::<GameEntity>();

    for i in 0..entity_count {
        let entity = world.spawn();
        world.add(entity, GameEntity {
            position: Vec3::new(i as f32, 0.0, 0.0),
            velocity: Vec3::ZERO,
            health: 100.0,
            armor: 25.0,
            inventory: vec![],
        });
    }

    world
}

criterion_group!(
    benches,
    bench_world_serialize,
    bench_world_deserialize,
    bench_serialization_formats,
);
criterion_main!(benches);
```

---

### **5. CI Integration** (Day 2-3)

**File:** `.github/workflows/benchmarks.yml`

```yaml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    name: Run Benchmarks
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Run benchmarks
        run: |
          cd benches
          cargo bench --all -- --output-format bencher | tee output.txt

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          name: Rust Benchmarks
          tool: 'cargo'
          output-file-path: benches/output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          # Show alert with commit comment on detecting possible performance regression
          alert-threshold: '200%'
          comment-on-alert: true
          fail-on-alert: true
          alert-comment-cc-users: '@maintainer'

      - name: Upload criterion results
        uses: actions/upload-artifact@v3
        with:
          name: criterion-results
          path: target/criterion/

  compare:
    name: Compare with main
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'

    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0

      - name: Install critcmp
        run: cargo install critcmp

      - name: Run benchmarks (PR)
        run: |
          cd benches
          cargo bench -- --save-baseline pr

      - name: Checkout main
        run: git checkout main

      - name: Run benchmarks (main)
        run: |
          cd benches
          cargo bench -- --save-baseline main

      - name: Compare results
        run: critcmp main pr
```

---

### **6. Performance Dashboard** (Day 3)

**File:** `benches/README.md`

```markdown
# Performance Benchmarks

This directory contains comprehensive benchmarks for the Silmaril.

## Running Benchmarks

Run all benchmarks:
```bash
cd benches
cargo bench
```

Run specific benchmark:
```bash
cargo bench --bench ecs
```

## Benchmark Results

Latest results: [GitHub Pages](https://yourusername.github.io/silmaril/benchmarks/)

### ECS Performance

| Operation | 100 entities | 1,000 entities | 10,000 entities |
|-----------|--------------|----------------|-----------------|
| Spawn | 5 μs | 50 μs | 500 μs |
| Add Component | 8 μs | 80 μs | 800 μs |
| Query (simple) | 2 μs | 20 μs | 200 μs |
| Query (complex) | 5 μs | 50 μs | 500 μs |
| Remove Component | 6 μs | 60 μs | 600 μs |
| Despawn | 10 μs | 100 μs | 1,000 μs |

### Networking Performance

| Operation | 64 bytes | 256 bytes | 1024 bytes |
|-----------|----------|-----------|------------|
| Serialize | 0.5 μs | 0.8 μs | 2.0 μs |
| Deserialize | 0.6 μs | 1.0 μs | 2.5 μs |
| Delta Compression | - | - | 50 μs |

## Performance Targets

### Critical Targets (Must Meet)

- Entity spawn: < 1 μs per entity
- Component add: < 1 μs per component
- Simple query: < 0.5 μs per 1k entities
- Packet serialize: < 10 μs for 1KB

### Optimal Targets (Should Meet)

- Entity spawn: < 0.5 μs per entity
- Component add: < 0.5 μs per component
- Simple query: < 0.2 μs per 1k entities
- Packet serialize: < 5 μs for 1KB

## Regression Detection

CI will fail if performance regresses by more than 200% on any benchmark.

## Profiling

Generate flamegraphs:
```bash
cargo bench --bench ecs -- --profile-time=5
```

View results in `target/criterion/*/profile/flamegraph.svg`
```

---

## ✅ **Acceptance Criteria**

- [ ] ECS benchmarks complete and passing
- [ ] Networking benchmarks complete
- [ ] Serialization benchmarks complete
- [ ] Rendering benchmarks complete (if applicable)
- [ ] Physics benchmarks complete (if applicable)
- [ ] CI runs benchmarks on every PR
- [ ] Performance regression detection works
- [ ] Historical data tracked over time
- [ ] Flamegraph generation available
- [ ] Documentation for running benchmarks
- [ ] All benchmarks meet critical targets
- [ ] Results published to GitHub Pages

---

## 🎯 **Performance Targets**

| System | Operation | Target | Critical |
|--------|-----------|--------|----------|
| ECS | Entity spawn | < 0.5 μs | < 1 μs |
| ECS | Component add | < 0.5 μs | < 1 μs |
| ECS | Simple query (1k) | < 0.2 μs | < 0.5 μs |
| Network | Serialize (1KB) | < 5 μs | < 10 μs |
| Network | Deserialize (1KB) | < 8 μs | < 15 μs |
| Serialization | World (1k entities) | < 5 ms | < 10 ms |

---

## 💡 **Best Practices**

### Benchmark Design

- Use realistic workloads
- Test multiple scales (10, 100, 1k, 10k)
- Warm up before measuring
- Run multiple iterations
- Control for variance
- Measure wall-clock time

### CI Integration

- Run on consistent hardware
- Track results over time
- Alert on regressions
- Compare PR vs main
- Generate reports
- Archive results

### Profiling

- Use flamegraphs for visualization
- Profile in release mode
- Focus on hot paths
- Measure allocations
- Check cache misses
- Analyze assembly

---

**Dependencies:** Phase 1-4 (All engine systems)
**Completes:** Phase 5 (Examples and Documentation)
