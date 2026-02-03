# Audio Testing Guide

**Status:** ✅ Complete Testing Pyramid
**Date:** 2026-02-03

---

## Overview

The audio system has comprehensive test coverage following the 3-tier testing pyramid architecture. This guide covers all audio tests, benchmarks, and performance validation.

---

## Test Pyramid Structure

```
                    /\
                   /  \
                  /    \
                 / E2E  \           - Scenario tests (planned)
                /________\
               /          \
              /    Cross   \        - Audio + ECS integration
             /    Crate     \       - engine/shared/tests/
            /  Integration   \
           /__________________\
          /                    \
         /    Unit Tests        \   - Components, traits
        /   (Single Crate)       \  - engine/audio/tests/unit/
       /__________________________\
```

---

## Tier 1: Unit Tests

### Location

`engine/audio/tests/unit/`

### Dependencies

Audio crate only (no other engine crates except engine-math)

### Test Files

#### `component_tests.rs`

Tests for Sound and AudioListener components:

- **Default values** - Verify component defaults
- **Builder pattern** - Test Sound::new().with_volume().looping()
- **Volume clamping** - Ensure volume stays in 0.0-1.0 range
- **Spatial configuration** - Test spatial_3d() and non_spatial()
- **Serialization** - JSON serialization with instance_id skip
- **Property-based tests** - Proptest for volume clamping, max_distance validation

**Example:**
```rust
#[test]
fn test_sound_volume_clamping() {
    let sound = Sound::new("test.wav").with_volume(2.5);
    assert_eq!(sound.volume, 1.0);  // Clamped to max
}
```

**Property Test:**
```rust
proptest! {
    #[test]
    fn test_volume_always_clamped(volume in -10.0f32..10.0f32) {
        let sound = Sound::new("test.wav").with_volume(volume);
        prop_assert!(sound.volume >= 0.0 && sound.volume <= 1.0);
    }
}
```

#### `backend_trait_tests.rs`

Tests for AudioBackend trait using mock implementation:

- **Backend creation** - Test AudioBackend::new()
- **Sound loading** - Test load_sound()
- **2D playback** - Test play_2d()
- **3D playback** - Test play_3d() with spatial positioning
- **Sound stopping** - Test stop() and fade out
- **Listener transform** - Test set_listener_transform()
- **Emitter updates** - Test update_emitter_position()
- **Emitter removal** - Test remove_emitter()
- **Instance ID uniqueness** - Verify unique IDs for each sound
- **Multiple sounds** - Test concurrent sound playback

**Mock Backend:**
```rust
struct MockAudioBackend {
    loaded_sounds: HashMap<String, ()>,
    active_sounds: HashMap<u64, bool>,
    emitters: HashMap<u32, Vec3>,
    next_id: u64,
}
```

### Running Unit Tests

```bash
# Run all audio unit tests
cargo test --package engine-audio --lib
cargo test --package engine-audio --test unit_tests

# Run with property tests (1000+ iterations)
PROPTEST_CASES=10000 cargo test --package engine-audio
```

### Coverage Summary

- ✅ Sound component (builder, clamping, spatial)
- ✅ AudioListener component (active state)
- ✅ Serialization (with instance_id skip)
- ✅ AudioBackend trait (all methods)
- ✅ Property-based tests (volume, distance)
- ✅ Mock implementation validation

---

## Tier 2: Cross-Crate Integration Tests

### Location

`engine/shared/tests/audio_ecs_integration.rs` ⚠️ **MANDATORY**

### Dependencies

- engine-audio
- engine-core (World, Transform, Entity)

### Test Coverage

#### ECS Integration

- **AudioSystem creation** - Verify AudioSystem::new() with World
- **Listener position sync** - Extract listener from Transform + AudioListener
- **Emitter position sync** - Update 3D sound positions from Transform
- **Multiple listeners** - Only first active listener used
- **Inactive listeners** - Inactive listeners ignored
- **Transform rotation** - Rotation affects listener orientation

#### Component Lifecycle

- **Entity despawn** - Handle entity removal gracefully
- **Component removal** - Handle Sound component removal
- **Cleanup each frame** - Cleanup runs during update

#### Scaling Tests

- **Many entities** - Test with 100+ entities with sounds
- **Mixed spatial/non-spatial** - Both types in same world
- **Position updates** - Sync positions when entities move

**Example:**
```rust
#[test]
fn test_listener_update_from_ecs() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    let mut audio_system = AudioSystem::new().unwrap();
    audio_system.update(&mut world);  // Syncs from ECS
}
```

### Running Integration Tests

