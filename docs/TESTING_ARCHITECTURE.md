# Testing Architecture

**Date:** 2026-02-01
**Status:** ✅ Enforced via CLAUDE.md and coding-standards.md

---

## 🎯 Overview

The Agent Game Engine uses a **3-tier test hierarchy** to ensure comprehensive coverage while maintaining clear separation of concerns:

1. **Unit Tests** - Test single crate in isolation
2. **Cross-Crate Integration Tests** - Test interaction between multiple crates
3. **End-to-End System Tests** - Test complete game scenarios

This architecture ensures:
- ✅ Fast feedback loops (unit tests run in <1s)
- ✅ Clear dependency boundaries (cross-crate tests catch integration bugs)
- ✅ Production validation (E2E tests verify user scenarios)
- ✅ No circular dependencies (enforced by test location)

---

## 📊 Test Hierarchy

### **Tier 1: Unit Tests** 🔬

**Purpose:** Test individual functions, modules, and crates in isolation

**Location:**
- `engine/{crate}/src/*.rs` - Inline tests using `#[cfg(test)] mod tests`
- `engine/{crate}/tests/*.rs` - Integration tests for single crate

**Dependencies:**
- May only use the crate being tested + helper crates (`engine-math`, test utilities)
- MUST NOT import from other engine crates (e.g., `engine-core`, `engine-physics`)

**Examples:**
```
engine/physics/tests/
├── raycast_tests.rs           ✅ Physics-only (uses engine-math)
├── character_controller_tests.rs  ✅ Physics-only
├── joint_tests.rs             ✅ Physics-only
└── deterministic_tests.rs     ✅ Physics-only
```

