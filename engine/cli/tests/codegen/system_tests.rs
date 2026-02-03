use silm::codegen::parser::{parse_query_components, to_snake_case, QueryAccess, QueryComponent};
use silm::codegen::system::{generate_system_code, SystemPhase};
use silm::codegen::validator::{validate_pascal_case, validate_snake_case};

// Query Parser Tests

#[test]
fn test_parse_single_immutable_component() {
    let result = parse_query_components("&Health").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[0].access, QueryAccess::Immutable);
}

#[test]
fn test_parse_single_mutable_component() {
    let result = parse_query_components("&mut Health").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[0].access, QueryAccess::Mutable);
}

#[test]
fn test_parse_multiple_immutable_components() {
    let result = parse_query_components("&Health,&Velocity,&Transform").unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|c| c.access == QueryAccess::Immutable));
}

#[test]
fn test_parse_mixed_access_components() {
    let result = parse_query_components("&mut Health,&RegenerationRate,&mut Transform").unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].access, QueryAccess::Mutable);
    assert_eq!(result[1].access, QueryAccess::Immutable);
    assert_eq!(result[2].access, QueryAccess::Mutable);
}

#[test]
fn test_parse_with_whitespace() {
    let result = parse_query_components("  &mut Health  ,  &RegenerationRate  ").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Health");
    assert_eq!(result[1].name, "RegenerationRate");
}

#[test]
fn test_parse_invalid_missing_ampersand() {
    let result = parse_query_components("Health");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_lowercase_component() {
    let result = parse_query_components("&health");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_empty_string() {
    let result = parse_query_components("");
    assert!(result.is_err());
}

#[test]
fn test_parse_component_with_numbers() {
    let result = parse_query_components("&Camera2D,&Transform3D").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_query_component_type_syntax() {
    let comp = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
    assert_eq!(comp.type_syntax(), "&Health");
    
    let comp = QueryComponent::new("Health".to_string(), QueryAccess::Mutable);
    assert_eq!(comp.type_syntax(), "&mut Health");
}

#[test]
fn test_to_snake_case() {
    assert_eq!(to_snake_case("Health"), "health");
    assert_eq!(to_snake_case("RegenerationRate"), "regeneration_rate");
    assert_eq!(to_snake_case("Camera2D"), "camera2_d");
}

// System Code Generator Tests

#[test]
fn test_generate_system_basic_structure() {
    let components = vec![
        QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
    ];
    
    let code = generate_system_code("health_regen", &components, SystemPhase::Update, None);
    
    assert!(code.contains("use engine_core::ecs::{Query, World}"));
    assert!(code.contains("use tracing::{debug, instrument}"));
    assert!(code.contains("pub fn health_regen(world: &mut World, delta_time: f32)"));
}

#[test]
fn test_generate_system_with_query() {
    let components = vec![
        QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
        QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable),
    ];
    
    let code = generate_system_code("health_regen", &components, SystemPhase::Update, None);
    
    assert!(code.contains("world.query::<(&mut Health, &RegenerationRate)>()"));
    assert!(code.contains("for (entity, (health, regeneration_rate))"));
}

#[test]
fn test_generate_system_with_tests() {
    let components = vec![
        QueryComponent::new("Health".to_string(), QueryAccess::Mutable),
    ];
    
    let code = generate_system_code("health_regen", &components, SystemPhase::Update, None);
    
    assert!(code.contains("#[cfg(test)]"));
    assert!(code.contains("fn test_health_regen_basic()"));
    assert!(code.contains("fn test_health_regen_multiple_entities()"));
    assert!(code.contains("fn test_health_regen_no_matching_entities()"));
}

#[test]
fn test_validate_snake_case() {
    assert!(validate_snake_case("health_regen").is_ok());
    assert!(validate_snake_case("movement").is_ok());
    assert!(validate_snake_case("HealthRegen").is_err());
}

#[test]
fn test_validate_pascal_case() {
    assert!(validate_pascal_case("Health").is_ok());
    assert!(validate_pascal_case("RegenerationRate").is_ok());
    assert!(validate_pascal_case("health").is_err());
}

#[test]
fn test_system_phase() {
    assert_eq!(SystemPhase::Update.as_str(), "update");
    assert_eq!(SystemPhase::FixedUpdate.as_str(), "fixed_update");
    assert_eq!(SystemPhase::Render.as_str(), "render");
}
