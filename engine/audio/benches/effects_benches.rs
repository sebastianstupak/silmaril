//! Benchmarks for audio effects
//!
//! Measures performance overhead of audio effects to ensure they meet the
//! target of < 0.1ms overhead per effect.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_audio::{AudioEffect, AudioEngine, EchoEffect, EqEffect, FilterEffect, ReverbEffect};

fn bench_reverb_creation(c: &mut Criterion) {
    c.bench_function("reverb_effect_creation", |b| {
        b.iter(|| {
            black_box(ReverbEffect::default());
        });
    });
}

fn bench_reverb_validation(c: &mut Criterion) {
    let reverb = ReverbEffect::default();

    c.bench_function("reverb_effect_validation", |b| {
        b.iter(|| {
            black_box(reverb.validate());
        });
    });
}

fn bench_reverb_presets(c: &mut Criterion) {
    c.bench_function("reverb_small_room_preset", |b| {
        b.iter(|| {
            black_box(ReverbEffect::small_room());
        });
    });

    c.bench_function("reverb_large_hall_preset", |b| {
        b.iter(|| {
            black_box(ReverbEffect::large_hall());
        });
    });

    c.bench_function("reverb_cathedral_preset", |b| {
        b.iter(|| {
            black_box(ReverbEffect::cathedral());
        });
    });
}

fn bench_echo_creation(c: &mut Criterion) {
    c.bench_function("echo_effect_creation", |b| {
        b.iter(|| {
            black_box(EchoEffect::default());
        });
    });
}

fn bench_echo_validation(c: &mut Criterion) {
    let echo = EchoEffect::default();

    c.bench_function("echo_effect_validation", |b| {
        b.iter(|| {
            black_box(echo.validate());
        });
    });
}

fn bench_echo_presets(c: &mut Criterion) {
    c.bench_function("echo_slapback_preset", |b| {
        b.iter(|| {
            black_box(EchoEffect::slapback());
        });
    });

    c.bench_function("echo_long_preset", |b| {
        b.iter(|| {
            black_box(EchoEffect::long_echo());
        });
    });
}

fn bench_filter_creation(c: &mut Criterion) {
    c.bench_function("filter_effect_creation", |b| {
        b.iter(|| {
            black_box(FilterEffect::default());
        });
    });
}

fn bench_filter_validation(c: &mut Criterion) {
    let filter = FilterEffect::default();

    c.bench_function("filter_effect_validation", |b| {
        b.iter(|| {
            black_box(filter.validate());
        });
    });
}

fn bench_filter_presets(c: &mut Criterion) {
    c.bench_function("filter_muffled_preset", |b| {
        b.iter(|| {
            black_box(FilterEffect::muffled());
        });
    });

    c.bench_function("filter_tinny_preset", |b| {
        b.iter(|| {
            black_box(FilterEffect::tinny());
        });
    });

    c.bench_function("filter_radio_preset", |b| {
        b.iter(|| {
            black_box(FilterEffect::radio());
        });
    });
}

fn bench_eq_creation(c: &mut Criterion) {
    c.bench_function("eq_effect_creation", |b| {
        b.iter(|| {
            black_box(EqEffect::default());
        });
    });
}

fn bench_eq_validation(c: &mut Criterion) {
    let eq = EqEffect::default();

    c.bench_function("eq_effect_validation", |b| {
        b.iter(|| {
            black_box(eq.validate());
        });
    });
}

fn bench_eq_presets(c: &mut Criterion) {
    c.bench_function("eq_bass_boost_preset", |b| {
        b.iter(|| {
            black_box(EqEffect::bass_boost());
        });
    });

    c.bench_function("eq_voice_clarity_preset", |b| {
        b.iter(|| {
            black_box(EqEffect::voice_clarity());
        });
    });

    c.bench_function("eq_bright_preset", |b| {
        b.iter(|| {
            black_box(EqEffect::bright());
        });
    });
}

fn bench_audio_effect_enum(c: &mut Criterion) {
    c.bench_function("audio_effect_enum_reverb", |b| {
        b.iter(|| {
            black_box(AudioEffect::Reverb(ReverbEffect::default()));
        });
    });

    c.bench_function("audio_effect_enum_echo", |b| {
        b.iter(|| {
            black_box(AudioEffect::Echo(EchoEffect::default()));
        });
    });

    c.bench_function("audio_effect_enum_filter", |b| {
        b.iter(|| {
            black_box(AudioEffect::Filter(FilterEffect::default()));
        });
    });

    c.bench_function("audio_effect_enum_eq", |b| {
        b.iter(|| {
            black_box(AudioEffect::Eq(EqEffect::default()));
        });
    });
}

