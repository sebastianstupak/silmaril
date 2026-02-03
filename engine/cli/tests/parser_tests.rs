//! Comprehensive unit tests for the parser module.
//!
//! This file contains 25+ tests covering all edge cases and scenarios
//! for field parsing, query parsing, array extraction, and default value generation.

use silm::codegen::{
    default_value_for_type, extract_array_type, parse_fields, parse_query_components, QueryAccess,
};

// ============================================================================
// Field Parsing Tests
// ============================================================================

#[test]
fn test_parse_simple_fields() {
    let result = parse_fields("current:f32,max:f32").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], ("current".to_string(), "f32".to_string()));
    assert_eq!(result[1], ("max".to_string(), "f32".to_string()));
}

#[test]
fn test_parse_single_field() {
    let result = parse_fields("value:i32").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], ("value".to_string(), "i32".to_string()));
}

#[test]
fn test_parse_complex_types() {
    let result = parse_fields("items:Vec<Item>,capacity:usize").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "items");
    assert_eq!(result[0].1, "Vec<Item>");
    assert_eq!(result[1].0, "capacity");
    assert_eq!(result[1].1, "usize");
}

#[test]
fn test_parse_array_types() {
    let result = parse_fields("position:[f32;3]").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "position");
    assert_eq!(result[0].1, "[f32;3]");
}

#[test]
fn test_parse_array_with_spaces() {
    let result = parse_fields("position:[f32; 3]").unwrap();
    assert_eq!(result[0].1, "[f32; 3]");
}

#[test]
fn test_parse_nested_generics() {
    let result = parse_fields("data:Vec<Option<String>>").unwrap();
    assert_eq!(result[0].1, "Vec<Option<String>>");
}

#[test]
fn test_parse_multiple_fields_with_spaces() {
    let result = parse_fields(" current : f32 , max : f32 ").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "current");
    assert_eq!(result[0].1, "f32");
}

#[test]
fn test_parse_invalid_format_no_colon() {
    let result = parse_fields("invalid");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_format_multiple_colons() {
    let result = parse_fields("name:type:extra");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_field_name() {
    let result = parse_fields(":f32");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_type() {
    let result = parse_fields("name:");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_field_name_pascal_case() {
    let result = parse_fields("InvalidName:f32");
    assert!(result.is_err());
}

#[test]
fn test_parse_field_with_underscore() {
    let result = parse_fields("_internal_field:i32").unwrap();
    assert_eq!(result[0].0, "_internal_field");
}

#[test]
fn test_parse_many_fields() {
    let result = parse_fields("a:i32,b:f32,c:bool,d:String").unwrap();
    assert_eq!(result.len(), 4);
}

// ============================================================================
// Query Component Parsing Tests
// ============================================================================

#[test]
fn test_parse_immutable_query() {
    let result = parse_query_components("&Health,&Velocity").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[0].access, QueryAccess::Immutable);
    assert_eq!(result[1].name, "Velocity");
    assert_eq!(result[1].access, QueryAccess::Immutable);
}

#[test]
fn test_parse_mutable_query() {
    let result = parse_query_components("&mut Health,&RegenerationRate").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[0].access, QueryAccess::Mutable);
    assert_eq!(result[1].name, "RegenerationRate");
    assert_eq!(result[1].access, QueryAccess::Immutable);
}

#[test]
fn test_parse_single_component() {
    let result = parse_query_components("&Transform").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Transform");
    assert_eq!(result[0].access, QueryAccess::Immutable);
}

#[test]
fn test_parse_all_mutable() {
    let result = parse_query_components("&mut Transform,&mut Velocity").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].access, QueryAccess::Mutable);
    assert_eq!(result[1].access, QueryAccess::Mutable);
}

#[test]
fn test_parse_query_with_spaces() {
    let result = parse_query_components(" &mut Health , &Velocity ").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[0].access, QueryAccess::Mutable);
}

#[test]
fn test_parse_query_missing_ampersand() {
    let result = parse_query_components("Health");
    assert!(result.is_err());
}

#[test]
fn test_parse_query_invalid_component_name() {
    let result = parse_query_components("&invalid_name");
    assert!(result.is_err());
}

#[test]
fn test_parse_query_mut_without_ampersand() {
    let result = parse_query_components("mut Health");
    assert!(result.is_err());
}

// ============================================================================
// Array Type Extraction Tests
// ============================================================================

#[test]
fn test_extract_f32_array() {
    let result = extract_array_type("[f32; 3]");
    assert_eq!(result, Some(("f32".to_string(), 3)));
}

