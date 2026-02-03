//! Real-world game scenario benchmarks
//!
//! Measures audio system performance under realistic game loads:
//! - FPS game: Gunshots, footsteps, ambient, music
//! - MMO: 100+ players, ambient sounds, UI sounds
//! - Racing game: Engine sounds, tire screeches, wind, collisions
//! - Battle royale: Distant gunfire, close combat, zone effects
//! - Horror game: Ambient, jump scares, whispers
//!
//! Performance targets:
//! - FPS scenario: < 2ms total audio time per frame
//! - MMO scenario: < 5ms total audio time per frame
//! - Racing scenario: < 2ms total audio time per frame
//! - Battle royale scenario: < 3ms total audio time per frame
//! - Horror scenario: < 1.5ms total audio time per frame

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::{AudioEngine, EchoEffect, FilterEffect, ReverbEffect};
use glam::Vec3;
use std::time::{Duration, Instant};

/// FPS game scenario (e.g., CS:GO, Call of Duty)
///
/// Audio sources per frame:
/// - 2-5 gunshots (high priority, 3D positioned)
/// - 10-20 footsteps (player + nearby enemies)
/// - 3-5 ambient sounds (wind, debris, environmental)
/// - 1 background music (streamed)
/// - 2-4 UI sounds (HUD updates, reload notifications)
fn bench_fps_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("fps_scenario");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Simulate 60 frames (1 second at 60fps)
        b.iter(|| {
            let start = Instant::now();

            for frame in 0..60 {
                // Gunshots (2-5 per frame, varies)
                let gunshot_count = 2 + (frame % 4);
                for i in 0..gunshot_count {
                    let pos =
                        Vec3::new((i as f32 * 10.0) - 20.0, 1.5, (frame as f32 * 0.5).sin() * 15.0);
                    engine.update_emitter_position(100 + i, pos);
                }

                // Footsteps (15 average)
                for i in 0..15 {
                    let pos = Vec3::new(
                        (i as f32 * 5.0) - 35.0,
                        0.0,
                        (frame as f32 * 0.3 + i as f32).cos() * 10.0,
                    );
                    engine.update_emitter_position(200 + i, pos);
                }

                // Ambient sources (4)
                for i in 0..4 {
                    let pos = Vec3::new((i as f32 * 20.0) - 30.0, 5.0, (i as f32 * 15.0) - 30.0);
                    engine.update_emitter_position(300 + i, pos);
                }

                // UI sounds (3 average)
                for i in 0..3 {
                    // UI sounds are 2D, no position update needed
                    let _ = i; // Suppress unused warning
                }

                // Update listener (camera)
                engine.set_listener_transform(
                    Vec3::new(0.0, 1.8, 0.0),
                    Vec3::new((frame as f32 * 0.1).sin(), 0.0, (frame as f32 * 0.1).cos()),
                    Vec3::new(0.0, 1.0, 0.0),
                );

                // Cleanup finished sounds
                engine.cleanup_finished();
            }

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.bench_function("per_frame_breakdown", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let total_start = Instant::now();

            // Gunshots
            let gunshots_start = Instant::now();
            for i in 0..3 {
                let pos = Vec3::new((i as f32 * 10.0) - 10.0, 1.5, 5.0);
                engine.update_emitter_position(100 + i, pos);
            }
            let gunshots_time = gunshots_start.elapsed();

            // Footsteps
            let footsteps_start = Instant::now();
            for i in 0..15 {
                let pos = Vec3::new((i as f32 * 3.0) - 20.0, 0.0, i as f32 * 2.0);
                engine.update_emitter_position(200 + i, pos);
            }
            let footsteps_time = footsteps_start.elapsed();

            // Ambient
            let ambient_start = Instant::now();
            for i in 0..4 {
                let pos = Vec3::new((i as f32 * 15.0) - 20.0, 3.0, 10.0);
                engine.update_emitter_position(300 + i, pos);
            }
            let ambient_time = ambient_start.elapsed();

            // Listener update
            let listener_start = Instant::now();
            engine.set_listener_transform(
                Vec3::new(0.0, 1.8, 0.0),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            let listener_time = listener_start.elapsed();

            // Cleanup
            let cleanup_start = Instant::now();
            engine.cleanup_finished();
            let cleanup_time = cleanup_start.elapsed();

            let total_time = total_start.elapsed();

            black_box((
                gunshots_time,
                footsteps_time,
                ambient_time,
                listener_time,
                cleanup_time,
                total_time,
            ));
        });
    });

    group.finish();
}

