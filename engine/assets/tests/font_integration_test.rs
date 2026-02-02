//! Integration tests for font loading
//!
//! Tests font loading from TTF/OTF formats with various configurations.
//! Note: Creating minimal valid TTF fonts is complex, so we focus on error handling
//! and API testing with mock data. Real font file testing should be done with actual fonts.

use engine_assets::{FontData, FontError, FontMetrics, FontStyle, FontWeight};

// Note: Creating a minimal VALID TTF that ttf-parser will accept is quite complex
// as it requires exact table offsets, checksums, and specific required tables.
// For integration testing with real fonts, use actual TTF files from system fonts.
// These tests focus on error handling and API correctness.

#[test]
fn test_invalid_font_data() {
    let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Invalid font data

    let result = FontData::from_ttf(&invalid_data);
    assert!(result.is_err());

    match result {
        Err(FontError::InvalidFormat { .. }) => {
            // Expected error
        }
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[test]
fn test_empty_font_data() {
    let empty_data: Vec<u8> = vec![];

    let result = FontData::from_ttf(&empty_data);
    assert!(result.is_err());

    match result {
        Err(FontError::InvalidFormat { .. }) => {
            // Expected error
        }
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[test]
fn test_wrong_signature_ttf() {
    // Create data with wrong signature
    let mut data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Wrong signature
    data.extend(vec![0u8; 100]); // Add some padding

    let result = FontData::from_ttf(&data);
    assert!(result.is_err());
}

#[test]
fn test_wrong_signature_otf() {
    // Create data with wrong signature
    let mut data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Wrong signature
    data.extend(vec![0u8; 100]); // Add some padding

    let result = FontData::from_otf(&data);
    assert!(result.is_err());
}

#[test]
fn test_truncated_font_data() {
    // Create a TTF signature but truncated data
    let data = vec![
        0x00, 0x01, 0x00, 0x00, // TrueType signature
        0x00, 0x01, // only 6 bytes total (truncated)
    ];

    let result = FontData::from_ttf(&data);
    assert!(result.is_err());
}

#[test]
fn test_font_metrics_line_height() {
    let metrics = FontMetrics::new(800, -200, 100, 1000);
    assert_eq!(metrics.line_height(), 1100); // 800 - (-200) + 100
    assert_eq!(metrics.ascent, 800);
    assert_eq!(metrics.descent, -200);
    assert_eq!(metrics.line_gap, 100);
    assert_eq!(metrics.units_per_em, 1000);
}

#[test]
fn test_font_metrics_with_negative_descent() {
    let metrics = FontMetrics::new(1000, -300, 200, 2048);
    assert_eq!(metrics.line_height(), 1500); // 1000 - (-300) + 200
}

#[test]
fn test_font_weight_conversion() {
    assert_eq!(FontWeight::from_value(100), FontWeight::Thin);
    assert_eq!(FontWeight::from_value(200), FontWeight::ExtraLight);
    assert_eq!(FontWeight::from_value(300), FontWeight::Light);
    assert_eq!(FontWeight::from_value(400), FontWeight::Normal);
    assert_eq!(FontWeight::from_value(500), FontWeight::Medium);
    assert_eq!(FontWeight::from_value(600), FontWeight::SemiBold);
    assert_eq!(FontWeight::from_value(700), FontWeight::Bold);
    assert_eq!(FontWeight::from_value(800), FontWeight::ExtraBold);
    assert_eq!(FontWeight::from_value(900), FontWeight::Black);

    assert_eq!(FontWeight::Thin.to_value(), 100);
    assert_eq!(FontWeight::Normal.to_value(), 400);
    assert_eq!(FontWeight::Bold.to_value(), 700);
}

#[test]
fn test_font_weight_boundary_values() {
    // Test boundary cases
    assert_eq!(FontWeight::from_value(0), FontWeight::Thin);
    assert_eq!(FontWeight::from_value(150), FontWeight::Thin);
    assert_eq!(FontWeight::from_value(151), FontWeight::ExtraLight);
    assert_eq!(FontWeight::from_value(250), FontWeight::ExtraLight);
    assert_eq!(FontWeight::from_value(450), FontWeight::Normal);
    assert_eq!(FontWeight::from_value(1000), FontWeight::Black);
}

#[test]
fn test_font_style_default() {
    assert_eq!(FontStyle::default(), FontStyle::Normal);
}

#[test]
fn test_font_weight_default() {
    assert_eq!(FontWeight::default(), FontWeight::Normal);
}

#[test]
fn test_font_style_enum_values() {
    // Test that all style variants exist and are distinct
    let styles = vec![FontStyle::Normal, FontStyle::Italic, FontStyle::Oblique];
    assert_eq!(styles.len(), 3);
    assert_ne!(FontStyle::Normal, FontStyle::Italic);
    assert_ne!(FontStyle::Italic, FontStyle::Oblique);
    assert_ne!(FontStyle::Normal, FontStyle::Oblique);
}

#[test]
fn test_font_data_manual_construction() {
    // Test creating FontData directly with the constructor
    let test_data = vec![1, 2, 3, 4, 5];
    let metrics = FontMetrics::new(750, -250, 90, 2048);

    let font = FontData::new(
        "Test Family".to_string(),
        FontStyle::Italic,
        FontWeight::Bold,
        test_data.clone(),
        metrics,
    );

    assert_eq!(font.family, "Test Family");
    assert_eq!(font.style, FontStyle::Italic);
    assert_eq!(font.weight, FontWeight::Bold);
    assert_eq!(font.data, test_data);
    assert_eq!(font.metrics.ascent, 750);
    assert_eq!(font.metrics.descent, -250);
}

#[test]
fn test_font_data_serialization() {
    // Test that FontData can be serialized and deserialized
    let test_data = vec![1, 2, 3, 4, 5];
    let metrics = FontMetrics::new(800, -200, 100, 1000);

    let font = FontData::new(
        "Serif Font".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        test_data,
        metrics,
    );

    let serialized = bincode::serialize(&font).expect("Failed to serialize");
    let deserialized: FontData = bincode::deserialize(&serialized).expect("Failed to deserialize");

    assert_eq!(font.family, deserialized.family);
    assert_eq!(font.style, deserialized.style);
    assert_eq!(font.weight, deserialized.weight);
    assert_eq!(font.data, deserialized.data);
    assert_eq!(font.metrics, deserialized.metrics);
}

#[test]
fn test_font_data_memory_usage() {
    let data = vec![0u8; 10000];
    let font = FontData::new(
        "Large Font".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        data,
        FontMetrics::new(800, -200, 100, 1000),
    );

    // Should include size of struct + data + family string
    let usage = font.memory_usage();
    assert!(usage >= 10000); // At least the data size
    assert!(usage > 10000); // Plus struct and string overhead
}

#[test]
fn test_font_metrics_equality() {
    let metrics1 = FontMetrics::new(800, -200, 100, 1000);
    let metrics2 = FontMetrics::new(800, -200, 100, 1000);
    let metrics3 = FontMetrics::new(800, -200, 100, 2000);

    assert_eq!(metrics1, metrics2);
    assert_ne!(metrics1, metrics3);
}

#[test]
fn test_font_error_types() {
    // Test that all error variants can be constructed
    let _error1 = FontError::InvalidFormat { reason: "test".to_string() };
    let _error2 = FontError::MissingTable { table: "head".to_string() };
    let _error3 = FontError::ParseError { reason: "test".to_string() };
    let _error4 = FontError::UnsupportedFormat { format: "test".to_string() };
}

// Note: Tests for actual TTF/OTF parsing require real font files.
// The ttf-parser library is strict about font format and requires:
// - Correct offsets for all tables
// - Valid checksums
// - Required tables: head, hhea, maxp, name, etc.
// - Proper table directory structure
//
// For production testing, use actual system fonts or embed small test fonts.
// Example with a real font (commented out as it requires font files):
//
// #[test]
// fn test_load_real_ttf_font() {
//     let font_bytes = std::fs::read("/path/to/font.ttf").unwrap();
//     let font = FontData::from_ttf(&font_bytes).unwrap();
//     assert!(!font.family.is_empty());
//     assert!(font.glyph_count() > 0);
// }
