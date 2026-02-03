//! Comprehensive mesh rendering benchmarks (Phase 1.8)
//!
//! Tests end-to-end rendering performance with ECS integration:
//! - MVP matrix calculations
//! - GPU mesh upload and caching
//! - Draw call overhead
//! - Scalability (1, 10, 100, 1K, 10K meshes)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetId, AssetManager, MeshData};
use engine_core::{Camera, MeshRenderer, Transform, Vec3, World};
use engine_renderer::{GpuCache, VulkanContext};
use glam::Quat;

/// Create a world with camera and N mesh entities
fn create_world_with_meshes(mesh_count: usize) -> (World, AssetManager) {
    let mut world = World::new();
    let mut assets = AssetManager::new();

    // Create camera
    let camera_entity = world.spawn();
    world.add(
        camera_entity,
        Transform::new(Vec3::new(0.0, 0.0, 10.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(camera_entity, Camera::default());

    // Create cube mesh
    let cube = MeshData::cube();
    let mesh_id = AssetId::from(1u64);
    assets.insert_mesh(mesh_id, cube);

    // Create mesh entities
    for i in 0..mesh_count {
        let x = (i % 10) as f32 * 2.5;
        let z = (i / 10) as f32 * 2.5;

        let entity = world.spawn();
        world.add(entity, Transform::new(Vec3::new(x, 0.0, -z), Quat::IDENTITY, Vec3::ONE));
        world.add(entity, MeshRenderer::new(mesh_id.into()));
    }

    (world, assets)
}

/// Benchmark MVP matrix calculation performance
fn bench_mvp_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvp_matrix");

    let transform = Transform::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ONE);
    let mut camera = Camera::default();

    group.bench_function("single_mvp", |b| {
        b.iter(|| {
            let model = transform.matrix();
            let view = camera.view_matrix(&transform);
            let projection = camera.projection_matrix();
            black_box(projection * view * model)
        });
    });

    // Batch MVP calculations
    for mesh_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*mesh_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_meshes", mesh_count)),
            mesh_count,
            |b, &count| {
                let transforms: Vec<Transform> = (0..count)
                    .map(|i| {
                        Transform::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE)
                    })
                    .collect();

                b.iter(|| {
                    let view = camera.view_matrix(&transforms[0]);
                    let projection = camera.projection_matrix();

                    for transform in &transforms {
                        let model = transform.matrix();
                        black_box(projection * view * model);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark ECS query performance for rendering
fn bench_ecs_query_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_query_rendering");

    for mesh_count in [10, 100, 1000, 10000].iter() {
        let (world, _assets) = create_world_with_meshes(*mesh_count);

        group.throughput(Throughput::Elements(*mesh_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_entities", mesh_count)),
            mesh_count,
            |b, _| {
                b.iter(|| {
                    // Query all renderable entities
                    let count = world.query::<(&Transform, &MeshRenderer)>().count();
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark camera transform queries
fn bench_camera_query(c: &mut Criterion) {
    let (world, _assets) = create_world_with_meshes(100);

    c.bench_function("find_camera", |b| {
        b.iter(|| {
            for entity in world.entities() {
                if let (Some(transform), Some(camera)) =
                    (world.get::<Transform>(entity), world.get::<Camera>(entity))
                {
                    black_box((transform, camera));
                    break;
                }
            }
        });
    });
}

/// Benchmark GPU mesh upload throughput
fn bench_gpu_mesh_upload(c: &mut Criterion) {
    let context = match VulkanContext::new("MeshUploadBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping GPU benchmarks - no Vulkan support");
            return;
        }
    };

    let mut group = c.benchmark_group("gpu_mesh_upload");

    // Different mesh complexities
    let meshes = vec![("triangle", MeshData::triangle()), ("cube", MeshData::cube())];

    for (name, mesh) in meshes {
        group.bench_with_input(BenchmarkId::from_parameter(name), &mesh, |b, mesh| {
            b.iter_batched(
                || GpuCache::new(&context).expect("Cache creation failed"),
                |mut cache| {
                    let asset_id = AssetId::from_bytes(name.as_bytes());
                    cache
                        .upload_mesh(&context, black_box(asset_id), black_box(mesh))
                        .expect("Upload failed");
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch mesh upload (simulating level load)
fn bench_batch_mesh_upload(c: &mut Criterion) {
    let context = match VulkanContext::new("BatchUploadBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut group = c.benchmark_group("batch_mesh_upload");

    for mesh_count in [10, 100].iter() {
        let meshes: Vec<(AssetId, MeshData)> =
            (0..*mesh_count).map(|i| (AssetId::from(i as u64), MeshData::cube())).collect();

        group.throughput(Throughput::Elements(*mesh_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_meshes", mesh_count)),
            &meshes,
            |b, meshes| {
                b.iter_batched(
                    || GpuCache::new(&context).expect("Cache creation failed"),
                    |mut cache| {
                        for (asset_id, mesh) in meshes {
                            cache
                                .upload_mesh(&context, black_box(*asset_id), black_box(mesh))
                                .expect("Upload failed");
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark mesh cache hit performance
fn bench_cache_hit_rate(c: &mut Criterion) {
    let context = match VulkanContext::new("CacheHitBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut cache = GpuCache::new(&context).expect("Cache creation failed");

    // Pre-populate cache
    for i in 0..100 {
        let mesh = MeshData::cube();
        let asset_id = AssetId::from(i as u64);
        cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
    }

    c.bench_function("cache_lookup_hit", |b| {
        let asset_id = AssetId::from(50u64);
        b.iter(|| {
            black_box(cache.contains(black_box(asset_id)));
        });
    });

    c.bench_function("cache_lookup_miss", |b| {
        let asset_id = AssetId::from(999u64);
        b.iter(|| {
            black_box(cache.contains(black_box(asset_id)));
        });
    });
}

/// Benchmark full rendering pipeline setup (without actual GPU draw calls)
fn bench_render_setup_overhead(c: &mut Criterion) {
    let context = match VulkanContext::new("RenderSetupBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut group = c.benchmark_group("render_setup");

    for mesh_count in [10, 100, 1000].iter() {
        let (world, assets) = create_world_with_meshes(*mesh_count);

        group.throughput(Throughput::Elements(*mesh_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_meshes", mesh_count)),
            mesh_count,
            |b, _| {
                let mut gpu_cache = GpuCache::new(&context).expect("Cache creation failed");

                // Pre-upload meshes
                for asset_id in 1..=1 {
                    if let Some(mesh) = assets.get_mesh(AssetId::from(asset_id)) {
                        gpu_cache
                            .upload_mesh(&context, AssetId::from(asset_id), &*mesh)
                            .expect("Upload failed");
                    }
                }

                b.iter(|| {
                    // Find camera
                    let mut camera_transform: Option<&Transform> = None;
                    let mut camera_comp: Option<&mut Camera> = None;

                    for entity in world.entities() {
                        if let (Some(transform), Some(camera)) =
                            (world.get::<Transform>(entity), world.get::<Camera>(entity))
                        {
                            camera_transform = Some(transform);
                            camera_comp = Some(camera);
                            break;
                        }
                    }

                    let default_transform = Transform::default();
                    let mut default_camera = Camera::default();
                    let (cam_transform, cam) = match (camera_transform, camera_comp) {
                        (Some(t), Some(c)) => (t, c),
                        _ => (&default_transform, &mut default_camera),
                    };

                    // Calculate VP matrix
                    let view_matrix = cam.view_matrix(cam_transform);
                    let proj_matrix = cam.projection_matrix();
                    let vp_matrix = proj_matrix * view_matrix;

                    // Query and calculate MVP for all entities
                    let mut mvp_count = 0;
                    for entity in world.entities() {
                        if let (Some(transform), Some(mesh_renderer)) =
                            (world.get::<Transform>(entity), world.get::<MeshRenderer>(entity))
                        {
                            if !mesh_renderer.is_visible() {
                                continue;
                            }

                            let model_matrix = transform.matrix();
                            let _mvp_matrix = vp_matrix * model_matrix;
                            mvp_count += 1;
                        }
                    }

                    black_box(mvp_count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transform matrix calculation
fn bench_transform_matrix(c: &mut Criterion) {
    let transform =
        Transform::new(Vec3::new(1.0, 2.0, 3.0), Quat::from_axis_angle(Vec3::Y, 0.5), Vec3::ONE);

    c.bench_function("transform_to_matrix", |b| {
        b.iter(|| {
            black_box(transform.matrix());
        });
    });

    c.bench_function("transform_matrix_cached", |b| {
        b.iter(|| {
            // Using to_matrix which uses cached affine
            black_box(transform.to_matrix());
        });
    });
}

/// Benchmark camera matrix calculations
fn bench_camera_matrices(c: &mut Criterion) {
    let mut camera = Camera::default();
    let camera_transform = Transform::new(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY, Vec3::ONE);

    c.bench_function("camera_view_matrix", |b| {
        b.iter(|| {
            black_box(camera.view_matrix(&camera_transform));
        });
    });

    c.bench_function("camera_projection_matrix", |b| {
        b.iter(|| {
            black_box(camera.projection_matrix());
        });
    });

    c.bench_function("camera_view_projection", |b| {
        b.iter(|| {
            black_box(camera.view_projection_matrix(&camera_transform));
        });
    });
}

criterion_group!(
    benches,
    bench_mvp_calculation,
    bench_ecs_query_rendering,
    bench_camera_query,
    bench_gpu_mesh_upload,
    bench_batch_mesh_upload,
    bench_cache_hit_rate,
    bench_render_setup_overhead,
    bench_transform_matrix,
    bench_camera_matrices
);
criterion_main!(benches);
