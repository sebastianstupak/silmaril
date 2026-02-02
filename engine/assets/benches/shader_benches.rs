use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_assets::{ShaderData, ShaderStage};

// ============================================================================
// GLSL Parsing Benchmarks
// ============================================================================

fn bench_glsl_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("glsl_parsing");

    // Small vertex shader
    let small_vertex = r#"
        #version 450
        layout(location = 0) in vec3 position;
        layout(location = 1) in vec3 normal;
        layout(location = 2) in vec2 uv;

        layout(set = 0, binding = 0) uniform UniformBufferObject {
            mat4 model;
            mat4 view;
            mat4 proj;
        } ubo;

        layout(location = 0) out vec3 fragNormal;
        layout(location = 1) out vec2 fragUV;

        void main() {
            gl_Position = ubo.proj * ubo.view * ubo.model * vec4(position, 1.0);
            fragNormal = normal;
            fragUV = uv;
        }
    "#
    .to_string();

    // Medium fragment shader with lighting
    let medium_fragment = r#"
        #version 450

        layout(location = 0) in vec3 fragNormal;
        layout(location = 1) in vec2 fragUV;
        layout(location = 2) in vec3 fragWorldPos;

        layout(location = 0) out vec4 outColor;

        layout(set = 0, binding = 1) uniform sampler2D texSampler;
        layout(set = 0, binding = 2) uniform LightData {
            vec3 lightPos;
            vec3 lightColor;
            vec3 viewPos;
        } light;

        void main() {
            // Ambient
            float ambientStrength = 0.1;
            vec3 ambient = ambientStrength * light.lightColor;

            // Diffuse
            vec3 norm = normalize(fragNormal);
            vec3 lightDir = normalize(light.lightPos - fragWorldPos);
            float diff = max(dot(norm, lightDir), 0.0);
            vec3 diffuse = diff * light.lightColor;

            // Specular
            float specularStrength = 0.5;
            vec3 viewDir = normalize(light.viewPos - fragWorldPos);
            vec3 reflectDir = reflect(-lightDir, norm);
            float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
            vec3 specular = specularStrength * spec * light.lightColor;

            // Combine
            vec3 texColor = texture(texSampler, fragUV).rgb;
            vec3 result = (ambient + diffuse + specular) * texColor;
            outColor = vec4(result, 1.0);
        }
    "#
    .to_string();

    // Large compute shader
    let mut large_compute = String::from(
        r#"
        #version 450

        layout(local_size_x = 256) in;

        layout(set = 0, binding = 0) buffer InputBuffer {
            float data[];
        } inputBuffer;

        layout(set = 0, binding = 1) buffer OutputBuffer {
            float data[];
        } outputBuffer;

        layout(push_constant) uniform PushConstants {
            uint dataSize;
            float multiplier;
        } pc;

        void main() {
            uint index = gl_GlobalInvocationID.x;
            if (index < pc.dataSize) {
                float value = inputBuffer.data[index];
    "#,
    );

    // Add lots of computation to make it large
    for i in 0..50 {
        large_compute.push_str(&format!(
            "
                value = value * pc.multiplier + {};
                value = sqrt(abs(value));",
            i
        ));
    }

    large_compute.push_str(
        r#"
                outputBuffer.data[index] = value;
            }
        }
    "#,
    );

    group.bench_function("small_vertex_shader", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Vertex),
                black_box(small_vertex.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("medium_fragment_shader", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Fragment),
                black_box(medium_fragment.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("large_compute_shader", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Compute),
                black_box(large_compute.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    // Benchmark different source sizes
    for size in [100, 1_000, 10_000, 100_000].iter() {
        let mut source = String::from("#version 450\nvoid main() {\n");
        for i in 0..*size {
            source.push_str(&format!("    float var{} = {};\n", i, i));
        }
        source.push_str("}\n");

        group.bench_with_input(BenchmarkId::new("variable_size", size), size, |b, _| {
            b.iter(|| {
                ShaderData::from_glsl(
                    black_box(ShaderStage::Vertex),
                    black_box(source.clone()),
                    black_box(None),
                )
                .unwrap()
            })
        });
    }

    group.finish();
}

// ============================================================================
// SPIR-V Loading Benchmarks
// ============================================================================

fn bench_spirv_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("spirv_loading");

    // Small SPIR-V binary (minimal valid header)
    let small_spirv = vec![
        0x07230203, // Magic
        0x00010000, // Version 1.0
        0x00000000, // Generator
        0x00000005, // Bound
        0x00000000, // Schema
    ];

    // Medium SPIR-V (simulated compiled shader)
    let mut medium_spirv = small_spirv.clone();
    medium_spirv.extend(vec![0; 1000]); // ~4KB

    // Large SPIR-V (complex shader)
    let mut large_spirv = small_spirv.clone();
    large_spirv.extend(vec![0; 10000]); // ~40KB

    group.bench_function("small_spirv", |b| {
        b.iter(|| {
            ShaderData::from_spirv(
                black_box(ShaderStage::Vertex),
                black_box(small_spirv.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("medium_spirv", |b| {
        b.iter(|| {
            ShaderData::from_spirv(
                black_box(ShaderStage::Fragment),
                black_box(medium_spirv.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("large_spirv", |b| {
        b.iter(|| {
            ShaderData::from_spirv(
                black_box(ShaderStage::Compute),
                black_box(large_spirv.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    // Benchmark different SPIR-V sizes
    for size in [10, 100, 1_000, 10_000, 100_000].iter() {
        let mut spirv = vec![
            0x07230203, // Magic
            0x00010000, // Version
        ];
        spirv.extend(vec![0; *size]);

        group.bench_with_input(BenchmarkId::new("variable_size", size), size, |b, _| {
            b.iter(|| {
                ShaderData::from_spirv(
                    black_box(ShaderStage::Vertex),
                    black_box(spirv.clone()),
                    black_box(None),
                )
                .unwrap()
            })
        });
    }

    group.finish();
}

// ============================================================================
// Comparison: GLSL vs SPIR-V
// ============================================================================

fn bench_glsl_vs_spirv(c: &mut Criterion) {
    let mut group = c.benchmark_group("glsl_vs_spirv");

    let glsl_source = r#"
        #version 450
        layout(location = 0) in vec3 position;
        void main() {
            gl_Position = vec4(position, 1.0);
        }
    "#
    .to_string();

    let spirv_binary = vec![
        0x07230203, // Magic
        0x00010000, // Version
        0x00000000, // Generator
        0x00000005, // Bound
        0x00000000, // Schema
    ];

    group.bench_function("glsl_load", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Vertex),
                black_box(glsl_source.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("spirv_load", |b| {
        b.iter(|| {
            ShaderData::from_spirv(
                black_box(ShaderStage::Vertex),
                black_box(spirv_binary.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.finish();
}

// ============================================================================
// Entry Point Benchmarks
// ============================================================================

fn bench_entry_points(c: &mut Criterion) {
    let mut group = c.benchmark_group("entry_points");

    let source = "#version 450\nvoid main() {}".to_string();

    group.bench_function("default_entry_point", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Vertex),
                black_box(source.clone()),
                black_box(None),
            )
            .unwrap()
        })
    });

    group.bench_function("custom_entry_point", |b| {
        b.iter(|| {
            ShaderData::from_glsl(
                black_box(ShaderStage::Vertex),
                black_box(source.clone()),
                black_box(Some("custom_main".to_string())),
            )
            .unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_glsl_parsing,
    bench_spirv_loading,
    bench_glsl_vs_spirv,
    bench_entry_points
);
criterion_main!(benches);
