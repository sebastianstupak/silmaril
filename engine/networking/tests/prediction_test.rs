//! Client-side prediction integration tests

use engine_networking::{
    AdaptiveErrorCorrector, ClientPredictor, ErrorCorrector, InputBuffer, PredictionConfig,
};
use glam::{Quat, Vec3};

#[test]
fn test_input_buffer_basic() {
    let mut buffer = InputBuffer::new(100);

    // Add inputs
    let _seq1 = buffer.push_input(100, Vec3::X, Vec3::ZERO, 0);
    let _seq2 = buffer.push_input(110, Vec3::Y, Vec3::ZERO, 0);
    let _seq3 = buffer.push_input(120, Vec3::Z, Vec3::ZERO, 0);

    assert_eq!(buffer.len(), 3);

    // Verify contents
    let pending = buffer.pending_inputs();
    assert_eq!(pending[0].sequence, 0);
    assert_eq!(pending[1].sequence, 1);
    assert_eq!(pending[2].sequence, 2);
}

#[test]
fn test_input_acknowledgement() {
    let mut buffer = InputBuffer::new(100);

    // Add multiple inputs
    let _seq1 = buffer.push_input(100, Vec3::X, Vec3::ZERO, 0);
    let seq2 = buffer.push_input(110, Vec3::Y, Vec3::ZERO, 0);
    let seq3 = buffer.push_input(120, Vec3::Z, Vec3::ZERO, 0);
    let seq4 = buffer.push_input(130, Vec3::X, Vec3::ZERO, 0);

    assert_eq!(buffer.len(), 4);

    // Acknowledge first two
    buffer.acknowledge(seq2);

    assert_eq!(buffer.len(), 2);
    assert_eq!(buffer.pending_inputs()[0].sequence, seq3);
    assert_eq!(buffer.pending_inputs()[1].sequence, seq4);
}

#[test]
fn test_input_buffer_overflow() {
    let mut buffer = InputBuffer::new(10);

    // Add more than capacity
    for i in 0..20 {
        buffer.push_input(i * 10, Vec3::X * i as f32, Vec3::ZERO, 0);
    }

    // Should only keep last 10
    assert_eq!(buffer.len(), 10);

    // Oldest input should be sequence 10 (0-9 were dropped)
    assert_eq!(buffer.pending_inputs()[0].sequence, 10);
}

#[test]
fn test_inputs_from_sequence() {
    let mut buffer = InputBuffer::new(100);

    for i in 0..10 {
        buffer.push_input(i * 10, Vec3::X * i as f32, Vec3::ZERO, 0);
    }

    // Get inputs from sequence 5 onwards
    let inputs = buffer.inputs_from_sequence(5);
    assert_eq!(inputs.len(), 5);
    assert_eq!(inputs[0].sequence, 5);
    assert_eq!(inputs[4].sequence, 9);
}

#[test]
fn test_buffer_clear() {
    let mut buffer = InputBuffer::new(100);

    for i in 0..10 {
        buffer.push_input(i * 10, Vec3::X, Vec3::ZERO, 0);
    }

    assert_eq!(buffer.len(), 10);

    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_client_predictor_initialization() {
    let config = PredictionConfig {
        enabled: true,
        movement_speed: 10.0,
        error_threshold: 0.2,
        max_prediction_time_ms: 1000,
    };

    let predictor = ClientPredictor::new(config);

    assert_eq!(predictor.predicted_position(), Vec3::ZERO);
    assert_eq!(predictor.predicted_rotation(), Quat::IDENTITY);
    assert_eq!(predictor.predicted_velocity(), Vec3::ZERO);
    assert!(predictor.input_buffer().is_empty());
}

#[test]
fn test_prediction_forward_movement() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Process forward movement for 1 second at 60fps
    for _ in 0..60 {
        predictor.process_input(
            0,
            Vec3::new(0.0, 0.0, 1.0), // Forward
            Vec3::ZERO,
            0,
            1.0 / 60.0,
        );
    }

    let predicted_pos = predictor.predicted_position();

    // Should have moved ~5 units forward (speed 5.0 * 1 second)
    // Forward is -Z in our coordinate system
    assert!((predicted_pos.z.abs() - 5.0).abs() < 0.2);
}

#[test]
fn test_prediction_strafe_movement() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Process right movement for 1 second
    for _ in 0..60 {
        predictor.process_input(
            0,
            Vec3::new(1.0, 0.0, 0.0), // Right
            Vec3::ZERO,
            0,
            1.0 / 60.0,
        );
    }

    let predicted_pos = predictor.predicted_position();

    // Should have moved ~5 units right
    assert!((predicted_pos.x.abs() - 5.0).abs() < 0.2);
}

