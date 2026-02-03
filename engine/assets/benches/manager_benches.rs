//! Benchmarks for AssetManager.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_assets::{AssetId, AssetManager, LruCache, MemoryBudget, MeshData};
use std::sync::Arc;

fn bench_manager_creation(c: &mut Criterion) {
    c.bench_function("manager_creation", |b| {
        b.iter(|| {
            let manager = AssetManager::new();
            black_box(manager);
        });
    });
}

fn bench_mesh_insertion(c: &mut Criterion) {
    let manager = AssetManager::new();
    let mesh = MeshData::cube();

    c.bench_function("mesh_insertion", |b| {
        let mut id_counter = 0u64;
        b.iter(|| {
            let id = AssetId::from_content(&id_counter.to_le_bytes());
            id_counter += 1;
            let handle = manager.meshes().insert(id, mesh.clone());
            black_box(handle);
        });
    });
}

fn bench_mesh_retrieval(c: &mut Criterion) {
    let manager = AssetManager::new();
    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"bench_mesh");
    let _handle = manager.meshes().insert(id, mesh);

    c.bench_function("mesh_retrieval", |b| {
        b.iter(|| {
            let retrieved = manager.get_mesh(black_box(id));
            black_box(retrieved);
        });
    });
}

fn bench_handle_cloning(c: &mut Criterion) {
    let manager = AssetManager::new();
    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"clone_bench");
    let handle = manager.meshes().insert(id, mesh);

    c.bench_function("handle_cloning", |b| {
        b.iter(|| {
            let cloned = handle.clone();
            black_box(cloned);
        });
    });
}

fn bench_concurrent_access(c: &mut Criterion) {
    use std::thread;

    let mut group = c.benchmark_group("concurrent_access");

    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let manager = Arc::new(AssetManager::new());
                    let mut handles = vec![];

                    for i in 0..thread_count {
                        let manager_clone = Arc::clone(&manager);
                        let handle = thread::spawn(move || {
                            for j in 0u64..100 {
                                let mesh = MeshData::cube();
                                let id = AssetId::from_content(&(i * 1000 + j).to_le_bytes());
                                manager_clone.meshes().insert(id, mesh);
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }

                    black_box(manager);
                });
            },
        );
    }

    group.finish();
}

fn bench_lru_cache_access(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    c.bench_function("lru_access", |b| {
        let mut id_counter = 0u64;
        b.iter(|| {
            let id = AssetId::from_content(&id_counter.to_le_bytes());
            id_counter += 1;
            cache.access(black_box(id), engine_assets::AssetType::Mesh);
        });
    });
}

fn bench_lru_memory_tracking(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    c.bench_function("lru_memory_tracking", |b| {
        b.iter(|| {
            cache.update_memory_usage(black_box(1_000_000), engine_assets::AssetType::Mesh);
        });
    });
}

fn bench_lru_eviction_candidates(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let manager = AssetManager::new();

    // Insert many assets
    for i in 0u64..1000 {
        let id = AssetId::from_content(&i.to_le_bytes());
        cache.access(id, engine_assets::AssetType::Mesh);
    }

    c.bench_function("lru_eviction_candidates", |b| {
        b.iter(|| {
            let candidates = cache.eviction_candidates(
                engine_assets::AssetType::Mesh,
                manager.meshes().as_ref(),
                black_box(10),
            );
            black_box(candidates);
        });
    });
}

fn bench_manager_with_lru(c: &mut Criterion) {
    let mut group = c.benchmark_group("manager_with_lru");

    for asset_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(asset_count),
            asset_count,
            |b, &asset_count| {
                b.iter(|| {
                    let manager = AssetManager::new();
                    let budget = MemoryBudget::default();
                    let cache = LruCache::new(budget);

                    // Insert assets
                    for i in 0u64..asset_count as u64 {
                        let mesh = MeshData::cube();
                        let id = AssetId::from_content(&i.to_le_bytes());
                        let handle = manager.meshes().insert(id, mesh);
                        cache.access(handle.id(), engine_assets::AssetType::Mesh);
                    }

                    black_box((manager, cache));
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_stats(c: &mut Criterion) {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Setup some usage
    cache.update_memory_usage(1_000_000, engine_assets::AssetType::Mesh);
    cache.update_memory_usage(2_000_000, engine_assets::AssetType::Texture);

    c.bench_function("memory_stats_read", |b| {
        b.iter(|| {
            let stats = cache.stats();
            black_box(stats);
        });
    });
}

criterion_group!(
    benches,
    bench_manager_creation,
    bench_mesh_insertion,
    bench_mesh_retrieval,
    bench_handle_cloning,
    bench_concurrent_access,
    bench_lru_cache_access,
    bench_lru_memory_tracking,
    bench_lru_eviction_candidates,
    bench_manager_with_lru,
    bench_memory_stats,
);

criterion_main!(benches);