#[test]
fn test_extract_i32_array() {
    let result = extract_array_type("[i32; 10]");
    assert_eq!(result, Some(("i32".to_string(), 10)));
}

#[test]
fn test_extract_with_spaces() {
    let result = extract_array_type("[ f32 ; 3 ]");
    assert_eq!(result, Some(("f32".to_string(), 3)));
}

#[test]
fn test_extract_no_spaces() {
    let result = extract_array_type("[f32;3]");
    assert_eq!(result, Some(("f32".to_string(), 3)));
}

#[test]
fn test_extract_complex_type() {
    let result = extract_array_type("[Vec<Item>; 5]");
    assert_eq!(result, Some(("Vec<Item>".to_string(), 5)));
}

#[test]
fn test_extract_not_array() {
    let result = extract_array_type("Vec<Item>");
    assert_eq!(result, None);
}

#[test]
fn test_extract_invalid_no_semicolon() {
    let result = extract_array_type("[f32 3]");
    assert_eq!(result, None);
}

#[test]
fn test_extract_invalid_size() {
    let result = extract_array_type("[f32; abc]");
    assert_eq!(result, None);
}

#[test]
fn test_extract_zero_size() {
    let result = extract_array_type("[f32; 0]");
    assert_eq!(result, Some(("f32".to_string(), 0)));
}

#[test]
fn test_extract_large_size() {
    let result = extract_array_type("[i32; 1000]");
    assert_eq!(result, Some(("i32".to_string(), 1000)));
}

// ============================================================================
// Default Value Generation Tests
// ============================================================================

#[test]
fn test_default_f32() {
    assert_eq!(default_value_for_type("f32"), "0.0");
}

#[test]
fn test_default_f64() {
    assert_eq!(default_value_for_type("f64"), "0.0");
}

#[test]
fn test_default_i8() {
    assert_eq!(default_value_for_type("i8"), "0");
}

#[test]
fn test_default_i16() {
    assert_eq!(default_value_for_type("i16"), "0");
}

#[test]
fn test_default_i32() {
    assert_eq!(default_value_for_type("i32"), "0");
}

#[test]
fn test_default_i64() {
    assert_eq!(default_value_for_type("i64"), "0");
}

#[test]
fn test_default_i128() {
    assert_eq!(default_value_for_type("i128"), "0");
}

#[test]
fn test_default_u32() {
    assert_eq!(default_value_for_type("u32"), "0");
}

#[test]
fn test_default_u64() {
    assert_eq!(default_value_for_type("u64"), "0");
}

#[test]
fn test_default_usize() {
    assert_eq!(default_value_for_type("usize"), "0");
}

#[test]
fn test_default_isize() {
    assert_eq!(default_value_for_type("isize"), "0");
}

#[test]
fn test_default_bool() {
    assert_eq!(default_value_for_type("bool"), "false");
}

#[test]
fn test_default_string() {
    assert_eq!(default_value_for_type("String"), "String::new()");
}

#[test]
fn test_default_vec() {
    assert_eq!(default_value_for_type("Vec<Item>"), "Vec::new()");
}

#[test]
fn test_default_option() {
    assert_eq!(default_value_for_type("Option<String>"), "None");
}

#[test]
fn test_default_hashmap() {
    assert_eq!(default_value_for_type("HashMap<K,V>"), "HashMap::new()");
}

#[test]
fn test_default_hashset() {
    assert_eq!(default_value_for_type("HashSet<T>"), "HashSet::new()");
}

#[test]
fn test_default_array_f32() {
    assert_eq!(default_value_for_type("[f32; 3]"), "[0.0; 3]");
}

#[test]
fn test_default_array_i32() {
    assert_eq!(default_value_for_type("[i32; 5]"), "[0; 5]");
}

#[test]
fn test_default_array_bool() {
    assert_eq!(default_value_for_type("[bool; 2]"), "[false; 2]");
}

#[test]
fn test_default_array_string() {
    assert_eq!(default_value_for_type("[String; 4]"), "[String::new(); 4]");
}

#[test]
fn test_default_custom_type() {
    assert_eq!(default_value_for_type("CustomType"), "Default::default()");
}

#[test]
fn test_default_with_spaces() {
    assert_eq!(default_value_for_type("  f32  "), "0.0");
}

#[test]
fn test_default_nested_array() {
    // Arrays of arrays might use Default::default
    assert_eq!(default_value_for_type("[[f32; 3]; 2]"), "Default::default()");
}
