# Audio System Debug Guide for AI Agents

This guide explains how to use the audio diagnostics system to verify that audio works correctly.

## Quick Verification

To quickly verify the audio system is working:

```bash
cargo test --package engine-audio e2e_validator
```

Expected output:
```
✅ PASS: Audio system creation
✅ PASS: Listener position update
✅ PASS: Sound entity creation
...
✅ ALL TESTS PASSED
```

## Diagnostic Tools Overview

The audio system provides three main diagnostic tools:

1. **AudioDiagnostics** - State inspection and validation
2. **AudioEventLogger** - Event history and debugging
3. **E2E Validator** - Automated verification suite

## Using AudioDiagnostics

### Basic State Inspection

```rust
use engine_audio::AudioSystem;
use engine_core::ecs::World;

let audio_system = AudioSystem::new()?;
let world = World::new();

// Get number of active sounds
let active_sounds = audio_system.diagnostics().active_sounds_count(&world);
println!("Active sounds: {}", active_sounds);

// Get listener position
if let Some(pos) = audio_system.diagnostics().listener_position(&world) {
    println!("Listener at: ({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z);
}

// Get all sound states
let sound_states = audio_system.diagnostics().get_sound_states(&world);
for state in sound_states {
    println!("Entity {}: '{}' - {}",
        state.entity_id,
        state.sound_name,
        if state.is_playing { "PLAYING" } else { "STOPPED" }
    );
}
```

### Performance Metrics

```rust
let metrics = audio_system.diagnostics().performance_metrics();

println!("Average update time: {:.2}ms", metrics.avg_update_time_ms);
println!("Peak update time: {:.2}ms", metrics.peak_update_time_ms);
println!("Updates: {}", metrics.update_count);

// Check if performance target is met
if metrics.avg_update_time_ms < 1.0 {
    println!("✅ Performance target met");
} else {
    println!("❌ Performance target exceeded");
}
```

### Validation

```rust
// Validate audio state and get issues
let issues = audio_system.validate(&world);

if issues.is_empty() {
    println!("✅ Validation PASS");
} else {
    println!("❌ Validation FAIL: {} issues", issues.len());
    for issue in issues {
        println!("  - {}", issue);
    }
}
```

### Generate Full Report

```rust
// Get comprehensive diagnostic report
let report = audio_system.generate_diagnostic_report(&world);
println!("{}", report);
```

Expected output:
```
=== Audio System Diagnostics ===

Active Sounds: 3 / 5
Engine: 3 active, 10 loaded
Listener Position: (0.00, 1.80, 0.00)
Performance: 0.45ms/frame (avg: 0.42ms, peak: 0.78ms)
✅ PASS Target: <1ms

Sound States:
  ▶ Entity 42: 'footstep.wav' (vol: 0.80, 3D, looping)
     Position: (5.00, 0.00, 0.00)
  ⏸ Entity 43: 'ambient.ogg' (vol: 0.50, 2D, looping)
  ▶ Entity 44: 'gunshot.wav' (vol: 1.00, 3D)
     Position: (10.00, 2.00, 5.00)

✅ Validation: PASS (no issues found)
```

## Using AudioEventLogger

### Query Events

```rust
// Get all events
let events = audio_system.event_logger().events();
println!("Total events: {}", events.len());

// Get last 10 events
let recent = audio_system.event_logger().last_n(10);

// Filter events by entity
let filter = engine_audio::EventFilter::new()
    .with_entity(42);
let entity_events = audio_system.event_logger().query(&filter);

// Filter events by sound name
let filter = engine_audio::EventFilter::new()
    .with_sound_name("footstep.wav");
let sound_events = audio_system.event_logger().query(&filter);

// Filter events by type
let filter = engine_audio::EventFilter::new()
    .with_event_type("Doppler");
let doppler_events = audio_system.event_logger().query(&filter);
```

### Get Formatted Log

```rust
// Get last 20 events as formatted string
let log = audio_system.get_event_log(Some(20));
println!("{}", log);
```

