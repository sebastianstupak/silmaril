//! Benchmarks for client-side prediction system

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_networking::{
    AdaptiveErrorCorrector, ClientPredictor, ErrorCorrector, InputBuffer, PredictionConfig,
};
use glam::{Quat, Vec3};

fn bench_input_buffer_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_buffer_push");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buffer = InputBuffer::new(size);

            b.iter(|| {
                buffer.push_input(
                    black_box(100),
                    black_box(Vec3::X),
                    black_box(Vec3::ZERO),
                    black_box(0),
                );
            });
        });
    }

    group.finish();
}

fn bench_input_buffer_acknowledge(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_buffer_acknowledge");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut buffer = InputBuffer::new(size);
                    // Fill buffer
                    for i in 0..size {
                        buffer.push_input(i as u64 * 10, Vec3::X, Vec3::ZERO, 0);
                    }
                    buffer
                },
                |mut buffer| {
                    buffer.acknowledge(black_box(size as u32 / 2));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_input_buffer_from_sequence(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_buffer_from_sequence");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buffer = InputBuffer::new(size);
            // Fill buffer
            for i in 0..size {
                buffer.push_input(i as u64 * 10, Vec3::X, Vec3::ZERO, 0);
            }

            b.iter(|| {
                let inputs = buffer.inputs_from_sequence(black_box(size as u32 / 2));
                black_box(inputs);
            });
        });
    }

    group.finish();
}

fn bench_process_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_input");

    let mut predictor = ClientPredictor::new(PredictionConfig::default());

    group.bench_function("process_single_input", |b| {
        b.iter(|| {
            predictor.process_input(
                black_box(0),
                black_box(Vec3::new(0.0, 0.0, 1.0)),
                black_box(Vec3::ZERO),
                black_box(0),
                black_box(0.016),
            );
        });
    });

    group.finish();
}

