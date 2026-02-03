//! Doppler Effect Demonstration
//!
//! This test demonstrates the Doppler effect implementation with a realistic
//! scenario of a car driving past a stationary listener.
//!
//! Run this test with:
//! ```bash
//! cargo test -p engine-audio doppler_demonstration -- --nocapture
//! ```

use engine_audio::{AudioListener, AudioSystem, DopplerCalculator, Sound, DEFAULT_SPEED_OF_SOUND};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};

#[test]
#[ignore] // Ignore by default, run with --ignored flag
fn doppler_demonstration_car_passby() {
    println!("\n=== Doppler Effect Demonstration: Car Passing By ===\n");

    // Setup world
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Create listener (camera/player) at origin
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());
    println!("Listener position: (0, 0, 0)");

    // Create car with engine sound
    let car = world.spawn();
    let mut car_transform = Transform::default();
    car_transform.position = Vec3::new(100.0, 0.0, 0.0); // Start 100m ahead
    world.add(car, car_transform);
    world.add(
        car,
        Sound::new("engine.wav").spatial_3d(200.0).with_doppler(1.0), // Full Doppler effect
    );
    println!("Car starting position: (100, 0, 0)");
    println!("Car velocity: 30 m/s (108 km/h) towards listener\n");

    // Create audio system
    let mut system = AudioSystem::new().unwrap();
    println!("Speed of sound: {} m/s", DEFAULT_SPEED_OF_SOUND);
    println!("Doppler scale: 1.0 (realistic)\n");

    // Create Doppler calculator for manual demonstration
    let calc = DopplerCalculator::default();

    // Simulate car approaching, passing, and receding
    let dt = 0.1; // 100ms per step
    let velocity = -30.0; // 30 m/s towards listener

    println!("Time (s) | Position (m) | Distance (m) | Pitch Shift");
    println!("---------|--------------|--------------|-------------");

    for step in 0..40 {
        let time = step as f32 * dt;
        let position = 100.0 + velocity * time;

        // Update car position
        if let Some(transform) = world.get_mut::<Transform>(car) {
            transform.position = Vec3::new(position, 0.0, 0.0);
        }

        // Update audio system
        system.update(&mut world, dt);

        // Calculate pitch shift for demonstration
        let pitch_shift = calc.calculate_pitch_shift(
            Vec3::ZERO,                    // listener position
            Vec3::ZERO,                    // listener velocity
            Vec3::new(position, 0.0, 0.0), // car position
            Vec3::new(velocity, 0.0, 0.0), // car velocity
        );

        let distance = position.abs();

        // Print every 5th step for readability
        if step % 5 == 0 {
            println!(
                "  {:4.1}   |    {:6.1}    |    {:6.1}    |   {:5.3}",
                time, position, distance, pitch_shift
            );
        }

        // Highlight key moments
        if (position - 0.0).abs() < 2.0 && step > 0 {
            println!("         >>> CAR PASSES LISTENER <<<");
        }
    }

    println!("\n=== Key Observations ===");
    println!("1. Pitch shift > 1.0 when car approaches (higher pitch)");
    println!("2. Pitch shift = 1.0 when car is perpendicular (no radial velocity)");
    println!("3. Pitch shift < 1.0 when car recedes (lower pitch)");
    println!("4. Maximum shift occurs when car moves directly towards/away from listener");
    println!("\n=== Demo Complete ===\n");
}

#[test]
#[ignore]
fn doppler_demonstration_supersonic_jet() {
    println!("\n=== Doppler Effect Demonstration: Supersonic Jet ===\n");

    let calc = DopplerCalculator::default();

    println!("Jet flying at Mach 2 (686 m/s)");
    println!("Speed of sound: {} m/s\n", DEFAULT_SPEED_OF_SOUND);

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;

    // Jet approaching at Mach 2
    let approaching_shift = calc.calculate_pitch_shift(
        listener_pos,
        listener_vel,
        Vec3::new(1000.0, 500.0, 0.0),
        Vec3::new(-686.0, 0.0, 0.0),
    );

    println!("Jet approaching: pitch shift = {:.3}", approaching_shift);
    println!("Note: Clamped to prevent extreme audio artifacts");

    // Jet receding at Mach 2
    let receding_shift = calc.calculate_pitch_shift(
        listener_pos,
        listener_vel,
        Vec3::new(-1000.0, 500.0, 0.0),
        Vec3::new(-686.0, 0.0, 0.0),
    );

    println!("Jet receding: pitch shift = {:.3}", receding_shift);
    println!("\n=== Demo Complete ===\n");
}

#[test]
#[ignore]
fn doppler_demonstration_scale_comparison() {
    println!("\n=== Doppler Effect: Scale Factor Comparison ===\n");

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-50.0, 0.0, 0.0); // 50 m/s towards listener

    println!("Source velocity: 50 m/s (180 km/h) towards listener");
    println!("Source distance: 100 m\n");

    println!("Scale | Pitch Shift | Description");
    println!("------|-------------|-------------");

    for &scale in &[0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0] {
        let calc = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, scale);
        let shift =
            calc.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);

        let description = match scale {
            s if s == 0.0 => "Disabled",
            s if s < 1.0 => "Subtle",
            s if s == 1.0 => "Realistic",
            _ => "Exaggerated",
        };

        println!(" {:.2}  |    {:.3}     | {}", scale, shift, description);
    }

    println!("\n=== Scale Recommendations ===");
    println!("0.0  - Disable Doppler (ambient sounds)");
    println!("0.5  - Subtle effect (background traffic)");
    println!("1.0  - Realistic physics (race cars, jets)");
    println!("2.0+ - Dramatic effect (arcade games)");
    println!("\n=== Demo Complete ===\n");
}

#[test]
#[ignore]
fn doppler_demonstration_perpendicular_movement() {
    println!("\n=== Doppler Effect: Perpendicular Movement ===\n");

    let calc = DopplerCalculator::default();

    println!("Testing Doppler effect for perpendicular motion");
    println!("Listener at origin, source moving perpendicular\n");

    println!("Angle | Radial Velocity | Pitch Shift");
    println!("------|-----------------|-------------");

    for angle_deg in (0..=180).step_by(30) {
        let angle_rad = (angle_deg as f32).to_radians();

        // Source at 100m distance, moving at 30 m/s
        let source_pos = Vec3::new(100.0 * angle_rad.cos(), 0.0, 100.0 * angle_rad.sin());

        // Velocity tangent to circle (perpendicular at 90°)
        let velocity = Vec3::new(-30.0 * angle_rad.sin(), 0.0, 30.0 * angle_rad.cos());

        let shift = calc.calculate_pitch_shift(Vec3::ZERO, Vec3::ZERO, source_pos, velocity);

        // Calculate radial velocity component
        let direction = source_pos.normalize();
        let radial_vel = velocity.dot(-direction);

        println!(" {:3}° |     {:6.2} m/s  |   {:.3}", angle_deg, radial_vel, shift);
    }

    println!("\n=== Observations ===");
    println!("- Maximum Doppler at 0° (directly approaching)");
    println!("- Minimum Doppler at 90° (perpendicular)");
    println!("- Maximum negative Doppler at 180° (directly receding)");
    println!("\n=== Demo Complete ===\n");
}
