//! Asset validation infrastructure
//!
//! Multi-layer validation system for asset data:
//! - Format validation (magic numbers, versions)
//! - Data integrity (NaN/Inf detection, bounds checking)
//! - Checksum verification (Blake3 hashing)

use blake3::Hasher;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum ValidationError {
        InvalidMagic { expected: String, got: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        UnsupportedVersion { version: u32, max_supported: u32 } = ErrorCode::VersionMismatch, ErrorSeverity::Error,
        InvalidVertexData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        IndexOutOfBounds { index: u32, vertex_count: usize } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        ChecksumMismatch { expected: [u8; 32], actual: [u8; 32] } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        NaNDetected { field: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        InfinityDetected { field: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        InvalidDimensions { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        MipmapChainInconsistent { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        InvalidFactorRange { field: String, value: f32, min: f32, max: f32 } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        InvalidYamlStructure { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        EmptyData {} = ErrorCode::InvalidFormat, ErrorSeverity::Error,
    }
}

/// Validation warning (non-fatal issues)
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarning {
    /// Unusual but valid data
    UnusualValue {
        /// Field name
        field: String,
        /// Reason for warning
        reason: String,
    },
    /// Performance concern
    PerformanceConcern {
        /// Reason for concern
        reason: String,
    },
    /// Missing optional data
    MissingOptional {
        /// Field name
        field: String,
    },
}

/// Result of asset validation
// Note: Clone removed temporarily until ValidationError supports Clone
#[derive(Debug)]
pub struct ValidationReport {
    /// Fatal errors that prevent asset usage
    pub errors: Vec<ValidationError>,
    /// Non-fatal warnings
    pub warnings: Vec<ValidationWarning>,
    /// Whether validation passed (no errors)
    pub passed: bool,
}

impl ValidationReport {
    /// Create a new validation report
    pub fn new() -> Self {
        Self { errors: Vec::new(), warnings: Vec::new(), passed: true }
    }

    /// Add an error to the report
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.passed = false;
    }

    /// Add a warning to the report
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.passed && self.errors.is_empty()
    }

    /// Get first error if any
    pub fn first_error(&self) -> Option<&ValidationError> {
        self.errors.first()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for asset validation
pub trait AssetValidator {
    /// Validate binary format (magic numbers, version, structure)
    fn validate_format(data: &[u8]) -> Result<(), ValidationError>;

    /// Validate data integrity (NaN/Inf, bounds, consistency)
    fn validate_data(&self) -> Result<(), ValidationError>;

    /// Validate checksum against expected hash
    fn validate_checksum(&self, expected: &[u8; 32]) -> Result<(), ValidationError>;

    /// Compute Blake3 hash of the asset data
    fn compute_checksum(&self) -> [u8; 32];

    /// Perform full validation (all layers)
    fn validate_all(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Data integrity validation
        if let Err(e) = self.validate_data() {
            report.add_error(e);
        }

        report
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if f32 value is valid (not NaN or Inf)
#[inline]
pub fn check_f32(value: f32, field: &str) -> Result<(), ValidationError> {
    if value.is_nan() {
        return Err(ValidationError::nandetected(field.to_string()));
    }
    if value.is_infinite() {
        return Err(ValidationError::infinitydetected(field.to_string()));
    }
    Ok(())
}

/// Check if f32 value is in valid range
#[inline]
pub fn check_f32_range(value: f32, field: &str, min: f32, max: f32) -> Result<(), ValidationError> {
    check_f32(value, field)?;
    if value < min || value > max {
        return Err(ValidationError::invalidfactorrange(field.to_string(), value, min, max));
    }
    Ok(())
}

/// Compute Blake3 hash of data
#[inline]
pub fn compute_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new();
        assert!(report.passed);
        assert!(report.errors.is_empty());
        assert!(report.warnings.is_empty());
        assert!(report.is_valid());
    }

    #[test]
    fn test_validation_report_add_error() {
        let mut report = ValidationReport::new();
        assert!(report.is_valid());

        report.add_error(ValidationError::emptydata());
        assert!(!report.is_valid());
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn test_validation_report_add_warning() {
        let mut report = ValidationReport::new();
        report.add_warning(ValidationWarning::MissingOptional { field: "test".to_string() });

        assert!(report.is_valid()); // Warnings don't fail validation
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn test_validation_report_first_error() {
        let mut report = ValidationReport::new();
        assert!(report.first_error().is_none());

        report.add_error(ValidationError::emptydata());
        assert!(report.first_error().is_some());
    }

    #[test]
    fn test_check_f32_valid() {
        assert!(check_f32(0.0, "test").is_ok());
        assert!(check_f32(1.5, "test").is_ok());
        assert!(check_f32(-100.0, "test").is_ok());
    }

    #[test]
    fn test_check_f32_nan() {
        let result = check_f32(f32::NAN, "test_field");
        assert!(result.is_err());
        match result {
            Err(ValidationError::NaNDetected { field }) => {
                assert_eq!(field, "test_field");
            }
            _ => panic!("Expected NaNDetected error"),
        }
    }

    #[test]
    fn test_check_f32_infinity() {
        let result = check_f32(f32::INFINITY, "test_field");
        assert!(result.is_err());
        match result {
            Err(ValidationError::InfinityDetected { field }) => {
                assert_eq!(field, "test_field");
            }
            _ => panic!("Expected InfinityDetected error"),
        }
    }

    #[test]
    fn test_check_f32_neg_infinity() {
        let result = check_f32(f32::NEG_INFINITY, "test_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_f32_range_valid() {
        assert!(check_f32_range(0.5, "factor", 0.0, 1.0).is_ok());
        assert!(check_f32_range(0.0, "factor", 0.0, 1.0).is_ok());
        assert!(check_f32_range(1.0, "factor", 0.0, 1.0).is_ok());
    }

    #[test]
    fn test_check_f32_range_out_of_bounds() {
        let result = check_f32_range(1.5, "factor", 0.0, 1.0);
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidFactorRange { field, value, min, max }) => {
                assert_eq!(field, "factor");
                assert_eq!(value, 1.5);
                assert_eq!(min, 0.0);
                assert_eq!(max, 1.0);
            }
            _ => panic!("Expected InvalidFactorRange error"),
        }
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let data = b"test data";
        let hash1 = compute_hash(data);
        let hash2 = compute_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different_data() {
        let hash1 = compute_hash(b"data1");
        let hash2 = compute_hash(b"data2");
        assert_ne!(hash1, hash2);
    }
}