fn bench_reconciliation(c: &mut Criterion) {
    let mut group = c.benchmark_group("reconciliation");

    // No error (fast path)
    group.bench_function("reconcile_no_error", |b| {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        let seq = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);
        let pos = predictor.predicted_position();

        b.iter(|| {
            predictor.reconcile(
                black_box(seq),
                black_box(pos),
                black_box(Vec3::ZERO),
                black_box(Quat::IDENTITY),
                black_box(0),
            );
        });
    });

    // With error and replay
    group.bench_function("reconcile_with_error", |b| {
        b.iter_batched(
            || {
                let mut predictor = ClientPredictor::new(PredictionConfig::default());
                predictor.set_position(Vec3::ZERO);

                // Add multiple pending inputs
                let seq = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);
                for _ in 0..10 {
                    predictor.process_input(16, Vec3::X, Vec3::ZERO, 0, 0.016);
                }

                (predictor, seq)
            },
            |(mut predictor, seq)| {
                // Server position is different (triggers replay)
                predictor.reconcile(
                    black_box(seq),
                    black_box(Vec3::new(10.0, 0.0, 0.0)),
                    black_box(Vec3::ZERO),
                    black_box(Quat::IDENTITY),
                    black_box(0),
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_input_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_replay");

    for num_inputs in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_inputs),
            num_inputs,
            |b, &num_inputs| {
                b.iter_batched(
                    || {
                        let mut predictor = ClientPredictor::new(PredictionConfig::default());
                        predictor.set_position(Vec3::ZERO);

                        let seq = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);

                        // Add pending inputs
                        for _ in 0..num_inputs {
                            predictor.process_input(16, Vec3::X, Vec3::ZERO, 0, 0.016);
                        }

                        (predictor, seq)
                    },
                    |(mut predictor, seq)| {
                        // Trigger replay by using different server position
                        predictor.reconcile(
                            black_box(seq),
                            black_box(Vec3::new(5.0, 0.0, 0.0)),
                            black_box(Vec3::ZERO),
                            black_box(Quat::IDENTITY),
                            black_box(0),
                        );
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_error_corrector_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_corrector");

    group.bench_function("update_small_error", |b| {
        let mut corrector = ErrorCorrector::new(10.0);
        corrector.set_error(Vec3::new(0.5, 0.0, 0.0));

        b.iter(|| {
            let correction = corrector.update(black_box(0.016));
            black_box(correction);
        });
    });

    group.bench_function("update_large_error", |b| {
        let mut corrector = ErrorCorrector::new(10.0);
        corrector.set_error(Vec3::new(10.0, 0.0, 0.0));

        b.iter(|| {
            let correction = corrector.update(black_box(0.016));
            black_box(correction);
        });
    });

    group.finish();
}

fn bench_adaptive_error_corrector(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_error_corrector");

    group.bench_function("update_small_error", |b| {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);
        corrector.set_error(Vec3::new(0.5, 0.0, 0.0));

        b.iter(|| {
            let correction = corrector.update(black_box(0.016));
            black_box(correction);
        });
    });

    group.bench_function("update_medium_error", |b| {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);
        corrector.set_error(Vec3::new(2.0, 0.0, 0.0));

        b.iter(|| {
            let correction = corrector.update(black_box(0.016));
            black_box(correction);
        });
    });

    group.bench_function("update_large_error_snap", |b| {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);
        corrector.set_error(Vec3::new(10.0, 0.0, 0.0));

        b.iter(|| {
            let correction = corrector.update(black_box(0.016));
            black_box(correction);
        });
    });

    group.finish();
}

fn bench_full_prediction_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_prediction_cycle");

    group.bench_function("60fps_typical_frame", |b| {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        b.iter(|| {
            // Process input (typical player movement)
            let seq = predictor.process_input(
                black_box(0),
                black_box(Vec3::new(0.5, 0.0, 0.8)),
                black_box(Vec3::new(10.0, 5.0, 0.0)),
                black_box(0),
                black_box(0.016),
            );

            // Get predicted state
            let _pos = predictor.predicted_position();
            let _rot = predictor.predicted_rotation();

            black_box(seq);
        });
    });

    group.bench_function("60fps_with_reconciliation", |b| {
        b.iter_batched(
            || {
                let mut predictor = ClientPredictor::new(PredictionConfig::default());
                predictor.set_position(Vec3::ZERO);

                // Add some pending inputs
                let seq = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);
                for _ in 0..5 {
                    predictor.process_input(16, Vec3::X, Vec3::ZERO, 0, 0.016);
                }

                (predictor, seq)
            },
            |(mut predictor, seq)| {
                // Process new input
                predictor.process_input(
                    black_box(100),
                    black_box(Vec3::new(0.5, 0.0, 0.8)),
                    black_box(Vec3::new(10.0, 5.0, 0.0)),
                    black_box(0),
                    black_box(0.016),
                );

                // Reconcile with server
                predictor.reconcile(
                    black_box(seq),
                    black_box(Vec3::new(1.0, 0.0, 0.0)),
                    black_box(Vec3::ZERO),
                    black_box(Quat::IDENTITY),
                    black_box(100),
                );

                // Get predicted state
                let _pos = predictor.predicted_position();
                let _rot = predictor.predicted_rotation();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_prediction_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("prediction_overhead");

    // Measure total overhead per frame
    group.bench_function("complete_frame_overhead", |b| {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        let mut error_corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

        predictor.set_position(Vec3::ZERO);
        error_corrector.set_error(Vec3::new(0.1, 0.0, 0.0));

        b.iter(|| {
            // Process input
            let _seq = predictor.process_input(
                black_box(0),
                black_box(Vec3::new(0.5, 0.0, 0.8)),
                black_box(Vec3::new(10.0, 5.0, 0.0)),
                black_box(0),
                black_box(0.016),
            );

            // Get predicted state
            let _pos = predictor.predicted_position();
            let _rot = predictor.predicted_rotation();
            let _vel = predictor.predicted_velocity();

            // Apply error correction
            let _correction = error_corrector.update(black_box(0.016));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_input_buffer_push,
    bench_input_buffer_acknowledge,
    bench_input_buffer_from_sequence,
    bench_process_input,
    bench_reconciliation,
    bench_input_replay,
    bench_error_corrector_update,
    bench_adaptive_error_corrector,
    bench_full_prediction_cycle,
    bench_prediction_overhead,
);

criterion_main!(benches);
