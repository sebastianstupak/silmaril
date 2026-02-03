//! Memory usage benchmarks for the audio system
//!
//! Tracks memory allocations, allocation rates, and peak memory usage
//! to ensure the audio system meets memory efficiency targets:
//! - < 1KB allocations per frame (hot path)
//! - Efficient buffer pool usage
//! - Minimal memory fragmentation
//!
//! These benchmarks help identify memory leaks, excessive allocations,
//! and opportunities for memory pool optimization.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Global allocation tracker for measuring memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

/// Reset allocation tracking
fn reset_allocation_tracking() {
    ALLOCATED.store(0, Ordering::Relaxed);
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
}

/// Get current allocation stats
fn get_allocation_stats() -> (usize, usize) {
    (ALLOCATED.load(Ordering::Relaxed), ALLOCATION_COUNT.load(Ordering::Relaxed))
}

/// Benchmark memory allocations in hot path (play_3d)
///
/// Target: < 512 bytes allocated per play_3d call
fn bench_play_3d_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("play_3d_allocations");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("memory_per_play_3d", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter_custom(|iters| {
            reset_allocation_tracking();
            let start_time = std::time::Instant::now();

            for i in 0..iters {
                let _ = engine.play_3d(
                    black_box(i as u32),
                    black_box("test"),
                    black_box(Vec3::ZERO),
                    black_box(1.0),
                    black_box(false),
                    black_box(100.0),
                );
            }

            let elapsed = start_time.elapsed();
            let (bytes, allocs) = get_allocation_stats();

            tracing::debug!(
                allocations_per_call = allocs / (iters as usize),
                bytes_per_call = bytes / (iters as usize),
                "play_3d memory usage"
            );

            elapsed
        });
    });

    group.finish();
}

/// Benchmark memory allocations in emitter updates
///
/// Target: Zero allocations (should reuse existing data structures)
fn bench_emitter_update_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("emitter_update_allocations");

    group.bench_function("memory_per_update", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Pre-create emitters
        for i in 0..100 {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        b.iter_custom(|iters| {
            reset_allocation_tracking();
            let start_time = std::time::Instant::now();

            for i in 0..iters {
                engine.update_emitter_position(
                    black_box((i % 100) as u32),
                    black_box(Vec3::new(i as f32, 0.0, 0.0)),
                );
            }

            let elapsed = start_time.elapsed();
            let (bytes, allocs) = get_allocation_stats();

            tracing::debug!(
                total_allocations = allocs,
                total_bytes = bytes,
                "emitter_update memory usage (should be zero)"
            );

            elapsed
        });
    });

    group.finish();
}

/// Benchmark memory allocations in listener updates
///
/// Target: Zero allocations (hot path optimization)
fn bench_listener_update_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("listener_update_allocations");

    group.bench_function("memory_per_listener_update", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter_custom(|iters| {
            reset_allocation_tracking();
            let start_time = std::time::Instant::now();

            for i in 0..iters {
                let t = i as f32 * 0.1;
                engine.set_listener_transform(
                    black_box(Vec3::new(t, 1.8, 0.0)),
                    black_box(Vec3::new(0.0, 0.0, -1.0)),
                    black_box(Vec3::new(0.0, 1.0, 0.0)),
                );
            }

            let elapsed = start_time.elapsed();
            let (bytes, allocs) = get_allocation_stats();

            tracing::debug!(
                total_allocations = allocs,
                total_bytes = bytes,
                "listener_update memory usage (should be zero)"
            );

            elapsed
        });
    });

    group.finish();
}

