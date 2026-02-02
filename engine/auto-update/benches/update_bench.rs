//! Benchmarks for the auto-update system.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_auto_update::{
    patcher::{apply_patch, create_patch},
    verifier::{compute_file_hash, verify_file_hash},
    version::Version,
};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn bench_version_parsing(c: &mut Criterion) {
    c.bench_function("version_parsing", |b| {
        b.iter(|| {
            let v: Version = black_box("1.2.3").parse().unwrap();
            v
        });
    });
}

fn bench_version_comparison(c: &mut Criterion) {
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(1, 0, 1);

    c.bench_function("version_comparison", |b| {
        b.iter(|| black_box(&v1) < black_box(&v2));
    });
}

fn bench_file_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_hashing");

    for size_kb in [1, 10, 100, 1000].iter() {
        let size = size_kb * 1024;
        group.throughput(Throughput::Bytes(size as u64));

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.bin");

        // Create test file with random data
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        fs::write(&test_file, data).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size_kb), &test_file, |b, path| {
            b.iter(|| compute_file_hash(black_box(path)).unwrap());
        });
    }

    group.finish();
}

fn bench_file_verification(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.bin");

    // Create 100KB test file
    let data: Vec<u8> = (0..102400).map(|i| (i % 256) as u8).collect();
    fs::write(&test_file, data).unwrap();

    let hash = compute_file_hash(&test_file).unwrap();

    c.bench_function("file_verification_100kb", |b| {
        b.iter(|| verify_file_hash(black_box(&test_file), black_box(&hash)).unwrap());
    });
}

fn bench_patch_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("patch_creation");

    for size_kb in [10, 100, 1000].iter() {
        let size = size_kb * 1024;
        group.throughput(Throughput::Bytes(size as u64));
        group.sample_size(10); // Reduce sample size for large files

        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.bin");
        let new_file = temp_dir.path().join("new.bin");
        let patch_file = temp_dir.path().join("patch.bin");

        // Create old file
        let old_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        fs::write(&old_file, old_data).unwrap();

        // Create new file (with 10% changes)
        let mut new_data = old_data.clone();
        for i in (0..size).step_by(10) {
            new_data[i] = ((new_data[i] as usize + 1) % 256) as u8;
        }
        fs::write(&new_file, new_data).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size_kb),
            &(old_file, new_file, patch_file),
            |b, (old, new, patch)| {
                b.iter(|| {
                    create_patch(black_box(old), black_box(new), black_box(patch)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_patch_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("patch_application");

    for size_kb in [10, 100, 1000].iter() {
        let size = size_kb * 1024;
        group.throughput(Throughput::Bytes(size as u64));
        group.sample_size(10);

        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.bin");
        let new_file = temp_dir.path().join("new.bin");
        let patch_file = temp_dir.path().join("patch.bin");
        let result_file = temp_dir.path().join("result.bin");

        // Create files and patch
        let old_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        fs::write(&old_file, &old_data).unwrap();

        let mut new_data = old_data.clone();
        for i in (0..size).step_by(10) {
            new_data[i] = ((new_data[i] as usize + 1) % 256) as u8;
        }
        fs::write(&new_file, new_data).unwrap();

        create_patch(&old_file, &new_file, &patch_file).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size_kb),
            &(old_file, patch_file, result_file),
            |b, (old, patch, result)| {
                b.iter(|| {
                    apply_patch(black_box(old), black_box(patch), black_box(result)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    // This benchmark measures patch size vs original size
    let mut group = c.benchmark_group("compression_analysis");

    for change_percent in [1, 5, 10, 25, 50].iter() {
        let size = 100 * 1024; // 100KB
        let temp_dir = TempDir::new().unwrap();
        let old_file = temp_dir.path().join("old.bin");
        let new_file = temp_dir.path().join("new.bin");
        let patch_file = temp_dir.path().join("patch.bin");

        // Create old file
        let old_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        fs::write(&old_file, &old_data).unwrap();

        // Create new file with specified percentage of changes
        let mut new_data = old_data.clone();
        let change_step = 100 / change_percent;
        for i in (0..size).step_by(change_step) {
            new_data[i] = ((new_data[i] as usize + 1) % 256) as u8;
        }
        fs::write(&new_file, new_data).unwrap();

        create_patch(&old_file, &new_file, &patch_file).unwrap();

        let patch_size = fs::metadata(&patch_file).unwrap().len();
        let ratio = (patch_size as f64 / size as f64) * 100.0;

        println!("{}% changes -> patch is {:.1}% of original size", change_percent, ratio);
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_version_parsing,
    bench_version_comparison,
    bench_file_hashing,
    bench_file_verification,
    bench_patch_creation,
    bench_patch_application,
    bench_compression_ratio,
);
criterion_main!(benches);
