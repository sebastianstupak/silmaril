use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_simple(c: &mut Criterion) {
    c.bench_function("simple_add", |b| {
        b.iter(|| {
            let x = black_box(5);
            let y = black_box(10);
            black_box(x + y)
        });
    });
}

criterion_group!(benches, bench_simple);
criterion_main!(benches);
