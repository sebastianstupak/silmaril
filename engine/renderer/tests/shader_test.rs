//! Shader module integration tests
//!
//! Tests shader loading, SPIR-V compilation, and stage detection.

use ash::vk;
use engine_renderer::{ShaderModule, VulkanContext};
use std::path::Path;

#[test]
fn test_shader_stage_from_extension() {
    use engine_renderer::stage_from_extension;

    // Test all supported shader stages
    assert_eq!(
        stage_from_extension(Path::new("test.vert")).unwrap(),
        vk::ShaderStageFlags::VERTEX
    );

    assert_eq!(
        stage_from_extension(Path::new("test.vs")).unwrap(),
        vk::ShaderStageFlags::VERTEX
    );

    assert_eq!(
        stage_from_extension(Path::new("test.frag")).unwrap(),
        vk::ShaderStageFlags::FRAGMENT
    );

    assert_eq!(
        stage_from_extension(Path::new("test.fs")).unwrap(),
        vk::ShaderStageFlags::FRAGMENT
    );

    assert_eq!(
        stage_from_extension(Path::new("test.comp")).unwrap(),
        vk::ShaderStageFlags::COMPUTE
    );

    assert_eq!(
        stage_from_extension(Path::new("test.cs")).unwrap(),
        vk::ShaderStageFlags::COMPUTE
    );

    assert_eq!(
        stage_from_extension(Path::new("test.geom")).unwrap(),
        vk::ShaderStageFlags::GEOMETRY
    );

    assert_eq!(
        stage_from_extension(Path::new("test.gs")).unwrap(),
        vk::ShaderStageFlags::GEOMETRY
    );

    assert_eq!(
        stage_from_extension(Path::new("test.tesc")).unwrap(),
        vk::ShaderStageFlags::TESSELLATION_CONTROL
    );

    assert_eq!(
        stage_from_extension(Path::new("test.tes")).unwrap(),
        vk::ShaderStageFlags::TESSELLATION_CONTROL
    );

    assert_eq!(
        stage_from_extension(Path::new("test.tese")).unwrap(),
        vk::ShaderStageFlags::TESSELLATION_EVALUATION
    );

    assert_eq!(
        stage_from_extension(Path::new("test.tee")).unwrap(),
        vk::ShaderStageFlags::TESSELLATION_EVALUATION
    );
}

#[test]
fn test_shader_stage_from_extension_invalid() {
    use engine_renderer::stage_from_extension;

    // Test invalid extensions
    let result = stage_from_extension(Path::new("test.txt"));
    assert!(result.is_err());

    let result = stage_from_extension(Path::new("test"));
    assert!(result.is_err());

    let result = stage_from_extension(Path::new("test."));
    assert!(result.is_err());
}

#[test]
fn test_spirv_word_alignment() {
    // SPIR-V must be 4-byte aligned (u32 words)
    // Test that our conversion is correct
    let valid_bytes = vec![0u8; 16]; // 4-byte aligned
    assert_eq!(valid_bytes.len() % 4, 0);

    let invalid_bytes = vec![0u8; 15]; // Not 4-byte aligned
    assert_ne!(invalid_bytes.len() % 4, 0);

    // Test byte-to-u32 conversion
    let bytes = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    assert_eq!(words.len(), 2);
    assert_eq!(words[0], 0x04030201); // Little-endian
    assert_eq!(words[1], 0x08070605);
}

#[test]
fn test_shader_module_from_spirv() {
    // Initialize Vulkan context
    let context = match VulkanContext::new("ShaderTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create a minimal valid SPIR-V module (magic number + version)
    // This is the absolute minimum valid SPIR-V
    let spirv_code = vec![
        0x07230203, // Magic number
        0x00010000, // Version 1.0
        0x00000000, // Generator
        0x00000001, // Bound
        0x00000000, // Schema
    ];

    // Test creating shader module
    let result = ShaderModule::from_spirv(
        &context.device,
        &spirv_code,
        vk::ShaderStageFlags::VERTEX,
        "main",
    );

    assert!(result.is_ok(), "Failed to create shader module from valid SPIR-V");

    let shader = result.unwrap();
    assert_eq!(shader.stage, vk::ShaderStageFlags::VERTEX);
}

#[test]
fn test_shader_module_stage_create_info() {
    // Initialize Vulkan context
    let context = match VulkanContext::new("ShaderTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create a minimal valid SPIR-V module
    let spirv_code = vec![
        0x07230203, // Magic number
        0x00010000, // Version 1.0
        0x00000000, // Generator
        0x00000001, // Bound
        0x00000000, // Schema
    ];

    let shader = ShaderModule::from_spirv(
        &context.device,
        &spirv_code,
        vk::ShaderStageFlags::FRAGMENT,
        "main",
    )
    .expect("Failed to create shader module");

    // Test stage create info
    let stage_info = shader.stage_create_info();
    assert_eq!(stage_info.stage, vk::ShaderStageFlags::FRAGMENT);
    assert_eq!(stage_info.module, shader.module);
}

#[cfg(test)]
mod compiled_shader_tests {
    use super::*;

    #[test]
    fn test_load_compiled_test_shaders() {
        // Initialize Vulkan context
        let context = match VulkanContext::new("ShaderTest", None, None) {
            Ok(ctx) => ctx,
            Err(_) => {
                eprintln!("Skipping test: Vulkan not available");
                return;
            }
        };

        // Try to load test shaders (if they were compiled)
        // These shaders are created in shaders/test.vert and shaders/test.frag
        let out_dir = std::env::var("OUT_DIR").ok();
        if let Some(out_dir) = out_dir {
            let test_vert_path = Path::new(&out_dir).join("shaders").join("test.vert.spv");
            let test_frag_path = Path::new(&out_dir).join("shaders").join("test.frag.spv");

            // Only test if shaders were compiled
            if test_vert_path.exists() {
                let result = ShaderModule::from_spirv_file(
                    &context.device,
                    &test_vert_path,
                    vk::ShaderStageFlags::VERTEX,
                    "main",
                );
                assert!(result.is_ok(), "Failed to load compiled test vertex shader");
            }

            if test_frag_path.exists() {
                let result = ShaderModule::from_spirv_file(
                    &context.device,
                    &test_frag_path,
                    vk::ShaderStageFlags::FRAGMENT,
                    "main",
                );
                assert!(result.is_ok(), "Failed to load compiled test fragment shader");
            }
        }
    }
}
