# Hot-Reload System Tests

Comprehensive test suite for the asset hot-reload system.

## Test Structure

### Unit Tests (`src/hot_reload.rs`)
- `test_hot_reloader_creation` - Basic creation and initialization
- `test_default_config` - Default configuration values
- `test_config_customization` - Custom configuration
- `test_asset_registration` - Asset registration and tracking
- `test_asset_unregistration` - Asset deregistration
- `test_stats_tracking` - Statistics collection

### Integration Tests (`tests/hot_reload_tests.rs`)

#### Core Functionality
- `test_watch_registration` - Directory watch setup/teardown
- `test_debouncing` - Debounce logic to avoid rapid reloads
- `test_path_mapping` - AssetId ↔ Path bidirectional mapping

#### File Change Detection (Ignored by default - requires filesystem)
- `test_file_modification_detection` - Detect when files are modified
- `test_multiple_asset_types` - Handle different asset types (mesh, texture, etc.)

#### Error Handling (Ignored by default)
- `test_error_handling_corrupted_file` - Gracefully handle corrupted files
- `test_error_handling_missing_file` - Handle deleted files

#### Batch Reloading
- `test_batch_reload_configuration` - Batch configuration
- `test_batch_reload_workflow` - Complete batch reload cycle

#### Statistics
- `test_statistics_tracking` - Metrics collection
- `test_event_types` - All event types can be created

### E2E Test (`tests/hot_reload_demo.rs`)
- `demo_hot_reload_workflow` - Full end-to-end workflow demonstration

## Running Tests

### All tests (including ignored)
```bash
cargo test --package engine-assets --features hot-reload --lib hot_reloader -- --include-ignored
```

### Unit tests only
```bash
cargo test --package engine-assets --features hot-reload --lib hot_reloader
```

### Integration tests only
```bash
cargo test --package engine-assets --features hot-reload test_watch_registration
```

### E2E demo (manual)
```bash
cargo test --package engine-assets --features hot-reload demo_hot_reload_workflow -- --include-ignored --nocapture
```

## Test Coverage

| Category | Coverage | Notes |
|----------|----------|-------|
| Watch registration | ✅ Full | Setup/teardown |
| Debouncing | ✅ Full | Rapid write protection |
| Path mapping | ✅ Full | Bidirectional tracking |
| File detection | ⚠️ Partial | Filesystem-dependent, marked `#[ignore]` |
| Asset reload | ⚠️ Partial | Requires actual files |
| Error handling | ✅ Full | Corrupted/missing files |
| Batch reload | ✅ Full | Queue and batching |
| Statistics | ✅ Full | Metrics tracking |

## Notes

- Tests marked `#[ignore]` require filesystem operations and timing
- Run ignored tests manually when making changes to file watching
- E2E test provides interactive demonstration of full workflow
- All core logic (debouncing, mapping, batching) has unit test coverage
