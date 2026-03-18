//! Module wiring block generation, detection, and removal.
//!
//! Extracted from `engine/cli/src/codegen/module_wiring.rs`.

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
/// - `"combat"` -> `"CombatModule"`
/// - `"health_regen"` -> `"HealthRegenModule"`
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
/// - `"combat"` -> `"silmaril-module-combat"`
/// - `"health_regen"` -> `"silmaril-module-health-regen"`
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

    let after_start = &content[start + marker.len()..];

    let next_block = after_start.find(next_marker);
    let blank_line = after_start.find("\n\n").map(|i| i + 1);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_type_from_name() {
        assert_eq!(module_type_from_name("combat"), "CombatModule");
        assert_eq!(module_type_from_name("health_regen"), "HealthRegenModule");
    }

    #[test]
    fn test_crate_name_from_module_name() {
        assert_eq!(
            crate_name_from_module_name("combat"),
            "silmaril-module-combat"
        );
        assert_eq!(
            crate_name_from_module_name("health_regen"),
            "silmaril-module-health-regen"
        );
    }

    #[test]
    fn test_generate_wiring_block() {
        let block = generate_wiring_block(
            "combat",
            "silmaril-module-combat",
            "1.0.0",
            "CombatModule",
            "CombatModule::new()",
        );
        assert!(block.contains("// --- silmaril module: combat"));
        assert!(block.contains("use silmaril_module_combat::CombatModule;"));
    }

    #[test]
    fn test_has_wiring_block() {
        let content =
            "// --- silmaril module: combat (silmaril-module-combat v1.0.0) ---\nuse x;\n";
        assert!(has_wiring_block(content, "combat"));
        assert!(!has_wiring_block(content, "physics"));
    }

    #[test]
    fn test_remove_wiring_block() {
        let content = "before\n// --- silmaril module: combat (silmaril-module-combat v1.0.0) ---\nuse x;\n\nafter\n";
        let result = remove_wiring_block(content, "combat");
        assert!(!result.contains("combat"));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn test_parse_module_metadata_valid() {
        let toml = r#"
[package]
name = "silmaril-module-combat"
version = "1.0.0"

[package.metadata.silmaril]
module_type = "CombatModule"
target = "shared"
init = "CombatModule::new()"
"#;
        let meta = parse_module_metadata(toml).unwrap();
        assert_eq!(meta.module_type, "CombatModule");
        assert_eq!(meta.target, "shared");
        assert_eq!(meta.init, "CombatModule::new()");
    }

    #[test]
    fn test_parse_module_metadata_missing() {
        let toml = r#"
[package]
name = "some-crate"
version = "0.1.0"
"#;
        assert!(parse_module_metadata(toml).is_none());
    }

    #[test]
    fn test_parse_cargo_lock_version() {
        let lock = r#"
[[package]]
name = "silmaril-module-combat"
version = "1.2.3"
source = "registry"
"#;
        assert_eq!(
            parse_cargo_lock_version(lock, "silmaril-module-combat"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            parse_cargo_lock_version(lock, "nonexistent"),
            None
        );
    }
}