#[test]
fn test_prediction_rotation() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);
    predictor.set_rotation(Quat::IDENTITY);

    // Look right (positive X look delta = yaw right)
    predictor.process_input(
        0,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0), // Look delta
        0,
        1.0 / 60.0,
    );

    let rotation = predictor.predicted_rotation();

    // Should have rotated (not identity anymore)
    assert_ne!(rotation, Quat::IDENTITY);
}

#[test]
fn test_reconciliation_no_error() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Predict movement
    let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

    let predicted_before = predictor.predicted_position();

    // Server confirms our prediction exactly
    predictor.reconcile(seq, predicted_before, Vec3::ZERO, Quat::IDENTITY, 0);

    // Should remain the same (within threshold)
    let predicted_after = predictor.predicted_position();
    assert!((predicted_after - predicted_before).length() < 0.01);

    // Input should be acknowledged
    assert_eq!(predictor.input_buffer().len(), 0);
}

#[test]
fn test_reconciliation_with_small_error() {
    let mut predictor =
        ClientPredictor::new(PredictionConfig { error_threshold: 0.1, ..Default::default() });
    predictor.set_position(Vec3::ZERO);

    // Predict movement
    let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

    let predicted_before = predictor.predicted_position();

    // Server says we're slightly off (beyond threshold)
    let server_pos = predicted_before + Vec3::new(0.0, 0.0, 0.5);

    predictor.reconcile(seq, server_pos, Vec3::ZERO, Quat::IDENTITY, 0);

    // Should correct to match server
    let predicted_after = predictor.predicted_position();
    assert!((predicted_after - server_pos).length() < 0.1);
}

#[test]
fn test_reconciliation_with_large_error() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Predict movement
    let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

    let predicted_before = predictor.predicted_position();

    // Server says we're way off
    let server_pos = predicted_before + Vec3::new(5.0, 0.0, 0.0);

    predictor.reconcile(seq, server_pos, Vec3::ZERO, Quat::IDENTITY, 0);

    // Should snap to server position
    let predicted_after = predictor.predicted_position();
    assert!((predicted_after - server_pos).length() < 0.1);
}

#[test]
fn test_input_replay_after_reconciliation() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Send multiple inputs
    let seq1 = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);
    let _seq2 = predictor.process_input(16, Vec3::X, Vec3::ZERO, 0, 0.016);
    let _seq3 = predictor.process_input(32, Vec3::X, Vec3::ZERO, 0, 0.016);

    assert_eq!(predictor.input_buffer().len(), 3);

    // Server acknowledges first input
    predictor.reconcile(seq1, Vec3::new(0.1, 0.0, 0.0), Vec3::ZERO, Quat::IDENTITY, 32);

    // Should have replayed seq2 and seq3
    assert_eq!(predictor.input_buffer().len(), 2);

    // Position should reflect all three inputs
    let pos = predictor.predicted_position();
    assert!(pos.x > 0.2); // More than just the server position
}

#[test]
fn test_prediction_disabled() {
    let mut predictor =
        ClientPredictor::new(PredictionConfig { enabled: false, ..Default::default() });

    // Process input (should be ignored)
    let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

    assert_eq!(seq, 0);
    assert_eq!(predictor.predicted_position(), Vec3::ZERO);
}

#[test]
fn test_error_corrector_basic() {
    let mut corrector = ErrorCorrector::new(10.0); // 10 units/sec

    // Set 5 unit error
    corrector.set_error(Vec3::new(5.0, 0.0, 0.0));

    assert!(!corrector.is_corrected());
    assert_eq!(corrector.current_error().x, 5.0);

    // Correct over 0.5 seconds (should correct fully)
    let correction = corrector.update(0.5);

    assert!(correction.length() >= 4.9);
    assert!(corrector.is_corrected());
}

#[test]
fn test_error_corrector_partial() {
    let mut corrector = ErrorCorrector::new(10.0);

    corrector.set_error(Vec3::new(10.0, 0.0, 0.0));

    // Correct for 0.1 seconds (should only correct 1 unit)
    let correction = corrector.update(0.1);

    assert!((correction.length() - 1.0).abs() < 0.15);
    assert!(!corrector.is_corrected());
    assert!((corrector.current_error().length() - 9.0).abs() < 0.15);
}

#[test]
fn test_error_corrector_multiple_updates() {
    let mut corrector = ErrorCorrector::new(5.0); // 5 units/sec

    corrector.set_error(Vec3::new(10.0, 0.0, 0.0));

    // Correct over multiple frames (2 seconds total)
    for _ in 0..120 {
        corrector.update(1.0 / 60.0);
    }

    // Should be fully corrected
    assert!(corrector.is_corrected());
    assert!(corrector.current_error().length() < 0.01);
}

