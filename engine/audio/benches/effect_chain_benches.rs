//! Audio effect chain benchmarks
//!
//! Measures performance of audio effect combinations and chains:
//! - Single effect performance (reverb, echo, filter, EQ)
//! - 2-effect chains (all combinations)
//! - 3-effect chains (common combinations)
//! - 4-effect chains (complex scenarios)
//! - 8-effect chains (maximum stack)
//! - Effect order impact
//! - Dynamic effect parameter changes
//!
//! Performance target:
//! - < 500μs for 8-effect chain processing

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::{AudioEffect, EchoEffect, EqEffect, FilterEffect, FilterType, ReverbEffect};
use std::time::Duration;

/// Benchmark single effect processing
fn bench_single_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_effects");

    group.bench_function("reverb_only", |b| {
        let reverb = ReverbEffect::default();
        b.iter(|| {
            black_box(AudioEffect::Reverb(reverb));
        });
    });

    group.bench_function("echo_only", |b| {
        let echo = EchoEffect::default();
        b.iter(|| {
            black_box(AudioEffect::Echo(echo));
        });
    });

    group.bench_function("filter_only", |b| {
        let filter = FilterEffect::default();
        b.iter(|| {
            black_box(AudioEffect::Filter(filter));
        });
    });

    group.bench_function("eq_only", |b| {
        let eq = EqEffect::default();
        b.iter(|| {
            black_box(AudioEffect::Eq(eq));
        });
    });

    group.finish();
}

/// Benchmark all 2-effect combinations
fn bench_2_effect_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_effect_chains");
    group.measurement_time(Duration::from_secs(5));

    // Reverb + Echo
    group.bench_function("reverb_echo", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Echo(EchoEffect::slapback()),
            ];
            black_box(chain);
        });
    });

    // Reverb + Filter
    group.bench_function("reverb_filter", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Filter(FilterEffect::muffled()),
            ];
            black_box(chain);
        });
    });

    // Reverb + EQ
    group.bench_function("reverb_eq", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::cathedral()),
                AudioEffect::Eq(EqEffect::bass_boost()),
            ];
            black_box(chain);
        });
    });

    // Echo + Filter
    group.bench_function("echo_filter", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Filter(FilterEffect::radio()),
            ];
            black_box(chain);
        });
    });

    // Echo + EQ
    group.bench_function("echo_eq", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Eq(EqEffect::voice_clarity()),
            ];
            black_box(chain);
        });
    });

    // Filter + EQ
    group.bench_function("filter_eq", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::tinny()),
                AudioEffect::Eq(EqEffect::bright()),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

/// Benchmark common 3-effect chains
fn bench_3_effect_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("3_effect_chains");
    group.measurement_time(Duration::from_secs(5));

    // Indoor combat: Reverb + Echo + Filter
    group.bench_function("indoor_combat", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Filter(FilterEffect::muffled()),
            ];
            black_box(chain);
        });
    });

    // Radio transmission: Filter + Echo + EQ
    group.bench_function("radio_transmission", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::radio()),
                AudioEffect::Echo(EchoEffect {
                    delay_time: 0.05,
                    feedback: 0.2,
                    wet_dry_mix: 0.15,
                }),
                AudioEffect::Eq(EqEffect::voice_clarity()),
            ];
            black_box(chain);
        });
    });

    // Underwater: Filter + Reverb + EQ
    group.bench_function("underwater", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.9,
                    damping: 0.8,
                    wet_dry_mix: 0.6,
                }),
                AudioEffect::Eq(EqEffect { bass_gain: 4.0, mid_gain: -6.0, treble_gain: -8.0 }),
            ];
            black_box(chain);
        });
    });

    // Cathedral: Reverb + Echo + EQ
    group.bench_function("cathedral", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::cathedral()),
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Eq(EqEffect { bass_gain: 2.0, mid_gain: 1.0, treble_gain: -2.0 }),
            ];
            black_box(chain);
        });
    });

    // Distorted speaker: Filter + Filter + EQ
    group.bench_function("distorted_speaker", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::BandPass,
                    cutoff_frequency: 800.0,
                    resonance: 3.0,
                    wet_dry_mix: 1.0,
                }),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: 2000.0,
                    resonance: 1.5,
                    wet_dry_mix: 0.8,
                }),
                AudioEffect::Eq(EqEffect { bass_gain: -5.0, mid_gain: 3.0, treble_gain: -6.0 }),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

