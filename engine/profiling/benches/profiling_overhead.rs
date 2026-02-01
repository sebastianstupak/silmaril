//! Benchmarks for profiling overhead.
//!
//! These benchmarks measure the runtime cost of profiling when enabled and disabled.
//!
//! Target performance:
//! - Overhead when profiling OFF: <1ns per scope
//! - Overhead when profiling ON: <200ns per scope

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(feature = "metrics")]
use agent_game_engine_profiling::{ProfileCategory, Profiler, ProfilerConfig};

/// Baseline: No profiling at all (control group).
fn baseline_no_profiling(c: &mut Criterion) {
    c.bench_function("baseline_no_profiling", |b| {
        b.iter(|| {
            // Simulate some work
            let mut sum = 0u64;
            for i in 0..100 {
                sum += i;
            }
            black_box(sum);
        });
    });
}

/// Benchmark profiling overhead when feature is disabled.
///
/// This should compile to nearly zero overhead.
#[cfg(not(feature = "profiling-puffin"))]
fn profiling_disabled_overhead(c: &mut Criterion) {
    c.bench_function("profiling_disabled_overhead", |b| {
        b.iter(|| {
            agent_game_engine_profiling::profile_scope!("test_scope");

            // Simulate some work
            let mut sum = 0u64;
            for i in 0..100 {
                sum += i;
            }
            black_box(sum);
        });
    });
}

/// Benchmark profiling overhead when metrics are enabled.
#[cfg(feature = "metrics")]
fn profiling_enabled_overhead(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("profiling_enabled_overhead", |b| {
        b.iter(|| {
            profiler.begin_frame();

            {
                let _guard = profiler.scope("test_scope", ProfileCategory::ECS);

                // Simulate some work
                let mut sum = 0u64;
                for i in 0..100 {
                    sum += i;
                }
                black_box(sum);
            }

            profiler.end_frame();
        });
    });
}

/// Benchmark scope creation and destruction only.
#[cfg(feature = "metrics")]
fn scope_creation_overhead(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("scope_creation_overhead", |b| {
        b.iter(|| {
            profiler.begin_frame();

            {
                let _guard = profiler.scope("test_scope", ProfileCategory::ECS);
                // No work - just measure scope overhead
            }

            profiler.end_frame();
        });
    });
}

/// Benchmark multiple nested scopes.
#[cfg(feature = "metrics")]
fn nested_scopes_overhead(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("nested_scopes_overhead", |b| {
        b.iter(|| {
            profiler.begin_frame();

            {
                let _outer = profiler.scope("outer", ProfileCategory::ECS);
                {
                    let _middle = profiler.scope("middle", ProfileCategory::Rendering);
                    {
                        let _inner = profiler.scope("inner", ProfileCategory::Physics);
                        // No work
                    }
                }
            }

            profiler.end_frame();
        });
    });
}

/// Benchmark many sequential scopes.
#[cfg(feature = "metrics")]
fn many_sequential_scopes(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("many_sequential_scopes", |b| {
        b.iter(|| {
            profiler.begin_frame();

            for i in 0..10 {
                let name = format!("scope_{}", i);
                let _guard = profiler.scope(&name, ProfileCategory::ECS);
                // Minimal work
                black_box(i);
            }

            profiler.end_frame();
        });
    });
}

/// Benchmark frame begin/end overhead.
#[cfg(feature = "metrics")]
fn frame_begin_end_overhead(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("frame_begin_end_overhead", |b| {
        b.iter(|| {
            profiler.begin_frame();
            profiler.end_frame();
        });
    });
}

/// Benchmark with disabled profiler (config.enabled = false).
#[cfg(feature = "metrics")]
fn disabled_profiler_overhead(c: &mut Criterion) {
    let profiler = Profiler::new(ProfilerConfig::default_release());

    c.bench_function("disabled_profiler_overhead", |b| {
        b.iter(|| {
            profiler.begin_frame();

            {
                let _guard = profiler.scope("test_scope", ProfileCategory::ECS);

                // Simulate some work
                let mut sum = 0u64;
                for i in 0..100 {
                    sum += i;
                }
                black_box(sum);
            }

            profiler.end_frame();
        });
    });
}

// Conditional benchmark groups based on features
#[cfg(all(feature = "metrics", not(feature = "profiling-puffin")))]
criterion_group!(
    benches,
    baseline_no_profiling,
    profiling_enabled_overhead,
    scope_creation_overhead,
    nested_scopes_overhead,
    many_sequential_scopes,
    frame_begin_end_overhead,
    disabled_profiler_overhead,
    profiling_disabled_overhead
);

#[cfg(all(feature = "metrics", feature = "profiling-puffin"))]
criterion_group!(
    benches,
    baseline_no_profiling,
    profiling_enabled_overhead,
    scope_creation_overhead,
    nested_scopes_overhead,
    many_sequential_scopes,
    frame_begin_end_overhead,
    disabled_profiler_overhead
);

#[cfg(all(not(feature = "metrics"), not(feature = "profiling-puffin")))]
criterion_group!(benches, baseline_no_profiling, profiling_disabled_overhead);

#[cfg(all(not(feature = "metrics"), feature = "profiling-puffin"))]
criterion_group!(benches, baseline_no_profiling);

criterion_main!(benches);
