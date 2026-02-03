//! Benchmarks for memory management and LRU cache operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{
    AssetId, AssetRegistry, AssetType, LruCache, MemoryBudget, MemorySized, MeshData, RefType,
    TextureData,
};

fn bench_eviction_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction");

    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let budget = MemoryBudget::default();
                let cache = LruCache::new(budget);
                let registry = AssetRegistry::<MeshData>::new();
                let mesh = MeshData::cube();

                // Load assets
                for i in 0..count {
                    let id = AssetId::from_content(&i.to_le_bytes());
                    let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
                    cache.access(id, AssetType::Mesh);
                }

                // Get eviction candidates
                let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, count / 2);

                // Evict
                for id in candidates {
                    registry.remove(id);
                    cache.remove(id, AssetType::Mesh);
                }

                black_box(registry.len())
            });
        });
    }

    group.finish();
}

fn bench_memory_tracking_overhead(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();
    let mesh = MeshData::cube();

    // Load 100 assets
    for i in 0..100 {
        let id = AssetId::from_content(&i.to_le_bytes());
        let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
        cache.access(id, AssetType::Mesh);
    }

    c.bench_function("memory_tracking_overhead", |b| {
        b.iter(|| {
            // Compute actual memory usage
            let mut total = 0;
            for id in registry.iter_ids() {
                if let Some(mesh) = registry.get(id) {
                    total += mesh.size_bytes();
                }
            }
            cache.update_memory_usage(total, AssetType::Mesh);
            black_box(total)
        });
    });
}

fn bench_lru_access_operation(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let id = AssetId::from_content(b"test");

    c.bench_function("lru_access", |b| {
        b.iter(|| {
            cache.access(black_box(id), AssetType::Mesh);
        });
    });
}

fn bench_budget_check_operation(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    cache.update_memory_usage(50 * 1024 * 1024, AssetType::Mesh);

    c.bench_function("budget_check", |b| {
        b.iter(|| {
            black_box(cache.is_over_budget(AssetType::Mesh));
        });
    });
}

fn bench_bulk_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_eviction");

    for count in [100, 500, 1000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let budget = MemoryBudget::default();
                let cache = LruCache::new(budget);
                let registry = AssetRegistry::<MeshData>::new();
                let mesh = MeshData::cube();

                // Load assets
                for i in 0..count {
                    let id = AssetId::from_content(&i.to_le_bytes());
                    let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
                    cache.access(id, AssetType::Mesh);
                }

                // Evict all
                let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, count);
                for id in candidates {
                    registry.remove(id);
                    cache.remove(id, AssetType::Mesh);
                }

                black_box(registry.len())
            });
        });
    }

    group.finish();
}

fn bench_memory_usage_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("mesh_cube", |b| {
        let mesh = MeshData::cube();
        b.iter(|| black_box(mesh.size_bytes()));
    });

    group.bench_function("texture_1k", |b| {
        let texture = TextureData::solid_color([255, 0, 0, 255], 1024, 1024);
        b.iter(|| black_box(texture.size_bytes()));
    });

    group.bench_function("texture_4k", |b| {
        let texture = TextureData::solid_color([255, 0, 0, 255], 4096, 4096);
        b.iter(|| black_box(texture.size_bytes()));
    });

    group.finish();
}

fn bench_eviction_candidate_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("candidate_selection");

    for total in [100, 500, 1000] {
        group.throughput(Throughput::Elements(total));
        group.bench_with_input(BenchmarkId::from_parameter(total), &total, |b, &total| {
            let budget = MemoryBudget::default();
            let cache = LruCache::new(budget);
            let registry = AssetRegistry::<MeshData>::new();
            let mesh = MeshData::cube();

            // Load assets with mixed ref types
            for i in 0..total {
                let id = AssetId::from_content(&i.to_le_bytes());
                let ref_type = if i % 3 == 0 { RefType::Hard } else { RefType::Soft };
                let _handle = registry.insert_with_reftype(id, mesh.clone(), ref_type);
                cache.access(id, AssetType::Mesh);
            }

            b.iter(|| {
                let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, total / 2);
                black_box(candidates.len())
            });
        });
    }

    group.finish();
}

fn bench_lru_with_high_churn(c: &mut Criterion) {
    c.bench_function("lru_high_churn", |b| {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        b.iter(|| {
            // Simulate high churn - many accesses to different assets
            for i in 0..1000u32 {
                let id = AssetId::from_content(&i.to_le_bytes());
                cache.access(id, AssetType::Mesh);
            }
        });
    });
}

fn bench_memory_stats_update(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    c.bench_function("memory_stats_update", |b| {
        b.iter(|| {
            cache.update_memory_usage(black_box(100_000_000), AssetType::Mesh);
            cache.update_memory_usage(black_box(200_000_000), AssetType::Texture);
            cache.update_memory_usage(black_box(50_000_000), AssetType::Audio);
        });
    });
}

fn bench_concurrent_access(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    c.bench_function("concurrent_lru_access", |b| {
        let budget = MemoryBudget::default();
        let cache = Arc::new(LruCache::new(budget));

        b.iter(|| {
            let mut handles = vec![];

            for thread_id in 0..4 {
                let cache_clone = Arc::clone(&cache);
                let handle = thread::spawn(move || {
                    for i in 0..100 {
                        let id = AssetId::from_content(&(thread_id * 100 + i).to_le_bytes());
                        cache_clone.access(id, AssetType::Mesh);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_eviction_speed,
    bench_memory_tracking_overhead,
    bench_lru_access_operation,
    bench_budget_check_operation,
    bench_bulk_eviction,
    bench_memory_usage_calculation,
    bench_eviction_candidate_selection,
    bench_lru_with_high_churn,
    bench_memory_stats_update,
    bench_concurrent_access,
);
criterion_main!(benches);
