//! Benchmarks for asset handle system
//!
//! Performance targets:
//! - AssetId generation: < 1 µs for 1KB data
//! - Handle creation: < 10 ns
//! - Handle clone: < 10 ns
//! - Registry get: < 20 ns

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetHandle, AssetId, AssetRegistry, RefType};

// Test asset types
#[derive(Clone)]
struct SmallAsset {
    #[allow(dead_code)]
    value: u32,
}

#[derive(Clone)]
struct MediumAsset {
    data: [u8; 1024], // 1KB
}

impl serde::Serialize for MediumAsset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.data)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct LargeAsset {
    data: Vec<u8>, // Variable size
}

fn bench_asset_id_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("asset_id_generation");

    // Benchmark different data sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data = vec![0u8; *size];
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| AssetId::from_content(black_box(&data)));
        });
    }

    group.finish();
}

fn bench_asset_id_seed_params(c: &mut Criterion) {
    let mut group = c.benchmark_group("asset_id_seed_params");

    let seed = 12345_u64;
    let params = b"procedural_terrain_params";

    group.bench_function("from_seed_and_params", |b| {
        b.iter(|| AssetId::from_seed_and_params(black_box(seed), black_box(params)));
    });

    group.finish();
}

fn bench_asset_id_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("asset_id_serialization");

    let id = AssetId::from_content(b"test data");

    group.bench_function("serialize", |b| {
        b.iter(|| bincode::serialize(black_box(&id)).unwrap());
    });

    let serialized = bincode::serialize(&id).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| bincode::deserialize::<AssetId>(black_box(&serialized)).unwrap());
    });

    group.finish();
}

fn bench_handle_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("handle_creation");

    let id = AssetId::from_content(b"test");

    group.bench_function("new_hard", |b| {
        b.iter(|| AssetHandle::<SmallAsset>::new(black_box(id), RefType::Hard));
    });

    group.bench_function("new_soft", |b| {
        b.iter(|| AssetHandle::<SmallAsset>::new(black_box(id), RefType::Soft));
    });

    group.finish();
}

fn bench_handle_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("handle_clone");

    let id = AssetId::from_content(b"test");
    let handle = AssetHandle::<SmallAsset>::new(id, RefType::Hard);

    group.bench_function("clone", |b| {
        b.iter(|| black_box(&handle).clone());
    });

    group.finish();
}

fn bench_handle_upgrade_downgrade(c: &mut Criterion) {
    let mut group = c.benchmark_group("handle_upgrade_downgrade");

    let id = AssetId::from_content(b"test");
    let hard_handle = AssetHandle::<SmallAsset>::new(id, RefType::Hard);
    let soft_handle = AssetHandle::<SmallAsset>::new(id, RefType::Soft);

    group.bench_function("to_soft", |b| {
        b.iter(|| black_box(&hard_handle).to_soft());
    });

    group.bench_function("to_hard", |b| {
        b.iter(|| black_box(&soft_handle).to_hard());
    });

    group.finish();
}

fn bench_registry_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_insert");

    group.bench_function("small_asset", |b| {
        let registry = AssetRegistry::new();
        let mut counter = 0u32;

        b.iter(|| {
            let id = AssetId::from_content(&counter.to_le_bytes());
            counter = counter.wrapping_add(1);
            registry.insert(id, SmallAsset { value: counter })
        });
    });

    group.bench_function("medium_asset", |b| {
        let registry = AssetRegistry::new();
        let mut counter = 0u32;

        b.iter(|| {
            let id = AssetId::from_content(&counter.to_le_bytes());
            counter = counter.wrapping_add(1);
            registry.insert(id, MediumAsset { data: [0; 1024] })
        });
    });

    group.finish();
}

fn bench_registry_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_get");

    // Pre-populate registry
    let registry = AssetRegistry::new();
    let ids: Vec<_> = (0..1000_u32)
        .map(|i| {
            let id = AssetId::from_content(&i.to_le_bytes());
            registry.insert(id, SmallAsset { value: i });
            id
        })
        .collect();

    group.bench_function("get", |b| {
        let mut counter = 0;
        b.iter(|| {
            let id = ids[counter % ids.len()];
            counter += 1;
            registry.get(black_box(id))
        });
    });

    group.bench_function("get_mut", |b| {
        let mut counter = 0;
        b.iter(|| {
            let id = ids[counter % ids.len()];
            counter += 1;
            registry.get_mut(black_box(id))
        });
    });

    group.finish();
}