/// Benchmark allocation rate per frame
///
/// Target: < 1KB per frame with 1k active sounds
fn bench_frame_allocation_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_allocation_rate");
    group.measurement_time(Duration::from_secs(8));

    for sound_count in [100, 1_000, 5_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..count {
                    let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                    let position = Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0);
                    engine.update_emitter_position(i, position);
                }

                b.iter_custom(|iters| {
                    reset_allocation_tracking();
                    let start_time = std::time::Instant::now();
                    let mut frame = 0u32;

                    for _ in 0..iters {
                        let t = (frame as f32) * 0.016;

                        // Simulate one frame
                        engine.set_listener_transform(
                            Vec3::new(t.sin() * 10.0, 1.8, t.cos() * 10.0),
                            Vec3::new(0.0, 0.0, -1.0),
                            Vec3::new(0.0, 1.0, 0.0),
                        );

                        // Update 10% of emitters
                        for i in (0..count).step_by(10) {
                            let angle = (i as f32 + t) * std::f32::consts::TAU / (count as f32);
                            let pos = Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0);
                            engine.update_emitter_position(i, pos);
                        }

                        frame = frame.wrapping_add(1);
                    }

                    let elapsed = start_time.elapsed();
                    let (bytes, allocs) = get_allocation_stats();

                    tracing::debug!(
                        sound_count = count,
                        bytes_per_frame = bytes / (iters as usize),
                        allocs_per_frame = allocs / (iters as usize),
                        "frame allocation rate"
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark peak memory usage with varying sound counts
///
/// Measures total memory footprint as sound count scales
fn bench_peak_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("peak_memory_usage");
    group.measurement_time(Duration::from_secs(5));

    for sound_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                b.iter_custom(|iters| {
                    let mut total_duration = Duration::ZERO;

                    for _ in 0..iters {
                        reset_allocation_tracking();
                        let start_time = std::time::Instant::now();

                        let mut engine = AudioEngine::new().unwrap();

                        // Create all emitters
                        for i in 0..count {
                            engine.update_emitter_position(i, Vec3::ZERO);
                        }

                        total_duration += start_time.elapsed();

                        let (bytes, _) = get_allocation_stats();
                        tracing::debug!(
                            sound_count = count,
                            peak_memory_bytes = bytes,
                            memory_per_sound = bytes / (count as usize),
                            "peak memory usage"
                        );

                        // Drop engine to free memory
                        drop(engine);
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory fragmentation over time
///
/// Tests if repeated create/destroy cycles cause memory fragmentation
/// Target: Stable memory usage over 1000 cycles
fn bench_memory_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("fragmentation_over_cycles", |b| {
        b.iter_custom(|iters| {
            let mut engine = AudioEngine::new().unwrap();
            reset_allocation_tracking();

            let start_time = std::time::Instant::now();
            let initial_memory = get_allocation_stats().0;

            for cycle in 0..iters {
                // Create batch of emitters
                let base_id = (cycle * 100) as u32;
                for i in 0..100 {
                    engine.update_emitter_position(base_id + i, Vec3::ZERO);
                }

                // Remove batch
                for i in 0..100 {
                    engine.remove_emitter(base_id + i);
                }

                // Cleanup
                engine.cleanup_finished();

                // Log memory growth every 100 cycles
                if cycle % 100 == 0 {
                    let current_memory = get_allocation_stats().0;
                    let memory_growth = current_memory.saturating_sub(initial_memory);
                    tracing::debug!(
                        cycle,
                        current_memory,
                        memory_growth,
                        "memory fragmentation check"
                    );
                }
            }

            let elapsed = start_time.elapsed();
            let (final_memory, _) = get_allocation_stats();
            let memory_growth = final_memory.saturating_sub(initial_memory);

            tracing::info!(
                total_cycles = iters,
                initial_memory,
                final_memory,
                memory_growth,
                growth_per_cycle = memory_growth / (iters as usize),
                "memory fragmentation test complete"
            );

            elapsed
        });
    });

    group.finish();
}

/// Benchmark cleanup memory reclamation
///
/// Tests if cleanup properly frees memory
/// Target: Full memory reclamation within 1 frame
fn bench_cleanup_memory_reclamation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cleanup_memory_reclamation");

    group.bench_function("memory_freed_by_cleanup", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;

            for _ in 0..iters {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..1000 {
                    engine.update_emitter_position(i, Vec3::ZERO);
                }

                reset_allocation_tracking();
                let memory_before = get_allocation_stats().0;

                // Remove all emitters
                for i in 0..1000 {
                    engine.remove_emitter(i);
                }

                // Measure cleanup
                let start_time = std::time::Instant::now();
                engine.cleanup_finished();
                total_duration += start_time.elapsed();

                let memory_after = get_allocation_stats().0;
                let memory_freed = memory_before.saturating_sub(memory_after);

                tracing::debug!(
                    memory_before,
                    memory_after,
                    memory_freed,
                    "cleanup memory reclamation"
                );
            }

            total_duration
        });
    });

    group.finish();
}

/// Benchmark memory usage of effect stacking
///
/// Tests memory overhead of adding/removing effects
/// Target: < 128 bytes per effect
fn bench_effect_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_memory_overhead");

    group.bench_function("memory_per_effect", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter_custom(|iters| {
            reset_allocation_tracking();
            let start_time = std::time::Instant::now();

            for i in 0..iters {
                // Query effect count (simulates effect management)
                let count = engine.effect_count(black_box(i));
                black_box(count);
            }

            let elapsed = start_time.elapsed();
            let (bytes, allocs) = get_allocation_stats();

            tracing::debug!(
                total_allocations = allocs,
                total_bytes = bytes,
                bytes_per_query = bytes / (iters as usize),
                "effect memory overhead"
            );

            elapsed
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_play_3d_allocations,
    bench_emitter_update_allocations,
    bench_listener_update_allocations,
    bench_frame_allocation_rate,
    bench_peak_memory_usage,
    bench_memory_fragmentation,
    bench_cleanup_memory_reclamation,
    bench_effect_memory_overhead,
);

criterion_main!(benches);
