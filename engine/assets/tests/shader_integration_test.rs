use engine_assets::{ShaderData, ShaderError, ShaderSource, ShaderStage};

// ============================================================================
// Integration Tests: GLSL Shaders
// ============================================================================

#[test]
fn test_vertex_shader_complete() {
    let source = r#"
        #version 450

        layout(location = 0) in vec3 inPosition;
        layout(location = 1) in vec3 inNormal;
        layout(location = 2) in vec2 inTexCoord;

        layout(set = 0, binding = 0) uniform UniformBufferObject {
            mat4 model;
            mat4 view;
            mat4 projection;
        } ubo;

        layout(location = 0) out vec3 fragNormal;
        layout(location = 1) out vec2 fragTexCoord;

        void main() {
            gl_Position = ubo.projection * ubo.view * ubo.model * vec4(inPosition, 1.0);
            fragNormal = mat3(transpose(inverse(ubo.model))) * inNormal;
            fragTexCoord = inTexCoord;
        }
    "#
    .to_string();

    let shader = ShaderData::from_glsl(ShaderStage::Vertex, source, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Vertex);
    assert_eq!(shader.entry_point(), "main");
    assert!(shader.is_glsl());

    if let ShaderSource::Glsl(glsl) = shader.source() {
        assert!(glsl.contains("#version 450"));
        assert!(glsl.contains("void main()"));
        assert!(glsl.contains("gl_Position"));
    } else {
        panic!("Expected GLSL source");
    }
}

#[test]
fn test_fragment_shader_complete() {
    let source = r#"
        #version 450

        layout(location = 0) in vec3 fragNormal;
        layout(location = 1) in vec2 fragTexCoord;

        layout(location = 0) out vec4 outColor;

        layout(set = 0, binding = 1) uniform sampler2D texSampler;

        void main() {
            vec3 color = texture(texSampler, fragTexCoord).rgb;
            outColor = vec4(color, 1.0);
        }
    "#
    .to_string();

    let shader = ShaderData::from_glsl(ShaderStage::Fragment, source, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Fragment);
    assert!(shader.is_glsl());
}

#[test]
fn test_compute_shader_complete() {
    let source = r#"
        #version 450

        layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;

        layout(set = 0, binding = 0) buffer InputBuffer {
            float data[];
        } inputBuffer;

        layout(set = 0, binding = 1) buffer OutputBuffer {
            float data[];
        } outputBuffer;

        layout(push_constant) uniform PushConstants {
            uint count;
        } pc;

        void main() {
            uint index = gl_GlobalInvocationID.x;
            if (index < pc.count) {
                outputBuffer.data[index] = inputBuffer.data[index] * 2.0;
            }
        }
    "#
    .to_string();

    let shader = ShaderData::from_glsl(ShaderStage::Compute, source, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Compute);
    assert!(shader.is_glsl());
}

// ============================================================================
// Integration Tests: SPIR-V Shaders
// ============================================================================

#[test]
fn test_spirv_vertex_shader() {
    // Minimal valid SPIR-V
    let spirv = vec![
        0x07230203, // Magic
        0x00010500, // Version 1.5
        0x00000008, // Generator (Khronos)
        0x0000000D, // Bound
        0x00000000, // Schema
        // OpCapability Shader
        0x00020011, 0x00000001, // OpMemoryModel
        0x0003000E, 0x00000000, 0x00000001,
    ];

    let shader = ShaderData::from_spirv(ShaderStage::Vertex, spirv, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Vertex);
    assert_eq!(shader.entry_point(), "main");
    assert!(shader.is_spirv());

    if let ShaderSource::Spirv(data) = shader.source() {
        assert_eq!(data[0], 0x07230203); // Magic
        assert!(data.len() > 5);
    } else {
        panic!("Expected SPIR-V source");
    }
}

#[test]
fn test_spirv_fragment_shader() {
    let spirv = vec![
        0x07230203, // Magic
        0x00010000, // Version
        0x00000000, // Generator
        0x00000001, // Bound
        0x00000000, // Schema
    ];

    let shader = ShaderData::from_spirv(ShaderStage::Fragment, spirv, None).unwrap();
    assert_eq!(shader.stage(), ShaderStage::Fragment);
}

#[test]
fn test_spirv_compute_shader() {
    let spirv = vec![
        0x07230203, // Magic
        0x00010000, // Version
        0x00000000, // Generator
        0x00000001, // Bound
        0x00000000, // Schema
    ];

    let shader = ShaderData::from_spirv(ShaderStage::Compute, spirv, None).unwrap();
    assert_eq!(shader.stage(), ShaderStage::Compute);
}