fn bench_registry_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_contains");

    let registry = AssetRegistry::new();
    let ids: Vec<_> = (0..1000_u32)
        .map(|i| {
            let id = AssetId::from_content(&i.to_le_bytes());
            registry.insert(id, SmallAsset { value: i });
            id
        })
        .collect();

    group.bench_function("contains", |b| {
        let mut counter = 0;
        b.iter(|| {
            let id = ids[counter % ids.len()];
            counter += 1;
            registry.contains(black_box(id))
        });
    });

    group.finish();
}

fn bench_registry_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_remove");

    group.bench_function("remove", |b| {
        b.iter_batched(
            || {
                let registry = AssetRegistry::new();
                let id = AssetId::from_content(b"test");
                registry.insert(id, SmallAsset { value: 42 });
                (registry, id)
            },
            |(registry, id)| registry.remove(id),
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_registry_refcount_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_refcount");

    let registry = AssetRegistry::new();
    let id = AssetId::from_content(b"test");
    registry.insert(id, SmallAsset { value: 42 });

    group.bench_function("increment_hard", |b| {
        b.iter(|| registry.increment_refcount(black_box(id), RefType::Hard));
    });

    group.bench_function("increment_soft", |b| {
        b.iter(|| registry.increment_refcount(black_box(id), RefType::Soft));
    });

    group.bench_function("decrement_hard", |b| {
        b.iter(|| registry.decrement_refcount(black_box(id), RefType::Hard));
    });

    group.bench_function("refcount", |b| {
        b.iter(|| registry.refcount(black_box(id)));
    });

    group.bench_function("is_hard_referenced", |b| {
        b.iter(|| registry.is_hard_referenced(black_box(id)));
    });

    group.finish();
}

fn bench_registry_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_iteration");

    for size in [10, 100, 1000, 10000].iter() {
        let registry = AssetRegistry::new();
        for i in 0..*size {
            let id = AssetId::from_content(&(i as u32).to_le_bytes());
            registry.insert(id, SmallAsset { value: i as u32 });
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let count = registry.iter_ids().count();
                black_box(count)
            });
        });
    }

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    let registry = std::sync::Arc::new(AssetRegistry::new());

    // Pre-populate
    for i in 0..100_u32 {
        let id = AssetId::from_content(&i.to_le_bytes());
        registry.insert(id, SmallAsset { value: i });
    }

    group.bench_function("parallel_get", |b| {
        b.iter(|| {
            use std::sync::Arc;
            use std::thread;

            let handles: Vec<_> = (0..4)
                .map(|thread_id| {
                    let registry_clone = Arc::clone(&registry);
                    thread::spawn(move || {
                        for i in 0..25_u32 {
                            let id =
                                AssetId::from_content(&(thread_id as u32 * 25 + i).to_le_bytes());
                            let _ = registry_clone.get(id);
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    group.bench_function("full_lifecycle", |b| {
        b.iter(|| {
            // Create registry
            let registry = AssetRegistry::new();

            // Generate ID from content
            let data = MediumAsset { data: [42; 1024] };
            let serialized = bincode::serialize(&data).unwrap();
            let id = AssetId::from_content(&serialized);

            // Insert asset
            let handle = registry.insert(id, data);

            // Clone handle (increment refcount)
            let handle2 = handle.clone();

            // Access asset
            let asset = registry.get(id).unwrap();
            black_box(&asset.data);

            // Upgrade to hard reference
            let _hard = handle2.to_hard();

            // Drop handles (decrement refcount)
            drop(handle);
            drop(handle2);

            // Remove asset
            registry.remove(id)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_asset_id_generation,
    bench_asset_id_seed_params,
    bench_asset_id_serialization,
    bench_handle_creation,
    bench_handle_clone,
    bench_handle_upgrade_downgrade,
    bench_registry_insert,
    bench_registry_get,
    bench_registry_contains,
    bench_registry_remove,
    bench_registry_refcount_ops,
    bench_registry_iteration,
    bench_concurrent_access,
    bench_end_to_end,
);
criterion_main!(benches);