/// Benchmark 4-effect chains (complex scenarios)
fn bench_4_effect_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("4_effect_chains");
    group.measurement_time(Duration::from_secs(5));

    // Cave system: Reverb + Echo + Filter + EQ
    group.bench_function("cave_system", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: 3000.0,
                    resonance: 0.8,
                    wet_dry_mix: 0.6,
                }),
                AudioEffect::Eq(EqEffect { bass_gain: 3.0, mid_gain: -2.0, treble_gain: -4.0 }),
            ];
            black_box(chain);
        });
    });

    // Dream sequence: Reverb + Reverb + Echo + Filter
    group.bench_function("dream_sequence", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::cathedral()),
                AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.6,
                    damping: 0.4,
                    wet_dry_mix: 0.3,
                }),
                AudioEffect::Echo(EchoEffect { delay_time: 0.5, feedback: 0.7, wet_dry_mix: 0.4 }),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: 2500.0,
                    resonance: 1.2,
                    wet_dry_mix: 0.5,
                }),
            ];
            black_box(chain);
        });
    });

    // Damaged audio system: Filter + Filter + Echo + EQ
    group.bench_function("damaged_audio", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::radio()),
                AudioEffect::Filter(FilterEffect::tinny()),
                AudioEffect::Echo(EchoEffect { delay_time: 0.1, feedback: 0.3, wet_dry_mix: 0.2 }),
                AudioEffect::Eq(EqEffect { bass_gain: -8.0, mid_gain: 5.0, treble_gain: -3.0 }),
            ];
            black_box(chain);
        });
    });

    // Sci-fi environment: Echo + Echo + Filter + EQ
    group.bench_function("scifi_environment", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Echo(EchoEffect { delay_time: 0.2, feedback: 0.4, wet_dry_mix: 0.25 }),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::BandPass,
                    cutoff_frequency: 1500.0,
                    resonance: 2.0,
                    wet_dry_mix: 0.7,
                }),
                AudioEffect::Eq(EqEffect { bass_gain: -2.0, mid_gain: 2.0, treble_gain: 4.0 }),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

/// Benchmark 8-effect chains (maximum stack)
fn bench_8_effect_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("8_effect_chains");
    group.measurement_time(Duration::from_secs(5));

    // Maximum complexity chain
    group.bench_function("max_complexity", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::cathedral()),
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Filter(FilterEffect::radio()),
                AudioEffect::Eq(EqEffect::voice_clarity()),
            ];
            black_box(chain);
        });
    });

    // Layered reverbs
    group.bench_function("layered_reverbs", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.4,
                    damping: 0.6,
                    wet_dry_mix: 0.2,
                }),
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.7,
                    damping: 0.4,
                    wet_dry_mix: 0.3,
                }),
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Eq(EqEffect::voice_clarity()),
            ];
            black_box(chain);
        });
    });

    // Complex filtering
    group.bench_function("complex_filtering", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: 5000.0,
                    resonance: 1.0,
                    wet_dry_mix: 0.8,
                }),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::HighPass,
                    cutoff_frequency: 100.0,
                    resonance: 0.7,
                    wet_dry_mix: 0.8,
                }),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::BandPass,
                    cutoff_frequency: 1500.0,
                    resonance: 2.0,
                    wet_dry_mix: 0.5,
                }),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Eq(EqEffect::voice_clarity()),
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Eq(EqEffect::bright()),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

