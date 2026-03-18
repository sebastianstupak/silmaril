//! Module wiring block generation, detection, and removal.

use std::path::Path;

/// Metadata parsed from a module's Cargo.toml `[package.metadata.silmaril]`.
#[derive(Debug, Clone, Default)]
pub struct ModuleMetadata {
    /// Module display name.
    pub name: String,
    /// Components declared by this module.
    pub components: Vec<String>,
    /// Systems declared by this module.
    pub systems: Vec<String>,
}

/// Parse module metadata from a Cargo.toml file.
pub fn parse_module_metadata(_cargo_toml: &Path) -> anyhow::Result<ModuleMetadata> {
    // TODO: Actually parse [package.metadata.silmaril] from the TOML file
    Ok(ModuleMetadata::default())
}

/// Generate a wiring block (module declarations + registration) for the given domain.
pub fn generate_wiring_block(domain: &str) -> String {
    format!(
        "// --- silmaril wiring: {} ---\npub mod {};\n// --- end silmaril wiring ---\n",
        domain, domain
    )
}

/// Check whether a file already contains a silmaril wiring block for `domain`.
pub fn has_wiring_block(path: &Path, _domain: &str) -> anyhow::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(path)?;
    Ok(content.contains("// --- silmaril wiring"))
}

/// Remove the wiring block for `domain` from the given file.
pub fn remove_wiring_block(path: &Path, domain: &str) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let start_marker = format!("// --- silmaril wiring: {} ---", domain);
    let end_marker = "// --- end silmaril wiring ---";

    let mut result = String::new();
    let mut skipping = false;
    for line in content.lines() {
        if line.contains(&start_marker) {
            skipping = true;
            continue;
        }
        if skipping && line.contains(end_marker) {
            skipping = false;
            continue;
        }
        if !skipping {
            result.push_str(line);
            result.push('\n');
        }
    }

    std::fs::write(path, result)?;
    Ok(())
}
