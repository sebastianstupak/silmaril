# Testing Strategy

> **Comprehensive testing approach for all code**
>
> ⚠️ **All features require: unit + integration + E2E + property tests**

---

## 🎯 **Testing Pyramid**

```
         /\
        /  \  E2E Tests (Slow, High Value)
       /____\
      /      \  Integration Tests
     /________\
    /          \  Unit Tests (Fast, Many)
   /__Property__\
```

---

## 📋 **Test Types**

### **1. Unit Tests** (In same file or `tests/` module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_spawn() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));
    }

    #[test]
    fn test_component_add_remove() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, Transform::default());
        assert!(world.get::<Transform>(entity).is_some());

        world.remove::<Transform>(entity);
        assert!(world.get::<Transform>(entity).is_none());
    }
}
```

**Requirements:**
- Fast (< 1ms each)
- No I/O (mock file system)
- No network
- Isolated (no shared state)

---

### **2. Integration Tests** (In `tests/` directory)

```rust
// engine/renderer/tests/vulkan_init.rs
use agent_game_engine_renderer::*;

#[test]
fn test_vulkan_initialization() {
    let config = RendererConfig::headless();
    let renderer = VulkanRenderer::new(config).unwrap();

    assert!(renderer.is_initialized());
}

#[test]
fn test_render_triangle() {
    let mut renderer = VulkanRenderer::new(RendererConfig::headless()).unwrap();

    let vertices = vec![
        Vertex { pos: [0.0, -0.5, 0.0], color: [1.0, 0.0, 0.0] },
        Vertex { pos: [0.5, 0.5, 0.0], color: [0.0, 1.0, 0.0] },
        Vertex { pos: [-0.5, 0.5, 0.0], color: [0.0, 0.0, 1.0] },
    ];

    let mesh = renderer.upload_mesh(&vertices, &[0, 1, 2]).unwrap();
    let result = renderer.render(&[mesh], &RenderOptions::default()).unwrap();

    assert!(result.color.is_some());
    // Verify triangle actually rendered (pixel check)
}
```

**Requirements:**
- Real dependencies (no mocks)
- Headless where possible
- Platform-independent
- CI runs on all platforms

---

### **3. E2E Tests** (With Docker Compose)

```yaml
# tests/e2e/docker-compose.test.yml
version: '3.8'
services:
  server:
    build: ../../
    command: cargo test --bin server -- --nocapture

  client:
    build: ../../
    environment:
      - SERVER_URL=ws://server:8080
    command: cargo test --bin client -- --nocapture
    depends_on:
      - server
```

```rust
// tests/e2e/multiplayer_test.rs
#[tokio::test]
async fn test_client_server_connection() {
    // Start server
    let server = TestServer::start("0.0.0.0:8080").await;

    // Connect client
    let client = TestClient::connect("ws://localhost:8080").await.unwrap();

    // Verify connection
    assert!(client.is_connected());

    // Test state sync
    server.spawn_entity_with_transform(Transform::from_xyz(1.0, 2.0, 3.0));

    tokio::time::sleep(Duration::from_millis(100)).await;

    let entities = client.get_visible_entities();
    assert_eq!(entities.len(), 1);
}
```

---

### **4. Property-Based Tests** (With `proptest`)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_transform_serialization_roundtrip(
        pos in prop::array::uniform3(-1000.0f32..1000.0),
        rot in prop::array::uniform4(-1.0f32..1.0),
    ) {
        let transform = Transform {
            position: pos.into(),
            rotation: Quat::from_array(rot).normalize(),
            scale: Vec3::ONE,
        };

        let bytes = transform.to_bytes().unwrap();
        let decoded = Transform::from_bytes(&bytes).unwrap();

        assert!((transform.position - decoded.position).length() < 0.001);
    }

    #[test]
    fn test_entity_spawn_unique_ids(entities in prop::collection::vec(0u32..100, 0..1000)) {
        let mut world = World::new();
        let spawned: Vec<Entity> = entities.iter().map(|_| world.spawn()).collect();

        // All IDs must be unique
        let mut ids: Vec<_> = spawned.iter().map(|e| e.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), spawned.len());
    }
}
```

---

## 🎯 **Performance Benchmarks** (With `criterion`)

```rust
// benches/ecs_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_entity_spawn(c: &mut Criterion) {
    c.bench_function("spawn 10k entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            for _ in 0..10_000 {
                black_box(world.spawn());
            }
        });
    });
}

fn bench_query_iteration(c: &mut Criterion) {
    let mut world = World::new();
    for _ in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
    }

    c.bench_function("query 10k transforms", |b| {
        b.iter(|| {
            for (_, transform) in world.query::<&Transform>() {
                black_box(transform);
            }
        });
    });
}

criterion_group!(benches, bench_entity_spawn, bench_query_iteration);
criterion_main!(benches);
```

**Run benchmarks:**
```bash
cargo bench
```

---

## 🔄 **CI Test Matrix**

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-13, macos-14]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --lib --all-features

  integration-tests:
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-13, macos-14]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --tests --all-features

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: docker-compose -f tests/e2e/docker-compose.test.yml up --abort-on-container-exit

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

---

## 📊 **Test Coverage Goals**

| Component | Target | Critical |
|-----------|--------|----------|
| Core ECS | > 90% | > 80% |
| Renderer | > 80% | > 70% |
| Networking | > 85% | > 75% |
| Physics | > 80% | > 70% |
| Overall | > 80% | > 70% |

---

## ✅ **Test Requirements per Feature**

Before merging any feature:

- [ ] Unit tests pass (100%)
- [ ] Integration tests pass (100%)
- [ ] E2E tests pass (if applicable)
- [ ] Property tests added (for serialization/math)
- [ ] Benchmarks added (for performance-critical code)
- [ ] Coverage > 80% for new code
- [ ] All platforms pass CI

---

**Last Updated:** 2026-01-31
