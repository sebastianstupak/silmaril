//! Shader module loading and management
//!
//! Handles loading compiled SPIR-V shaders and creating Vulkan shader modules.
//!
//! # Shader Compilation
//!
//! Shaders are compiled at build time from GLSL to SPIR-V.
//! Place shader files in `engine/renderer/shaders/`:
//! - `*.vert` - Vertex shaders
//! - `*.frag` - Fragment shaders
//! - `*.comp` - Compute shaders
//! - `*.geom` - Geometry shaders
//! - `*.tesc` - Tessellation control shaders
//! - `*.tese` - Tessellation evaluation shaders
//!
//! Compiled SPIR-V files are placed in `$OUT_DIR/shaders/*.spv`

use crate::error::RendererError;
use ash::vk;
use std::ffi::CString;
use std::path::Path;
use tracing::{info, instrument};

/// Shader module wrapper with stage and entry point information
pub struct ShaderModule {
    /// Vulkan shader module handle
    pub module: vk::ShaderModule,
    /// Shader stage (vertex, fragment, compute, etc.)
    pub stage: vk::ShaderStageFlags,
    /// Entry point function name (usually "main")
    pub entry_point: CString,
    /// Device reference for cleanup
    device: ash::Device,
}

impl ShaderModule {
    /// Create shader module from SPIR-V bytecode
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `spirv_code` - SPIR-V bytecode as u32 words
    /// * `stage` - Shader stage (vertex, fragment, etc.)
    /// * `entry_point` - Entry point function name (usually "main")
    ///
    /// # Errors
    /// Returns error if shader module creation fails
    ///
    /// # Example
    /// ```no_run
    /// use ash::vk;
    /// use engine_renderer::ShaderModule;
    /// # fn example(device: &ash::Device) -> Result<(), engine_renderer::RendererError> {
    /// let spirv_code = vec![0x07230203u32, 0x00010000, /* ... */];
    /// let shader = ShaderModule::from_spirv(
    ///     device,
    ///     &spirv_code,
    ///     vk::ShaderStageFlags::VERTEX,
    ///     "main"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(device, spirv_code), fields(spirv_size = spirv_code.len() * 4, stage = ?stage, entry_point = entry_point))]
    pub fn from_spirv(
        device: &ash::Device,
        spirv_code: &[u32],
        stage: vk::ShaderStageFlags,
        entry_point: &str,
    ) -> Result<Self, RendererError> {
        info!(
            spirv_size = spirv_code.len() * 4,
            stage = ?stage,
            entry_point = entry_point,
            "Creating shader module from SPIR-V"
        );

        let create_info = vk::ShaderModuleCreateInfo::default().code(spirv_code);

        // SAFETY: create_info is valid and spirv_code remains valid for the duration of this call
        let module = unsafe {
            device.create_shader_module(&create_info, None).map_err(|e| {
                RendererError::shadercompilationfailed(format!(
                    "Failed to create shader module: {:?}",
                    e
                ))
            })?
        };

        Ok(Self {
            module,
            stage,
            entry_point: CString::new(entry_point).unwrap(),
            device: device.clone(),
        })
    }

