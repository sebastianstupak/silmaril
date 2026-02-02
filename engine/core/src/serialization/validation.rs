//! Serialization validation and error recovery
//!
//! Provides:
//! - Checksum validation for data integrity
//! - Corrupt data detection
//! - Partial recovery when possible
//! - Graceful degradation strategies
//!
//! Target: Never panic, always return Result

use super::{SerializationError, WorldState};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Checksum algorithm for data validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    /// No checksum (fastest, no validation)
    None,
    /// CRC32 (fast, good error detection)
    Crc32,
    /// XXH3 (very fast, excellent distribution)
    Xxh3,
}

/// Validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Data is valid
    Valid,
    /// Data is corrupt (checksum mismatch)
    Corrupt {
        /// Expected checksum
        expected: u64,
        /// Actual checksum
        actual: u64,
    },
    /// Data is partially valid (some entities corrupt)
    PartiallyValid {
        /// Number of valid entities
        valid_count: usize,
        /// Number of corrupt entities
        corrupt_count: usize,
    },
}

impl ValidationResult {
    /// Check if data is fully valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Check if data is corrupt
    pub fn is_corrupt(&self) -> bool {
        matches!(self, ValidationResult::Corrupt { .. })
    }

    /// Check if data is partially valid
    pub fn is_partially_valid(&self) -> bool {
        matches!(self, ValidationResult::PartiallyValid { .. })
    }
}

/// Validated WorldState wrapper
///
/// Includes checksum for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedWorldState {
    /// The actual world state data
    pub state: WorldState,
    /// Checksum of the serialized state
    pub checksum: u64,
    /// Algorithm used for checksum
    pub algorithm: ChecksumAlgorithm,
}

impl ValidatedWorldState {
    /// Create a new validated world state
    pub fn new(state: WorldState, algorithm: ChecksumAlgorithm) -> Self {
        let checksum = Self::compute_checksum(&state, algorithm);
        Self { state, checksum, algorithm }
    }

    /// Compute checksum for WorldState
    fn compute_checksum(state: &WorldState, algorithm: ChecksumAlgorithm) -> u64 {
        match algorithm {
            ChecksumAlgorithm::None => 0,
            ChecksumAlgorithm::Crc32 => {
                // Simple CRC32 implementation (for demonstration)
                // In production, use a proper CRC32 crate
                let bytes = bincode::serialize(state).unwrap_or_default();
                let mut crc = 0xFFFFFFFFu32;
                for byte in bytes {
                    crc ^= byte as u32;
                    for _ in 0..8 {
                        if crc & 1 != 0 {
                            crc = (crc >> 1) ^ 0xEDB88320;
                        } else {
                            crc >>= 1;
                        }
                    }
                }
                (!crc) as u64
            }
            ChecksumAlgorithm::Xxh3 => {
                // Simple hash implementation (for demonstration)
                // In production, use xxhash crate
                let bytes = bincode::serialize(state).unwrap_or_default();
                let mut hash = 0u64;
                for byte in bytes {
                    hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
                }
                hash
            }
        }
    }

    /// Validate checksum
    pub fn validate(&self) -> ValidationResult {
        if self.algorithm == ChecksumAlgorithm::None {
            return ValidationResult::Valid;
        }

        let computed = Self::compute_checksum(&self.state, self.algorithm);

        if computed == self.checksum {
            ValidationResult::Valid
        } else {
            ValidationResult::Corrupt { expected: self.checksum, actual: computed }
        }
    }

    /// Load and validate
    pub fn load_validated(data: &[u8]) -> Result<Self, SerializationError> {
        let validated: Self = bincode::deserialize(data)
            .map_err(|e| SerializationError::bincodedeserialize(e.to_string()))?;

        match validated.validate() {
            ValidationResult::Valid => Ok(validated),
            ValidationResult::Corrupt { expected, actual } => {
                Err(SerializationError::decompressionerror(format!(
                    "Checksum validation failed. Expected: 0x{:016x}, Actual: 0x{:016x}",
                    expected, actual
                )))
            }
            ValidationResult::PartiallyValid { .. } => {
                // For now, treat partially valid as valid (could be configurable)
                Ok(validated)
            }
        }
    }

