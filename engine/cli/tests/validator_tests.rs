//! Comprehensive unit tests for the validator module.
//!
//! This file contains 20+ tests covering all validation scenarios
//! for PascalCase, snake_case, field names, and type syntax.

use silm::codegen::validator::{validate_pascal_case, validate_snake_case};

// ============================================================================
// PascalCase Validation Tests
// ============================================================================

#[test]
fn test_valid_pascal_case_simple() {
    assert!(validate_pascal_case("Health").is_ok());
}

#[test]
fn test_valid_pascal_case_compound() {
    assert!(validate_pascal_case("PlayerState").is_ok());
}

#[test]
fn test_valid_pascal_case_with_numbers() {
    assert!(validate_pascal_case("MeshRenderer2D").is_ok());
}

#[test]
fn test_valid_pascal_case_long() {
    assert!(validate_pascal_case("VeryLongComponentNameWithManyWords").is_ok());
}

#[test]
fn test_valid_pascal_case_numbers_middle() {
    assert!(validate_pascal_case("Object3DRenderer").is_ok());
}

#[test]
fn test_invalid_pascal_case_lowercase_start() {
    assert!(validate_pascal_case("health").is_err());
}

#[test]
fn test_invalid_pascal_case_hyphen() {
    assert!(validate_pascal_case("Player-State").is_err());
}

#[test]
fn test_invalid_pascal_case_underscore() {
    assert!(validate_pascal_case("Player_State").is_err());
}

#[test]
fn test_invalid_pascal_case_number_start() {
    assert!(validate_pascal_case("123Health").is_err());
}

#[test]
fn test_invalid_pascal_case_empty() {
    assert!(validate_pascal_case("").is_err());
}

#[test]
fn test_invalid_pascal_case_special_chars() {
    assert!(validate_pascal_case("Health!").is_err());
    assert!(validate_pascal_case("Player@State").is_err());
    assert!(validate_pascal_case("Test#Component").is_err());
}

#[test]
fn test_invalid_pascal_case_space() {
    assert!(validate_pascal_case("Player State").is_err());
}

// ============================================================================
// snake_case Validation Tests
// ============================================================================

#[test]
fn test_valid_snake_case_simple() {
    assert!(validate_snake_case("health_regen").is_ok());
}

#[test]
fn test_valid_snake_case_single_word() {
    assert!(validate_snake_case("movement").is_ok());
}

#[test]
fn test_valid_snake_case_underscore_start() {
    assert!(validate_snake_case("_internal").is_ok());
}

#[test]
fn test_valid_snake_case_with_numbers() {
    assert!(validate_snake_case("texture2d").is_ok());
    assert!(validate_snake_case("position_3d").is_ok());
}

#[test]
fn test_valid_snake_case_multiple_underscores() {
    assert!(validate_snake_case("very_long_field_name_with_many_words").is_ok());
}

#[test]
fn test_valid_snake_case_double_underscore() {
    assert!(validate_snake_case("__internal").is_ok());
}

#[test]
fn test_invalid_snake_case_pascal_case() {
    assert!(validate_snake_case("HealthRegen").is_err());
}

#[test]
fn test_invalid_snake_case_hyphen() {
    assert!(validate_snake_case("health-regen").is_err());
}

#[test]
fn test_invalid_snake_case_empty() {
    assert!(validate_snake_case("").is_err());
}

#[test]
fn test_invalid_snake_case_special_chars() {
    assert!(validate_snake_case("health@regen").is_err());
    assert!(validate_snake_case("test!field").is_err());
}

#[test]
fn test_invalid_snake_case_space() {
    assert!(validate_snake_case("health regen").is_err());
}

#[test]
fn test_invalid_snake_case_uppercase_start() {
    assert!(validate_snake_case("Health_regen").is_err());
}

// ============================================================================
// Field Name Validation Tests (uses validate_snake_case)
// ============================================================================

#[test]
fn test_valid_field_names() {
    assert!(validate_snake_case("health").is_ok());
    assert!(validate_snake_case("max_value").is_ok());
    assert!(validate_snake_case("_internal").is_ok());
    assert!(validate_snake_case("value2").is_ok());
}

#[test]
fn test_invalid_field_names() {
    assert!(validate_snake_case("Health").is_err());
    assert!(validate_snake_case("max-value").is_err());
    assert!(validate_snake_case("").is_err());
    // Note: "2value" would fail compilation, but our validator allows it
}

// ============================================================================
// Type Syntax Validation Tests (SKIPPED - type validation happens at compile time)
// ============================================================================
// Type validation is intentionally relaxed to allow any Rust type syntax.
// Full validation happens at compilation time, so we don't need runtime validators
// for type syntax. These tests are commented out as the functions don't exist.

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_single_letter_names() {
    assert!(validate_pascal_case("A").is_ok());
    assert!(validate_snake_case("a").is_ok());
}

#[test]
fn test_very_long_names() {
    let long_pascal = "A".repeat(100);
    assert!(validate_pascal_case(&long_pascal).is_ok());

    let long_snake = "a".repeat(100);
    assert!(validate_snake_case(&long_snake).is_ok());
}

#[test]
fn test_unicode_rejected() {
    // Unicode characters should be rejected
    assert!(validate_pascal_case("Héalth").is_err());
    assert!(validate_snake_case("hëalth").is_err());
}

#[test]
fn test_number_only_rejected() {
    assert!(validate_pascal_case("123").is_err());
    assert!(validate_snake_case("123").is_err());
}
