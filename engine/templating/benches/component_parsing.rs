//! Benchmarks for component parsing performance.
//!
//! This module benchmarks the conversion of YAML values to component types:
//! - Transform parsing (position, rotation, scale)
//! - Health parsing (simple component)
//! - MeshRenderer parsing (with mesh_id or mesh path)
//! - Camera parsing (fov, aspect, near, far)
//! - Parsing with missing optional fields (defaults)
//!
//! # Performance Targets
//!
//! - Simple component (Health): < 1µs
//! - Complex component (Transform): < 5µs
//! - Very complex (MeshRenderer with hashing): < 10µs
//! - Camera component: < 3µs
//! - Defaults handling: < 500ns

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::rendering::{Camera, MeshRenderer};
use serde_yaml::Value;

/// Helper to simulate the parse_transform function
fn parse_transform(value: &Value) -> Transform {
    use engine_core::math::{Quat, Vec3};

    if value.is_null() {
        return Transform::default();
    }

    let position = if let Some(pos) = value.get("position") { parse_vec3(pos) } else { Vec3::ZERO };

    let rotation = if let Some(rot) = value.get("rotation") {
        parse_quat(rot)
    } else {
        Quat::IDENTITY
    };

    let scale = if let Some(scl) = value.get("scale") { parse_vec3(scl) } else { Vec3::ONE };

    Transform::new(position, rotation, scale)
}

/// Helper to simulate the parse_health function
fn parse_health(value: &Value) -> Health {
    if value.is_null() {
        return Health::new(100.0, 100.0);
    }

    let current = value.get("current").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
    let max = value.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;

    Health::new(current, max)
}

/// Helper to simulate the parse_mesh_renderer function
fn parse_mesh_renderer(value: &Value) -> MeshRenderer {
    if value.is_null() {
        return MeshRenderer::new(0);
    }

    let mesh_id = if let Some(id) = value.get("mesh_id") {
        id.as_u64().unwrap_or(0)
    } else if let Some(mesh_path) = value.get("mesh") {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        mesh_path.as_str().unwrap_or("").hash(&mut hasher);
        hasher.finish()
    } else {
        0
    };

    let visible = value.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);

    MeshRenderer::with_visibility(mesh_id, visible)
}

/// Helper to simulate the parse_camera function
fn parse_camera(value: &Value) -> Camera {
    if value.is_null() {
        return Camera::default();
    }

    let fov = value.get("fov").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
    let fov_radians = fov.to_radians();
    let aspect = value.get("aspect").and_then(|v| v.as_f64()).unwrap_or(16.0 / 9.0) as f32;
    let near = value.get("near").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
    let far = value.get("far").and_then(|v| v.as_f64()).unwrap_or(1000.0) as f32;

    Camera::with_planes(fov_radians, aspect, near, far)
}

/// Helper to parse Vec3
fn parse_vec3(value: &Value) -> engine_core::math::Vec3 {
    use engine_core::math::Vec3;

    if let Some(seq) = value.as_sequence() {
        if seq.len() >= 3 {
            let x = seq[0].as_f64().unwrap_or(0.0) as f32;
            let y = seq[1].as_f64().unwrap_or(0.0) as f32;
            let z = seq[2].as_f64().unwrap_or(0.0) as f32;
            return Vec3::new(x, y, z);
        }
    }

    Vec3::ZERO
}

/// Helper to parse Quat
fn parse_quat(value: &Value) -> engine_core::math::Quat {
    use engine_core::math::Quat;

    if let Some(seq) = value.as_sequence() {
        if seq.len() >= 4 {
            let x = seq[0].as_f64().unwrap_or(0.0) as f32;
            let y = seq[1].as_f64().unwrap_or(0.0) as f32;
            let z = seq[2].as_f64().unwrap_or(0.0) as f32;
            let w = seq[3].as_f64().unwrap_or(1.0) as f32;
            return Quat::from_xyzw(x, y, z, w);
        }
    }

    Quat::IDENTITY
}