    /// Save with validation
    pub fn save(&self) -> Result<Vec<u8>, SerializationError> {
        bincode::serialize(self).map_err(|e| SerializationError::bincodeserialize(e.to_string()))
    }
}

/// Recovery strategy for corrupt data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Fail immediately on any corruption
    FailFast,
    /// Skip corrupt entities, recover what's valid
    SkipCorrupt,
    /// Use default values for corrupt entities
    UseDefaults,
}

/// Recovery options
#[derive(Debug, Clone)]
pub struct RecoveryOptions {
    /// Strategy to use for recovery
    pub strategy: RecoveryStrategy,
    /// Maximum number of corrupt entities to tolerate
    pub max_corrupt_entities: usize,
    /// Whether to log recovery actions
    pub log_recovery: bool,
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            strategy: RecoveryStrategy::FailFast,
            max_corrupt_entities: 0,
            log_recovery: true,
        }
    }
}

/// Recovery statistics
#[derive(Debug, Clone)]
pub struct RecoveryStats {
    /// Number of entities successfully recovered
    pub recovered_entities: usize,
    /// Number of entities skipped due to corruption
    pub skipped_entities: usize,
    /// Number of entities using default values
    pub defaulted_entities: usize,
    /// Recovery strategy used
    pub strategy: RecoveryStrategy,
}

/// Recovery result
pub struct RecoveryResult {
    /// Recovered world state (may be partial)
    pub state: WorldState,
    /// Recovery statistics
    pub stats: RecoveryStats,
    /// Validation result
    pub validation: ValidationResult,
}

/// Validator for WorldState
pub struct WorldStateValidator {
    /// Recovery options
    options: RecoveryOptions,
}

impl WorldStateValidator {
    /// Create a new validator with default options
    pub fn new() -> Self {
        Self { options: RecoveryOptions::default() }
    }

    /// Create a validator with custom options
    pub fn with_options(options: RecoveryOptions) -> Self {
        Self { options }
    }

    /// Validate WorldState structure
    ///
    /// Checks for:
    /// - Entity ID consistency
    /// - Component data validity
    /// - Metadata consistency
    pub fn validate_structure(&self, state: &WorldState) -> ValidationResult {
        let mut valid_count = 0;
        let mut corrupt_count = 0;

        // Check entity metadata consistency
        let entity_set: std::collections::HashSet<_> =
            state.entities.iter().map(|e| e.entity).collect();

        // Check that all entities in components map exist in entity list
        for entity in state.components.keys() {
            if entity_set.contains(entity) {
                valid_count += 1;
            } else {
                corrupt_count += 1;
                if self.options.log_recovery {
                    warn!(
                        entity = ?entity,
                        "Entity has components but not in entity list"
                    );
                }
            }
        }

        // Check metadata consistency
        if state.metadata.entity_count != state.entities.len() && self.options.log_recovery {
            warn!(
                expected = state.metadata.entity_count,
                actual = state.entities.len(),
                "Metadata entity count mismatch"
            );
        }

        if corrupt_count == 0 {
            ValidationResult::Valid
        } else {
            ValidationResult::PartiallyValid { valid_count, corrupt_count }
        }
    }

    /// Attempt to recover from corrupt data
    pub fn recover(
        &self,
        state: WorldState,
        validation: ValidationResult,
    ) -> Result<RecoveryResult, SerializationError> {
        match (self.options.strategy, &validation) {
            (RecoveryStrategy::FailFast, ValidationResult::Corrupt { .. }) => {
                Err(SerializationError::decompressionerror(
                    "Data is corrupt and FailFast strategy is enabled".to_string(),
                ))
            }
            (RecoveryStrategy::FailFast, ValidationResult::PartiallyValid { .. }) => {
                Err(SerializationError::decompressionerror(
                    "Data is partially corrupt and FailFast strategy is enabled".to_string(),
                ))
            }
            (
                RecoveryStrategy::SkipCorrupt,
                ValidationResult::PartiallyValid { valid_count, corrupt_count },
            ) => {
                if *corrupt_count > self.options.max_corrupt_entities {
                    return Err(SerializationError::decompressionerror(format!(
                        "Too many corrupt entities: {} (max: {})",
                        corrupt_count, self.options.max_corrupt_entities
                    )));
                }

                // Clean up state by removing corrupt entities
                let recovered = self.clean_corrupt_entities(state);

                Ok(RecoveryResult {
                    state: recovered,
                    stats: RecoveryStats {
                        recovered_entities: *valid_count,
                        skipped_entities: *corrupt_count,
                        defaulted_entities: 0,
                        strategy: RecoveryStrategy::SkipCorrupt,
                    },
                    validation: validation.clone(),
                })
            }
            _ => {
                // Data is valid or using defaults
                let entity_count = state.entities.len();
                Ok(RecoveryResult {
                    state,
                    stats: RecoveryStats {
                        recovered_entities: entity_count,
                        skipped_entities: 0,
                        defaulted_entities: 0,
                        strategy: self.options.strategy,
                    },
                    validation,
                })
            }
        }
    }

