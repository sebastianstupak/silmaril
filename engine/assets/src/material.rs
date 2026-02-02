//! Material data structures and loaders
//!
//! Pure data structures for PBR materials. No rendering or GPU dependencies.
//! Can be used by server, tools, or client.

use crate::validation::{
    check_f32, check_f32_range, compute_hash, AssetValidator, ValidationError,
};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

define_error! {
    pub enum MaterialError {
        InvalidYamlFormat { reason: String } = ErrorCode::MaterialLoadFailed, ErrorSeverity::Error,
        InvalidGltfMaterial { reason: String } = ErrorCode::MaterialLoadFailed, ErrorSeverity::Error,
        IoError { reason: String } = ErrorCode::MaterialLoadFailed, ErrorSeverity::Error,
    }
}

/// Material data for PBR rendering
///
/// This is a pure data structure - rendering backends define their own
/// material systems based on this data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialData {
    /// Material name
    pub name: String,
    /// Path to base color/albedo texture
    pub base_color_texture: Option<String>,
    /// Path to metallic-roughness texture (metallic in B channel, roughness in G channel)
    pub metallic_roughness_texture: Option<String>,
    /// Path to normal map texture
    pub normal_texture: Option<String>,
    /// Path to emissive texture
    pub emissive_texture: Option<String>,
    /// Base color factor [R, G, B, A]
    pub base_color_factor: [f32; 4],
    /// Metallic factor (0.0 = dielectric, 1.0 = metal)
    pub metallic_factor: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness_factor: f32,
    /// Emissive factor [R, G, B]
    pub emissive_factor: [f32; 3],
}

impl MaterialData {
    /// Create a new material with default PBR values
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_texture: None,
            emissive_texture: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            emissive_factor: [0.0, 0.0, 0.0],
        }
    }

    /// Load material from YAML file
    #[instrument(skip(yaml_content))]
    pub fn from_yaml(yaml_content: &str) -> Result<Self, MaterialError> {
        info!("Parsing material from YAML");
        serde_yaml::from_str(yaml_content)
            .map_err(|e| MaterialError::InvalidYamlFormat { reason: e.to_string() })
    }

    /// Save material to YAML format
    #[instrument(skip(self))]
    pub fn to_yaml(&self) -> Result<String, MaterialError> {
        info!(material_name = %self.name, "Serializing material to YAML");
        serde_yaml::to_string(self)
            .map_err(|e| MaterialError::InvalidYamlFormat { reason: e.to_string() })
    }

    /// Parse material from glTF material data
    #[instrument(skip(gltf_material))]
    pub fn from_gltf(gltf_material: &gltf::Material) -> Result<Self, MaterialError> {
        let material_index = gltf_material.index().unwrap_or(0);
        info!(material_index = material_index, "Parsing material from glTF");

        let pbr = gltf_material.pbr_metallic_roughness();
        let name = format!("material_{}", material_index);

        let base_color_texture =
            pbr.base_color_texture().map(|tex| format!("texture_{}", tex.texture().index()));

        let metallic_roughness_texture = pbr
            .metallic_roughness_texture()
            .map(|tex| format!("texture_{}", tex.texture().index()));

        let normal_texture = gltf_material
            .normal_texture()
            .map(|tex| format!("texture_{}", tex.texture().index()));

        let emissive_texture = gltf_material
            .emissive_texture()
            .map(|tex| format!("texture_{}", tex.texture().index()));

        let base_color_factor = pbr.base_color_factor();
        let metallic_factor = pbr.metallic_factor();
        let roughness_factor = pbr.roughness_factor();
        let emissive_factor = gltf_material.emissive_factor();

        Ok(Self {
            name,
            base_color_texture,
            metallic_roughness_texture,
            normal_texture,
            emissive_texture,
            base_color_factor,
            metallic_factor,
            roughness_factor,
            emissive_factor,
        })
    }
}

impl Default for MaterialData {
    fn default() -> Self {
        Self::new("default")
    }
}

// ============================================================================
// Validation Implementation
// ============================================================================

impl AssetValidator for MaterialData {
    /// Validate material YAML structure
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        // Try to parse as YAML
        let yaml_str = std::str::from_utf8(data).map_err(|_| {
            ValidationError::invalidyamlstructure("Invalid UTF-8 in YAML".to_string())
        })?;