/// Benchmark: Parse Transform from YAML Value
fn bench_parse_transform(c: &mut Criterion) {
    // Full transform with all fields
    let yaml_full = serde_yaml::from_str::<Value>(
        r#"
position: [1.0, 2.0, 3.0]
rotation: [0.0, 0.0, 0.0, 1.0]
scale: [2.0, 2.0, 2.0]
"#,
    )
    .unwrap();

    c.bench_function("component_parse_transform_full", |b| {
        b.iter(|| {
            let transform = parse_transform(black_box(&yaml_full));
            black_box(transform);
        });
    });

    // Transform with only position (partial)
    let yaml_partial = serde_yaml::from_str::<Value>(
        r#"
position: [5.0, 10.0, 15.0]
"#,
    )
    .unwrap();

    c.bench_function("component_parse_transform_partial", |b| {
        b.iter(|| {
            let transform = parse_transform(black_box(&yaml_partial));
            black_box(transform);
        });
    });

    // Null transform (all defaults)
    let yaml_null = Value::Null;

    c.bench_function("component_parse_transform_null", |b| {
        b.iter(|| {
            let transform = parse_transform(black_box(&yaml_null));
            black_box(transform);
        });
    });
}

/// Benchmark: Parse Health component
fn bench_parse_health(c: &mut Criterion) {
    // Full health with current and max
    let yaml_full = serde_yaml::from_str::<Value>(
        r#"
current: 75.0
max: 100.0
"#,
    )
    .unwrap();

    c.bench_function("component_parse_health_full", |b| {
        b.iter(|| {
            let health = parse_health(black_box(&yaml_full));
            black_box(health);
        });
    });

    // Partial health (only current, max uses default)
    let yaml_partial = serde_yaml::from_str::<Value>(
        r#"
current: 50.0
"#,
    )
    .unwrap();

    c.bench_function("component_parse_health_partial", |b| {
        b.iter(|| {
            let health = parse_health(black_box(&yaml_partial));
            black_box(health);
        });
    });

    // Null health (all defaults)
    let yaml_null = Value::Null;

    c.bench_function("component_parse_health_null", |b| {
        b.iter(|| {
            let health = parse_health(black_box(&yaml_null));
            black_box(health);
        });
    });
}

/// Benchmark: Parse MeshRenderer component
fn bench_parse_mesh_renderer(c: &mut Criterion) {
    // MeshRenderer with explicit mesh_id
    let yaml_id = serde_yaml::from_str::<Value>(
        r#"
mesh_id: 12345
visible: true
"#,
    )
    .unwrap();

    c.bench_function("component_parse_mesh_renderer_id", |b| {
        b.iter(|| {
            let renderer = parse_mesh_renderer(black_box(&yaml_id));
            black_box(renderer);
        });
    });

    // MeshRenderer with mesh path (requires hashing)
    let yaml_path = serde_yaml::from_str::<Value>(
        r#"
mesh: "assets/models/character/warrior.obj"
visible: true
"#,
    )
    .unwrap();

    c.bench_function("component_parse_mesh_renderer_path", |b| {
        b.iter(|| {
            let renderer = parse_mesh_renderer(black_box(&yaml_path));
            black_box(renderer);
        });
    });

    // MeshRenderer with long path (stress test hashing)
    let yaml_long_path = serde_yaml::from_str::<Value>(
        r#"
mesh: "assets/environments/fantasy/castles/fortress_walls/stone_wall_detailed_highpoly_lod0.fbx"
visible: false
"#,
    )
    .unwrap();

    c.bench_function("component_parse_mesh_renderer_long_path", |b| {
        b.iter(|| {
            let renderer = parse_mesh_renderer(black_box(&yaml_long_path));
            black_box(renderer);
        });
    });

    // Null mesh renderer (defaults)
    let yaml_null = Value::Null;

    c.bench_function("component_parse_mesh_renderer_null", |b| {
        b.iter(|| {
            let renderer = parse_mesh_renderer(black_box(&yaml_null));
            black_box(renderer);
        });
    });
}