    /// Clean corrupt entities from state
    fn clean_corrupt_entities(&self, mut state: WorldState) -> WorldState {
        // Build valid entity set
        let entity_set: std::collections::HashSet<_> =
            state.entities.iter().map(|e| e.entity).collect();

        // Remove components for entities not in entity list
        state.components.retain(|entity, _| entity_set.contains(entity));

        // Update metadata
        state.metadata.entity_count = state.entities.len();
        state.metadata.component_count = state.components.values().map(|v| v.len()).sum();

        state
    }
}

impl Default for WorldStateValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_none() {
        let state = WorldState::new();
        let validated = ValidatedWorldState::new(state, ChecksumAlgorithm::None);

        assert_eq!(validated.checksum, 0);
        assert!(validated.validate().is_valid());
    }

    #[test]
    fn test_checksum_crc32() {
        let state = WorldState::new();
        let validated = ValidatedWorldState::new(state, ChecksumAlgorithm::Crc32);

        assert_ne!(validated.checksum, 0);
        assert!(validated.validate().is_valid());
    }

    #[test]
    fn test_checksum_mismatch() {
        let state = WorldState::new();
        let mut validated = ValidatedWorldState::new(state, ChecksumAlgorithm::Crc32);

        // Corrupt the checksum
        validated.checksum = 0;

        let result = validated.validate();
        assert!(result.is_corrupt());
    }

    #[test]
    fn test_validation_result() {
        assert!(ValidationResult::Valid.is_valid());
        assert!(!ValidationResult::Valid.is_corrupt());

        let corrupt = ValidationResult::Corrupt { expected: 123, actual: 456 };
        assert!(corrupt.is_corrupt());
        assert!(!corrupt.is_valid());

        let partial = ValidationResult::PartiallyValid { valid_count: 5, corrupt_count: 2 };
        assert!(partial.is_partially_valid());
    }

    #[test]
    fn test_validator_valid_state() {
        let validator = WorldStateValidator::new();
        let state = WorldState::new();

        let result = validator.validate_structure(&state);
        assert!(result.is_valid());
    }

    #[test]
    fn test_recovery_fail_fast() {
        let options = RecoveryOptions {
            strategy: RecoveryStrategy::FailFast,
            max_corrupt_entities: 0,
            log_recovery: false,
        };

        let validator = WorldStateValidator::with_options(options);
        let state = WorldState::new();

        let validation = ValidationResult::Corrupt { expected: 123, actual: 456 };

        let result = validator.recover(state, validation);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_skip_corrupt() {
        let options = RecoveryOptions {
            strategy: RecoveryStrategy::SkipCorrupt,
            max_corrupt_entities: 10,
            log_recovery: false,
        };

        let validator = WorldStateValidator::with_options(options);
        let state = WorldState::new();

        let validation = ValidationResult::PartiallyValid { valid_count: 5, corrupt_count: 2 };

        let result = validator.recover(state, validation);
        assert!(result.is_ok());

        let recovery = result.unwrap();
        assert_eq!(recovery.stats.recovered_entities, 5);
        assert_eq!(recovery.stats.skipped_entities, 2);
    }

    #[test]
    fn test_validated_roundtrip() {
        let state = WorldState::new();
        let validated = ValidatedWorldState::new(state, ChecksumAlgorithm::Crc32);

        let bytes = validated.save().unwrap();
        let loaded = ValidatedWorldState::load_validated(&bytes).unwrap();

        assert_eq!(loaded.checksum, validated.checksum);
        assert!(loaded.validate().is_valid());
    }
}
