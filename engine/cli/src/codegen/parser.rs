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
        .map(|token| {
            let token = token.trim();

            if token.is_empty() {
                bail!("Empty component in query");
            }

            // Reject old &mut / & syntax with a helpful message
            if token.starts_with('&') {
                bail!(
                    "use 'mut:ComponentName' syntax, not '&mut ComponentName' or '&ComponentName': '{}'",
                    token
                );
            }

            let (access, name) = if let Some(rest) = token.strip_prefix("mut:") {
                (QueryAccess::Mutable, rest.trim())
            } else {
                (QueryAccess::Immutable, token)
            };

            if name.is_empty() {
                bail!("Component name cannot be empty after 'mut:'");
            }

            // Must be PascalCase (starts with uppercase)
            if !name.starts_with(|c: char| c.is_uppercase()) {
                bail!(
                    "invalid query token '{}': expected 'ComponentName' or 'mut:ComponentName'",
                    token
                );
            }

            validate_pascal_case(name)?;

            Ok(QueryComponent { name: name.to_string(), access })
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
    fn test_parse_bare_immutable() {
        let result = parse_query_components("Health,Velocity").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
        assert_eq!(result[1].name, "Velocity");
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_mut_prefix() {
        let result = parse_query_components("mut:Health,RegenerationRate").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].name, "RegenerationRate");
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_multiple_mutable() {
        let result = parse_query_components("mut:Health,mut:Velocity,Mass").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].access, QueryAccess::Mutable);
        assert_eq!(result[2].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_single_component() {
        let result = parse_query_components("Health").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_whitespace_trimmed() {
        let result = parse_query_components("  mut:Health  ,  RegenerationRate  ").unwrap();
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[1].name, "RegenerationRate");
    }

    #[test]
    fn test_old_ampersand_syntax_rejected() {
        let result = parse_query_components("&mut Health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("use 'mut:ComponentName' syntax"), "got: {msg}");
    }

    #[test]
    fn test_old_ampersand_immutable_rejected() {
        let result = parse_query_components("&Health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("use 'mut:ComponentName' syntax"), "got: {msg}");
    }

    #[test]
    fn test_lowercase_component_rejected() {
        let result = parse_query_components("health");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid query token"), "got: {msg}");
    }

    #[test]
    fn test_empty_string_rejected() {
        assert!(parse_query_components("").is_err());
    }

    #[test]
    fn test_empty_component_rejected() {
        assert!(parse_query_components("Health,,Velocity").is_err());
    }

    #[test]
    fn test_numbers_in_name_ok() {
        let result = parse_query_components("Camera2D,Transform3D").unwrap();
        assert_eq!(result[0].name, "Camera2D");
        assert_eq!(result[1].name, "Transform3D");
    }

    #[test]
    fn test_type_syntax_immutable() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.type_syntax(), "&Health");
    }

    #[test]
    fn test_type_syntax_mutable() {
        let comp = QueryComponent::new("Health".to_string(), QueryAccess::Mutable);
        assert_eq!(comp.type_syntax(), "&mut Health");
    }

    #[test]
    fn test_var_name() {
        let comp = QueryComponent::new("RegenerationRate".to_string(), QueryAccess::Immutable);
        assert_eq!(comp.var_name(), "regeneration_rate");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Health"), "health");
        assert_eq!(to_snake_case("RegenerationRate"), "regeneration_rate");
        assert_eq!(to_snake_case("Transform"), "transform");
    }
}
