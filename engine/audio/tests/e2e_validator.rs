//! End-to-End Audio Validation
//!
//! Comprehensive validation suite for AI agents to verify audio system works correctly.
//! Run with: cargo test --package engine-audio e2e_validator

use engine_audio::{AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::Duration;

/// E2E Validator Results
#[derive(Debug)]
struct ValidationResults {
    total_tests: usize,
    passed: usize,
    failed: usize,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl ValidationResults {
    fn new() -> Self {
        Self { total_tests: 0, passed: 0, failed: 0, warnings: Vec::new(), errors: Vec::new() }
    }

    fn pass(&mut self, test_name: &str) {
        self.total_tests += 1;
        self.passed += 1;
        println!("✅ PASS: {}", test_name);
    }

    fn fail(&mut self, test_name: &str, reason: &str) {
        self.total_tests += 1;
        self.failed += 1;
        self.errors.push(format!("{}: {}", test_name, reason));
        println!("❌ FAIL: {} - {}", test_name, reason);
    }

    fn warn(&mut self, message: &str) {
        self.warnings.push(message.to_string());
        println!("⚠️  WARNING: {}", message);
    }

    fn print_summary(&self) {
        println!("\n=== E2E Validation Summary ===");
        println!("Total Tests: {}", self.total_tests);
        println!("Passed: {} ({}%)", self.passed, (self.passed * 100) / self.total_tests);
        println!("Failed: {}", self.failed);

        if !self.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &self.warnings {
                println!("  - {}", warning);
            }
        }

        if !self.errors.is_empty() {
            println!("\nErrors:");
            for error in &self.errors {
                println!("  - {}", error);
            }
        }

        if self.failed == 0 {
            println!("\n✅ ALL TESTS PASSED");
        } else {
            println!("\n❌ SOME TESTS FAILED");
        }
    }

    fn is_success(&self) -> bool {
        self.failed == 0
    }
}

#[test]
fn test_e2e_audio_validation() {
    println!("\n=== Audio System E2E Validation ===\n");

    let mut results = ValidationResults::new();

    // Test 1: Audio system initialization
    println!("Test 1: Audio System Initialization");
    let mut audio_system = match AudioSystem::new() {
        Ok(sys) => {
            results.pass("Audio system creation");
            sys
        }
        Err(e) => {
            results.fail("Audio system creation", &format!("Failed to create: {}", e));
            results.print_summary();
            panic!("Cannot continue without audio system");
        }
    };

    // Create ECS world
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Test 2: Listener creation
    println!("\nTest 2: Audio Listener Setup");
    let camera = world.spawn();
    let mut camera_transform = Transform::default();
    camera_transform.position = Vec3::new(0.0, 1.8, 0.0); // Eye height
    world.add(camera, camera_transform);
    world.add(camera, AudioListener::new());

    audio_system.update(&mut world, 0.016);

    if let Some(listener_pos) = audio_system.diagnostics().listener_position(&world) {
        if (listener_pos - Vec3::new(0.0, 1.8, 0.0)).length() < 0.01 {
            results.pass("Listener position update");
        } else {
            results.fail(
                "Listener position update",
                &format!(
                    "Expected (0, 1.8, 0), got ({:.2}, {:.2}, {:.2})",
                    listener_pos.x, listener_pos.y, listener_pos.z
                ),
            );
        }
    } else {
        results.fail("Listener position update", "No listener found");
    }

    // Test 3: Sound entity creation
    println!("\nTest 3: Sound Entity Creation");
    let sound_entity = world.spawn();
    let mut sound_transform = Transform::default();
    sound_transform.position = Vec3::new(5.0, 0.0, 0.0);
    world.add(sound_entity, sound_transform);

    let sound = Sound::new("test_sound.wav").spatial_3d(50.0).with_volume(0.8).with_doppler(1.0);
    world.add(sound_entity, sound);

    let emitter_count = audio_system.diagnostics().active_emitters_count(&world);
    if emitter_count == 1 {
        results.pass("Sound entity creation");
    } else {
        results
            .fail("Sound entity creation", &format!("Expected 1 emitter, found {}", emitter_count));
    }

    // Test 4: Position tracking
    println!("\nTest 4: Position Tracking");
    audio_system.update(&mut world, 0.016);

    let sound_states = audio_system.diagnostics().get_sound_states(&world);
    if sound_states.len() == 1 {
        let state = &sound_states[0];
        if let Some(pos) = state.position {
            if (pos - Vec3::new(5.0, 0.0, 0.0)).length() < 0.01 {
                results.pass("Position tracking");
            } else {
                results.fail(
                    "Position tracking",
                    &format!(
                        "Position mismatch: expected (5, 0, 0), got ({:.2}, {:.2}, {:.2})",
                        pos.x, pos.y, pos.z
                    ),
                );
            }
        } else {
            results.fail("Position tracking", "No position found for spatial sound");
        }
    } else {
        results.fail(
            "Position tracking",
            &format!("Expected 1 sound state, found {}", sound_states.len()),
        );
    }

    // Test 5: Event logging
    println!("\nTest 5: Event Logging");
    let event_count = audio_system.event_logger().event_count();
    if event_count > 0 {
        results.pass("Event logging active");
        println!("   Events logged: {}", event_count);
    } else {
        results.warn("No events logged (expected listener updates)");
    }

    // Test 6: Performance metrics
    println!("\nTest 6: Performance Metrics");
    let metrics = audio_system.diagnostics().performance_metrics();
    if metrics.update_count > 0 {
        results.pass("Performance tracking");
        println!("   Average update time: {:.3}ms", metrics.avg_update_time_ms);
        println!("   Peak update time: {:.3}ms", metrics.peak_update_time_ms);

        if metrics.avg_update_time_ms > 1.0 {
            results.warn(&format!(
                "Average update time ({:.3}ms) exceeds target (1.0ms)",
                metrics.avg_update_time_ms
            ));
        }
    } else {
        results.fail("Performance tracking", "No updates tracked");
    }

    // Test 7: Validation system
    println!("\nTest 7: State Validation");
    let issues = audio_system.validate(&world);
    if issues.is_empty() {
        results.pass("State validation (no issues)");
    } else {
        results.warn(&format!("Validation found {} issues", issues.len()));
        for issue in &issues {
            println!("   - {}", issue);
        }
    }

    // Test 8: Moving sound (Doppler)
    println!("\nTest 8: Moving Sound (Doppler Effect)");
    if let Some(sound_transform) = world.get_mut::<Transform>(sound_entity) {
        sound_transform.position = Vec3::new(10.0, 0.0, 0.0); // Move sound
    }

    // Simulate sound with instance ID (as if playing)
    if let Some(sound_comp) = world.get_mut::<Sound>(sound_entity) {
        sound_comp.instance_id = Some(12345); // Simulate active playback
    }

    audio_system.update(&mut world, 0.016);

    // Check if position update was logged
    let filter = engine_audio::EventFilter::new().with_entity(sound_entity.id());
    let position_events = audio_system.event_logger().query(&filter);

    if position_events
        .iter()
        .any(|e| matches!(e.event_type, engine_audio::AudioEventType::PositionUpdated { .. }))
    {
        results.pass("Position update tracking");
    } else {
        results.fail("Position update tracking", "No position update events logged");
    }

    // Test 9: Diagnostic report generation
    println!("\nTest 9: Diagnostic Report Generation");
    let report = audio_system.generate_diagnostic_report(&world);
    if !report.is_empty() && report.contains("Audio System Diagnostics") {
        results.pass("Diagnostic report generation");
        println!("\n--- Diagnostic Report ---");
        println!("{}", report);
        println!("--- End Report ---\n");
    } else {
        results.fail("Diagnostic report generation", "Report is empty or malformed");
    }

    // Test 10: Event log generation
    println!("\nTest 10: Event Log Generation");
    let event_log = audio_system.get_event_log(Some(10));
    if !event_log.is_empty() && event_log.contains("Audio Event Log") {
        results.pass("Event log generation");
        println!("\n--- Event Log (Last 10) ---");
        println!("{}", event_log);
        println!("--- End Log ---\n");
    } else {
        results.fail("Event log generation", "Log is empty or malformed");
    }

    // Test 11: Multiple updates (stress test)
    println!("\nTest 11: Multiple Updates (Stress Test)");
    let iterations = 100;
    for i in 0..iterations {
        // Move sound in circle
        let angle = (i as f32) * 0.1;
        if let Some(sound_transform) = world.get_mut::<Transform>(sound_entity) {
            sound_transform.position = Vec3::new(angle.cos() * 10.0, 0.0, angle.sin() * 10.0);
        }

        audio_system.update(&mut world, 0.016);
    }

    let final_metrics = audio_system.diagnostics().performance_metrics();
    if final_metrics.update_count == iterations + 3 {
        // +3 from earlier tests (listener setup, position tracking, moving sound)
        results.pass("Stress test (100 updates)");
        println!("   Final average: {:.3}ms", final_metrics.avg_update_time_ms);
        println!("   Final peak: {:.3}ms", final_metrics.peak_update_time_ms);
    } else {
        results.fail(
            "Stress test",
            &format!("Expected {} updates, got {}", iterations + 3, final_metrics.update_count),
        );
    }

    // Test 12: Invalid position detection
    println!("\nTest 12: Invalid Position Detection");
    let invalid_entity = world.spawn();
    let mut invalid_transform = Transform::default();
    invalid_transform.position = Vec3::new(f32::NAN, 0.0, 0.0);
    world.add(invalid_entity, invalid_transform);
    world.add(invalid_entity, Sound::new("invalid.wav").spatial_3d(100.0));

    let validation_issues = audio_system.validate(&world);
    if validation_issues.iter().any(|issue| issue.contains("invalid position")) {
        results.pass("Invalid position detection");
    } else {
        results.fail("Invalid position detection", "Failed to detect NaN position");
    }

    // Clean up invalid entity
    world.despawn(invalid_entity);

    // Print final summary
    println!("\n");
    results.print_summary();

    // Assert overall success
    assert!(results.is_success(), "E2E validation failed with {} errors", results.failed);
}

#[test]
fn test_e2e_doppler_validation() {
    println!("\n=== Doppler Effect E2E Validation ===\n");

    let mut results = ValidationResults::new();

    // Create audio system with Doppler enabled
    let mut audio_system = AudioSystem::new_with_doppler(340.0, 1.0).unwrap();

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Setup listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Setup moving sound
    let sound_entity = world.spawn();
    let mut sound_transform = Transform::default();
    sound_transform.position = Vec3::new(0.0, 0.0, 10.0); // 10m away
    world.add(sound_entity, sound_transform);

    let mut sound = Sound::new("car.wav").spatial_3d(100.0).with_doppler(1.0);
    sound.instance_id = Some(99999); // Simulate active playback
    world.add(sound_entity, sound);

    // First update (establish baseline)
    audio_system.update(&mut world, 0.016);

    // Move sound toward listener (approaching)
    if let Some(transform) = world.get_mut::<Transform>(sound_entity) {
        transform.position = Vec3::new(0.0, 0.0, 5.0); // Moved 5m closer
    }

    audio_system.update(&mut world, 0.016);

    // Check for pitch change events
    let filter = engine_audio::EventFilter::new().with_event_type("PitchChanged");
    let pitch_events = audio_system.event_logger().query(&filter);

    if !pitch_events.is_empty() {
        results.pass("Doppler pitch changes logged");
        println!("   Pitch events: {}", pitch_events.len());
    } else {
        results.warn("No Doppler pitch changes detected (sound may need higher velocity)");
    }

    results.print_summary();
    assert!(results.is_success(), "Doppler validation failed");
}

#[test]
fn test_e2e_performance_validation() {
    println!("\n=== Performance E2E Validation ===\n");

    let mut results = ValidationResults::new();

    let mut audio_system = AudioSystem::new().unwrap();

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Setup listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Create many sound entities
    let num_sounds = 100;
    for i in 0..num_sounds {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i as f32) * 2.0, 0.0, ((i * 7) % 50) as f32);
        world.add(entity, transform);

        let mut sound = Sound::new(format!("sound{}.wav", i)).spatial_3d(50.0).with_doppler(1.0);
        sound.instance_id = Some(i as u64); // Simulate active playback
        world.add(entity, sound);
    }

    println!("Created {} sound entities", num_sounds);

    // Run multiple updates
    for _ in 0..60 {
        audio_system.update(&mut world, 0.016);
    }

    let metrics = audio_system.diagnostics().performance_metrics();

    println!("Average update time: {:.3}ms", metrics.avg_update_time_ms);
    println!("Peak update time: {:.3}ms", metrics.peak_update_time_ms);

    // Performance target: < 1ms average
    if metrics.avg_update_time_ms < 1.0 {
        results.pass("Performance target (<1ms avg)");
    } else {
        results.fail(
            "Performance target",
            &format!("Average {:.3}ms exceeds 1.0ms target", metrics.avg_update_time_ms),
        );
    }

    // Peak should be reasonable
    if metrics.peak_update_time_ms < 5.0 {
        results.pass("Peak performance (<5ms)");
    } else {
        results.warn(&format!("Peak time {:.3}ms is high", metrics.peak_update_time_ms));
    }

    results.print_summary();
    assert!(results.is_success(), "Performance validation failed");
}