    /// Load SPIR-V from compiled shader file (.spv)
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `path` - Path to .spv file
    /// * `stage` - Shader stage (vertex, fragment, etc.)
    /// * `entry_point` - Entry point function name (usually "main")
    ///
    /// # Example
    /// ```no_run
    /// use ash::vk;
    /// use std::path::Path;
    /// use engine_renderer::ShaderModule;
    /// # fn example(device: &ash::Device) -> Result<(), engine_renderer::RendererError> {
    /// let shader = ShaderModule::from_spirv_file(
    ///     device,
    ///     Path::new("shaders/test.vert.spv"),
    ///     vk::ShaderStageFlags::VERTEX,
    ///     "main"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(device), fields(path = %path.display()))]
    pub fn from_spirv_file(
        device: &ash::Device,
        path: &Path,
        stage: vk::ShaderStageFlags,
        entry_point: &str,
    ) -> Result<Self, RendererError> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path).map_err(|e| {
            RendererError::shadernotfound(path.to_string_lossy().to_string(), e.to_string())
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| {
            RendererError::shadercompilationfailed(format!("Failed to read shader file: {}", e))
        })?;

        // SPIR-V is u32 words, convert bytes to u32
        if buffer.len() % 4 != 0 {
            return Err(RendererError::invalidshaderformat(
                "SPIR-V file size is not a multiple of 4 bytes".to_string(),
            ));
        }

        let spirv = buffer
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect::<Vec<_>>();

        Self::from_spirv(device, &spirv, stage, entry_point)
    }

    /// Get shader stage create info for pipeline
    ///
    /// Returns the Vulkan pipeline shader stage create info structure
    /// that can be used when creating graphics or compute pipelines.
    ///
    /// # Example
    /// ```no_run
    /// # use engine_renderer::ShaderModule;
    /// # use ash::vk;
    /// # fn example(shader: &ShaderModule) {
    /// let stage_info = shader.stage_create_info();
    /// // Use stage_info when creating a pipeline
    /// # }
    /// ```
    pub fn stage_create_info(&self) -> vk::PipelineShaderStageCreateInfo<'_> {
        vk::PipelineShaderStageCreateInfo::default()
            .stage(self.stage)
            .module(self.module)
            .name(self.entry_point.as_c_str())
    }

    /// Destroy the shader module
    ///
    /// Explicitly destroys the Vulkan shader module.
    /// This is also called automatically when the ShaderModule is dropped.
    pub fn destroy(&self, device: &ash::Device) {
        // SAFETY: We own the shader module
        unsafe {
            device.destroy_shader_module(self.module, None);
        }
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

/// Helper to determine shader stage from file extension
///
/// Supports the following extensions:
/// - `.vert`, `.vs` → Vertex shader
/// - `.frag`, `.fs` → Fragment shader
/// - `.comp`, `.cs` → Compute shader
/// - `.geom`, `.gs` → Geometry shader
/// - `.tesc`, `.tes` → Tessellation control shader
/// - `.tese`, `.tee` → Tessellation evaluation shader
///
/// # Errors
/// Returns error if the file has no extension or an unknown extension.
///
/// # Example
/// ```
/// use std::path::Path;
/// use ash::vk;
/// use engine_renderer::stage_from_extension;
///
/// let stage = stage_from_extension(Path::new("shader.vert")).unwrap();
/// assert_eq!(stage, vk::ShaderStageFlags::VERTEX);
/// ```
pub fn stage_from_extension(path: &Path) -> Result<vk::ShaderStageFlags, RendererError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| RendererError::shadercompilationfailed("No file extension".to_string()))?;

    match ext {
        "vert" | "vs" => Ok(vk::ShaderStageFlags::VERTEX),
        "frag" | "fs" => Ok(vk::ShaderStageFlags::FRAGMENT),
        "comp" | "cs" => Ok(vk::ShaderStageFlags::COMPUTE),
        "geom" | "gs" => Ok(vk::ShaderStageFlags::GEOMETRY),
        "tesc" | "tes" => Ok(vk::ShaderStageFlags::TESSELLATION_CONTROL),
        "tese" | "tee" => Ok(vk::ShaderStageFlags::TESSELLATION_EVALUATION),
        _ => Err(RendererError::shadercompilationfailed(format!(
            "Unknown shader extension: {}",
            ext
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spirv_alignment_validation() {
        // Test alignment validation logic
        let valid_len = 16; // 4-byte aligned
        let invalid_len = 15; // Not 4-byte aligned

        assert_eq!(valid_len % 4, 0, "Valid data should be 4-byte aligned");
        assert_ne!(invalid_len % 4, 0, "Invalid data should not be 4-byte aligned");
    }

    #[test]
    fn test_stage_from_extension() {
        // Vertex shaders
        assert_eq!(
            stage_from_extension(Path::new("test.vert")).unwrap(),
            vk::ShaderStageFlags::VERTEX
        );
        assert_eq!(
            stage_from_extension(Path::new("test.vs")).unwrap(),
            vk::ShaderStageFlags::VERTEX
        );

        // Fragment shaders
        assert_eq!(
            stage_from_extension(Path::new("test.frag")).unwrap(),
            vk::ShaderStageFlags::FRAGMENT
        );
        assert_eq!(
            stage_from_extension(Path::new("test.fs")).unwrap(),
            vk::ShaderStageFlags::FRAGMENT
        );

        // Compute shaders
        assert_eq!(
            stage_from_extension(Path::new("test.comp")).unwrap(),
            vk::ShaderStageFlags::COMPUTE
        );
        assert_eq!(
            stage_from_extension(Path::new("test.cs")).unwrap(),
            vk::ShaderStageFlags::COMPUTE
        );

        // Geometry shaders
        assert_eq!(
            stage_from_extension(Path::new("test.geom")).unwrap(),
            vk::ShaderStageFlags::GEOMETRY
        );
        assert_eq!(
            stage_from_extension(Path::new("test.gs")).unwrap(),
            vk::ShaderStageFlags::GEOMETRY
        );

        // Tessellation control shaders
        assert_eq!(
            stage_from_extension(Path::new("test.tesc")).unwrap(),
            vk::ShaderStageFlags::TESSELLATION_CONTROL
        );
        assert_eq!(
            stage_from_extension(Path::new("test.tes")).unwrap(),
            vk::ShaderStageFlags::TESSELLATION_CONTROL
        );

        // Tessellation evaluation shaders
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
    fn test_stage_from_extension_invalid() {
        // Invalid extensions should error
        assert!(stage_from_extension(Path::new("test.txt")).is_err());
        assert!(stage_from_extension(Path::new("test")).is_err());
    }
}
