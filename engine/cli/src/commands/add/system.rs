use anyhow::{bail, Result};
use std::env;

use crate::codegen::{generate_system_code_inner, parse_query_components, validate_snake_case};

use super::wiring::{
    append_to_domain_file, crate_dir, domain_file, find_project_root, has_duplicate_system,
    rollback_domain_file, wire_module_declaration, wiring_target, Target,
};

pub fn add_system(
    name: &str,
    query_str: &str,
    target: Target,
    domain: &str,
) -> Result<()> {
    validate_snake_case(name)?;

    let components = parse_query_components(query_str)?;
    if components.is_empty() {
        bail!("Query must have at least one component");
    }

    // Find project root and resolve paths
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;
    let crate_root = crate_dir(&project_root, target)?;
    let domain_mod = domain_file(&crate_root, domain);
    let wiring = wiring_target(&crate_root, target);

    // Check for duplicate before writing
    if has_duplicate_system(&domain_mod, name)? {
        bail!(
            "system '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    // Generate code — skip imports if domain file already has a top-level World import
    // (check for line starting with "use engine_core::ecs::World;" to avoid matching test-module imports)
    let file_has_world_import = domain_mod.exists() && {
        let content = std::fs::read_to_string(&domain_mod).unwrap_or_default();
        content.lines().any(|line| line.starts_with("use engine_core::ecs::World;"))
    };
    let code = generate_system_code_inner(name, &components, !file_has_world_import);

    // Step 1: Append to domain file (atomic)
    let original_domain = append_to_domain_file(&domain_mod, &code)?;

    // Step 2: Wire module declaration (atomic) — rollback domain if this fails
    match wire_module_declaration(&wiring, domain) {
        Ok(_) => {}
        Err(e) => {
            rollback_domain_file(&domain_mod, original_domain)?;
            return Err(e);
        }
    }

    // Success output
    tracing::info!(
        "[silm] {} {}/src/{}/mod.rs",
        if original_domain.is_none() { "created" } else { "updated" },
        target.crate_subdir(),
        domain
    );
    tracing::info!(
        "[silm] wired: added `pub mod {};` to {}/src/{}",
        domain,
        target.crate_subdir(),
        target.entry_file()
    );

    Ok(())
}
