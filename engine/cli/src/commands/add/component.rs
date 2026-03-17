use anyhow::{bail, Result};
use std::env;

use crate::codegen::{generate_component_code, parse_fields, to_snake_case, validate_pascal_case};

use super::wiring::{
    append_to_domain_file, crate_dir, domain_file, find_project_root, has_duplicate_component,
    rollback_domain_file, wire_module_declaration, wiring_target, Target,
};

pub fn add_component(
    name: &str,
    fields_str: &str,
    target: Target,
    domain: &str,
) -> Result<()> {
    // Validate inputs
    validate_pascal_case(name)?;

    let fields = parse_fields(fields_str)?;
    if fields.is_empty() {
        bail!("Component must have at least one field");
    }

    // Find project root and resolve paths
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;
    let crate_root = crate_dir(&project_root, target)?;
    let domain_mod = domain_file(&crate_root, domain);
    let wiring = wiring_target(&crate_root, target);

    // Check for duplicate before writing
    if has_duplicate_component(&domain_mod, name)? {
        bail!(
            "component '{}' already exists in {}",
            name,
            domain_mod.display()
        );
    }

    // Generate code
    let _snake_name = to_snake_case(name);
    let code = generate_component_code(name, &fields);

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