fn bench_effect_serialization(c: &mut Criterion) {
    let reverb = ReverbEffect::default();
    let echo = EchoEffect::default();
    let filter = FilterEffect::default();
    let eq = EqEffect::default();

    c.bench_function("reverb_serialization", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&reverb).unwrap();
            black_box(json);
        });
    });

    c.bench_function("reverb_deserialization", |b| {
        let json = serde_json::to_string(&reverb).unwrap();
        b.iter(|| {
            let deserialized: ReverbEffect = serde_json::from_str(&json).unwrap();
            black_box(deserialized);
        });
    });

    c.bench_function("echo_serialization", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&echo).unwrap();
            black_box(json);
        });
    });

    c.bench_function("filter_serialization", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&filter).unwrap();
            black_box(json);
        });
    });

    c.bench_function("eq_serialization", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&eq).unwrap();
            black_box(json);
        });
    });
}

fn bench_effect_cloning(c: &mut Criterion) {
    let reverb = ReverbEffect::default();
    let echo = EchoEffect::default();
    let filter = FilterEffect::default();
    let eq = EqEffect::default();

    c.bench_function("reverb_clone", |b| {
        b.iter(|| {
            black_box(reverb);
        });
    });

    c.bench_function("echo_clone", |b| {
        b.iter(|| {
            black_box(echo);
        });
    });

    c.bench_function("filter_clone", |b| {
        b.iter(|| {
            black_box(filter);
        });
    });

    c.bench_function("eq_clone", |b| {
        b.iter(|| {
            black_box(eq);
        });
    });
}

fn bench_multiple_effect_stack(c: &mut Criterion) {
    c.bench_function("create_3_effect_stack", |b| {
        b.iter(|| {
            let effects = vec![
                AudioEffect::Reverb(ReverbEffect::small_room()),
                AudioEffect::Echo(EchoEffect::slapback()),
                AudioEffect::Filter(FilterEffect::muffled()),
            ];
            black_box(effects);
        });
    });

    c.bench_function("create_5_effect_stack", |b| {
        b.iter(|| {
            let effects = vec![
                AudioEffect::Reverb(ReverbEffect::large_hall()),
                AudioEffect::Echo(EchoEffect::long_echo()),
                AudioEffect::Filter(FilterEffect::radio()),
                AudioEffect::Eq(EqEffect::bass_boost()),
                AudioEffect::Eq(EqEffect::voice_clarity()),
            ];
            black_box(effects);
        });
    });
}

fn bench_engine_effect_api(c: &mut Criterion) {
    c.bench_function("engine_effect_count", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let count = engine.effect_count(0);
            black_box(count);
        });
    });
}

fn bench_effect_combinations(c: &mut Criterion) {
    c.bench_function("indoor_gunshot_effects", |b| {
        b.iter(|| {
            let reverb = ReverbEffect::small_room();
            let filter = FilterEffect::muffled();
            black_box((reverb, filter));
        });
    });

    c.bench_function("outdoor_echo_effects", |b| {
        b.iter(|| {
            let echo = EchoEffect::long_echo();
            let reverb = ReverbEffect { room_size: 0.2, damping: 0.8, wet_dry_mix: 0.1 };
            black_box((echo, reverb));
        });
    });

    c.bench_function("radio_transmission_effects", |b| {
        b.iter(|| {
            let filter = FilterEffect::radio();
            let echo = EchoEffect { delay_time: 0.05, feedback: 0.2, wet_dry_mix: 0.15 };
            black_box((filter, echo));
        });
    });
}

criterion_group!(
    benches,
    bench_reverb_creation,
    bench_reverb_validation,
    bench_reverb_presets,
    bench_echo_creation,
    bench_echo_validation,
    bench_echo_presets,
    bench_filter_creation,
    bench_filter_validation,
    bench_filter_presets,
    bench_eq_creation,
    bench_eq_validation,
    bench_eq_presets,
    bench_audio_effect_enum,
    bench_effect_serialization,
    bench_effect_cloning,
    bench_multiple_effect_stack,
    bench_engine_effect_api,
    bench_effect_combinations
);

criterion_main!(benches);
