//! Benchmarks for asset manifest system.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetEntry, AssetId, AssetManifest, AssetType};
use std::path::PathBuf;

fn create_test_entry(name: &str, deps: &[AssetId]) -> AssetEntry {
    let id = AssetId::from_content(name.as_bytes());
    let data = format!("test data for {name}");
    let mut entry = AssetEntry::new(
        id,
        PathBuf::from(format!("{name}.dat")),
        AssetType::Mesh,
        data.len() as u64,
        *blake3::hash(data.as_bytes()).as_bytes(),
    );

    for &dep in deps {
        entry.add_dependency(dep);
    }

    entry
}

fn bench_manifest_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_creation");

    for size in [10, 100, 1000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut manifest = AssetManifest::new();
                for i in 0..size {
                    let entry = create_test_entry(&format!("asset{i}"), &[]);
                    manifest.add_asset(black_box(entry));
                }
                black_box(manifest);
            });
        });
    }

    group.finish();
}

fn bench_manifest_serialization_yaml(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_serialization_yaml");

    for size in [10, 100, 1000] {
        let mut manifest = AssetManifest::new();
        for i in 0..size {
            manifest.add_asset(create_test_entry(&format!("asset{i}"), &[]));
        }

        let size_bytes = manifest.to_yaml().unwrap().len() as u64;
        group.throughput(Throughput::Bytes(size_bytes));

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let yaml = manifest.to_yaml().unwrap();
                black_box(yaml);
            });
        });
    }

    group.finish();
}

fn bench_manifest_deserialization_yaml(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_deserialization_yaml");

    for size in [10, 100, 1000] {
        let mut manifest = AssetManifest::new();
        for i in 0..size {
            manifest.add_asset(create_test_entry(&format!("asset{i}"), &[]));
        }

        let yaml = manifest.to_yaml().unwrap();
        let size_bytes = yaml.len() as u64;
        group.throughput(Throughput::Bytes(size_bytes));

        group.bench_with_input(BenchmarkId::from_parameter(size), &yaml, |b, yaml| {
            b.iter(|| {
                let manifest = AssetManifest::from_yaml(yaml).unwrap();
                black_box(manifest);
            });
        });
    }

    group.finish();
}

fn bench_manifest_serialization_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_serialization_bincode");

    for size in [10, 100, 1000, 10_000] {
        let mut manifest = AssetManifest::new();
        for i in 0..size {
            manifest.add_asset(create_test_entry(&format!("asset{i}"), &[]));
        }

        let size_bytes = manifest.to_bincode().unwrap().len() as u64;
        group.throughput(Throughput::Bytes(size_bytes));

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let bytes = manifest.to_bincode().unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_manifest_deserialization_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_deserialization_bincode");

    for size in [10, 100, 1000, 10_000] {
        let mut manifest = AssetManifest::new();
        for i in 0..size {
            manifest.add_asset(create_test_entry(&format!("asset{i}"), &[]));
        }

        let bytes = manifest.to_bincode().unwrap();
        let size_bytes = bytes.len() as u64;
        group.throughput(Throughput::Bytes(size_bytes));

        group.bench_with_input(BenchmarkId::from_parameter(size), &bytes, |b, bytes| {
            b.iter(|| {
                let manifest = AssetManifest::from_bincode(bytes).unwrap();
                black_box(manifest);
            });
        });
    }

    group.finish();
}

fn bench_dependency_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_resolution");

    for size in [10, 100, 1000] {
        // Create linear dependency chain
        let mut manifest = AssetManifest::new();
        let mut prev_id = None;

        for i in 0..size {
            let deps = if let Some(id) = prev_id { vec![id] } else { vec![] };
            let entry = create_test_entry(&format!("asset{i}"), &deps);
            prev_id = Some(entry.id);
            manifest.add_asset(entry);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let sorted = manifest.topological_sort().unwrap();
                black_box(sorted);
            });
        });
    }

    group.finish();
}

fn bench_cyclic_dependency_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("cyclic_dependency_detection");

    for size in [10, 50, 100] {
        // Create chain with cycle at end
        let mut manifest = AssetManifest::new();
        let mut ids = Vec::new();

        for i in 0..size {
            let entry = create_test_entry(&format!("asset{i}"), &[]);
            ids.push(entry.id);
            manifest.add_asset(entry);
        }

        // Create cycle: last -> first
        if let Some(entry) = manifest.get_asset_mut(ids[ids.len() - 1]) {
            entry.add_dependency(ids[0]);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let result = manifest.validate();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_manifest_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_validation");

    for size in [10, 100, 1000, 10_000] {
        let mut manifest = AssetManifest::new();

        // Create some dependencies but no cycles
        for i in 0..size {
            let deps = if i > 0 {
                let prev_id = AssetId::from_content(format!("asset{}", i - 1).as_bytes());
                vec![prev_id]
            } else {
                vec![]
            };
            manifest.add_asset(create_test_entry(&format!("asset{i}"), &deps));
        }

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let result = manifest.validate();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_manifest_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_lookup");

    for size in [100, 1000, 10_000] {
        let mut manifest = AssetManifest::new();
        let mut ids = Vec::new();

        for i in 0..size {
            let entry = create_test_entry(&format!("asset{i}"), &[]);
            ids.push(entry.id);
            manifest.add_asset(entry);
        }

        let lookup_id = ids[size / 2]; // Middle element

        group.bench_with_input(BenchmarkId::from_parameter(size), &manifest, |b, manifest| {
            b.iter(|| {
                let entry = manifest.get_asset(black_box(lookup_id));
                black_box(entry);
            });
        });
    }

    group.finish();
}

fn bench_manifest_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_merge");

    for size in [10, 100, 1000] {
        let mut manifest1 = AssetManifest::new();
        let mut manifest2 = AssetManifest::new();

        for i in 0..size {
            manifest1.add_asset(create_test_entry(&format!("asset1_{i}"), &[]));
            manifest2.add_asset(create_test_entry(&format!("asset2_{i}"), &[]));
        }

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(manifest1, manifest2),
            |b, (m1, m2)| {
                b.iter(|| {
                    let mut manifest = m1.clone();
                    manifest.merge(black_box(m2));
                    black_box(manifest);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_manifest_creation,
    bench_manifest_serialization_yaml,
    bench_manifest_deserialization_yaml,
    bench_manifest_serialization_bincode,
    bench_manifest_deserialization_bincode,
    bench_dependency_resolution,
    bench_cyclic_dependency_detection,
    bench_manifest_validation,
    bench_manifest_lookup,
    bench_manifest_merge,
);

criterion_main!(benches);
