use silm::codegen::component::{default_value_for_type, generate_component_code, to_snake_case};

#[test]
fn test_generate_simple_component() {
    let fields = vec![
        ("current".to_string(), "f32".to_string()),
        ("max".to_string(), "f32".to_string()),
    ];

    let code = generate_component_code("Health", &fields);

    // Check struct definition
    assert!(code.contains("pub struct Health"));
    assert!(code.contains("pub current: f32"));
    assert!(code.contains("pub max: f32"));

    // Check fixed derives
    assert!(code.contains("#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]"));

    // Check test module
    assert!(code.contains("#[cfg(test)]"));
    assert!(code.contains("mod health_tests {"));
    assert!(code.contains("test_health_add_get"));
    assert!(code.contains("test_health_serialization"));
    assert!(code.contains("test_health_remove"));
}

#[test]
fn test_generate_component_without_default() {
    let fields = vec![("value".to_string(), "String".to_string())];

    let code = generate_component_code("Name", &fields);

    assert!(code.contains("pub struct Name"));
    assert!(code.contains("pub value: String"));
    assert!(code.contains("#[cfg(test)]"));
    assert!(code.contains("mod name_tests {"));
}

#[test]
fn test_generate_component_with_vec_field() {
    let fields = vec![
        ("items".to_string(), "Vec<Item>".to_string()),
        ("capacity".to_string(), "usize".to_string()),
    ];

    let code = generate_component_code("Inventory", &fields);

    assert!(code.contains("pub items: Vec<Item>"));
    assert!(code.contains("pub capacity: usize"));
    assert!(code.contains("items: Vec::new()"));
    assert!(code.contains("capacity: 0"));
}

#[test]
fn test_generate_component_with_array_field() {
    let fields = vec![("position".to_string(), "[f32; 3]".to_string())];

    let code = generate_component_code("Transform", &fields);

    assert!(code.contains("pub position: [f32; 3]"));
    assert!(code.contains("position: [0.0, 0.0, 0.0]"));
}

#[test]
fn test_generate_component_with_option_field() {
    let fields = vec![("metadata".to_string(), "Option<String>".to_string())];

    let code = generate_component_code("Entity", &fields);

    assert!(code.contains("pub metadata: Option<String>"));
    assert!(code.contains("metadata: None"));
}

#[test]
fn test_generate_component_fixed_derives() {
    let fields = vec![("value".to_string(), "i32".to_string())];

    let code = generate_component_code("Counter", &fields);

    // Should have fixed derives
    assert!(code.contains("#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]"));
    // Should only have one Debug in derives line
    let derives_section = code
        .lines()
        .find(|line| line.contains("#[derive("))
        .unwrap();
    assert_eq!(derives_section.matches("Debug").count(), 1);
    assert_eq!(derives_section.matches("Clone").count(), 1);
}

#[test]
fn test_generate_component_uses_serde_json() {
    let fields = vec![("hp".to_string(), "f32".to_string())];

    let code = generate_component_code("Health", &fields);

    assert!(code.contains("serde_json"));
    assert!(!code.contains("serde_yaml"));
}

#[test]
fn test_generate_component_domain_scoped_test_module() {
    let fields = vec![("hp".to_string(), "f32".to_string())];

    let code = generate_component_code("Health", &fields);

    assert!(code.contains("mod health_tests {"));
    assert!(!code.contains("mod tests {"));
}

#[test]
fn test_generate_component_multiple_fields() {
    let fields = vec![
        ("x".to_string(), "f32".to_string()),
        ("y".to_string(), "f32".to_string()),
        ("z".to_string(), "f32".to_string()),
    ];

    let code = generate_component_code("Position", &fields);

    assert!(code.contains("pub x: f32"));
    assert!(code.contains("pub y: f32"));
    assert!(code.contains("pub z: f32"));
    assert!(code.contains("x: 0.0"));
    assert!(code.contains("y: 0.0"));
    assert!(code.contains("z: 0.0"));
}

#[test]
fn test_default_value_primitives() {
    assert_eq!(default_value_for_type("f32"), "0.0");
    assert_eq!(default_value_for_type("f64"), "0.0");
    assert_eq!(default_value_for_type("i32"), "0");
    assert_eq!(default_value_for_type("u64"), "0");
    assert_eq!(default_value_for_type("bool"), "false");
}

#[test]
fn test_default_value_string() {
    assert_eq!(default_value_for_type("String"), "String::new()");
}

#[test]
fn test_default_value_vec() {
    assert_eq!(default_value_for_type("Vec<Item>"), "Vec::new()");
    assert_eq!(default_value_for_type("Vec<String>"), "Vec::new()");
}

#[test]
fn test_default_value_option() {
    assert_eq!(default_value_for_type("Option<String>"), "None");
    assert_eq!(default_value_for_type("Option<i32>"), "None");
}

#[test]
fn test_default_value_array() {
    assert_eq!(default_value_for_type("[f32; 3]"), "[0.0, 0.0, 0.0]");
    assert_eq!(default_value_for_type("[i32; 2]"), "[0, 0]");
    assert_eq!(default_value_for_type("[bool; 4]"), "[false, false, false, false]");
}

#[test]
fn test_default_value_custom_type() {
    assert_eq!(default_value_for_type("CustomType"), "Default::default()");
    assert_eq!(default_value_for_type("MyStruct"), "Default::default()");
}

#[test]
fn test_to_snake_case_simple() {
    assert_eq!(to_snake_case("Health"), "health");
}

#[test]
fn test_to_snake_case_compound() {
    assert_eq!(to_snake_case("PlayerState"), "player_state");
}

#[test]
fn test_to_snake_case_with_numbers() {
    assert_eq!(to_snake_case("MeshRenderer2D"), "mesh_renderer2_d");
}

#[test]
fn test_to_snake_case_consecutive_caps() {
    assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
}

#[test]
fn test_to_snake_case_single_letter() {
    assert_eq!(to_snake_case("A"), "a");
}
