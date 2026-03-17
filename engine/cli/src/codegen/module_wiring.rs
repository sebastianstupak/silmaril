// Allow dead code for now - these functions are part of the module wiring API
// and will be used when module add/remove commands are implemented
#![allow(dead_code)]

use serde::Deserialize;

/// Metadata declared under `[package.metadata.silmaril]` in a module's Cargo.toml.
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub module_type: String,
    pub target: String,
    pub init: String,
}

#[derive(Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    metadata: Option<CargoMetadata>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    silmaril: Option<SilmarilMeta>,
}

#[derive(Deserialize)]
struct SilmarilMeta {
    module_type: String,
    target: String,
    init: String,
}

/// Convert snake_case module name to the conventional Rust type name.
///
/// # Examples
/// ```
/// use silm::codegen::module_wiring::module_type_from_name;
/// assert_eq!(module_type_from_name("combat"), "CombatModule");
/// assert_eq!(module_type_from_name("health_regen"), "HealthRegenModule");
/// ```
pub fn module_type_from_name(name: &str) -> String {
    let pascal: String = name
        .split('_')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect();
    format!("{}Module", pascal)
}

/// Convert a snake_case module name to the conventional silmaril crate name.
///
/// # Examples
/// ```
/// use silm::codegen::module_wiring::crate_name_from_module_name;
/// assert_eq!(crate_name_from_module_name("combat"), "silmaril-module-combat");
/// assert_eq!(crate_name_from_module_name("health_regen"), "silmaril-module-health-regen");
/// ```
pub fn crate_name_from_module_name(name: &str) -> String {
    format!("silmaril-module-{}", name.replace('_', "-"))
}

/// Parse `[package.metadata.silmaril]` from a Cargo.toml string.
///
/// Returns `None` if the section is absent or parsing fails.
pub fn parse_module_metadata(cargo_toml_content: &str) -> Option<ModuleMetadata> {
    let parsed: CargoToml = toml::from_str(cargo_toml_content).ok()?;
    let meta = parsed.package?.metadata?.silmaril?;
    Some(ModuleMetadata {
        module_type: meta.module_type,
        target: meta.target,
        init: meta.init,
    })
}

/// Generate the wiring block to insert into `lib.rs` / `main.rs`.
///
/// The block is a self-contained comment-delimited section that can be detected
/// and removed later via [`has_wiring_block`] and [`remove_wiring_block`].
pub fn generate_wiring_block(
    module_name: &str,
    crate_name: &str,
    version: &str,
    module_type: &str,
    init: &str,
) -> String {
    let use_path = crate_name.replace('-', "_");
    format!(
        "// --- silmaril module: {module_name} ({crate_name} v{version}) ---\nuse {use_path}::{module_type};\n// TODO: register \u{2192} world.add_module({init});\n"
    )
}

/// Check whether an entry-file string contains a wiring block for `module_name`.
///
/// Uses `" ("` suffix to prevent prefix collisions (e.g. `"combat"` must not
/// match `"combat-extended"`).
pub fn has_wiring_block(content: &str, module_name: &str) -> bool {
    let marker = format!("// --- silmaril module: {} (", module_name);
    content.contains(&marker)
}

/// Remove the wiring block for `module_name` from `content`.
///
/// A wiring block starts with `// --- silmaril module: {name} (` and ends
/// immediately before the next `// --- silmaril module:` marker, a blank line
/// separator, or at EOF.
pub fn remove_wiring_block(content: &str, module_name: &str) -> String {
    let marker = format!("// --- silmaril module: {} (", module_name);
    let next_marker = "// --- silmaril module:";

    let start = match content.find(&marker) {
        Some(i) => i,
        None => return content.to_string(),
    };

    // Find where the block ends: next marker, blank line (\n\n), or EOF.
    // Uses byte-offset search (str::find) to avoid line-ending arithmetic bugs.
    let after_start = &content[start + marker.len()..];

    // Candidates: position of next block marker or blank line within after_start.
    let next_block = after_start.find(next_marker);
    let blank_line = after_start.find("\n\n").map(|i| i + 1); // +1: keep first \n, cut before second

    let end_offset = match (next_block, blank_line) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => after_start.len(),
    };

    let end = start + marker.len() + end_offset;

    let before = content[..start].trim_end_matches('\n').to_string();
    let after = content[end..].to_string();
    if before.is_empty() {
        after.trim_start_matches('\n').to_string()
    } else {
        format!("{}\n{}", before, after.trim_start_matches('\n'))
    }
}

/// Scan Cargo.lock content for the resolved version of `crate_name`.
///
/// Expects the standard Cargo.lock format where `name` and `version` appear
/// on consecutive lines within a `[[package]]` section.
pub fn parse_cargo_lock_version(lock_content: &str, crate_name: &str) -> Option<String> {
    let name_line = format!("name = \"{}\"", crate_name);
    let mut lines = lock_content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == name_line {
            if let Some(ver_line) = lines.next() {
                let ver_line = ver_line.trim();
                if let Some(rest) = ver_line.strip_prefix("version = \"") {
                    if let Some(ver) = rest.strip_suffix('"') {
                        return Some(ver.to_string());
                    }
                }
            }
        }
    }
    None
}
