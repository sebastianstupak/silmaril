# Quick Start - ECS + Rendering Integration Tests

## Run Tests Immediately

```bash
# Change to project root
cd D:\dev\agent-game-engine

# Run all non-Vulkan tests (fastest)
cargo test --package engine-shared-tests --test ecs_rendering_integration -- --skip "Requires Vulkan"

# Run all tests (including Vulkan)
cargo test --package engine-shared-tests --test ecs_rendering_integration

# Run single test
cargo test --package engine-shared-tests --test ecs_rendering_integration test_entity_spawn_and_query
```

## Test Categories

| Category | Count | Run Time | Description |
|----------|-------|----------|-------------|
| Entity Lifecycle | 7 | < 1s | Spawn, despawn, updates |
| Edge Cases | 7 | < 1s | Zero scale, negative scale, etc. |
| Performance | 3 | ~2-3s | 10,000 entities |
| GPU Tests | 3 | Variable | Requires Vulkan device |
| Stress Tests | 2 | ~5-10s | Entity/component churn |

## Quick Test Examples

### Test 1: Basic Entity Spawning
```bash
cargo test --package engine-shared-tests --test ecs_rendering_integration test_entity_spawn_and_query
```
**What it tests:** Entity spawning, component queries, data integrity

### Test 2: Edge Case - Zero Scale
```bash
cargo test --package engine-shared-tests --test ecs_rendering_integration test_zero_scale_entity
```
**What it tests:** Entities with zero scale are queryable

### Test 3: Performance - Large Entity Count
```bash
cargo test --package engine-shared-tests --test ecs_rendering_integration test_large_entity_count_queries
```
**What it tests:** Query 10,000 entities efficiently

### Test 4: Stress - Entity Churn
```bash
cargo test --package engine-shared-tests --test ecs_rendering_integration test_entity_churn
```
**What it tests:** 100,000 spawn/despawn operations

## Expected Output

### Successful Test Run
```
running 19 tests
test test_component_add_remove_churn ... ok
test test_component_removal_during_iteration ... ok
test test_component_removal_updates_queries ... ok
test test_component_storage_efficiency ... ok
test test_documented_rendering_workflow ... ok
test test_entity_archetype_change ... ok
test test_entity_churn ... ok
test test_entity_despawn_removes_from_queries ... ok
test test_entity_despawn_safety ... ok
test test_entity_spawn_and_query ... ok
test test_entity_without_required_components ... ok
test test_extreme_transform_values ... ok
test test_hidden_entities_not_rendered ... ok
test test_large_entity_count_queries ... ok
test test_multiple_entities_with_different_components ... ok
test test_negative_scale_entity ... ok
test test_query_memory_efficiency ... ok
test test_zero_scale_entity ... ok
test test_component_update_reflects_in_queries ... ok

test result: ok. 19 passed; 0 failed; 3 ignored
```

## Troubleshooting

### Error: "Vulkan not available"
**Solution:** Run with `--skip "Requires Vulkan"` flag
```bash
cargo test --package engine-shared-tests --test ecs_rendering_integration -- --skip "Requires Vulkan"
```

### Error: "Package not found"
**Solution:** Ensure you're in the project root directory
```bash
cd D:\dev\agent-game-engine
```

### Tests Running Slow
**Solution:** Run specific category instead of all tests
```bash
# Run only fast tests (lifecycle + edge cases)
cargo test --package engine-shared-tests --test ecs_rendering_integration test_entity
cargo test --package engine-shared-tests --test ecs_rendering_integration test_component
```

## Documentation Files

- **Detailed Docs:** `engine/shared/tests/ECS_RENDERING_TEST_DOCUMENTATION.md`
- **Summary:** `engine/shared/tests/ECS_RENDERING_TEST_SUMMARY.md`
- **This File:** `engine/shared/tests/QUICK_START.md`

## Need Help?

1. Read `ECS_RENDERING_TEST_DOCUMENTATION.md` for test details
2. Check `ECS_RENDERING_TEST_SUMMARY.md` for overview
3. See `CLAUDE.md` for project guidelines
4. See `docs/TESTING_ARCHITECTURE.md` for testing rules
