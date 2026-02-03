use anyhow::{bail, Result};

use super::validator::{validate_snake_case, validate_type_syntax};

/// Parse field specifications from command-line format
///
/// # Format
/// `name:type[,name:type]*`
///
/// # Examples
/// - `"current:f32,max:f32"` → `[(current, f32), (max, f32)]`
/// - `"position:[f32;3]"` → `[(position, [f32; 3])]`
/// - `"items:Vec<Item>,capacity:usize"` → `[(items, Vec<Item>), (capacity, usize)]`
pub fn parse_fields(input: &str) -> Result<Vec<(String, String)>> {
    input
        .split(',')
        .map(|field| {
            let parts: Vec<&str> = field.trim().split(':').collect();
            if parts.len() != 2 {
                bail!("Invalid field format: '{}'. Expected 'name:type'", field);
            }

            let name = parts[0].trim().to_string();
            let type_str = parts[1].trim().to_string();

            validate_snake_case(&name)?;
            validate_type_syntax(&type_str)?;

            Ok((name, type_str))
        })
        .collect()
}

/// Extract array type and size from array syntax
///
/// # Examples
/// - `"[f32; 3]"` → `Some(("f32", 3))`
/// - `"[i32; 10]"` → `Some(("i32", 10))`
/// - `"Vec<Item>"` → `None`
pub fn extract_array_type(s: &str) -> Option<(String, usize)> {
    let s = s.trim();

    // Check if it starts with '[' and ends with ']'
    if !s.starts_with('[') || !s.ends_with(']') {
        return None;
    }

    // Remove brackets
    let inner = &s[1..s.len() - 1];

    // Split by ';'
    let parts: Vec<&str> = inner.split(';').collect();
    if parts.len() != 2 {
        return None;
    }

    let element_type = parts[0].trim().to_string();
    let size = parts[1].trim().parse::<usize>().ok()?;

    Some((element_type, size))
}

/// Convert PascalCase to snake_case
///
/// # Examples
/// - "Health" → "health"
/// - "PlayerState" → "player_state"
/// - "MeshRenderer2D" → "mesh_renderer2d"
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// Generate default value for a given Rust type
///
/// # Examples
/// - "f32" → "0.0"
/// - "String" → "String::new()"
/// - "Vec<Item>" → "Vec::new()"
/// - "[f32; 3]" → "[0.0, 0.0, 0.0]"
pub fn default_value_for_type(type_str: &str) -> String {
    match type_str {
        "f32" | "f64" => "0.0".to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" | "isize"
        | "usize" => "0".to_string(),
        "bool" => "false".to_string(),
        "String" => "String::new()".to_string(),
        s if s.starts_with("Vec<") => "Vec::new()".to_string(),
        s if s.starts_with("Option<") => "None".to_string(),
        s if s.starts_with('[') && s.contains(';') => {
            // Array type: [f32; 3] -> [0.0, 0.0, 0.0]
            if let Some((inner_type, size)) = parse_array_type(s) {
                let element_default = default_value_for_type(&inner_type);
                let elements: Vec<String> = (0..size).map(|_| element_default.clone()).collect();
                format!("[{}]", elements.join(", "))
            } else {
                "Default::default()".to_string()
            }
        }
        _ => "Default::default()".to_string(),
    }
}

/// Parse array type to extract inner type and size
///
/// # Examples
/// - "[f32; 3]" → Some(("f32", 3))
/// - "[u8; 16]" → Some(("u8", 16))
fn parse_array_type(type_str: &str) -> Option<(String, usize)> {
    let trimmed = type_str.trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = trimmed.split(';').map(|s| s.trim()).collect();

    if parts.len() != 2 {
        return None;
    }

    let inner_type = parts[0].to_string();
    let size = parts[1].parse::<usize>().ok()?;

    Some((inner_type, size))
}

