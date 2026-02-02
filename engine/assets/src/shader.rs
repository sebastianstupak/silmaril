//! Shader data structures (GLSL source and SPIR-V binaries)
//!
//! Can be used by:
//! - Renderer for pipeline creation
//! - Build tools for shader compilation
//! - Asset cooker for shader processing

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{info, instrument, warn};

define_error! {
    pub enum ShaderError {
        InvalidGlsl { reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        InvalidSpirv { reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        InvalidStage { stage: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        IoError { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        MissingEntryPoint { } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
    }
}

/// Shader stage (vertex, fragment, compute)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader stage
    Vertex,
    /// Fragment (pixel) shader stage
    Fragment,
    /// Compute shader stage
    Compute,
}

impl ShaderStage {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "vertex",
            ShaderStage::Fragment => "fragment",
            ShaderStage::Compute => "compute",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vertex" | "vert" | "vs" => Some(ShaderStage::Vertex),
            "fragment" | "frag" | "fs" | "pixel" | "ps" => Some(ShaderStage::Fragment),
            "compute" | "comp" | "cs" => Some(ShaderStage::Compute),
            _ => None,
        }
    }
}

/// Shader source (GLSL or pre-compiled SPIR-V)
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderSource {
    /// GLSL source code
    Glsl(String),
    /// Pre-compiled SPIR-V binary (little-endian u32 array)
    Spirv(Vec<u32>),
}

impl ShaderSource {
    /// Check if this is GLSL source
    pub fn is_glsl(&self) -> bool {
        matches!(self, ShaderSource::Glsl(_))
    }

    /// Check if this is SPIR-V binary
    pub fn is_spirv(&self) -> bool {
        matches!(self, ShaderSource::Spirv(_))
    }

    /// Get GLSL source if available
    pub fn as_glsl(&self) -> Option<&str> {
        match self {
            ShaderSource::Glsl(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get SPIR-V binary if available
    pub fn as_spirv(&self) -> Option<&[u32]> {
        match self {
            ShaderSource::Spirv(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

/// Shader data (stage, source, entry point)
///
/// Pure data structure - no GPU or rendering dependencies.
/// Rendering backends create shader modules from this data.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderData {
    /// Shader stage
    pub stage: ShaderStage,
    /// Shader source (GLSL or SPIR-V)
    pub source: ShaderSource,
    /// Entry point function name
    pub entry_point: String,
}

impl ShaderData {
    /// Create a new shader from GLSL source
    #[instrument(skip(source), fields(source_len = source.len()))]
    pub fn from_glsl(
        stage: ShaderStage,
        source: String,
        entry_point: Option<String>,
    ) -> Result<Self, ShaderError> {
        // Validate non-empty source
        if source.trim().is_empty() {
            return Err(ShaderError::invalidglsl("GLSL source is empty".to_string()));
        }

        // Validate entry point
        let entry_point = entry_point.unwrap_or_else(|| "main".to_string());
        if entry_point.is_empty() {
            return Err(ShaderError::missingentrypoint());
        }

        // Basic GLSL validation (check for version directive)
        let has_version = source.contains("#version");
        if !has_version {
            warn!(
                stage = %stage.as_str(),
                "GLSL source missing #version directive"
            );
        }

        info!(
            stage = %stage.as_str(),
            entry_point = %entry_point,
            source_len = source.len(),
            "Loaded GLSL shader"
        );

        Ok(Self { stage, source: ShaderSource::Glsl(source), entry_point })
    }

    /// Create a new shader from SPIR-V binary
    #[instrument(skip(spirv), fields(spirv_len = spirv.len()))]
    pub fn from_spirv(
        stage: ShaderStage,
        spirv: Vec<u32>,
        entry_point: Option<String>,
    ) -> Result<Self, ShaderError> {
        // Validate non-empty binary
        if spirv.is_empty() {
            return Err(ShaderError::invalidspirv("SPIR-V binary is empty".to_string()));
        }

        // Validate SPIR-V magic number (0x07230203)
        if spirv[0] != 0x07230203 {
            return Err(ShaderError::invalidspirv(format!(
                "Invalid SPIR-V magic number: 0x{:08X} (expected 0x07230203)",
                spirv[0]
            )));
        }

        // Validate entry point
        let entry_point = entry_point.unwrap_or_else(|| "main".to_string());
        if entry_point.is_empty() {
            return Err(ShaderError::missingentrypoint());
        }

        info!(
            stage = %stage.as_str(),
            entry_point = %entry_point,
            spirv_words = spirv.len(),
            "Loaded SPIR-V shader"
        );

        Ok(Self { stage, source: ShaderSource::Spirv(spirv), entry_point })
    }

    /// Load GLSL from file path
    #[cfg(not(target_arch = "wasm32"))]
    #[instrument]
    pub fn load_glsl_file(
        path: &std::path::Path,
        stage: ShaderStage,
        entry_point: Option<String>,
    ) -> Result<Self, ShaderError> {
        use std::fs;

        let source = fs::read_to_string(path)
            .map_err(|e| ShaderError::ioerror(format!("Failed to read GLSL file: {}", e)))?;

        Self::from_glsl(stage, source, entry_point)
    }

    /// Load SPIR-V from file path (binary file)
    #[cfg(not(target_arch = "wasm32"))]
    #[instrument]
    pub fn load_spirv_file(
        path: &std::path::Path,
        stage: ShaderStage,
        entry_point: Option<String>,
    ) -> Result<Self, ShaderError> {
        use std::fs;

        let bytes = fs::read(path)
            .map_err(|e| ShaderError::ioerror(format!("Failed to read SPIR-V file: {}", e)))?;

        // Convert bytes to u32 array (little-endian)
        if bytes.len() % 4 != 0 {
            return Err(ShaderError::invalidspirv(format!(
                "SPIR-V file size ({} bytes) is not a multiple of 4",
                bytes.len()
            )));
        }

        let spirv: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        Self::from_spirv(stage, spirv, entry_point)
    }

    /// Get shader stage
    pub fn stage(&self) -> ShaderStage {
        self.stage
    }

    /// Get shader source
    pub fn source(&self) -> &ShaderSource {
        &self.source
    }

    /// Get entry point
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Check if this is a GLSL shader
    pub fn is_glsl(&self) -> bool {
        self.source.is_glsl()
    }

    /// Check if this is a SPIR-V shader
    pub fn is_spirv(&self) -> bool {
        self.source.is_spirv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // GLSL Tests
    // ============================================================================

    #[test]
    fn test_load_glsl_vertex_shader() {
        let source = r#"
            #version 450
            layout(location = 0) in vec3 position;
            void main() {
                gl_Position = vec4(position, 1.0);
            }
        "#
        .to_string();

        let result = ShaderData::from_glsl(ShaderStage::Vertex, source, None);
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.stage(), ShaderStage::Vertex);
        assert_eq!(shader.entry_point(), "main");
        assert!(shader.is_glsl());
        assert!(!shader.is_spirv());
    }

    #[test]
    fn test_load_glsl_fragment_shader() {
        let source = r#"
            #version 450
            layout(location = 0) out vec4 fragColor;
            void main() {
                fragColor = vec4(1.0, 0.0, 0.0, 1.0);
            }
        "#
        .to_string();

        let result = ShaderData::from_glsl(ShaderStage::Fragment, source, None);
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.stage(), ShaderStage::Fragment);
        assert_eq!(shader.entry_point(), "main");
        assert!(shader.is_glsl());
    }

    #[test]
    fn test_load_glsl_compute_shader() {
        let source = r#"
            #version 450
            layout(local_size_x = 256) in;
            void main() {
                // Compute work
            }
        "#
        .to_string();

        let result = ShaderData::from_glsl(ShaderStage::Compute, source, None);
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.stage(), ShaderStage::Compute);
    }

    #[test]
    fn test_glsl_custom_entry_point() {
        let source = r#"
            #version 450
            void custom_main() {
                gl_Position = vec4(0.0);
            }
        "#
        .to_string();

        let result =
            ShaderData::from_glsl(ShaderStage::Vertex, source, Some("custom_main".to_string()));
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.entry_point(), "custom_main");
    }

    #[test]
    fn test_glsl_invalid_empty_source() {
        let result = ShaderData::from_glsl(ShaderStage::Vertex, "".to_string(), None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::InvalidGlsl { .. }));
    }

    #[test]
    fn test_glsl_invalid_whitespace_only() {
        let result = ShaderData::from_glsl(ShaderStage::Vertex, "   \n\t  ".to_string(), None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::InvalidGlsl { .. }));
    }

    // ============================================================================
    // SPIR-V Tests
    // ============================================================================

    #[test]
    fn test_load_spirv_binary() {
        // Valid SPIR-V header (magic + version + generator + bound + schema)
        let spirv = vec![
            0x07230203, // Magic number
            0x00010000, // Version 1.0
            0x00000000, // Generator
            0x00000001, // Bound
            0x00000000, // Schema
        ];

        let result = ShaderData::from_spirv(ShaderStage::Vertex, spirv, None);
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.stage(), ShaderStage::Vertex);
        assert_eq!(shader.entry_point(), "main");
        assert!(shader.is_spirv());
        assert!(!shader.is_glsl());
    }

    #[test]
    fn test_spirv_custom_entry_point() {
        let spirv = vec![
            0x07230203, // Magic
            0x00010000, // Version
        ];

        let result =
            ShaderData::from_spirv(ShaderStage::Fragment, spirv, Some("frag_main".to_string()));
        assert!(result.is_ok());

        let shader = result.unwrap();
        assert_eq!(shader.entry_point(), "frag_main");
    }

    #[test]
    fn test_spirv_invalid_empty() {
        let result = ShaderData::from_spirv(ShaderStage::Vertex, vec![], None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::InvalidSpirv { .. }));
    }

    #[test]
    fn test_spirv_invalid_magic_number() {
        let spirv = vec![0xDEADBEEF, 0x00010000];
        let result = ShaderData::from_spirv(ShaderStage::Vertex, spirv, None);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ShaderError::InvalidSpirv { .. }));
    }

    // ============================================================================
    // ShaderStage Tests
    // ============================================================================

    #[test]
    fn test_shader_stage_validation() {
        assert_eq!(ShaderStage::Vertex.as_str(), "vertex");
        assert_eq!(ShaderStage::Fragment.as_str(), "fragment");
        assert_eq!(ShaderStage::Compute.as_str(), "compute");
    }

    #[test]
    fn test_shader_stage_from_string() {
        assert_eq!(ShaderStage::from_str("vertex"), Some(ShaderStage::Vertex));
        assert_eq!(ShaderStage::from_str("vert"), Some(ShaderStage::Vertex));
        assert_eq!(ShaderStage::from_str("vs"), Some(ShaderStage::Vertex));

        assert_eq!(ShaderStage::from_str("fragment"), Some(ShaderStage::Fragment));
        assert_eq!(ShaderStage::from_str("frag"), Some(ShaderStage::Fragment));
        assert_eq!(ShaderStage::from_str("fs"), Some(ShaderStage::Fragment));
        assert_eq!(ShaderStage::from_str("pixel"), Some(ShaderStage::Fragment));

        assert_eq!(ShaderStage::from_str("compute"), Some(ShaderStage::Compute));
        assert_eq!(ShaderStage::from_str("comp"), Some(ShaderStage::Compute));
        assert_eq!(ShaderStage::from_str("cs"), Some(ShaderStage::Compute));

        assert_eq!(ShaderStage::from_str("invalid"), None);
    }

    // ============================================================================
    // Entry Point Tests
    // ============================================================================

    #[test]
    fn test_default_entry_point() {
        let source = "#version 450\nvoid main() {}".to_string();
        let shader = ShaderData::from_glsl(ShaderStage::Vertex, source, None).unwrap();
        assert_eq!(shader.entry_point(), "main");
    }

    #[test]
    fn test_empty_entry_point_rejected() {
        let source = "#version 450\nvoid main() {}".to_string();
        let result = ShaderData::from_glsl(ShaderStage::Vertex, source, Some("".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::MissingEntryPoint { .. }));
    }

    // ============================================================================
    // ShaderSource Tests
    // ============================================================================

    #[test]
    fn test_shader_source_type_checks() {
        let glsl_source = ShaderSource::Glsl("test".to_string());
        assert!(glsl_source.is_glsl());
        assert!(!glsl_source.is_spirv());
        assert_eq!(glsl_source.as_glsl(), Some("test"));
        assert_eq!(glsl_source.as_spirv(), None);

        let spirv_source = ShaderSource::Spirv(vec![0x07230203]);
        assert!(spirv_source.is_spirv());
        assert!(!spirv_source.is_glsl());
        assert_eq!(spirv_source.as_spirv(), Some(&[0x07230203][..]));
        assert_eq!(spirv_source.as_glsl(), None);
    }

    // ============================================================================
    // File I/O Tests (non-WASM only)
    // ============================================================================

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_glsl_file_not_found() {
        use std::path::Path;

        let result =
            ShaderData::load_glsl_file(Path::new("nonexistent.glsl"), ShaderStage::Vertex, None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::IoError { .. }));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_spirv_file_invalid_size() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temp file with invalid size (not multiple of 4)
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0x03, 0x02, 0x23]).unwrap(); // 3 bytes

        let result = ShaderData::load_spirv_file(file.path(), ShaderStage::Vertex, None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ShaderError::InvalidSpirv { .. }));
    }
}