/// Benchmark: Parse Camera component
fn bench_parse_camera(c: &mut Criterion) {
    // Full camera with all parameters
    let yaml_full = serde_yaml::from_str::<Value>(
        r#"
fov: 90.0
aspect: 1.777777
near: 0.1
far: 1000.0
"#,
    )
    .unwrap();

    c.bench_function("component_parse_camera_full", |b| {
        b.iter(|| {
            let camera = parse_camera(black_box(&yaml_full));
            black_box(camera);
        });
    });

    // Partial camera (only fov)
    let yaml_partial = serde_yaml::from_str::<Value>(
        r#"
fov: 75.0
"#,
    )
    .unwrap();

    c.bench_function("component_parse_camera_partial", |b| {
        b.iter(|| {
            let camera = parse_camera(black_box(&yaml_partial));
            black_box(camera);
        });
    });

    // Null camera (all defaults)
    let yaml_null = Value::Null;

    c.bench_function("component_parse_camera_null", |b| {
        b.iter(|| {
            let camera = parse_camera(black_box(&yaml_null));
            black_box(camera);
        });
    });
}

/// Benchmark: Parse with defaults (comparing overhead of default field handling)
fn bench_parse_with_defaults(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_parse_defaults");

    // Health: null vs partial vs full
    let health_null = Value::Null;
    let health_partial = serde_yaml::from_str::<Value>("current: 50.0").unwrap();
    let health_full = serde_yaml::from_str::<Value>("current: 50.0\nmax: 100.0").unwrap();

    group.bench_function("health_null", |b| {
        b.iter(|| black_box(parse_health(black_box(&health_null))));
    });

    group.bench_function("health_partial", |b| {
        b.iter(|| black_box(parse_health(black_box(&health_partial))));
    });

    group.bench_function("health_full", |b| {
        b.iter(|| black_box(parse_health(black_box(&health_full))));
    });

    // Transform: null vs partial vs full
    let transform_null = Value::Null;
    let transform_partial = serde_yaml::from_str::<Value>("position: [1.0, 2.0, 3.0]").unwrap();
    let transform_full = serde_yaml::from_str::<Value>(
        "position: [1.0, 2.0, 3.0]\nrotation: [0.0, 0.0, 0.0, 1.0]\nscale: [1.0, 1.0, 1.0]",
    )
    .unwrap();

    group.bench_function("transform_null", |b| {
        b.iter(|| black_box(parse_transform(black_box(&transform_null))));
    });

    group.bench_function("transform_partial", |b| {
        b.iter(|| black_box(parse_transform(black_box(&transform_partial))));
    });

    group.bench_function("transform_full", |b| {
        b.iter(|| black_box(parse_transform(black_box(&transform_full))));
    });

    group.finish();
}

/// Benchmark: Varying data sizes
fn bench_parse_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_parse_scaling");

    // MeshRenderer with varying path lengths
    for path_len in [10, 50, 100, 200].iter() {
        let path = "a".repeat(*path_len);
        let yaml = serde_yaml::from_str::<Value>(&format!("mesh: \"{}\"", path)).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(path_len), path_len, |b, _| {
            b.iter(|| {
                let renderer = parse_mesh_renderer(black_box(&yaml));
                black_box(renderer);
            });
        });
    }

    group.finish();
}

/// Benchmark: Batch component parsing (simulating loading multiple components)
fn bench_parse_batch(c: &mut Criterion) {
    // Create arrays of YAML values
    let transforms: Vec<Value> = (0..100)
        .map(|i| {
            serde_yaml::from_str(&format!(
                "position: [{}, {}, {}]\nrotation: [0, 0, 0, 1]\nscale: [1, 1, 1]",
                i,
                i * 2,
                i * 3
            ))
            .unwrap()
        })
        .collect();

    let healths: Vec<Value> = (0..100)
        .map(|i| serde_yaml::from_str(&format!("current: {}.0\nmax: 100.0", i)).unwrap())
        .collect();

    c.bench_function("component_parse_batch_100_transforms", |b| {
        b.iter(|| {
            for transform_yaml in &transforms {
                let transform = parse_transform(black_box(transform_yaml));
                black_box(transform);
            }
        });
    });

    c.bench_function("component_parse_batch_100_healths", |b| {
        b.iter(|| {
            for health_yaml in &healths {
                let health = parse_health(black_box(health_yaml));
                black_box(health);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_parse_transform,
    bench_parse_health,
    bench_parse_mesh_renderer,
    bench_parse_camera,
    bench_parse_with_defaults,
    bench_parse_scaling,
    bench_parse_batch,
);

criterion_main!(benches);