Expected output:
```
=== Audio Event Log ===

Total Events: 45
Current Frame: 30

[  0.000s] Frame      0 | Loaded: 'footstep.wav' from 'assets/footstep.wav'
[  0.016s] Frame      1 | Listener: pos (0.0, 1.8, 0.0), fwd (0.00, 0.00, -1.00), up (0.00, 1.00, 0.00)
[  0.016s] Frame      1 | Play3D: 'footstep.wav' (Entity: 42, ID: 1001, pos: (5.0, 0.0, 0.0), vol: 0.80, loop: false)
[  0.032s] Frame      2 | PosUpdate: Entity 42 -> (5.5, 0.0, 0.0)
[  0.032s] Frame      2 | Pitch: ID 1001 -> 1.023
...
```

## Running E2E Validator

### Full Validation Suite

```bash
cargo test --package engine-audio e2e_validator -- --nocapture
```

This runs all validation tests and prints detailed results:

```
=== Audio System E2E Validation ===

Test 1: Audio System Initialization
✅ PASS: Audio system creation

Test 2: Audio Listener Setup
✅ PASS: Listener position update

Test 3: Sound Entity Creation
✅ PASS: Sound entity creation

Test 4: Position Tracking
✅ PASS: Position tracking

Test 5: Event Logging
✅ PASS: Event logging active
   Events logged: 3

Test 6: Performance Metrics
✅ PASS: Performance tracking
   Average update time: 0.123ms
   Peak update time: 0.456ms

...

=== E2E Validation Summary ===
Total Tests: 12
Passed: 12 (100%)
Failed: 0

✅ ALL TESTS PASSED
```

### Specific Validation Tests

```bash
# Test Doppler effect
cargo test --package engine-audio e2e_doppler_validation -- --nocapture

# Test performance
cargo test --package engine-audio e2e_performance_validation -- --nocapture
```

### Interactive Demo

```bash
cargo test --package engine-audio e2e_interactive_demo -- --ignored --nocapture
```

This runs a live demo that prints diagnostic reports each frame.

## Common Issues and Solutions

### Issue: No active listener found

**Symptom:**
```
❌ WARNING: No active audio listener found
```

**Solution:**
```rust
// Create an active listener
let camera = world.spawn();
world.add(camera, Transform::default());
world.add(camera, AudioListener::new());
```

### Issue: Multiple active listeners

**Symptom:**
```
❌ WARNING: Multiple active audio listeners found (2)
```

**Solution:**
```rust
// Deactivate extra listeners
if let Some(listener) = world.get_mut::<AudioListener>(old_camera) {
    listener.active = false;
}
```

### Issue: Performance target exceeded

**Symptom:**
```
❌ WARNING: Average update time (1.23ms) exceeds target (1ms)
```

**Solution:**
- Reduce number of active sounds
- Increase spatial culling distance
- Disable Doppler effect for distant sounds
- Check for position NaN/Inf values

### Issue: Invalid position (NaN/Inf)

**Symptom:**
```
❌ ERROR: Entity 42 has invalid position (NaN/Inf)
```

**Solution:**
```rust
// Validate positions before setting
let position = calculate_position();
if position.is_finite() {
    transform.position = position;
} else {
    tracing::error!("Invalid position calculated");
}
```

### Issue: Sound claims to be playing but isn't

**Symptom:**
```
❌ WARNING: Sound on entity 42 claims to be playing but engine reports not playing
```

**Solution:**
```rust
// Clear stale instance IDs
audio_system.update(&mut world, delta_time);
audio_system.engine_mut().cleanup_finished();
```

## Agent Workflow

### Step 1: Initialize and validate

```rust
let mut audio_system = AudioSystem::new()?;
let mut world = World::new();
world.register::<Transform>();
world.register::<Sound>();
world.register::<AudioListener>();

// Validate initialization
let issues = audio_system.validate(&world);
assert!(issues.iter().any(|i| i.contains("No active audio listener")));
```

### Step 2: Setup scene

```rust
// Add listener
let camera = world.spawn();
world.add(camera, Transform::default());
world.add(camera, AudioListener::new());

// Add sound
let entity = world.spawn();
let mut transform = Transform::default();
transform.position = Vec3::new(5.0, 0.0, 0.0);
world.add(entity, transform);
world.add(entity, Sound::new("test.wav").spatial_3d(100.0));
```

### Step 3: Run and monitor

