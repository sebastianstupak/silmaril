//! Component code generation.
//!
//! Extracted from `engine/cli/src/codegen/component.rs`.

use anyhow::{bail, Result};

use super::validate_snake_case;

/// Parse field specifications from command-line format.
///
/// # Format
/// `name:type[,name:type]*`
///
/// # Examples
/// - `"current:f32,max:f32"` -> `[(current, f32), (max, f32)]`
/// - `"position:[f32;3]"` -> `[(position, [f32; 3])]`
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

            Ok((name, type_str))
        })
        .collect()
}

/// Extract array type and size from array syntax.
///
/// # Examples
/// - `"[f32; 3]"` -> `Some(("f32", 3))`
/// - `"Vec<Item>"` -> `None`
pub fn extract_array_type(s: &str) -> Option<(String, usize)> {
    let s = s.trim();

    if !s.starts_with('[') || !s.ends_with(']') {
        return None;
    }

    let inner = &s[1..s.len() - 1];
    let parts: Vec<&str> = inner.split(';').collect();
    if parts.len() != 2 {
        return None;
    }

    let element_type = parts[0].trim().to_string();
    let size = parts[1].trim().parse::<usize>().ok()?;

    Some((element_type, size))
}

/// Convert PascalCase to snake_case.
///
/// # Examples
/// - "Health" -> "health"
/// - "PlayerState" -> "player_state"
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

/// Generate default value for a given Rust type.
pub fn default_value_for_type(type_str: &str) -> String {
    match type_str {
        "f32" | "f64" => "0.0".to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "isize" | "usize" => "0".to_string(),
        "bool" => "false".to_string(),
        "String" => "String::new()".to_string(),
        s if s.starts_with("Vec<") => "Vec::new()".to_string(),
        s if s.starts_with("Option<") => "None".to_string(),
        s if s.starts_with('[') && s.contains(';') => {
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

/// Parse array type to extract inner type and size.
fn parse_array_type(type_str: &str) -> Option<(String, usize)> {
    let s = type_str.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return None;
    }
    let inner = s[1..s.len() - 1].trim();
    let parts: Vec<&str> = inner.splitn(2, ';').collect();
    if parts.len() != 2 {
        return None;
    }
    let inner_type = parts[0].trim().to_string();
    let size = parts[1].trim().parse::<usize>().ok()?;
    Some((inner_type, size))
}

/// Generate the standard imports for component code.
pub fn component_imports() -> &'static str {
    "use engine_core::ecs::Component;\nuse serde::{Deserialize, Serialize};\n"
}

/// Generate complete component code (with imports).
pub fn generate_component_code(name: &str, fields: &[(String, String)]) -> String {
    generate_component_code_inner(name, fields, true)
}

/// Generate component code with optional imports.
pub fn generate_component_code_inner(
    name: &str,
    fields: &[(String, String)],
    include_imports: bool,
) -> String {
    let snake_name = to_snake_case(name);

    let derives_str = "Component, Debug, Clone, PartialEq, Serialize, Deserialize";

    let mut fields_code = String::new();
    for (field_name, field_type) in fields {
        fields_code.push_str(&format!(
            "    /// TODO: Document this field\n    pub {}: {},\n",
            field_name, field_type
        ));
    }

    let test_module = generate_test_module(name, &snake_name, fields);

    let imports = if include_imports {
        format!("{}\n", component_imports())
    } else {
        String::new()
    };
    format!(
        "{imports}#[derive({derives})]\npub struct {name} {{\n{fields}}}\n\n{tests}",
        imports = imports,
        derives = derives_str,
        name = name,
        fields = fields_code,
        tests = test_module
    )
}

/// Generate test module for component.
fn generate_test_module(
    name: &str,
    snake_name: &str,
    fields: &[(String, String)],
) -> String {
    let mut field_inits = String::new();
    for (field_name, field_type) in fields {
        let default_val = default_value_for_type(field_type);
        field_inits.push_str(&format!("\n            {}: {},", field_name, default_val));
    }
    let creation = format!(
        "        let component = {} {{{}\n        }};",
        name, field_inits
    );

    format!(
        r#"#[cfg(test)]
mod {snake_name}_tests {{
    use super::*;
    use engine_core::ecs::World;

    #[test]
    fn test_{snake_name}_add_get() {{
        let mut world = World::new();
        world.register::<{name}>();
        let entity = world.spawn();
        let component = {name} {{{field_inits}
        }};
        world.add(entity, component);
        assert!(world.get::<{name}>(entity).is_some());
        let retrieved = world.get::<{name}>(entity).unwrap();
        let _ = retrieved;
    }}

    #[test]
    fn test_{snake_name}_serialization() {{
{creation}
        let json = serde_json::to_string(&component).unwrap();
        let _deserialized: {name} = serde_json::from_str(&json).unwrap();
    }}

    #[test]
    fn test_{snake_name}_remove() {{
        let mut world = World::new();
        world.register::<{name}>();
        let entity = world.spawn();
        let component = {name} {{{field_inits}
        }};
        world.add(entity, component);
        assert!(world.get::<{name}>(entity).is_some());
        world.remove::<{name}>(entity);
        assert!(world.get::<{name}>(entity).is_none());
    }}
}}
"#,
        snake_name = snake_name,
        name = name,
        field_inits = field_inits,
        creation = creation,
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
    fn test_default_value_for_type_primitives() {
        assert_eq!(default_value_for_type("f32"), "0.0");
        assert_eq!(default_value_for_type("i32"), "0");
        assert_eq!(default_value_for_type("bool"), "false");
    }

    #[test]
    fn test_default_value_for_type_collections() {
        assert_eq!(default_value_for_type("String"), "String::new()");
        assert_eq!(default_value_for_type("Vec<Item>"), "Vec::new()");
        assert_eq!(default_value_for_type("Option<String>"), "None");
    }

    #[test]
    fn test_default_value_for_type_array() {
        assert_eq!(default_value_for_type("[f32; 3]"), "[0.0, 0.0, 0.0]");
    }

    #[test]
    fn test_generate_simple_component() {
        let fields = vec![
            ("current".to_string(), "f32".to_string()),
            ("max".to_string(), "f32".to_string()),
        ];

        let code = generate_component_code("Health", &fields);

        assert!(code.contains("pub struct Health"));
        assert!(code.contains("pub current: f32"));
        assert!(code.contains("pub max: f32"));
        assert!(code.contains("#[cfg(test)]"));
        assert!(code.contains("test_health_add_get"));
    }

    #[test]
    fn test_generate_derives_fixed() {
        let fields = vec![("value".to_string(), "i32".to_string())];
        let code = generate_component_code("Counter", &fields);
        assert!(code.contains(
            "#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]"
        ));
    }

    #[test]
    fn test_extract_array_type_valid() {
        assert_eq!(
            extract_array_type("[f32; 3]"),
            Some(("f32".to_string(), 3))
        );
    }

    #[test]
    fn test_extract_array_type_invalid() {
        assert_eq!(extract_array_type("Vec<Item>"), None);
    }

    #[test]
    fn test_parse_fields_valid() {
        let fields = parse_fields("current:f32,max:f32").unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("current".to_string(), "f32".to_string()));
    }

    #[test]
    fn test_parse_fields_invalid() {
        assert!(parse_fields("bad_format").is_err());
    }
}