**When to Add Unit Tests:**
- Testing physics raycasting logic (doesn't need ECS)
- Testing character controller movement (pure physics)
- Testing joint constraints (pure physics)
- Any functionality that works without other crates

---

### **Tier 2: Cross-Crate Integration Tests** 🔗

**Purpose:** Test interaction between multiple engine crates

**Location:**
- `engine/shared/tests/` - All cross-crate integration tests
- `engine/shared/benches/` - All cross-crate benchmarks

**Dependencies:**
- May use 2+ engine crates (e.g., `engine-core` + `engine-physics`)
- Tests integration points between systems
- Verifies components work together correctly

**Examples:**
```
engine/shared/tests/
├── integration_simd_test.rs       ✅ Physics + ECS (World, Transform)
├── prediction_tests.rs            ✅ Physics + ECS (EntityAllocator)
└── future_renderer_ecs_tests.rs   ✅ Renderer + ECS

engine/shared/benches/
├── integration_bench.rs           ✅ Physics + ECS scaling
├── physics_integration_comparison.rs  ✅ SIMD vs scalar with ECS
└── prediction_benches.rs          ✅ Prediction + ECS performance
```

**When to Add Cross-Crate Integration Tests:**
- Testing physics synchronization to ECS components
- Testing SIMD physics updates with ECS `World`
- Testing renderer drawing ECS entities
- Any test that imports from 2+ engine crates

**MANDATORY:** If your test imports from 2+ engine crates, it MUST go in `engine/shared/tests/`.

---

### **Tier 3: End-to-End System Tests** 🎮

**Purpose:** Test complete game scenarios from user perspective

**Location:**
- `examples/` directory (actual runnable games)
- E2E test scripts in `scripts/e2e-tests/`

**Dependencies:**
- Uses all engine systems together
- Runs actual client/server binaries
- Tests user workflows (login, gameplay, matchmaking)

**Examples:**
```
examples/
├── singleplayer/       ✅ Complete singleplayer game
├── mmorpg/            ✅ MMO server + client test
└── moba/              ✅ MOBA match test

scripts/e2e-tests/
├── test-multiplayer-match.sh   ✅ Full match from start to finish
└── test-server-stress.sh       ✅ 1000 concurrent players
```

**When to Add E2E Tests:**
- Testing complete multiplayer matches
- Testing player login → gameplay → logout flow
- Testing server under load (1000+ players)
- Any test that requires actual game binaries running

---

## 🔧 Benchmark Organization

Benchmarks follow the same 3-tier hierarchy:

### **Tier 1: Single-Crate Benchmarks**

**Location:** `engine/{crate}/benches/`

**Examples:**
```
engine/physics/benches/
├── component_benches.rs        ✅ Physics components only
├── character_benches.rs        ✅ Character controller only
├── raycast_benches.rs          ✅ Raycasting only
├── deterministic_benches.rs    ✅ Determinism overhead
└── joint_benches.rs            ✅ Joint performance
```

---

### **Tier 2: Cross-Crate Benchmarks**

**Location:** `engine/shared/benches/`

**Examples:**
```
engine/shared/benches/
├── integration_bench.rs               ✅ Physics + ECS scaling
├── physics_integration_comparison.rs  ✅ SIMD vs scalar with ECS
└── prediction_benches.rs              ✅ Prediction + ECS
```

**MANDATORY:** If your benchmark uses 2+ engine crates, it MUST go in `engine/shared/benches/`.

---

## 📁 Directory Structure

```
agent-game-engine/
├── engine/
│   ├── core/
│   │   ├── src/           # Unit tests inline
│   │   └── tests/         # Core-only integration tests
│   │
│   ├── physics/
│   │   ├── src/           # Unit tests inline
│   │   ├── tests/         # Physics-only integration tests
│   │   │   ├── raycast_tests.rs         ✅ Physics-only
│   │   │   ├── joint_tests.rs           ✅ Physics-only
│   │   │   └── character_controller_tests.rs  ✅ Physics-only
│   │   └── benches/       # Physics-only benchmarks
│   │       ├── character_benches.rs     ✅ Physics-only
│   │       └── raycast_benches.rs       ✅ Physics-only
│   │
│   ├── renderer/
│   │   ├── src/           # Unit tests inline
│   │   ├── tests/         # Renderer-only integration tests
│   │   └── benches/       # Renderer-only benchmarks
│   │
│   └── shared/            # ⭐ CROSS-CRATE TESTS HERE
│       ├── src/lib.rs     # Shared test infrastructure
│       ├── tests/         # ⭐ ALL cross-crate integration tests
│       │   ├── integration_simd_test.rs      ✅ Physics + ECS
│       │   ├── prediction_tests.rs           ✅ Physics + ECS
│       │   └── future_renderer_ecs_test.rs   ✅ Renderer + ECS
│       └── benches/       # ⭐ ALL cross-crate benchmarks
│           ├── integration_bench.rs          ✅ Physics + ECS
│           ├── physics_integration_comparison.rs  ✅ SIMD + ECS
│           └── prediction_benches.rs         ✅ Prediction + ECS
│
├── examples/              # E2E system tests (runnable games)
│   ├── singleplayer/
│   ├── mmorpg/
│   └── moba/
│
└── scripts/
    └── e2e-tests/         # E2E test scripts
        ├── test-multiplayer-match.sh
        └── test-server-stress.sh
```

---

## ✅ Decision Tree: Where Should My Test Go?

```
Does your test import from 2+ engine crates?
│
├─ YES → engine/shared/tests/
│   Examples:
│   - Uses engine-core + engine-physics
│   - Uses engine-renderer + engine-core
│   - Tests interaction between systems
│
└─ NO → Does it test a complete game scenario?
    │
    ├─ YES → examples/ or scripts/e2e-tests/
    │   Examples:
    │   - Runs actual client/server binaries
    │   - Tests full multiplayer match
    │   - Tests login → gameplay → logout
    │
    └─ NO → engine/{crate}/tests/
        Examples:
        - Tests raycast logic (physics-only)
        - Tests renderer shader compilation (renderer-only)
        - Tests ECS queries (core-only)
```

---

## 🚨 Enforcement Rules

### **MANDATORY:** Cross-Crate Test Placement

**Rule:** Any test that imports from 2+ engine crates MUST be in `engine/shared/tests/`

**Violation Example:**
```rust
// ❌ FORBIDDEN - Cross-crate test in single-crate location
// File: engine/physics/tests/physics_ecs_integration.rs

use engine_core::ecs::World;        // ❌ Imports engine-core
use engine_physics::PhysicsWorld;   // ❌ Uses multiple crates

#[test]
fn test_physics_sync_to_ecs() {
    // This MUST be in engine/shared/tests/
}
```

**Correct:**
```rust
// ✅ CORRECT - Cross-crate test in shared location
// File: engine/shared/tests/physics_ecs_integration.rs

use engine_core::ecs::World;
use engine_physics::PhysicsWorld;

#[test]
fn test_physics_sync_to_ecs() {
    // Now in correct location
}
```

---

### **MANDATORY:** Cross-Crate Benchmark Placement

**Rule:** Any benchmark that uses 2+ engine crates MUST be in `engine/shared/benches/`

**Violation Example:**
```rust
// ❌ FORBIDDEN - Cross-crate benchmark in single-crate location
// File: engine/physics/benches/physics_ecs_bench.rs

use criterion::Criterion;
use engine_core::ecs::World;        // ❌ Imports engine-core
use engine_physics::PhysicsWorld;   // ❌ Uses multiple crates

fn bench_physics_ecs(c: &mut Criterion) {
    // This MUST be in engine/shared/benches/
}
```

**Correct:**
```rust
// ✅ CORRECT - Cross-crate benchmark in shared location
// File: engine/shared/benches/physics_ecs_bench.rs

use criterion::Criterion;
use engine_core::ecs::World;
use engine_physics::PhysicsWorld;

fn bench_physics_ecs(c: &mut Criterion) {
    // Now in correct location
}
```

---

## 🎯 Benefits of This Architecture

### **1. Fast Feedback Loops** ⚡

**Problem:** Large test suites become slow, developers stop running tests

**Solution:**
- Unit tests run in <1s (no dependencies)
- Cross-crate tests run in <10s (minimal dependencies)
- E2E tests run on CI only (slow but comprehensive)

**Result:** Developers run tests frequently, catch bugs early

---

### **2. Clear Dependency Boundaries** 🔒

**Problem:** Circular dependencies break builds, unclear crate boundaries

**Solution:**
- Test location enforces dependency direction
- Can't have cross-crate tests in single-crate location
- Build fails if dependencies are wrong

**Result:** Architecture stays clean, no circular dependencies

---

### **3. Parallel Test Execution** ⚙️

**Problem:** All tests run sequentially, slow CI builds

**Solution:**
```bash
# Run unit tests in parallel (fast)
cargo test --package engine-physics --lib

# Run cross-crate tests in parallel (medium)
cargo test --package engine-shared-tests

# Run E2E tests sequentially (slow)
./scripts/e2e-tests/run-all.sh
```

**Result:** CI builds complete 3x faster

---

### **4. Production Validation** ✅

**Problem:** Tests pass but production breaks (integration bugs)

**Solution:**
- Unit tests verify components work
- Cross-crate tests verify integration
- E2E tests verify user workflows

**Result:** High confidence in production deployments

---

## 📊 Coverage Requirements

### **Unit Tests (Tier 1)**

**Target:** 80% line coverage per crate

**What to Test:**
- All public APIs
- Edge cases (null, empty, max values)
- Error conditions
- Performance-critical paths

**What NOT to Test:**
- Trivial getters/setters
- Auto-generated code
- Private implementation details

---

### **Cross-Crate Integration Tests (Tier 2)**

**Target:** All integration points covered

**What to Test:**
- Data flow between crates (physics → ECS sync)
- Performance of integrated systems
- Error propagation across boundaries
- Resource sharing (ECS entities used by physics)

**What NOT to Test:**
- Single-crate functionality (that's Tier 1)
- Complete game scenarios (that's Tier 3)

---

### **E2E System Tests (Tier 3)**

**Target:** All user workflows covered

**What to Test:**
- Player login → gameplay → logout
- Multiplayer match from start to finish
- Server handling 1000+ concurrent players
- Client reconnection after disconnect

**What NOT to Test:**
- Individual component behavior (that's Tier 1)
- Integration logic (that's Tier 2)

---

## 🔄 Test Migration Guide

### **Moving Tests to Shared**

If you have a cross-crate test in the wrong location:

**Step 1:** Identify cross-crate tests
```bash
# Find tests that import from multiple crates
grep -r "use engine_core" engine/physics/tests/
grep -r "use engine_renderer" engine/core/tests/
```

**Step 2:** Move to `engine/shared/tests/`
```bash
mv engine/physics/tests/physics_ecs_test.rs engine/shared/tests/
```

**Step 3:** Update `engine/shared/Cargo.toml`
```toml
[[test]]
name = "physics_ecs_test"
path = "tests/physics_ecs_test.rs"
harness = true
```

**Step 4:** Remove from original crate's Cargo.toml (if present)

**Step 5:** Verify build
```bash
cargo test --package engine-shared-tests
```

---

### **Moving Benchmarks to Shared**

Same process as tests:

**Step 1:** Identify cross-crate benchmarks
```bash
grep -r "use engine_core" engine/*/benches/
```

**Step 2:** Move to `engine/shared/benches/`
```bash
mv engine/physics/benches/physics_ecs_bench.rs engine/shared/benches/
```

**Step 3:** Update `engine/shared/Cargo.toml`
```toml
[[bench]]
name = "physics_ecs_bench"
path = "benches/physics_ecs_bench.rs"
harness = false
```

**Step 4:** Verify benchmarks
```bash
cargo bench --package engine-shared-tests
```

---

## 📝 Examples

### **Example 1: Physics-Only Test (Tier 1)**

```rust
// File: engine/physics/tests/raycast_tests.rs
// ✅ CORRECT - Physics-only, no other engine crates

use engine_math::{Vec3, Quat};
use engine_physics::{PhysicsWorld, Collider, RigidBody};

#[test]
fn test_raycast_hits_ground() {
    let mut world = PhysicsWorld::new();
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    let hit = world.raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 20.0);
    assert!(hit.is_some());
}
```

**Why Tier 1?** Only uses `engine-physics` + helper `engine-math`

---

### **Example 2: Cross-Crate Integration Test (Tier 2)**

```rust
// File: engine/shared/tests/physics_ecs_integration.rs
// ✅ CORRECT - Uses engine-core + engine-physics

use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_physics::{PhysicsWorld, RigidBody, Velocity};

#[test]
fn test_physics_syncs_to_ecs() {
    let mut ecs_world = World::new();
    ecs_world.register::<Transform>();
    ecs_world.register::<Velocity>();

    let mut physics = PhysicsWorld::new();

    // Add entity to both worlds
    let entity = ecs_world.spawn();
    ecs_world.add(entity, Transform::identity());
    ecs_world.add(entity, Velocity::new(1.0, 0.0, 0.0));

    // Step physics and sync
    physics.step(0.016);
    physics.sync_to_ecs(&mut ecs_world);

    // Verify ECS got updated
    let transform = ecs_world.get::<Transform>(entity).unwrap();
    assert!(transform.translation.x > 0.0);
}
```

**Why Tier 2?** Uses both `engine-core` AND `engine-physics`

---

### **Example 3: E2E System Test (Tier 3)**

```bash
#!/bin/bash
# File: scripts/e2e-tests/test-multiplayer-match.sh
# ✅ CORRECT - Tests complete game scenario

# Start server
cargo run --bin server &
SERVER_PID=$!

# Wait for server to be ready
sleep 2

# Start 2 clients
cargo run --bin client -- --player-name "Player1" &
CLIENT1_PID=$!

cargo run --bin client -- --player-name "Player2" &
CLIENT2_PID=$!

# Wait for match to complete
sleep 30

# Check both clients connected successfully
if grep -q "Match completed" client1.log && grep -q "Match completed" client2.log; then
    echo "✅ E2E test passed"
    exit 0
else
    echo "❌ E2E test failed"
    exit 1
fi

# Cleanup
kill $SERVER_PID $CLIENT1_PID $CLIENT2_PID
```

**Why Tier 3?** Tests complete user workflow with actual binaries

---

## 🎖️ Summary

**3-Tier Test Hierarchy:**
1. **Unit Tests** (`engine/{crate}/tests/`) - Single crate only
2. **Cross-Crate Integration** (`engine/shared/tests/`) - 2+ crates
3. **E2E System Tests** (`examples/`, `scripts/e2e-tests/`) - Complete scenarios

**Enforcement:**
- ✅ Build fails if dependencies are wrong
- ✅ CLAUDE.md mandates test placement
- ✅ Code review checks test location
- ✅ CI verifies architecture

**Benefits:**
- ⚡ Fast feedback loops (unit tests <1s)
- 🔒 Clear dependency boundaries
- ⚙️ Parallel test execution
- ✅ Production validation

---

**This architecture is MANDATORY for all engine development.** See CLAUDE.md for enforcement details.
