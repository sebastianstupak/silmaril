//! Component and system code generation.
//!
//! Extracted from `engine/cli/src/codegen/` and `engine/cli/src/commands/add/`.
//!
//! # Modules
//!
//! - [`component`] — Component struct generation
//! - [`system`] — System function generation
//! - [`wiring`] — Module wiring block generation/detection/removal

pub mod component;
pub mod system;
pub mod wiring;

// Re-export commonly used items for convenience.
pub use component::{
    default_value_for_type, extract_array_type, generate_component_code,
    generate_component_code_inner, parse_fields, to_snake_case,
};
pub use system::{generate_system_code, generate_system_code_inner};
pub use wiring::{
    generate_wiring_block, has_wiring_block, parse_module_metadata, remove_wiring_block,
    ModuleMetadata,
};

// Validation helpers re-exported at codegen level.
pub use component::component_imports;

/// Validate that a name is in PascalCase (component names).
pub fn validate_pascal_case(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("Component name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_uppercase() {
        anyhow::bail!("Component name must start with uppercase: '{}'", name);
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        anyhow::bail!("Component name must be alphanumeric (ASCII only): '{}'", name);
    }

    Ok(())
}

/// Validate that a name is in snake_case (system names, field names).
pub fn validate_snake_case(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("Name cannot be empty");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_lowercase() && first_char != '_' {
        anyhow::bail!("Name must start with lowercase or underscore: '{}'", name);
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        anyhow::bail!("Name must be alphanumeric (ASCII only) or underscore: '{}'", name);
    }

    Ok(())
}

/// Query access mode for components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryAccess {
    Immutable,
    Mutable,
}

/// A component in a query with its access mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryComponent {
    pub name: String,
    pub access: QueryAccess,
}

impl QueryComponent {
    pub fn new(name: String, access: QueryAccess) -> Self {
        Self { name, access }
    }

    /// Returns the Rust type syntax for this query component.
    pub fn type_syntax(&self) -> String {
        match self.access {
            QueryAccess::Immutable => format!("&{}", self.name),
            QueryAccess::Mutable => format!("&mut {}", self.name),
        }
    }

    /// Returns the variable name for this query component in a query tuple.
    pub fn var_name(&self) -> String {
        to_snake_case(&self.name)
    }
}

