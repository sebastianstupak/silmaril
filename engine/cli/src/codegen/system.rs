use super::parser::QueryComponent;

/// Generate complete system code.
///
/// When `include_imports` is false, the top-level `use engine_core::ecs::World;` is omitted
/// (for appending to files that already have it).
#[allow(dead_code)]
pub fn generate_system_code(name: &str, components: &[QueryComponent]) -> String {
    generate_system_code_inner(name, components, true)
}

/// Generate system code with optional imports.
pub fn generate_system_code_inner(
    name: &str,
    components: &[QueryComponent],
    include_imports: bool,
) -> String {
    let fn_name = format!("{}_system", name);
    let test_mod_name = format!("{}_system_tests", name);

    let mut code = String::new();

    // Top-level import (only if this is the first system in the file)
    if include_imports {
        code.push_str("use engine_core::ecs::World;\n");
        code.push('\n');
    }

    // Registration comment + tracing attribute
    code.push_str(&format!(
        "// To register: app.add_system({});\n",
        fn_name
    ));
    code.push_str("#[tracing::instrument(skip(world))]\n");

    // Function signature
    code.push_str(&format!(
        "pub fn {}(world: &mut World, dt: f32) {{\n",
        fn_name
    ));

    // Query + iteration
    if !components.is_empty() {
        let has_mutable = components
            .iter()
            .any(|c| c.access == super::parser::QueryAccess::Mutable);

        // Build query type: single component uses bare type, multi uses tuple
        let query_types: Vec<String> = components.iter().map(|c| c.type_syntax()).collect();
        let query_type = if components.len() == 1 {
            query_types[0].clone()
        } else {
            format!("({})", query_types.join(", "))
        };

        // Build iter binding: iteration yields (Entity, data)
        let var_names: Vec<String> = components.iter().map(|c| c.var_name()).collect();
        let data_binding = if components.len() == 1 {
            var_names[0].clone()
        } else {
            format!("({})", var_names.join(", "))
        };

        let query_method = if has_mutable { "query_mut" } else { "query" };

        code.push_str(&format!(
            "    for (_entity, {}) in world.{}::<{}>() {{\n",
            data_binding, query_method, query_type
        ));
        code.push_str(&format!("        // TODO: implement {} logic\n", name));
        code.push_str("        let _ = dt;\n");
        code.push_str("    }\n");
    } else {
        code.push_str(&format!("    // TODO: implement {} logic\n", name));
        code.push_str("    let _ = dt;\n");
    }

    code.push_str("}\n\n");

    // Test module
    code.push_str("#[cfg(test)]\n");
    code.push_str(&format!("mod {} {{\n", test_mod_name));
    code.push_str("    use super::*;\n");
    code.push_str("    use engine_core::ecs::World;\n\n");

    code.push_str("    #[test]\n");
    code.push_str(&format!("    fn test_{}() {{\n", fn_name));
    code.push_str("        let mut world = World::new();\n");
    code.push_str("        // TODO: spawn test entity, run system, assert\n");
    code.push_str(&format!("        {}(&mut world, 0.016);\n", fn_name));
    code.push_str("    }\n");

    code.push_str("}\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::parser::{QueryAccess, QueryComponent};

    #[test]
    fn test_function_name_has_system_suffix() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(code.contains("pub fn health_regen_system("));
        assert!(!code.contains("pub fn health_regen("));
    }

    #[test]
    fn test_parameter_name_is_dt() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(code.contains("dt: f32"));
        assert!(!code.contains("delta_time"));
    }

    #[test]
    fn test_no_crate_components_import() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(!code.contains("use crate::components"));
    }

    #[test]
    fn test_direct_query_iteration() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(code.contains("for (_entity, health) in world.query_mut::<&mut Health>()"));
    }

    #[test]
    fn test_test_module_name_has_system_suffix() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(code.contains("mod health_regen_system_tests {"));
    }

    #[test]
    fn test_registration_comment() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        assert!(code.contains("// To register: app.add_system(health_regen_system)"));
    }

    #[test]
    fn test_generate_simple_system() {
        let components = vec![
            QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
            QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable),
        ];

        let code = generate_system_code("health_regen", &components);

        assert!(code.contains("pub fn health_regen_system"));
        assert!(code.contains("use engine_core::ecs::World"));
        assert!(code.contains("#[tracing::instrument(skip(world))]"));
        assert!(code.contains("world.query_mut::<(&mut Health, &RegenerationRate)>()"));
        assert!(code.contains("for (_entity, (health, regeneration_rate))"));
        assert!(code.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_generate_system_empty_components() {
        let code = generate_system_code("global_update", &[]);

        assert!(code.contains("pub fn global_update_system"));
        assert!(!code.contains("world.query"));
        assert!(code.contains("// TODO: implement global_update logic"));
    }

    #[test]
    fn test_single_component_bare_type() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_check", &components);
        assert!(code.contains("query_mut::<&mut Health>()"));
        assert!(code.contains("(_entity, health)"));
    }

    #[test]
    fn test_multiple_components_tuple() {
        let components = vec![
            QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
            QueryComponent::new("Velocity".to_string(), QueryAccess::Immutable),
        ];
        let code = generate_system_code("movement", &components);
        assert!(code.contains("(&mut Health, &Velocity)"));
        assert!(code.contains("(_entity, (health, velocity))"));
    }

    #[test]
    fn test_tracing_instrument_before_fn() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        let instrument_pos = code.find("#[tracing::instrument(skip(world))]").unwrap();
        let fn_pos = code.find("pub fn health_regen_system(").unwrap();
        assert!(instrument_pos < fn_pos);
    }

    #[test]
    fn test_registration_comment_before_instrument() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Mutable)];
        let code = generate_system_code("health_regen", &components);
        let comment_pos = code.find("// To register:").unwrap();
        let instrument_pos = code.find("#[tracing::instrument").unwrap();
        assert!(comment_pos < instrument_pos);
    }
}
