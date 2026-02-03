use anyhow::{bail, Result};

/// Validates that a name is in PascalCase (component names)
pub fn validate_pascal_case(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Component name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_uppercase() {
        bail!("Component name must start with uppercase: '{}'", name);
    }

    if !name.chars().all(|c| c.is_alphanumeric()) {
        bail!("Component name must be alphanumeric: '{}'", name);
    }

    Ok(())
}

/// Validates that a name is in snake_case (system names, field names)
pub fn validate_snake_case(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_lowercase() && first_char != '_' {
        bail!("Name must start with lowercase or underscore: '{}'", name);
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        bail!("Name must be alphanumeric or underscore: '{}'", name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pascal_case() {
        assert!(validate_pascal_case("Health").is_ok());
        assert!(validate_pascal_case("PlayerState").is_ok());
        assert!(validate_pascal_case("MeshRenderer2D").is_ok());
    }

    #[test]
    fn test_invalid_pascal_case() {
        assert!(validate_pascal_case("health").is_err()); // lowercase
        assert!(validate_pascal_case("player-state").is_err()); // hyphen
        assert!(validate_pascal_case("123Health").is_err()); // starts with number
        assert!(validate_pascal_case("").is_err()); // empty
    }

    #[test]
    fn test_valid_snake_case() {
        assert!(validate_snake_case("health_regen").is_ok());
        assert!(validate_snake_case("movement").is_ok());
        assert!(validate_snake_case("_internal").is_ok());
    }

    #[test]
    fn test_invalid_snake_case() {
        assert!(validate_snake_case("HealthRegen").is_err()); // PascalCase
        assert!(validate_snake_case("health-regen").is_err()); // hyphen
        assert!(validate_snake_case("").is_err()); // empty
    }
}
