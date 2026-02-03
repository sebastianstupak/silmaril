use super::parser::QueryComponent;

/// System execution phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPhase {
    Update,
    FixedUpdate,
    Render,
}

impl SystemPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemPhase::Update => "update",
            SystemPhase::FixedUpdate => "fixed_update",
            SystemPhase::Render => "render",
        }
    }
}

/// Generate complete system code
pub fn generate_system_code(
    name: &str,
    components: &[QueryComponent],
    phase: SystemPhase,
    doc: Option<String>,
) -> String {
    let mut code = String::new();

    // Imports
    code.push_str("use engine_core::ecs::{Query, World};\n");
    code.push_str("use tracing::{debug, instrument};\n");
    code.push('\n');

    // Component imports
    if !components.is_empty() {
        code.push_str("use crate::components::{");
        for (i, comp) in components.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&comp.name);
        }
        code.push_str("};\n\n");
    }

    // Documentation
    if let Some(doc_str) = doc {
        code.push_str(&format!("/// {}\n", doc_str));
    } else {
        code.push_str(&format!("/// System: {}\n", name));
    }
    code.push_str("///\n");
    code.push_str(&format!("/// # Phase\n"));
    code.push_str(&format!("/// {}\n", phase.as_str()));
    code.push_str("///\n");
    code.push_str("/// # Query\n");
    for comp in components {
        code.push_str(&format!("/// - {}\n", comp.type_syntax()));
    }

    // Function signature
    code.push_str("#[instrument(skip(world))]\n");
    code.push_str(&format!("pub fn {}(world: &mut World, delta_time: f32) {{\n", name));

    // Query construction
    if !components.is_empty() {
        code.push_str("    let query = world.query::<(");
        for (i, comp) in components.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&comp.type_syntax());
        }
        code.push_str(")>();\n\n");

        // Iteration
        code.push_str("    for (entity, (");
        for (i, comp) in components.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&comp.var_name());
        }
        code.push_str(")) in query.iter() {\n");
        code.push_str("        // TODO: Implement system logic\n\n");
        code.push_str("        debug!(?entity, \"Processing entity\");\n");
        code.push_str("    }\n");
    } else {
        code.push_str("    // TODO: Implement system logic\n");
    }

    code.push_str("}\n\n");

    // Tests module
    code.push_str("#[cfg(test)]\n");
    code.push_str("mod tests {\n");
    code.push_str("    use super::*;\n\n");

    // Test 1: Basic test
    code.push_str(&format!("    #[test]\n"));
    code.push_str(&format!("    fn test_{}_basic() {{\n", name));
    code.push_str("        let mut world = World::new();\n\n");
    code.push_str("        // TODO: Setup test entities\n");
    code.push_str("        let entity = world.spawn();\n");
    for comp in components {
        code.push_str(&format!("        // world.add(entity, {}::default());\n", comp.name));
    }
    code.push_str("\n");
    code.push_str(&format!("        {}(&mut world, 0.016);\n\n", name));
    code.push_str("        // TODO: Assert expected behavior\n");
    code.push_str("    }\n\n");

    // Test 2: Multiple entities
    code.push_str(&format!("    #[test]\n"));
    code.push_str(&format!("    fn test_{}_multiple_entities() {{\n", name));
    code.push_str("        let mut world = World::new();\n\n");
    code.push_str("        for _i in 0..10 {\n");
    code.push_str("            let entity = world.spawn();\n");
    for comp in components {
        code.push_str(&format!("            // world.add(entity, {}::default());\n", comp.name));
    }
    code.push_str("        }\n\n");
    code.push_str(&format!("        {}(&mut world, 0.016);\n\n", name));
    code.push_str("        // TODO: Verify all entities updated\n");
    code.push_str("    }\n\n");

    // Test 3: No matching entities
    code.push_str(&format!("    #[test]\n"));
    code.push_str(&format!("    fn test_{}_no_matching_entities() {{\n", name));
    code.push_str("        let mut world = World::new();\n\n");
    code.push_str("        // Should not crash with no entities\n");
    code.push_str(&format!("        {}(&mut world, 0.016);\n", name));
    code.push_str("    }\n");

    code.push_str("}\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::parser::{QueryAccess, QueryComponent};

    #[test]
    fn test_generate_simple_system() {
        let components = vec![
            QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
            QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable),
        ];

        let code = generate_system_code(
            "health_regen",
            &components,
            SystemPhase::Update,
            Some("Regenerate health over time".to_string()),
        );

        assert!(code.contains("pub fn health_regen"));
        assert!(code.contains("use engine_core::ecs::{Query, World}"));
        assert!(code.contains("use tracing::{debug, instrument}"));
        assert!(code.contains("use crate::components::{Health, RegenerationRate}"));
        assert!(code.contains("#[instrument(skip(world))]"));
        assert!(code.contains("Regenerate health over time"));
        assert!(code.contains("world.query::<(&mut Health, &RegenerationRate)>()"));
        assert!(code.contains("for (entity, (health, regeneration_rate))"));
        assert!(code.contains("#[cfg(test)]"));
        assert!(code.contains("fn test_health_regen_basic()"));
        assert!(code.contains("fn test_health_regen_multiple_entities()"));
        assert!(code.contains("fn test_health_regen_no_matching_entities()"));
    }

    #[test]
    fn test_generate_system_no_doc() {
        let components = vec![QueryComponent::new("Health".to_string(), QueryAccess::Immutable)];

        let code = generate_system_code("health_check", &components, SystemPhase::Update, None);

        assert!(code.contains("/// System: health_check"));
    }

    #[test]
    fn test_generate_system_fixed_update() {
        let components = vec![QueryComponent::new("Transform".to_string(), QueryAccess::Mutable)];

        let code =
            generate_system_code("physics_step", &components, SystemPhase::FixedUpdate, None);

        assert!(code.contains("/// fixed_update"));
    }

    #[test]
    fn test_generate_system_render_phase() {
        let components = vec![QueryComponent::new("Camera".to_string(), QueryAccess::Immutable)];

        let code = generate_system_code("camera_update", &components, SystemPhase::Render, None);

        assert!(code.contains("/// render"));
    }

    #[test]
    fn test_generate_system_empty_components() {
        let code = generate_system_code("global_update", &[], SystemPhase::Update, None);

        assert!(code.contains("pub fn global_update"));
        assert!(!code.contains("world.query"));
        assert!(code.contains("// TODO: Implement system logic"));
    }

    #[test]
    fn test_system_phase_as_str() {
        assert_eq!(SystemPhase::Update.as_str(), "update");
        assert_eq!(SystemPhase::FixedUpdate.as_str(), "fixed_update");
        assert_eq!(SystemPhase::Render.as_str(), "render");
    }
}
