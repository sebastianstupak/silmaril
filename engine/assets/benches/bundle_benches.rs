//! Benchmarks for asset bundle system.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{
    AssetBundle, AssetEntry, AssetId, AssetManifest, AssetType, CompressionFormat,
};
use std::path::PathBuf;

fn create_manifest_with_data(
    count: usize,
    data_size: usize,
) -> (AssetManifest, Vec<(AssetId, Vec<u8>)>) {
    let mut manifest = AssetManifest::new();
    let mut asset_data = Vec::new();

    for i in 0..count {
        let name = format!("asset{i}");
        // Create data of specified size
        let data = vec![b'A' + (i % 26) as u8; data_size];
        let id = AssetId::from_content(name.as_bytes());

        let entry = AssetEntry::new(
            id,
            PathBuf::from(format!("{name}.dat")),
            AssetType::Mesh,
            data.len() as u64,
            *blake3::hash(&data).as_bytes(),
        );

        manifest.add_asset(entry);
        asset_data.push((id, data));
    }

    (manifest, asset_data)
}

fn bench_bundle_packing_no_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_packing_no_compression");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let total_bytes = (count * size) as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &(manifest, asset_data),
            |b, (manifest, asset_data)| {
                b.iter(|| {
                    let mut bundle =
                        AssetBundle::from_manifest(manifest.clone(), CompressionFormat::None);
                    for (id, data) in asset_data {
                        bundle.add_asset(*id, data.clone()).unwrap();
                    }
                    let packed = bundle.pack().unwrap();
                    black_box(packed);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "lz4")]
fn bench_bundle_packing_lz4(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_packing_lz4");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let total_bytes = (count * size) as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &(manifest, asset_data),
            |b, (manifest, asset_data)| {
                b.iter(|| {
                    let mut bundle =
                        AssetBundle::from_manifest(manifest.clone(), CompressionFormat::Lz4);
                    for (id, data) in asset_data {
                        bundle.add_asset(*id, data.clone()).unwrap();
                    }
                    let packed = bundle.pack().unwrap();
                    black_box(packed);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "zstd")]
fn bench_bundle_packing_zstd(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_packing_zstd");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let total_bytes = (count * size) as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &(manifest, asset_data),
            |b, (manifest, asset_data)| {
                b.iter(|| {
                    let mut bundle =
                        AssetBundle::from_manifest(manifest.clone(), CompressionFormat::Zstd);
                    for (id, data) in asset_data {
                        bundle.add_asset(*id, data.clone()).unwrap();
                    }
                    let packed = bundle.pack().unwrap();
                    black_box(packed);
                });
            },
        );
    }

    group.finish();
}

fn bench_bundle_unpacking_no_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_unpacking_no_compression");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        for (id, data) in &asset_data {
            bundle.add_asset(*id, data.clone()).unwrap();
        }

        let packed = bundle.pack().unwrap();
        let total_bytes = packed.len() as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &packed,
            |b, packed| {
                b.iter(|| {
                    let unpacked = AssetBundle::unpack(black_box(packed)).unwrap();
                    black_box(unpacked);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "lz4")]
fn bench_bundle_unpacking_lz4(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_unpacking_lz4");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Lz4);

        for (id, data) in &asset_data {
            bundle.add_asset(*id, data.clone()).unwrap();
        }

        let packed = bundle.pack().unwrap();
        let total_bytes = packed.len() as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &packed,
            |b, packed| {
                b.iter(|| {
                    let unpacked = AssetBundle::unpack(black_box(packed)).unwrap();
                    black_box(unpacked);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "zstd")]
fn bench_bundle_unpacking_zstd(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_unpacking_zstd");

    for (count, size) in [(10, 1024), (100, 1024), (10, 102_400)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::Zstd);

        for (id, data) in &asset_data {
            bundle.add_asset(*id, data.clone()).unwrap();
        }

        let packed = bundle.pack().unwrap();
        let total_bytes = packed.len() as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &packed,
            |b, packed| {
                b.iter(|| {
                    let unpacked = AssetBundle::unpack(black_box(packed)).unwrap();
                    black_box(unpacked);
                });
            },
        );
    }

    group.finish();
}

fn bench_bundle_asset_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_asset_lookup");

    for count in [10, 100, 1000] {
        let (manifest, asset_data) = create_manifest_with_data(count, 1024);
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        for (id, data) in &asset_data {
            bundle.add_asset(*id, data.clone()).unwrap();
        }

        let lookup_id = asset_data[count / 2].0; // Middle element

        group.bench_with_input(BenchmarkId::from_parameter(count), &bundle, |b, bundle| {
            b.iter(|| {
                let data = bundle.get_asset(black_box(lookup_id));
                black_box(data);
            });
        });
    }

    group.finish();
}

#[cfg(all(feature = "lz4", feature = "zstd"))]
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    // Test with highly compressible data
    let (manifest, asset_data) = create_manifest_with_data(10, 10_240);

    for format in [CompressionFormat::None, CompressionFormat::Lz4, CompressionFormat::Zstd] {
        let mut bundle = AssetBundle::from_manifest(manifest.clone(), format);
        for (id, data) in &asset_data {
            bundle.add_asset(*id, data.clone()).unwrap();
        }

        let name = match format {
            CompressionFormat::None => "none",
            CompressionFormat::Lz4 => "lz4",
            CompressionFormat::Zstd => "zstd",
        };

        group.bench_with_input(BenchmarkId::from_parameter(name), &bundle, |b, bundle| {
            b.iter(|| {
                let packed = bundle.pack().unwrap();
                black_box(packed);
            });
        });
    }

    group.finish();
}

fn bench_bundle_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_round_trip");

    for (count, size) in [(10, 1024), (100, 1024)] {
        let (manifest, asset_data) = create_manifest_with_data(count, size);
        let total_bytes = (count * size) as u64;

        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}B", count, size)),
            &(manifest, asset_data),
            |b, (manifest, asset_data)| {
                b.iter(|| {
                    // Pack
                    let mut bundle =
                        AssetBundle::from_manifest(manifest.clone(), CompressionFormat::None);
                    for (id, data) in asset_data {
                        bundle.add_asset(*id, data.clone()).unwrap();
                    }
                    let packed = bundle.pack().unwrap();

                    // Unpack
                    let unpacked = AssetBundle::unpack(&packed).unwrap();
                    black_box(unpacked);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_bundle_packing_no_compression,
    bench_bundle_unpacking_no_compression,
    bench_bundle_asset_lookup,
    bench_bundle_round_trip,
);

#[cfg(feature = "lz4")]
criterion_group!(benches_lz4, bench_bundle_packing_lz4, bench_bundle_unpacking_lz4,);

#[cfg(feature = "zstd")]
criterion_group!(benches_zstd, bench_bundle_packing_zstd, bench_bundle_unpacking_zstd,);

#[cfg(all(feature = "lz4", feature = "zstd"))]
criterion_group!(benches_compression, bench_compression_ratio,);

#[cfg(all(feature = "lz4", feature = "zstd"))]
criterion_main!(benches, benches_lz4, benches_zstd, benches_compression);

#[cfg(all(feature = "lz4", not(feature = "zstd")))]
criterion_main!(benches, benches_lz4);

#[cfg(all(not(feature = "lz4"), feature = "zstd"))]
criterion_main!(benches, benches_zstd);

#[cfg(not(any(feature = "lz4", feature = "zstd")))]
criterion_main!(benches);