#[test]
fn test_error_corrector_reset() {
    let mut corrector = ErrorCorrector::new(10.0);

    corrector.set_error(Vec3::new(5.0, 0.0, 0.0));
    assert!(!corrector.is_corrected());

    corrector.reset();
    assert!(corrector.is_corrected());
    assert_eq!(corrector.current_error(), Vec3::ZERO);
}

#[test]
fn test_adaptive_corrector_small_error() {
    let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

    // Small error (should use base rate)
    corrector.set_error(Vec3::new(0.5, 0.0, 0.0));
    let correction = corrector.update(0.1);

    // Base rate 5.0 * 0.1s = 0.5 units (should fully correct)
    assert!((correction.length() - 0.5).abs() < 0.1);
    assert!(corrector.is_corrected());
}

#[test]
fn test_adaptive_corrector_medium_error() {
    let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

    // Medium error (should use max rate)
    corrector.set_error(Vec3::new(2.0, 0.0, 0.0));
    let correction = corrector.update(0.1);

    // Max rate 20.0 * 0.1s = 2.0 units (should fully correct)
    assert!((correction.length() - 2.0).abs() < 0.1);
}

#[test]
fn test_adaptive_corrector_large_error_snap() {
    let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

    // Large error (should snap)
    corrector.set_error(Vec3::new(10.0, 0.0, 0.0));
    let correction = corrector.update(0.016); // One frame at 60fps

    // Should correct most of it in one frame (snap behavior)
    assert!(correction.length() >= 9.5);
    assert!(corrector.is_corrected());
}

#[test]
fn test_adaptive_corrector_directional() {
    let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

    // Error in multiple directions
    corrector.set_error(Vec3::new(3.0, 4.0, 0.0));

    assert_eq!(corrector.current_error().length(), 5.0);

    let correction = corrector.update(0.25);

    // Should correct in the right direction
    assert!(correction.x > 0.0);
    assert!(correction.y > 0.0);
    assert_eq!(correction.z, 0.0);
}

#[test]
fn test_sequence_number_wrapping() {
    let mut buffer = InputBuffer::new(100);

    // Add many inputs to test sequence increment
    for _ in 0..10 {
        buffer.push_input(0, Vec3::ZERO, Vec3::ZERO, 0);
    }

    // Verify sequence numbers are incrementing
    let inputs = buffer.pending_inputs();
    assert_eq!(inputs[0].sequence, 0);
    assert_eq!(inputs[9].sequence, 9);
}

#[test]
fn test_complex_movement_pattern() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Complex movement: forward, then strafe, then back
    for _ in 0..20 {
        predictor.process_input(0, Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO, 0, 0.016);
    }

    for _ in 0..20 {
        predictor.process_input(16, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 0.016);
    }

    for _ in 0..20 {
        predictor.process_input(32, Vec3::new(0.0, 0.0, -1.0), Vec3::ZERO, 0, 0.016);
    }

    let pos = predictor.predicted_position();

    // Should have moved in X direction (strafe)
    assert!(pos.x > 0.5);

    // Z should be close to 0 (forward then back)
    assert!(pos.z.abs() < 1.0);
}

#[test]
fn test_last_server_state() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Initially no server state
    assert!(predictor.last_server_state().is_none());

    // Process input
    let seq = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);

    // Reconcile with server
    let server_pos = Vec3::new(1.0, 2.0, 3.0);
    let server_vel = Vec3::new(0.1, 0.2, 0.3);
    let server_rot = Quat::from_rotation_y(0.5);

    predictor.reconcile(seq, server_pos, server_vel, server_rot, 0);

    // Should have server state
    let state = predictor.last_server_state().unwrap();
    assert_eq!(state.0, seq);
    assert_eq!(state.1, server_pos);
    assert_eq!(state.2, server_vel);
    assert_eq!(state.3, server_rot);
}

#[test]
fn test_high_frequency_inputs() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Simulate high frequency inputs (144fps)
    let dt = 1.0 / 144.0;
    for _ in 0..144 {
        predictor.process_input(0, Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO, 0, dt);
    }

    let pos = predictor.predicted_position();

    // Should have moved ~5 units (1 second at speed 5)
    assert!((pos.length() - 5.0).abs() < 0.3);
}

#[test]
fn test_zero_movement_input() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::new(10.0, 0.0, 0.0));

    // No movement input
    for _ in 0..60 {
        predictor.process_input(0, Vec3::ZERO, Vec3::ZERO, 0, 1.0 / 60.0);
    }

    let pos = predictor.predicted_position();

    // Should not have moved
    assert!((pos.x - 10.0).abs() < 0.1);
}