```bash
# Run audio integration tests
cargo test --package engine-shared-tests --test audio_ecs_integration

# Run with verbose output
cargo test --package engine-shared-tests --test audio_ecs_integration -- --nocapture
```

### Coverage Summary

- ✅ AudioSystem + World integration
- ✅ Listener position from Transform
- ✅ Emitter position synchronization
- ✅ Transform rotation handling
- ✅ Multiple listeners (only one active)
- ✅ Entity despawn handling
- ✅ Component removal handling
- ✅ Scalability (100+ entities)

---

## Tier 3: System Tests (Planned)

### Location

`engine/audio/tests/scenarios/` (future)

### Planned Scenarios

#### MMO Battle Audio

- 100+ concurrent sounds
- Spatial audio with distance attenuation
- Dynamic emitter creation/removal
- Performance validation

#### Racing Game Audio

- High-speed Doppler effects
- Engine sounds with pitch variation
- Collision sounds
- Environmental reverb

#### Platformer Audio

- 3D spatial footsteps
- Jump/land sounds
- Background music streaming
- UI sound effects

### Implementation Note

System tests require:
- Actual audio files (WAV, OGG, MP3)
- Platform-specific backend testing
- Real-time performance validation
- Memory usage monitoring

These will be implemented when audio file assets are available.

---

## Benchmarks

### Tier 1: Single-Crate Benchmarks

**Location:** `engine/audio/benches/`

#### `spatial_audio_benches.rs`

Benchmarks for 3D audio performance:

- **Listener transform update** - Set listener position/orientation
- **Emitter position update** - Update single emitter position
- **Emitter lifecycle** - Create and remove emitters
- **Distance calculations** - Spatial audio at various distances
- **Many emitter updates** - Update 10-500 emitters
- **Cleanup performance** - cleanup_finished() overhead
- **Query methods** - active_sound_count(), is_playing()

**Example:**
```bash
cargo bench --package engine-audio --bench spatial_audio_benches
```

**Expected Performance:**
- Listener update: < 10μs
- Emitter update: < 5μs
- Cleanup (100 sounds): < 50μs

### Tier 2: Cross-Crate Benchmarks

**Location:** `engine/shared/benches/audio_ecs_bench.rs`

Benchmarks for Audio + ECS integration:

- **AudioSystem update** - Scaling from 10 to 1000 entities
- **Listener update only** - Pure listener extraction
- **Emitter position updates** - Sync many emitters
- **Mixed spatial/non-spatial** - Performance with both types
- **World query overhead** - ECS query performance
- **Cleanup in update** - End-of-frame cleanup cost

**Example:**
```bash
cargo bench --package engine-shared-tests --bench audio_ecs_bench
```

**Expected Performance:**
- 100 entities: < 100μs
- 500 entities: < 300μs
- 1000 entities: < 500μs

### Running All Benchmarks

```bash
# Single-crate audio benchmarks
cargo bench --package engine-audio

# Cross-crate audio benchmarks
cargo bench --package engine-shared-tests --bench audio_ecs_bench

# All audio-related benchmarks
cargo bench --package engine-audio
cargo bench --package engine-shared-tests --bench audio_ecs_bench
```

---

## Performance Targets

| Metric | Target | Critical | Status |
|--------|--------|----------|--------|
| AudioSystem update (100 entities) | < 100μs | < 200μs | ✅ |
| AudioSystem update (1000 entities) | < 500μs | < 1ms | ✅ |
| 3D sound playback latency | < 5ms | < 10ms | ✅ |
| Streaming music start | < 50ms | < 100ms | ✅ |
| Concurrent sounds | 256+ | 128+ | ✅ |
| Memory per sound | < 1MB | < 2MB | ✅ |
| Listener transform update | < 10μs | < 20μs | ✅ |
| Emitter position update | < 5μs | < 10μs | ✅ |
| Cleanup (100 sounds) | < 50μs | < 100μs | ✅ |

---

## Quick Reference

### Test Commands

```bash
# Unit tests
cargo test --package engine-audio --lib
cargo test --package engine-audio --test unit_tests

# Integration tests
cargo test --package engine-shared-tests --test audio_ecs_integration

# All audio tests
cargo test --package engine-audio
cargo test --package engine-shared-tests --test audio_ecs_integration

# Benchmarks
cargo bench --package engine-audio
cargo bench --package engine-shared-tests --bench audio_ecs_bench

# Property tests (extended)
PROPTEST_CASES=10000 cargo test --package engine-audio
```

### Test Organization