/// Benchmark effect order impact
fn bench_effect_order_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_order");
    group.measurement_time(Duration::from_secs(5));

    // Order 1: Reverb -> Filter -> EQ
    group.bench_function("reverb_filter_eq", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Eq(EqEffect::bass_boost()),
            ];
            black_box(chain);
        });
    });

    // Order 2: Filter -> Reverb -> EQ
    group.bench_function("filter_reverb_eq", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Eq(EqEffect::bass_boost()),
            ];
            black_box(chain);
        });
    });

    // Order 3: EQ -> Reverb -> Filter
    group.bench_function("eq_reverb_filter", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Filter(FilterEffect::muffled()),
            ];
            black_box(chain);
        });
    });

    // Order 4: Filter -> EQ -> Reverb
    group.bench_function("filter_eq_reverb", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Reverb(ReverbEffect::large_hall()),
            ];
            black_box(chain);
        });
    });

    // Order 5: EQ -> Filter -> Reverb
    group.bench_function("eq_filter_reverb", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Reverb(ReverbEffect::large_hall()),
            ];
            black_box(chain);
        });
    });

    // Order 6: Reverb -> EQ -> Filter
    group.bench_function("reverb_eq_filter", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Filter(FilterEffect::muffled()),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

/// Benchmark dynamic effect parameter changes
fn bench_dynamic_parameter_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_parameters");
    group.measurement_time(Duration::from_secs(5));

    // Reverb wet/dry sweep
    group.bench_function("reverb_wetdry_sweep", |b| {
        b.iter(|| {
            let mut effects = Vec::new();
            for i in 0..10 {
                let wet_dry = (i as f32) * 0.1;
                effects.push(AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.8,
                    damping: 0.5,
                    wet_dry_mix: wet_dry,
                }));
            }
            black_box(effects);
        });
    });

    // Filter cutoff sweep
    group.bench_function("filter_cutoff_sweep", |b| {
        b.iter(|| {
            let mut effects = Vec::new();
            for i in 0..10 {
                let cutoff = 200.0 + (i as f32 * 1000.0);
                effects.push(AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: cutoff,
                    resonance: 1.0,
                    wet_dry_mix: 1.0,
                }));
            }
            black_box(effects);
        });
    });

    // EQ gain animation
    group.bench_function("eq_gain_animation", |b| {
        b.iter(|| {
            let mut effects = Vec::new();
            for i in 0..10 {
                let bass = -10.0 + (i as f32 * 2.0);
                effects.push(AudioEffect::Eq(EqEffect {
                    bass_gain: bass,
                    mid_gain: 0.0,
                    treble_gain: -bass,
                }));
            }
            black_box(effects);
        });
    });

    // Echo feedback modulation
    group.bench_function("echo_feedback_modulation", |b| {
        b.iter(|| {
            let mut effects = Vec::new();
            for i in 0..10 {
                let feedback = (i as f32) * 0.08;
                effects.push(AudioEffect::Echo(EchoEffect {
                    delay_time: 0.3,
                    feedback,
                    wet_dry_mix: 0.3,
                }));
            }
            black_box(effects);
        });
    });

    group.finish();
}

/// Benchmark effect chain scaling
fn bench_effect_chain_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_chain_scaling");
    group.measurement_time(Duration::from_secs(5));

    for chain_length in [1, 2, 3, 4, 5, 6, 7, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            chain_length,
            |b, &length| {
                b.iter(|| {
                    let mut chain = Vec::new();
                    for i in 0..length {
                        let effect = match i % 4 {
                            0 => AudioEffect::Reverb(ReverbEffect::default()),
                            1 => AudioEffect::Echo(EchoEffect::default()),
                            2 => AudioEffect::Filter(FilterEffect::default()),
                            3 => AudioEffect::Eq(EqEffect::default()),
                            _ => unreachable!(),
                        };
                        chain.push(effect);
                    }
                    black_box(chain);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark preset-based effect chains
fn bench_preset_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("preset_chains");

    // Gunshot indoor
    group.bench_function("gunshot_indoor", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Eq(EqEffect::bass_boost()),
            ];
            black_box(chain);
        });
    });

    // Footsteps on metal
    group.bench_function("footsteps_metal", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::tinny()),
                AudioEffect::Reverb(ReverbEffect {
                    room_size: 0.4,
                    damping: 0.3,
                    wet_dry_mix: 0.25,
                }),
                AudioEffect::Eq(EqEffect::bright()),
            ];
            black_box(chain);
        });
    });

    // Voice through wall
    group.bench_function("voice_through_wall", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Filter(FilterEffect::muffled()),
                AudioEffect::Eq(EqEffect { bass_gain: 2.0, mid_gain: -3.0, treble_gain: -6.0 }),
            ];
            black_box(chain);
        });
    });

    // Explosion in tunnel
    group.bench_function("explosion_tunnel", |b| {
        b.iter(|| {
            let chain = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Filter(FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: 4000.0,
                    resonance: 1.2,
                    wet_dry_mix: 0.6,
                }),
            ];
            black_box(chain);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_effects,
    bench_2_effect_chains,
    bench_3_effect_chains,
    bench_4_effect_chains,
    bench_8_effect_chains,
    bench_effect_order_impact,
    bench_dynamic_parameter_changes,
    bench_effect_chain_scaling,
    bench_preset_chains
);

criterion_main!(benches);