// ============================================================================
// Integration Tests: Error Handling
// ============================================================================

#[test]
fn test_glsl_error_handling() {
    // Empty source
    let result = ShaderData::from_glsl(ShaderStage::Vertex, "".to_string(), None);
    assert!(matches!(result, Err(ShaderError::InvalidGlsl { .. })));

    // Whitespace only
    let result = ShaderData::from_glsl(ShaderStage::Vertex, "   \n  ".to_string(), None);
    assert!(matches!(result, Err(ShaderError::InvalidGlsl { .. })));

    // Empty entry point
    let result = ShaderData::from_glsl(
        ShaderStage::Vertex,
        "#version 450\nvoid main() {}".to_string(),
        Some("".to_string()),
    );
    assert!(matches!(result, Err(ShaderError::MissingEntryPoint { .. })));
}

#[test]
fn test_spirv_error_handling() {
    // Empty binary
    let result = ShaderData::from_spirv(ShaderStage::Vertex, vec![], None);
    assert!(matches!(result, Err(ShaderError::InvalidSpirv { .. })));

    // Invalid magic number
    let result = ShaderData::from_spirv(ShaderStage::Vertex, vec![0xDEADBEEF], None);
    assert!(matches!(result, Err(ShaderError::InvalidSpirv { .. })));

    // Empty entry point
    let result =
        ShaderData::from_spirv(ShaderStage::Vertex, vec![0x07230203], Some("".to_string()));
    assert!(matches!(result, Err(ShaderError::MissingEntryPoint { .. })));
}

// ============================================================================
// Integration Tests: File I/O (non-WASM only)
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_load_glsl_from_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let glsl_source = r#"
        #version 450
        layout(location = 0) in vec3 position;
        void main() {
            gl_Position = vec4(position, 1.0);
        }
    "#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(glsl_source.as_bytes()).unwrap();

    let shader = ShaderData::load_glsl_file(file.path(), ShaderStage::Vertex, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Vertex);
    assert!(shader.is_glsl());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_load_spirv_from_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Valid SPIR-V binary (little-endian)
    let spirv_bytes: Vec<u8> = vec![
        0x03, 0x02, 0x23, 0x07, // Magic: 0x07230203
        0x00, 0x00, 0x01, 0x00, // Version: 0x00010000
        0x00, 0x00, 0x00, 0x00, // Generator: 0
        0x05, 0x00, 0x00, 0x00, // Bound: 5
        0x00, 0x00, 0x00, 0x00, // Schema: 0
    ];

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&spirv_bytes).unwrap();

    let shader = ShaderData::load_spirv_file(file.path(), ShaderStage::Vertex, None).unwrap();

    assert_eq!(shader.stage(), ShaderStage::Vertex);
    assert!(shader.is_spirv());

    if let ShaderSource::Spirv(data) = shader.source() {
        assert_eq!(data[0], 0x07230203);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_load_glsl_file_not_found() {
    use std::path::Path;

    let result =
        ShaderData::load_glsl_file(Path::new("nonexistent_shader.glsl"), ShaderStage::Vertex, None);

    assert!(matches!(result, Err(ShaderError::IoError { .. })));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_load_spirv_file_invalid_size() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create file with invalid size (not multiple of 4)
    let invalid_bytes = vec![0x03, 0x02, 0x23]; // 3 bytes

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&invalid_bytes).unwrap();

    let result = ShaderData::load_spirv_file(file.path(), ShaderStage::Vertex, None);

    assert!(matches!(result, Err(ShaderError::InvalidSpirv { .. })));
}

// ============================================================================
// Integration Tests: Custom Entry Points
// ============================================================================

#[test]
fn test_custom_entry_point_glsl() {
    let source = r#"
        #version 450
        void vertex_main() {
            gl_Position = vec4(0.0);
        }
    "#
    .to_string();

    let shader =
        ShaderData::from_glsl(ShaderStage::Vertex, source, Some("vertex_main".to_string()))
            .unwrap();

    assert_eq!(shader.entry_point(), "vertex_main");
}

#[test]
fn test_custom_entry_point_spirv() {
    let spirv = vec![
        0x07230203, // Magic
        0x00010000, // Version
    ];

    let shader =
        ShaderData::from_spirv(ShaderStage::Fragment, spirv, Some("frag_main".to_string()))
            .unwrap();

    assert_eq!(shader.entry_point(), "frag_main");
}

// ============================================================================
// Integration Tests: Stage Validation
// ============================================================================

#[test]
fn test_all_shader_stages() {
    let source = "#version 450\nvoid main() {}".to_string();

    for stage in [ShaderStage::Vertex, ShaderStage::Fragment, ShaderStage::Compute] {
        let shader = ShaderData::from_glsl(stage, source.clone(), None).unwrap();
        assert_eq!(shader.stage(), stage);
        assert_eq!(shader.stage().as_str(), stage.as_str());
    }
}