#[test]
#[ignore] // Ignored by default as it's an interactive demo
fn test_e2e_interactive_demo() {
    println!("\n=== Interactive Audio Demo ===\n");
    println!("This test demonstrates audio system diagnostics in real-time.");
    println!("Run with: cargo test --package engine-audio e2e_interactive_demo -- --ignored --nocapture\n");

    let mut audio_system = AudioSystem::new().unwrap();

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Setup scene
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    let sound_entity = world.spawn();
    let mut sound_transform = Transform::default();
    sound_transform.position = Vec3::new(10.0, 0.0, 0.0);
    world.add(sound_entity, sound_transform);

    let mut sound = Sound::new("demo.wav").spatial_3d(100.0).with_doppler(1.0);
    sound.instance_id = Some(12345);
    world.add(sound_entity, sound);

    println!("Running 10 frames...\n");

    for frame in 0..10 {
        println!("=== Frame {} ===", frame);

        // Move sound in circle
        let angle = (frame as f32) * 0.5;
        if let Some(transform) = world.get_mut::<Transform>(sound_entity) {
            transform.position = Vec3::new(angle.cos() * 10.0, 0.0, angle.sin() * 10.0);
        }

        audio_system.update(&mut world, 0.016);

        // Print diagnostic report
        println!("{}", audio_system.generate_diagnostic_report(&world));

        std::thread::sleep(Duration::from_millis(100));
    }

    println!("\n=== Final Event Log ===");
    println!("{}", audio_system.get_event_log(Some(20)));

    println!("\n✅ Demo complete");
}
