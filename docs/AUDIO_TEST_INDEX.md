# Audio Testing - Quick Reference Index

**For complete details, see [AUDIO_TESTING_GUIDE.md](AUDIO_TESTING_GUIDE.md)**

---

## Quick Commands

```bash
# Run all audio tests
cargo test --package engine-audio
cargo test --package engine-shared-tests --test audio_ecs_integration

# Run benchmarks
cargo bench --package engine-audio
cargo bench --package engine-shared-tests --bench audio_ecs_bench

# Property tests (extended)
PROPTEST_CASES=10000 cargo test --package engine-audio
```

---

## Test Files

### Unit Tests (Tier 1)
- `engine/audio/tests/unit/component_tests.rs` - Sound, AudioListener
- `engine/audio/tests/unit/backend_trait_tests.rs` - AudioBackend trait

### Integration Tests (Tier 2)
- `engine/shared/tests/audio_ecs_integration.rs` - Audio + ECS

### Benchmarks
- `engine/audio/benches/spatial_audio_benches.rs` - Spatial audio
- `engine/shared/benches/audio_ecs_bench.rs` - Audio + ECS

---

## Documentation

- [AUDIO_TESTING_GUIDE.md](AUDIO_TESTING_GUIDE.md) - Comprehensive guide
- [AUDIO_TEST_PYRAMID_SUMMARY.md](AUDIO_TEST_PYRAMID_SUMMARY.md) - Implementation summary
- [TESTING_ARCHITECTURE.md](TESTING_ARCHITECTURE.md) - Overall test architecture

---

## Performance Targets

| Metric | Target |
|--------|--------|
| AudioSystem update (100 entities) | < 100μs |
| AudioSystem update (1000 entities) | < 500μs |
| Listener transform update | < 10μs |
| Emitter position update | < 5μs |

---

## Test Count

- **Unit Tests:** 28+
- **Integration Tests:** 15+
- **Property Tests:** 3+
- **Benchmarks:** 15+

**Total: 60+ tests and benchmarks**