#[test]
fn test_stage_string_conversion() {
    assert_eq!(ShaderStage::from_str("vertex"), Some(ShaderStage::Vertex));
    assert_eq!(ShaderStage::from_str("VERTEX"), Some(ShaderStage::Vertex));
    assert_eq!(ShaderStage::from_str("vert"), Some(ShaderStage::Vertex));
    assert_eq!(ShaderStage::from_str("vs"), Some(ShaderStage::Vertex));

    assert_eq!(ShaderStage::from_str("fragment"), Some(ShaderStage::Fragment));
    assert_eq!(ShaderStage::from_str("FRAGMENT"), Some(ShaderStage::Fragment));
    assert_eq!(ShaderStage::from_str("frag"), Some(ShaderStage::Fragment));
    assert_eq!(ShaderStage::from_str("fs"), Some(ShaderStage::Fragment));
    assert_eq!(ShaderStage::from_str("pixel"), Some(ShaderStage::Fragment));
    assert_eq!(ShaderStage::from_str("ps"), Some(ShaderStage::Fragment));

    assert_eq!(ShaderStage::from_str("compute"), Some(ShaderStage::Compute));
    assert_eq!(ShaderStage::from_str("COMPUTE"), Some(ShaderStage::Compute));
    assert_eq!(ShaderStage::from_str("comp"), Some(ShaderStage::Compute));
    assert_eq!(ShaderStage::from_str("cs"), Some(ShaderStage::Compute));

    assert_eq!(ShaderStage::from_str("invalid_stage"), None);
}

// ============================================================================
// Integration Tests: Real-world Shader Patterns
// ============================================================================

#[test]
fn test_pbr_vertex_shader() {
    let source = r#"
        #version 450

        layout(location = 0) in vec3 inPosition;
        layout(location = 1) in vec3 inNormal;
        layout(location = 2) in vec2 inTexCoord;
        layout(location = 3) in vec3 inTangent;

        layout(set = 0, binding = 0) uniform CameraUBO {
            mat4 view;
            mat4 projection;
            vec3 cameraPos;
        } camera;

        layout(push_constant) uniform PushConstants {
            mat4 model;
        } pc;

        layout(location = 0) out vec3 fragWorldPos;
        layout(location = 1) out vec3 fragNormal;
        layout(location = 2) out vec2 fragTexCoord;
        layout(location = 3) out vec3 fragTangent;

        void main() {
            vec4 worldPos = pc.model * vec4(inPosition, 1.0);
            fragWorldPos = worldPos.xyz;
            fragNormal = mat3(transpose(inverse(pc.model))) * inNormal;
            fragTexCoord = inTexCoord;
            fragTangent = mat3(pc.model) * inTangent;

            gl_Position = camera.projection * camera.view * worldPos;
        }
    "#
    .to_string();

    let shader = ShaderData::from_glsl(ShaderStage::Vertex, source, None).unwrap();
    assert!(shader.is_glsl());
}

#[test]
fn test_pbr_fragment_shader() {
    let source = r#"
        #version 450

        layout(location = 0) in vec3 fragWorldPos;
        layout(location = 1) in vec3 fragNormal;
        layout(location = 2) in vec2 fragTexCoord;

        layout(location = 0) out vec4 outColor;

        layout(set = 0, binding = 1) uniform sampler2D albedoMap;
        layout(set = 0, binding = 2) uniform sampler2D normalMap;
        layout(set = 0, binding = 3) uniform sampler2D metallicRoughnessMap;

        layout(set = 0, binding = 4) uniform MaterialUBO {
            vec3 baseColor;
            float metallic;
            float roughness;
        } material;

        const float PI = 3.14159265359;

        void main() {
            vec3 albedo = texture(albedoMap, fragTexCoord).rgb * material.baseColor;
            float metallic = texture(metallicRoughnessMap, fragTexCoord).b * material.metallic;
            float roughness = texture(metallicRoughnessMap, fragTexCoord).g * material.roughness;

            vec3 N = normalize(fragNormal);
            vec3 V = normalize(vec3(0.0) - fragWorldPos);

            vec3 F0 = mix(vec3(0.04), albedo, metallic);

            vec3 Lo = vec3(0.0);
            // Simplified PBR calculation
            outColor = vec4(Lo, 1.0);
        }
    "#
    .to_string();

    let shader = ShaderData::from_glsl(ShaderStage::Fragment, source, None).unwrap();
    assert!(shader.is_glsl());
}