```rust
for frame in 0..60 {
    audio_system.update(&mut world, 0.016);

    // Check performance every 10 frames
    if frame % 10 == 0 {
        let metrics = audio_system.diagnostics().performance_metrics();
        println!("Frame {}: {:.2}ms", frame, metrics.last_update_time_ms);
    }
}
```

### Step 4: Verify results

```rust
// Generate final report
let report = audio_system.generate_diagnostic_report(&world);
println!("{}", report);

// Check validation
let issues = audio_system.validate(&world);
assert!(issues.is_empty(), "Validation failed: {:?}", issues);

// Get event log
let log = audio_system.get_event_log(Some(20));
println!("{}", log);
```

## Interpreting Diagnostic Output

### Success Indicators

- ✅ `Validation: PASS (no issues found)`
- ✅ `Performance: <1ms target`
- ✅ `Active listener found`
- ✅ `All sounds have valid positions`

### Warning Indicators

- ⚠️ `Performance approaching target`
- ⚠️ `Many inactive sounds`
- ⚠️ `High event count`

### Error Indicators

- ❌ `No active listener`
- ❌ `Invalid position (NaN/Inf)`
- ❌ `Performance target exceeded`
- ❌ `Multiple active listeners`

## Advanced Debugging

### Custom Event Filters

```rust
use std::time::Duration;

// Get events in time range
let filter = EventFilter::new()
    .with_time_range(
        Duration::from_secs(0),
        Duration::from_secs(1)
    );

// Get all Doppler events for specific entity
let filter = EventFilter::new()
    .with_entity(42)
    .with_event_type("PitchChanged");
```

### Custom Diagnostics

```rust
// Get raw sound states for custom analysis
let states = audio_system.diagnostics().get_sound_states(&world);

// Count by type
let spatial_count = states.iter().filter(|s| s.spatial).count();
let playing_count = states.iter().filter(|s| s.is_playing).count();
let looping_count = states.iter().filter(|s| s.looping).count();

println!("Spatial: {}, Playing: {}, Looping: {}",
    spatial_count, playing_count, looping_count);
```

### Performance Profiling

```rust
// Reset metrics before profiling
audio_system.diagnostics_mut().reset_metrics();

// Run profiling scenario
for _ in 0..1000 {
    audio_system.update(&mut world, 0.016);
}

// Get results
let metrics = audio_system.diagnostics().performance_metrics();
println!("1000 updates:");
println!("  Average: {:.3}ms", metrics.avg_update_time_ms);
println!("  Peak: {:.3}ms", metrics.peak_update_time_ms);
println!("  Total: {:.3}ms", metrics.total_update_time_ms);
```

## Integration Testing

```rust
#[test]
fn test_audio_integration() {
    let mut audio_system = AudioSystem::new().unwrap();
    let mut world = setup_test_world();

    // Run simulation
    for _ in 0..100 {
        audio_system.update(&mut world, 0.016);
    }

    // Verify results
    let report = audio_system.generate_diagnostic_report(&world);
    assert!(report.contains("✅ Validation: PASS"));

    let metrics = audio_system.diagnostics().performance_metrics();
    assert!(metrics.avg_update_time_ms < 1.0,
        "Performance target not met: {:.2}ms", metrics.avg_update_time_ms);
}
```

## CI/CD Integration

```yaml
# In CI pipeline
- name: Validate Audio System
  run: |
    cargo test --package engine-audio e2e_validator
    cargo test --package engine-audio e2e_doppler_validation
    cargo test --package engine-audio e2e_performance_validation
```

## Troubleshooting Checklist

- [ ] Audio system created successfully
- [ ] At least one active listener exists
- [ ] Sound entities have valid Transform components
- [ ] Positions are finite (not NaN/Inf)
- [ ] Performance < 1ms average
- [ ] Events are being logged
- [ ] Validation passes with no errors
- [ ] Diagnostic report generates successfully

## Further Reading

- `engine/audio/src/diagnostics.rs` - Diagnostics implementation
- `engine/audio/src/event_logger.rs` - Event logging implementation
- `engine/audio/tests/e2e_validator.rs` - Validation suite
- `docs/audio.md` - Audio system architecture