/// Generate complete component code
///
/// # Arguments
/// - `name`: Component name in PascalCase (e.g., "Health")
/// - `fields`: List of (field_name, field_type) tuples
/// - `derive`: Optional additional derives (e.g., "Default,PartialEq")
/// - `doc`: Optional documentation string
///
/// # Returns
/// Complete Rust source code for the component
pub fn generate_component_code(
    name: &str,
    fields: &[(String, String)],
    derive: Option<String>,
    doc: Option<String>,
) -> String {
    let snake_name = to_snake_case(name);

    // Build derives list
    let mut derives = vec!["Debug", "Clone"];
    let has_default = if let Some(ref d) = derive {
        let custom_derives: Vec<&str> = d.split(',').map(|s| s.trim()).collect();
        for custom in custom_derives {
            if !derives.contains(&custom) {
                derives.push(custom);
            }
        }
        derives.contains(&"Default")
    } else {
        false
    };
    derives.extend_from_slice(&["Component", "Serialize", "Deserialize"]);

    let derives_str = derives.join(", ");

    // Generate documentation
    let doc_comment = if let Some(d) = doc {
        format!("/// {}\n", d)
    } else {
        format!("/// Component: {}\n", name)
    };

    // Generate struct fields
    let mut fields_code = String::new();
    for (field_name, field_type) in fields {
        fields_code.push_str(&format!(
            "    /// TODO: Document this field\n    pub {}: {},\n",
            field_name, field_type
        ));
    }

    // Generate Default implementation if requested
    let default_impl = if has_default {
        let mut default_fields = String::new();
        for (field_name, field_type) in fields {
            let default_val = default_value_for_type(field_type);
            default_fields.push_str(&format!("            {}: {},\n", field_name, default_val));
        }

        format!(
            "\nimpl Default for {} {{\n    fn default() -> Self {{\n        Self {{\n{        }}\n    }}\n}}\n",
            name, default_fields
        )
    } else {
        String::new()
    };

    // Generate test module
    let test_module = generate_test_module(name, &snake_name, has_default, fields);

    // Combine all parts
    format!(
        "use engine_core::ecs::Component;\nuse serde::{{Deserialize, Serialize}};\n\n{doc_comment}#[derive({derives})]\npub struct {name} {{\n{fields}}}\n{default_impl}\n{tests}",
        doc_comment = doc_comment,
        derives = derives_str,
        name = name,
        fields = fields_code,
        default_impl = default_impl,
        tests = test_module
    )
}

