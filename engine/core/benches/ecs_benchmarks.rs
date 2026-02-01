//! ECS performance benchmarks
//!
//! This file will contain benchmarks for the ECS implementation once it's complete.
//! For now, it serves as a placeholder.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| black_box(42));
    });
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