/// MMO scenario (e.g., World of Warcraft, Final Fantasy XIV)
///
/// Audio sources:
/// - 100+ player positions (many with footsteps, combat sounds)
/// - 20+ ambient environmental sounds
/// - 10+ UI sounds (chat, loot, notifications)
/// - Background music (streamed)
/// - NPC dialogue and environmental effects
fn bench_mmo_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("mmo_scenario");
    group.measurement_time(Duration::from_secs(5));

    for player_count in [50, 100, 200, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("players", player_count),
            player_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Simulate 60 frames
                b.iter(|| {
                    let start = Instant::now();

                    for frame in 0..60 {
                        // Update all player positions
                        for i in 0..count {
                            let angle = (i as f32 * 2.0 * std::f32::consts::PI) / count as f32;
                            let radius = 50.0 + (frame as f32 * 0.1).sin() * 10.0;
                            let pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                            engine.update_emitter_position(i, pos);
                        }

                        // Ambient environmental sounds (20)
                        for i in 0..20 {
                            let pos = Vec3::new(
                                (i as f32 * 10.0) - 100.0,
                                5.0,
                                (i as f32 * 10.0) - 100.0,
                            );
                            engine.update_emitter_position(10000 + i, pos);
                        }

                        // Update listener
                        engine.set_listener_transform(
                            Vec3::new(0.0, 2.0, 0.0),
                            Vec3::new(0.0, 0.0, -1.0),
                            Vec3::new(0.0, 1.0, 0.0),
                        );

                        engine.cleanup_finished();
                    }

                    let elapsed = start.elapsed();
                    black_box(elapsed);
                });
            },
        );
    }

    group.finish();
}

