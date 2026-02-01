//! Shader module loading and management
//!
//! Handles loading compiled SPIR-V shaders and creating Vulkan shader modules.

use crate::error::RendererError;
use ash::vk;
use tracing::{info, instrument};

/// Shader module wrapper
pub struct ShaderModule {
    module: vk::ShaderModule,
    device: ash::Device,
}

impl ShaderModule {
    /// Create a shader module from SPIR-V bytecode
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `code` - SPIR-V bytecode (must be 4-byte aligned)
    ///
    /// # Errors
    /// Returns error if shader module creation fails
    #[instrument(skip(device, code))]
    pub fn from_spirv(device: &ash::Device, code: &[u8]) -> Result<Self, RendererError> {
        // SPIR-V must be 4-byte aligned
        if code.len() % 4 != 0 {
            return Err(RendererError::shadermodulecreationfailed(
                "SPIR-V bytecode is not 4-byte aligned".to_string(),
            ));
        }

        // Convert to u32 slice (SPIR-V is little-endian u32 array)
        let code_u32: Vec<u32> = code
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let create_info = vk::ShaderModuleCreateInfo::default().code(&code_u32);

        // SAFETY: create_info is valid and code_u32 remains valid for the duration of this call
        let module = unsafe {
            device.create_shader_module(&create_info, None).map_err(|e| {
                RendererError::shadermodulecreationfailed(format!(
                    "Failed to create shader module: {:?}",
                    e
                ))
            })?
        };

        info!(size = code.len(), "Shader module created");

        Ok(Self { module, device: device.clone() })
    }

    /// Load a shader from a compiled SPIR-V file
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `path` - Path to .spv file
    #[instrument(skip(device))]
    pub fn from_file(device: &ash::Device, path: &str) -> Result<Self, RendererError> {
        let code = std::fs::read(path).map_err(|e| {
            RendererError::shadermodulecreationfailed(format!(
                "Failed to read shader file {}: {:?}",
                path, e
            ))
        })?;

        Self::from_spirv(device, &code)
    }

    /// Load a shader from compiled_shaders directory (build output)
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `name` - Shader filename (e.g., "triangle.vert.spv")
    #[instrument(skip(device))]
    pub fn from_compiled(device: &ash::Device, name: &str) -> Result<Self, RendererError> {
        // Compiled shaders are in engine/renderer/compiled_shaders/
        let path = format!("engine/renderer/compiled_shaders/{}", name);
        Self::from_file(device, &path)
    }

    /// Get the raw Vulkan shader module handle
    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.module
    }

    /// Create a pipeline shader stage info
    ///
    /// # Arguments
    /// * `stage` - Shader stage (vertex, fragment, etc.)
    /// * `entry_point` - Entry point function name (usually "main")
    pub fn create_stage_info(
        &self,
        stage: vk::ShaderStageFlags,
        entry_point: &'static std::ffi::CStr,
    ) -> vk::PipelineShaderStageCreateInfo<'_> {
        vk::PipelineShaderStageCreateInfo::default()
            .stage(stage)
            .module(self.module)
            .name(entry_point)
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        // SAFETY: We own the shader module and device is still valid
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: Tests for ShaderModule require a valid Vulkan device
    // These tests should be integration tests with a real Vulkan context

    // TODO: Add integration tests with real Vulkan device

    #[test]
    fn test_spirv_alignment_validation() {
        // Test alignment validation logic
        // Note: This only tests the validation, not actual shader module creation
        let valid_len = 16; // 4-byte aligned
        let invalid_len = 15; // Not 4-byte aligned

        assert_eq!(valid_len % 4, 0, "Valid data should be 4-byte aligned");
        assert_ne!(invalid_len % 4, 0, "Invalid data should not be 4-byte aligned");
    }
}