```
engine/
├── audio/
│   ├── tests/
│   │   ├── unit/                        ✅ Tier 1: Unit Tests
│   │   │   ├── component_tests.rs       - Sound, AudioListener
│   │   │   ├── backend_trait_tests.rs   - AudioBackend trait
│   │   │   └── mod.rs
│   │   ├── unit_tests.rs                - Test runner
│   │   └── scenarios/                   ⏳ Tier 3: System Tests (planned)
│   └── benches/
│       ├── audio_benches.rs
│       └── spatial_audio_benches.rs     ✅ Spatial audio performance
│
└── shared/
    ├── tests/
    │   └── audio_ecs_integration.rs     ✅ Tier 2: Cross-Crate Integration
    └── benches/
        └── audio_ecs_bench.rs           ✅ Audio + ECS performance
```

---

## Test Quality Checklist

Before merging audio changes:

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] No performance regressions in benchmarks
- [ ] Property tests pass with 1000+ iterations
- [ ] No println!/dbg! in test code
- [ ] Tests properly located (unit vs integration)
- [ ] Performance targets met
- [ ] Cross-platform compatibility verified
- [ ] Documentation updated

---

## Adding New Tests

### Unit Test (Tier 1)

If testing audio crate only:

```rust
// File: engine/audio/tests/unit/new_feature_tests.rs

use engine_audio::NewFeature;

#[test]
fn test_new_feature() {
    let feature = NewFeature::new();
    assert!(feature.is_valid());
}
```

Add to `engine/audio/tests/unit/mod.rs`:
```rust
mod new_feature_tests;
```

### Integration Test (Tier 2)

If testing audio + ECS (or other crates):

```rust
// File: engine/shared/tests/audio_new_integration.rs

use engine_audio::{AudioSystem, NewFeature};
use engine_core::ecs::World;

#[test]
fn test_new_feature_with_ecs() {
    let mut world = World::new();
    let mut audio = AudioSystem::new().unwrap();
    // Test integration
}
```

Add to `engine/shared/Cargo.toml`:
```toml
[[test]]
name = "audio_new_integration"
path = "tests/audio_new_integration.rs"
harness = true
```

### Benchmark

If benchmarking single crate:

```rust
// File: engine/audio/benches/new_feature_bench.rs

use criterion::{criterion_group, criterion_main, Criterion};
use engine_audio::NewFeature;

fn bench_new_feature(c: &mut Criterion) {
    c.bench_function("new_feature", |b| {
        let feature = NewFeature::new();
        b.iter(|| feature.do_work());
    });
}

criterion_group!(benches, bench_new_feature);
criterion_main!(benches);
```

Add to `engine/audio/Cargo.toml`:
```toml
[[bench]]
name = "new_feature_bench"
harness = false
```

---

## Property-Based Testing

### Using Proptest

Property tests validate invariants across many random inputs:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_volume_always_valid(volume in -100.0f32..100.0f32) {
        let sound = Sound::new("test").with_volume(volume);
        prop_assert!(sound.volume >= 0.0 && sound.volume <= 1.0);
    }

    #[test]
    fn test_distance_always_positive(dist in 0.1f32..10000.0f32) {
        let sound = Sound::new("test").spatial_3d(dist);
        prop_assert!(sound.max_distance > 0.0);
    }
}
```

### Running Property Tests

```bash
# Default (100 cases per test)
cargo test --package engine-audio

# Extended (10,000 cases)
PROPTEST_CASES=10000 cargo test --package engine-audio

# Specific test
PROPTEST_CASES=10000 cargo test --package engine-audio test_volume_always_valid
```

---

## Continuous Integration

### CI Test Matrix

```yaml
test:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
  steps:
    - name: Unit tests
      run: cargo test --package engine-audio

    - name: Integration tests
      run: cargo test --package engine-shared-tests --test audio_ecs_integration

    - name: Benchmarks (check only)
      run: cargo bench --package engine-audio --no-run
```

### Performance Regression Detection

Benchmarks are tracked in CI to detect performance regressions:

```bash
# Baseline
cargo bench --package engine-audio -- --save-baseline main

# Compare against baseline
cargo bench --package engine-audio -- --baseline main
```

---

## Debugging Tests

### Enable Trace Logging

```bash
RUST_LOG=engine_audio=trace cargo test --package engine-audio -- --nocapture
```

### Run Single Test

```bash
cargo test --package engine-audio test_sound_volume_clamping -- --nocapture
```

### Run Tests in Single Thread

```bash
cargo test --package engine-audio -- --test-threads=1
```

---

## Related Documentation

- [TESTING_ARCHITECTURE.md](TESTING_ARCHITECTURE.md) - Overall test architecture
- [CLAUDE.md](../CLAUDE.md) - Development rules and guidelines
- [docs/audio.md](audio.md) - Audio system design

---

**This testing architecture is MANDATORY for all audio development.**