/// Racing game scenario (e.g., Forza, Gran Turismo)
///
/// Audio sources:
/// - 20 cars (engine sounds with Doppler effect)
/// - Tire screeches (varies with cornering)
/// - Wind/aerodynamic sounds
/// - Collision sounds
/// - Environmental sounds (crowd, track ambience)
fn bench_racing_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("racing_scenario");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("20_cars", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            for frame in 0..60 {
                // 20 cars racing on track
                for i in 0..20 {
                    // Cars follow circular track
                    let angle = (i as f32 * 18.0 + frame as f32 * 2.0).to_radians();
                    let radius = 100.0;
                    let pos = Vec3::new(angle.cos() * radius, 0.5, angle.sin() * radius);
                    engine.update_emitter_position(i, pos);

                    // Calculate velocity for Doppler effect
                    let velocity = Vec3::new(
                        -angle.sin() * 50.0, // ~180 km/h
                        0.0,
                        angle.cos() * 50.0,
                    );

                    // Doppler pitch shift (simplified)
                    let speed = velocity.length();
                    let pitch = 1.0 + (speed / 343.0) * 0.1; // Simplified Doppler
                    engine.set_pitch(i as u64, pitch);
                }

                // Tire screeches (active on 5 cars cornering hard)
                for i in 20..25 {
                    let angle = (i as f32 * 18.0 + frame as f32 * 2.0).to_radians();
                    let radius = 95.0; // Inner track
                    let pos = Vec3::new(angle.cos() * radius, 0.1, angle.sin() * radius);
                    engine.update_emitter_position(100 + i, pos);
                }

                // Environmental sounds (crowd, track)
                for i in 0..10 {
                    let pos = Vec3::new((i as f32 * 30.0) - 150.0, 2.0, 150.0);
                    engine.update_emitter_position(200 + i, pos);
                }

                // Update listener (camera following player car)
                let player_angle = (frame as f32 * 2.0).to_radians();
                engine.set_listener_transform(
                    Vec3::new(player_angle.cos() * 100.0, 5.0, player_angle.sin() * 100.0),
                    Vec3::new(-player_angle.sin(), -0.1, player_angle.cos()),
                    Vec3::new(0.0, 1.0, 0.0),
                );

                engine.cleanup_finished();
            }

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

/// Battle royale scenario (e.g., PUBG, Fortnite)
///
/// Audio sources:
/// - Distant gunfire (20-40 sources at varying distances)
/// - Close combat (5-10 nearby players)
/// - Zone/storm effects (environmental)
/// - Footsteps (10-15 players)
/// - Vehicle sounds (2-5 vehicles)
fn bench_battle_royale_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("battle_royale_scenario");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("full_match", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            for frame in 0..60 {
                // Distant gunfire (30 sources, 100-500m away)
                for i in 0..30 {
                    let distance = 100.0 + (i as f32 * 15.0);
                    let angle = (i as f32 * 12.0 + frame as f32 * 0.5).to_radians();
                    let pos = Vec3::new(angle.cos() * distance, 1.5, angle.sin() * distance);
                    engine.update_emitter_position(i, pos);
                }

                // Close combat (8 nearby players)
                for i in 0..8 {
                    let pos = Vec3::new(
                        (i as f32 * 5.0) - 20.0 + (frame as f32 * 0.2).sin() * 3.0,
                        1.5,
                        (i as f32 * 4.0) - 15.0 + (frame as f32 * 0.3).cos() * 3.0,
                    );
                    engine.update_emitter_position(100 + i, pos);
                }

                // Footsteps (12 players)
                for i in 0..12 {
                    let pos = Vec3::new((i as f32 * 6.0) - 35.0, 0.0, (i as f32 * 5.0) - 30.0);
                    engine.update_emitter_position(200 + i, pos);
                }

                // Vehicle sounds (3 vehicles)
                for i in 0..3 {
                    let pos = Vec3::new(
                        (frame as f32 * 2.0 + i as f32 * 50.0).sin() * 80.0,
                        0.5,
                        (frame as f32 * 2.0 + i as f32 * 50.0).cos() * 80.0,
                    );
                    engine.update_emitter_position(300 + i, pos);
                }

                // Zone/storm effect (environmental)
                engine.update_emitter_position(400, Vec3::new(0.0, 10.0, 0.0));

                // Update listener
                engine.set_listener_transform(
                    Vec3::new(0.0, 1.7, 0.0),
                    Vec3::new((frame as f32 * 0.05).sin(), 0.0, (frame as f32 * 0.05).cos()),
                    Vec3::new(0.0, 1.0, 0.0),
                );

                engine.cleanup_finished();
            }

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

/// Horror game scenario (e.g., Resident Evil, Silent Hill)
///
/// Audio sources:
/// - Ambient environmental sounds (10-15 sources with reverb)
/// - Jump scare sounds (sudden loud sounds with effects)
/// - Whispers and distant voices (with filters)
/// - Footsteps (player + 2-3 enemies)
/// - Environmental creaks and groans
fn bench_horror_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("horror_scenario");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("atmospheric", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            for frame in 0..60 {
                // Ambient environmental sounds with reverb (12 sources)
                for i in 0..12 {
                    let pos = Vec3::new(
                        (i as f32 * 8.0) - 45.0 + (frame as f32 * 0.1).sin() * 2.0,
                        2.0 + (frame as f32 * 0.15).cos(),
                        (i as f32 * 7.0) - 40.0,
                    );
                    engine.update_emitter_position(i, pos);
                }

                // Whispers (5 sources, positioned mysteriously)
                for i in 0..5 {
                    let radius = 15.0 + (frame as f32 * 0.2 + i as f32).sin() * 5.0;
                    let angle = (frame as f32 * 0.3 + i as f32 * 1.2).to_radians();
                    let pos = Vec3::new(
                        angle.cos() * radius,
                        1.5 + (frame as f32 * 0.1).sin(),
                        angle.sin() * radius,
                    );
                    engine.update_emitter_position(100 + i, pos);
                }

                // Enemy footsteps (3 enemies)
                for i in 0..3 {
                    let pos = Vec3::new(
                        (i as f32 * 10.0) - 15.0 + (frame as f32 * 0.15).cos() * 3.0,
                        0.0,
                        (i as f32 * 8.0) - 12.0 + (frame as f32 * 0.2).sin() * 3.0,
                    );
                    engine.update_emitter_position(200 + i, pos);
                }

                // Environmental creaks (8 sources)
                for i in 0..8 {
                    let pos = Vec3::new((i as f32 * 12.0) - 45.0, 3.0, (i as f32 * 10.0) - 35.0);
                    engine.update_emitter_position(300 + i, pos);
                }

                // Player footsteps
                engine.update_emitter_position(400, Vec3::new(0.0, 0.0, 0.0));

                // Update listener (player camera, slow movement)
                engine.set_listener_transform(
                    Vec3::new(
                        (frame as f32 * 0.05).sin() * 2.0,
                        1.7,
                        (frame as f32 * 0.05).cos() * 2.0,
                    ),
                    Vec3::new((frame as f32 * 0.08).sin(), 0.0, (frame as f32 * 0.08).cos()),
                    Vec3::new(0.0, 1.0, 0.0),
                );

                engine.cleanup_finished();
            }

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // Jump scare scenario with effects
    group.bench_function("jump_scare_with_effects", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            // Simulate jump scare with multiple effects
            let pos = Vec3::new(2.0, 1.5, -3.0);
            engine.update_emitter_position(0, pos);

            // Add effects (reverb + echo for dramatic impact)
            let reverb = ReverbEffect::cathedral();
            let echo = EchoEffect::long_echo();
            let filter = FilterEffect::muffled();

            black_box((reverb, echo, filter, pos));
        });
    });

    group.finish();
}