/// Generate test module for component
fn generate_test_module(
    name: &str,
    snake_name: &str,
    has_default: bool,
    fields: &[(String, String)],
) -> String {
    let creation = if has_default {
        format!("        let component = {}::default();", name)
    } else {
        // Generate manual construction
        let mut field_inits = String::new();
        for (field_name, field_type) in fields {
            let default_val = default_value_for_type(field_type);
            field_inits.push_str(&format!("\n            {}: {},", field_name, default_val));
        }
        format!("        let component = {} {{{}\n        }};", name, field_inits)
    };

    // Generate field assertions for serialization test
    let mut field_assertions = String::new();
    for (field_name, _) in fields {
        field_assertions.push_str(&format!(
            "        // assert_eq!(deserialized.{}, component.{});\n",
            field_name, field_name
        ));
    }

    format!(
        r#"#[cfg(test)]
mod tests {{
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_{snake_name}_add_get() {{
        let mut world = World::new();
        let entity = world.spawn();

{creation}
        world.add(entity, component.clone());

        let retrieved = world.get::<{name}>(entity).unwrap();
        assert!(world.has::<{name}>(entity));
    }}

    #[test]
    fn test_{snake_name}_serialization() {{
{creation}

        let yaml = serde_yaml::to_string(&component).unwrap();
        let deserialized: {name} = serde_yaml::from_str(&yaml).unwrap();

        // Field-specific assertions
{field_assertions}    }}

    #[test]
    fn test_{snake_name}_remove() {{
        let mut world = World::new();
        let entity = world.spawn();

{creation}
        world.add(entity, component);

        assert!(world.has::<{name}>(entity));
        world.remove::<{name}>(entity);
        assert!(!world.has::<{name}>(entity));
    }}
}}
"#,
        snake_name = snake_name,
        name = name,
        creation = creation,
        field_assertions = field_assertions
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_default_value_for_type_primitives() {
        assert_eq!(default_value_for_type("f32"), "0.0");
        assert_eq!(default_value_for_type("f64"), "0.0");
        assert_eq!(default_value_for_type("i32"), "0");
        assert_eq!(default_value_for_type("u64"), "0");
        assert_eq!(default_value_for_type("bool"), "false");
    }

    #[test]
    fn test_default_value_for_type_string() {
        assert_eq!(default_value_for_type("String"), "String::new()");
    }

    #[test]
    fn test_default_value_for_type_vec() {
        assert_eq!(default_value_for_type("Vec<Item>"), "Vec::new()");
        assert_eq!(default_value_for_type("Vec<String>"), "Vec::new()");
    }

    #[test]
    fn test_default_value_for_type_option() {
        assert_eq!(default_value_for_type("Option<String>"), "None");
    }

    #[test]
    fn test_default_value_for_type_array() {
        assert_eq!(default_value_for_type("[f32; 3]"), "[0.0, 0.0, 0.0]");
        assert_eq!(default_value_for_type("[i32; 2]"), "[0, 0]");
    }

    #[test]
    fn test_default_value_for_type_custom() {
        assert_eq!(default_value_for_type("CustomType"), "Default::default()");
    }

    #[test]
    fn test_parse_array_type_valid() {
        assert_eq!(parse_array_type("[f32; 3]"), Some(("f32".to_string(), 3)));
        assert_eq!(parse_array_type("[u8; 16]"), Some(("u8".to_string(), 16)));
    }

    #[test]
    fn test_parse_array_type_invalid() {
        assert_eq!(parse_array_type("[f32]"), None);
        assert_eq!(parse_array_type("f32; 3"), None);
        assert_eq!(parse_array_type("[f32; abc]"), None);
    }

    #[test]
    fn test_generate_simple_component() {
        let fields = vec![
            ("current".to_string(), "f32".to_string()),
            ("max".to_string(), "f32".to_string()),
        ];

        let code = generate_component_code(
            "Health",
            &fields,
            Some("Default".to_string()),
            Some("Player health".to_string()),
        );

        assert!(code.contains("pub struct Health"));
        assert!(code.contains("pub current: f32"));
        assert!(code.contains("pub max: f32"));
        assert!(code.contains("impl Default for Health"));
        assert!(code.contains("#[cfg(test)]"));
        assert!(code.contains("test_health_add_get"));
        assert!(code.contains("test_health_serialization"));
        assert!(code.contains("test_health_remove"));
        assert!(code.contains("/// Player health"));
    }

    #[test]
    fn test_generate_without_default() {
        let fields = vec![("value".to_string(), "String".to_string())];

        let code = generate_component_code("Name", &fields, Some("Clone".to_string()), None);

        assert!(code.contains("pub struct Name"));
        assert!(code.contains("pub value: String"));
        assert!(!code.contains("impl Default for Name"));
        assert!(code.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_generate_with_complex_types() {
        let fields = vec![
            ("items".to_string(), "Vec<Item>".to_string()),
            ("capacity".to_string(), "usize".to_string()),
        ];

        let code = generate_component_code("Inventory", &fields, Some("Default".to_string()), None);

        assert!(code.contains("pub items: Vec<Item>"));
        assert!(code.contains("pub capacity: usize"));
        assert!(code.contains("items: Vec::new()"));
        assert!(code.contains("capacity: 0"));
    }

    #[test]
    fn test_generate_with_array_type() {
        let fields = vec![("position".to_string(), "[f32; 3]".to_string())];

        let code = generate_component_code("Transform", &fields, Some("Default".to_string()), None);

        assert!(code.contains("pub position: [f32; 3]"));
        assert!(code.contains("position: [0.0, 0.0, 0.0]"));
    }

    #[test]
    fn test_generate_derives_deduplication() {
        let fields = vec![("value".to_string(), "i32".to_string())];

        let code = generate_component_code(
            "Counter",
            &fields,
            Some("Debug,Clone,Default".to_string()),
            None,
        );

        // Debug and Clone should not be duplicated
        assert!(code.contains("#[derive("));
        let derive_count = code.matches("Debug").count();
        assert_eq!(derive_count, 1); // Only one Debug in derives
    }
}
