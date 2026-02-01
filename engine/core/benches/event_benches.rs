//! Event system performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use engine_core::ecs::{Event, World};

#[derive(Debug, Clone)]
struct TestEvent {
    value: i32,
}
impl Event for TestEvent {}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CollisionEvent {
    entity_a: u64,
    entity_b: u64,
    force: f32,
}
impl Event for CollisionEvent {}

/// Benchmark: Send single event
fn bench_send_event(c: &mut Criterion) {
    c.bench_function("event_send_single", |b| {
        let mut world = World::new();

        b.iter(|| {
            world.send_event(TestEvent { value: black_box(42) });
        });
    });
}

/// Benchmark: Send events in batch
fn bench_send_events_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_send_batch");

    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                let mut world = World::new();

                b.iter(|| {
                    for i in 0..size {
                        world.send_event(TestEvent { value: black_box(i) });
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Read events with single reader
fn bench_read_events_single_reader(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_read_single_reader");

    for event_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(event_count),
            event_count,
            |b, &count| {
                let mut world = World::new();

                // Pre-populate events
                for i in 0..count {
                    world.send_event(TestEvent { value: i });
                }

                b.iter(|| {
                    let mut reader = world.get_event_reader::<TestEvent>();
                    let mut sum = 0;
                    for event in world.read_events(&mut reader) {
                        sum += event.value;
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Read events with multiple readers (concurrent access)
fn bench_read_events_multiple_readers(c: &mut Criterion) {
    c.bench_function("event_read_multiple_readers", |b| {
        let mut world = World::new();

        // Pre-populate events
        for i in 0..1000 {
            world.send_event(TestEvent { value: i });
        }

        b.iter(|| {
            // Simulate 4 different systems reading the same events
            let mut reader1 = world.get_event_reader::<TestEvent>();
            let mut reader2 = world.get_event_reader::<TestEvent>();
            let mut reader3 = world.get_event_reader::<TestEvent>();
            let mut reader4 = world.get_event_reader::<TestEvent>();

            let mut sum1 = 0;
            let mut sum2 = 0;
            let mut sum3 = 0;
            let mut sum4 = 0;

            for event in world.read_events(&mut reader1) {
                sum1 += event.value;
            }
            for event in world.read_events(&mut reader2) {
                sum2 += event.value;
            }
            for event in world.read_events(&mut reader3) {
                sum3 += event.value;
            }
            for event in world.read_events(&mut reader4) {
                sum4 += event.value;
            }

            black_box((sum1, sum2, sum3, sum4));
        });
    });
}

/// Benchmark: Event iteration overhead
fn bench_event_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_iteration");

    for event_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(event_count),
            event_count,
            |b, &count| {
                let mut world = World::new();

                // Pre-populate events
                for i in 0..count {
                    world.send_event(CollisionEvent {
                        entity_a: i as u64,
                        entity_b: (i + 1) as u64,
                        force: i as f32 * 0.5,
                    });
                }

                b.iter(|| {
                    let mut reader = world.get_event_reader::<CollisionEvent>();
                    let mut total_force = 0.0f32;

                    for event in world.read_events(&mut reader) {
                        total_force += event.force;
                    }

                    black_box(total_force);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Ring buffer overflow behavior
fn bench_ring_buffer_overflow(c: &mut Criterion) {
    c.bench_function("event_ring_buffer_overflow", |b| {
        let mut world = World::new();

        b.iter(|| {
            // Send more than MAX_EVENTS_PER_TYPE (1024)
            for i in 0..1200 {
                world.send_event(TestEvent { value: i });
            }

            // Read should only get last 1024
            let mut reader = world.get_event_reader::<TestEvent>();
            let mut count = 0;
            for _ in world.read_events(&mut reader) {
                count += 1;
            }

            black_box(count);
        });
    });
}

/// Benchmark: Multiple event types (type dispatch overhead)
fn bench_multiple_event_types(c: &mut Criterion) {
    #[derive(Debug, Clone)]
    struct EventA {
        data: i32,
    }
    impl Event for EventA {}

    #[derive(Debug, Clone)]
    struct EventB {
        data: f32,
    }
    impl Event for EventB {}

    #[derive(Debug, Clone)]
    struct EventC {
        data: u64,
    }
    impl Event for EventC {}

    c.bench_function("event_multiple_types", |b| {
        let mut world = World::new();

        // Pre-populate with mixed event types
        for i in 0..100 {
            world.send_event(EventA { data: i });
            world.send_event(EventB { data: i as f32 });
            world.send_event(EventC { data: i as u64 });
        }

        b.iter(|| {
            let mut reader_a = world.get_event_reader::<EventA>();
            let mut reader_b = world.get_event_reader::<EventB>();
            let mut reader_c = world.get_event_reader::<EventC>();

            let mut sum_a = 0i32;
            let mut sum_b = 0.0f32;
            let mut sum_c = 0u64;

            for event in world.read_events(&mut reader_a) {
                sum_a += event.data;
            }
            for event in world.read_events(&mut reader_b) {
                sum_b += event.data;
            }
            for event in world.read_events(&mut reader_c) {
                sum_c += event.data;
            }

            black_box((sum_a, sum_b, sum_c));
        });
    });
}

criterion_group!(
    benches,
    bench_send_event,
    bench_send_events_batch,
    bench_read_events_single_reader,
    bench_read_events_multiple_readers,
    bench_event_iteration,
    bench_ring_buffer_overflow,
    bench_multiple_event_types,
);
criterion_main!(benches);
