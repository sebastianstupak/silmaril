use anyhow::{bail, Result};

use super::validator::validate_pascal_case;

// Allow dead code for QueryComponent::new which is part of the public API
#[allow(dead_code)]

/// Query access mode for components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryAccess {
    Immutable,
    Mutable,
}

/// A component in a query with its access mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryComponent {
    pub name: String,
    pub access: QueryAccess,
}

impl QueryComponent {
    #[allow(dead_code)]
    pub fn new(name: String, access: QueryAccess) -> Self {
        Self { name, access }
    }

    /// Returns the Rust type syntax for this query component
    pub fn type_syntax(&self) -> String {
        match self.access {
            QueryAccess::Immutable => format!("&{}", self.name),
            QueryAccess::Mutable => format!("&mut {}", self.name),
        }
    }

    /// Returns the variable name for this query component in a query tuple
    pub fn var_name(&self) -> String {
        to_snake_case(&self.name)
    }
}

/// Parse query components from a string
pub fn parse_query_components(input: &str) -> Result<Vec<QueryComponent>> {
    if input.trim().is_empty() {
        bail!("Query string cannot be empty");
    }

    input
        .split(',')
        .map(|comp| {
            let comp = comp.trim();

            if comp.is_empty() {
                bail!("Empty component in query");
            }

            let (access, name) = if let Some(stripped) = comp.strip_prefix("&mut ") {
                (QueryAccess::Mutable, stripped.trim())
            } else if let Some(stripped) = comp.strip_prefix("&mut") {
                (QueryAccess::Mutable, stripped.trim())
            } else if let Some(stripped) = comp.strip_prefix("& ") {
                (QueryAccess::Immutable, stripped.trim())
            } else if let Some(stripped) = comp.strip_prefix("&") {
                (QueryAccess::Immutable, stripped.trim())
            } else {
                bail!("Query component must start with '&' or '&mut': '{}'", comp);
            };

            if name.is_empty() {
                bail!("Component name cannot be empty in query component: '{}'", comp);
            }

            let name = name.to_string();
            validate_pascal_case(&name)?;

            Ok(QueryComponent { name, access })
        })
        .collect()
}

/// Convert PascalCase to snake_case
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_multiple_mutable() {
        let result = parse_query_components("&mut Transform,&mut Velocity,&Mass").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].access, QueryAccess::Mutable);
        assert_eq!(result[2].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_missing_ampersand() {
        let result = parse_query_components("Health");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse_query_components("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_query_components("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let result = parse_query_components("  &mut Health  ,  &RegenerationRate  ").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[1].name, "RegenerationRate");
    }

    #[test]
    fn test_parse_no_space_after_mut() {
        let result = parse_query_components("&mutHealth").unwrap();
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Mutable);
    }

    #[test]
    fn test_parse_invalid_component_name() {
        let result = parse_query_components("&health");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_component_with_number() {
        let result = parse_query_components("&Camera2D,&Transform3D").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Camera2D");
        assert_eq!(result[1].name, "Transform3D");
    }

    #[test]
    fn test_parse_single_component() {
        let result = parse_query_components("&Health").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Health");
    }

    #[test]
    fn test_parse_empty_component() {
        let result = parse_query_components("&Health,,&Velocity");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_only_ampersand() {
        let result = parse_query_components("&");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_only_mut() {
        let result = parse_query_components("&mut");
        assert!(result.is_err());
    }

    #[test]
    fn test_query_component_type_syntax() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.type_syntax(), "&Health");

        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Mutable);
        assert_eq!(comp.type_syntax(), "&mut Health");
    }

    #[test]
    fn test_query_component_var_name() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.var_name(), "health");

        let comp = QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.var_name(), "regeneration_rate");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Health"), "health");
        assert_eq!(to_snake_case("RegenerationRate"), "regeneration_rate");
        assert_eq!(to_snake_case("MeshRenderer2D"), "mesh_renderer2_d");
        assert_eq!(to_snake_case("Transform"), "transform");
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_to_snake_case_already_lowercase() {
        assert_eq!(to_snake_case("health"), "health");
    }
}