/// Parse query components from a comma-separated string.
///
/// Syntax: `ComponentName` (immutable) or `mut:ComponentName` (mutable).
pub fn parse_query_components(input: &str) -> anyhow::Result<Vec<QueryComponent>> {
    use anyhow::bail;

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

// ---------------------------------------------------------------------------
// Orchestrator functions
// ---------------------------------------------------------------------------

use crate::project::{self, Target};

/// Add a component to a Silmaril game project.
///
/// Generates the component code, appends it to the domain file, and wires
/// the module declaration into the entry file. On wiring failure the domain
/// file is rolled back.
pub fn add_component(
    name: &str,
    fields_str: &str,
    target: Target,
    domain: &str,
) -> anyhow::Result<()> {
    use anyhow::bail;

    validate_pascal_case(name)?;

    let fields = parse_fields(fields_str)?;
    if fields.is_empty() {
        bail!("Component must have at least one field");
    }

    let cwd = std::env::current_dir()?;
    let project_root = project::find_project_root(&cwd)?;
    let crate_root = project::crate_dir(&project_root, target)?;
    let domain_mod = project::domain_file(&crate_root, domain);
    let wiring = project::wiring_target(&crate_root, target);

    if project::has_duplicate_component(&domain_mod, name)? {
        bail!(
            "component '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    let include_imports = !domain_mod.exists();
    let code = generate_component_code_inner(name, &fields, include_imports);

    // Step 1: Append to domain file (atomic)
    let original_domain = project::append_to_domain_file(&domain_mod, &code)?;

    // Step 2: Wire module declaration (atomic) — rollback domain if this fails
    match project::wire_module_declaration(&wiring, domain) {
        Ok(_) => {}
        Err(e) => {
            project::rollback_domain_file(&domain_mod, original_domain)?;
            return Err(e);
        }
    }

    tracing::info!(
        "[ops] {} {}/src/{}/mod.rs",
        if original_domain.is_none() { "created" } else { "updated" },
        target.crate_subdir(),
        domain
    );
    tracing::info!(
        "[ops] wired: added `pub mod {};` to {}/src/{}",
        domain,
        target.crate_subdir(),
        target.entry_file()
    );

    Ok(())
}

/// Add a system to a Silmaril game project.
///
/// Generates the system code, appends it to the domain file, and wires
/// the module declaration into the entry file. On wiring failure the domain
/// file is rolled back.
pub fn add_system(
    name: &str,
    query_str: &str,
    target: Target,
    domain: &str,
) -> anyhow::Result<()> {
    use anyhow::bail;

    validate_snake_case(name)?;

    let components = parse_query_components(query_str)?;
    if components.is_empty() {
        bail!("Query must have at least one component");
    }

    let cwd = std::env::current_dir()?;
    let project_root = project::find_project_root(&cwd)?;
    let crate_root = project::crate_dir(&project_root, target)?;
    let domain_mod = project::domain_file(&crate_root, domain);
    let wiring = project::wiring_target(&crate_root, target);

    if project::has_duplicate_system(&domain_mod, name)? {
        bail!(
            "system '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    let file_has_world_import = domain_mod.exists() && {
        let content = std::fs::read_to_string(&domain_mod).unwrap_or_default();
        content.lines().any(|line| line.starts_with("use engine_core::ecs::World;"))
    };
    let code = generate_system_code_inner(name, &components, !file_has_world_import);

    let original_domain = project::append_to_domain_file(&domain_mod, &code)?;

    match project::wire_module_declaration(&wiring, domain) {
        Ok(_) => {}
        Err(e) => {
            project::rollback_domain_file(&domain_mod, original_domain)?;
            return Err(e);
        }
    }

    tracing::info!(
        "[ops] {} {}/src/{}/mod.rs",
        if original_domain.is_none() { "created" } else { "updated" },
        target.crate_subdir(),
        domain
    );
    tracing::info!(
        "[ops] wired: added `pub mod {};` to {}/src/{}",
        domain,
        target.crate_subdir(),
        target.entry_file()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pascal_case() {
        assert!(validate_pascal_case("Health").is_ok());
        assert!(validate_pascal_case("PlayerState").is_ok());
    }

    #[test]
    fn test_invalid_pascal_case() {
        assert!(validate_pascal_case("health").is_err());
        assert!(validate_pascal_case("").is_err());
    }

    #[test]
    fn test_valid_snake_case() {
        assert!(validate_snake_case("health_regen").is_ok());
        assert!(validate_snake_case("_internal").is_ok());
    }

    #[test]
    fn test_invalid_snake_case() {
        assert!(validate_snake_case("HealthRegen").is_err());
        assert!(validate_snake_case("").is_err());
    }

    #[test]
    fn test_parse_query_components_basic() {
        let result = parse_query_components("Health,Velocity").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Health");
        assert_eq!(result[0].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_query_components_mutable() {
        let result = parse_query_components("mut:Health,RegenerationRate").unwrap();
        assert_eq!(result[0].access, QueryAccess::Mutable);
        assert_eq!(result[1].access, QueryAccess::Immutable);
    }

    #[test]
    fn test_parse_query_components_empty_rejected() {
        assert!(parse_query_components("").is_err());
    }

    #[test]
    fn test_query_component_type_syntax() {
        let imm = QueryComponent::new("Health".to_string(), QueryAccess::Immutable);
        assert_eq!(imm.type_syntax(), "&Health");
        let mutable = QueryComponent::new("Health".to_string(), QueryAccess::Mutable);
        assert_eq!(mutable.type_syntax(), "&mut Health");
    }
}