/// Comprehensive scenario comparison
fn bench_scenario_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_comparison");
    group.measurement_time(Duration::from_secs(5));

    // Compare single frame performance across all scenarios
    group.bench_function("fps_single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();
        b.iter(|| {
            // 25 total sources (gunshots + footsteps + ambient)
            for i in 0..25 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 1.0, 0.0));
            }
            engine.set_listener_transform(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            engine.cleanup_finished();
        });
    });

    group.bench_function("mmo_single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();
        b.iter(|| {
            // 120 total sources (100 players + 20 ambient)
            for i in 0..120 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
            }
            engine.set_listener_transform(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            engine.cleanup_finished();
        });
    });

    group.bench_function("racing_single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();
        b.iter(|| {
            // 35 total sources (20 cars + 5 tires + 10 environmental)
            for i in 0..35 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 0.5, 0.0));
                if i < 20 {
                    engine.set_pitch(i as u64, 1.0 + (i as f32 * 0.01));
                }
            }
            engine.set_listener_transform(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            engine.cleanup_finished();
        });
    });

    group.bench_function("battle_royale_single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();
        b.iter(|| {
            // 54 total sources (30 distant + 8 close + 12 footsteps + 3 vehicles + 1 zone)
            for i in 0..54 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 1.0, 0.0));
            }
            engine.set_listener_transform(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            engine.cleanup_finished();
        });
    });

    group.bench_function("horror_single_frame", |b| {
        let mut engine = AudioEngine::new().unwrap();
        b.iter(|| {
            // 29 total sources (12 ambient + 5 whispers + 3 enemies + 8 creaks + 1 player)
            for i in 0..29 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 1.5, 0.0));
            }
            engine.set_listener_transform(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
            engine.cleanup_finished();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fps_scenario,
    bench_mmo_scenario,
    bench_racing_scenario,
    bench_battle_royale_scenario,
    bench_horror_scenario,
    bench_scenario_comparison
);

criterion_main!(benches);