        // Attempt to parse
        serde_yaml::from_str::<MaterialData>(yaml_str).map_err(|e| {
            ValidationError::invalidyamlstructure(format!("YAML parse failed: {}", e))
        })?;

        Ok(())
    }

    /// Validate material data integrity
    fn validate_data(&self) -> Result<(), ValidationError> {
        // Validate base color factor components [R, G, B, A]
        for (i, &component) in self.base_color_factor.iter().enumerate() {
            check_f32_range(component, &format!("base_color_factor[{}]", i), 0.0, 1.0)?;
        }

        // Validate metallic factor (0.0 = dielectric, 1.0 = metal)
        check_f32_range(self.metallic_factor, "metallic_factor", 0.0, 1.0)?;

        // Validate roughness factor (0.0 = smooth, 1.0 = rough)
        check_f32_range(self.roughness_factor, "roughness_factor", 0.0, 1.0)?;

        // Validate emissive factor [R, G, B]
        // Emissive can be > 1.0 for HDR, but check for NaN/Inf
        for (i, &component) in self.emissive_factor.iter().enumerate() {
            check_f32(component, &format!("emissive_factor[{}]", i))?;
            // Warn if emissive is extremely high (likely a mistake)
            if component > 100.0 {
                // This would be a warning, not an error
            }
        }

        Ok(())
    }

    /// Validate checksum
    fn validate_checksum(&self, expected: &[u8; 32]) -> Result<(), ValidationError> {
        let actual = self.compute_checksum();
        if &actual != expected {
            return Err(ValidationError::checksummismatch(*expected, actual));
        }
        Ok(())
    }

    /// Compute Blake3 checksum of material data
    fn compute_checksum(&self) -> [u8; 32] {
        // Serialize to YAML for consistent hashing
        let yaml = self.to_yaml().unwrap_or_default();
        compute_hash(yaml.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_material_with_pbr_parameters() {
        let material = MaterialData::new("test_material");

        assert_eq!(material.name, "test_material");
        assert_eq!(material.base_color_texture, None);
        assert_eq!(material.metallic_roughness_texture, None);
        assert_eq!(material.normal_texture, None);
        assert_eq!(material.emissive_texture, None);
        assert_eq!(material.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(material.metallic_factor, 0.0);
        assert_eq!(material.roughness_factor, 0.5);
        assert_eq!(material.emissive_factor, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_default_values() {
        let material = MaterialData::default();

        assert_eq!(material.name, "default");
        assert_eq!(material.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(material.metallic_factor, 0.0);
        assert_eq!(material.roughness_factor, 0.5);
        assert_eq!(material.emissive_factor, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_material_serialization_roundtrip() {
        let mut material = MaterialData::new("brick");
        material.base_color_texture = Some("brick_albedo.png".to_string());
        material.normal_texture = Some("brick_normal.png".to_string());
        material.base_color_factor = [0.8, 0.6, 0.4, 1.0];
        material.metallic_factor = 0.0;
        material.roughness_factor = 0.8;

        // Serialize to YAML
        let yaml = material.to_yaml().expect("Failed to serialize");

        // Deserialize back
        let deserialized = MaterialData::from_yaml(&yaml).expect("Failed to deserialize");

        assert_eq!(material, deserialized);
    }

    #[test]
    fn test_load_material_from_yaml() {
        let yaml = r#"
name: "brick"
base_color_texture: "brick_albedo.png"
metallic_roughness_texture: null
normal_texture: "brick_normal.png"
emissive_texture: null
base_color_factor: [1.0, 1.0, 1.0, 1.0]
metallic_factor: 0.0
roughness_factor: 0.8
emissive_factor: [0.0, 0.0, 0.0]
"#;

        let material = MaterialData::from_yaml(yaml).expect("Failed to parse YAML");

        assert_eq!(material.name, "brick");
        assert_eq!(material.base_color_texture, Some("brick_albedo.png".to_string()));
        assert_eq!(material.normal_texture, Some("brick_normal.png".to_string()));
        assert_eq!(material.metallic_factor, 0.0);
        assert_eq!(material.roughness_factor, 0.8);
    }

    #[test]
    fn test_save_material_to_yaml() {
        let mut material = MaterialData::new("metal");
        material.base_color_texture = Some("metal_albedo.png".to_string());
        material.metallic_factor = 1.0;
        material.roughness_factor = 0.2;

        let yaml = material.to_yaml().expect("Failed to serialize");

        assert!(yaml.contains("name: metal"));
        assert!(yaml.contains("metal_albedo.png"));
        assert!(yaml.contains("metallic_factor: 1"));
        assert!(yaml.contains("roughness_factor: 0.2"));
    }

    #[test]
    fn test_yaml_roundtrip_preserves_all_fields() {
        let original = MaterialData {
            name: "complex_material".to_string(),
            base_color_texture: Some("color.png".to_string()),
            metallic_roughness_texture: Some("metal_rough.png".to_string()),
            normal_texture: Some("normal.png".to_string()),
            emissive_texture: Some("emissive.png".to_string()),
            base_color_factor: [0.5, 0.6, 0.7, 0.8],
            metallic_factor: 0.9,
            roughness_factor: 0.3,
            emissive_factor: [0.1, 0.2, 0.3],
        };

        let yaml = original.to_yaml().expect("Failed to serialize");
        let deserialized = MaterialData::from_yaml(&yaml).expect("Failed to deserialize");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_invalid_yaml_returns_error() {
        let invalid_yaml = "this is not valid yaml: {{[";
        let result = MaterialData::from_yaml(invalid_yaml);

        assert!(result.is_err());
        match result {
            Err(MaterialError::InvalidYamlFormat { .. }) => (),
            _ => panic!("Expected InvalidYamlFormat error"),
        }
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    use crate::validation::{AssetValidator, ValidationError};

    #[test]
    fn test_valid_material_passes_validation() {
        let material = MaterialData::new("test");
        let report = material.validate_all();
        assert!(report.is_valid());
    }

    #[test]
    fn test_validate_format_valid_yaml() {
        let material = MaterialData::new("test");
        let yaml = material.to_yaml().unwrap();
        assert!(MaterialData::validate_format(yaml.as_bytes()).is_ok());
    }

    #[test]
    fn test_validate_format_invalid_yaml() {
        let invalid = b"this is not valid yaml: {{[";
        let result = MaterialData::validate_format(invalid);
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidYamlStructure { .. }) => {}
            _ => panic!("Expected InvalidYamlStructure error"),
        }
    }

    #[test]
    fn test_validate_data_base_color_out_of_range() {
        let mut material = MaterialData::new("test");
        material.base_color_factor[0] = 1.5; // Out of range

        let result = material.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidFactorRange { field, value, .. }) => {
                assert!(field.contains("base_color_factor"));
                assert_eq!(value, 1.5);
            }
            _ => panic!("Expected InvalidFactorRange error"),
        }
    }

    #[test]
    fn test_validate_data_metallic_out_of_range() {
        let mut material = MaterialData::new("test");
        material.metallic_factor = -0.1;

        let result = material.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidFactorRange { field, .. }) => {
                assert_eq!(field, "metallic_factor");
            }
            _ => panic!("Expected InvalidFactorRange error"),
        }
    }

    #[test]
    fn test_validate_data_roughness_out_of_range() {
        let mut material = MaterialData::new("test");
        material.roughness_factor = 2.0;

        let result = material.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidFactorRange { field, .. }) => {
                assert_eq!(field, "roughness_factor");
            }
            _ => panic!("Expected InvalidFactorRange error"),
        }
    }

    #[test]
    fn test_validate_data_emissive_nan() {
        let mut material = MaterialData::new("test");
        material.emissive_factor[1] = f32::NAN;

        let result = material.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::NaNDetected { field }) => {
                assert!(field.contains("emissive_factor"));
            }
            _ => panic!("Expected NaNDetected error"),
        }
    }

    #[test]
    fn test_validate_data_emissive_infinity() {
        let mut material = MaterialData::new("test");
        material.emissive_factor[2] = f32::INFINITY;

        let result = material.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InfinityDetected { field }) => {
                assert!(field.contains("emissive_factor"));
            }
            _ => panic!("Expected InfinityDetected error"),
        }
    }

    #[test]
    fn test_checksum_validation_passes() {
        let material = MaterialData::new("test");
        let checksum = material.compute_checksum();
        assert!(material.validate_checksum(&checksum).is_ok());
    }

    #[test]
    fn test_checksum_validation_fails() {
        let material = MaterialData::new("test");
        let wrong_checksum = [0u8; 32];
        let result = material.validate_checksum(&wrong_checksum);
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_deterministic() {
        let material = MaterialData::new("test");
        let hash1 = material.compute_checksum();
        let hash2 = material.compute_checksum();
        assert_eq!(hash1, hash2);
    }
}
