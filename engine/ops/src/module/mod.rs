//! Module management -- add, remove, list.
//!
//! Extracted from `engine/cli/src/commands/add/module.rs` and `commands/module/*`.

pub mod add;
pub mod list;
pub mod remove;

// -- Shared helpers -----------------------------------------------------------

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Which crate to target when wiring a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Shared,
    Server,
    Client,
}

impl Target {
    /// Subdirectory name relative to project root.
    pub fn crate_subdir(&self) -> &'static str {
        match self {
            Target::Shared => "shared",
            Target::Server => "server",
            Target::Client => "client",
        }
    }

    /// Entry point file within `src/` (`lib.rs` for shared, `main.rs` for server/client).
    pub fn entry_file(&self) -> &'static str {
        match self {
            Target::Shared => "lib.rs",
            Target::Server | Target::Client => "main.rs",
        }
    }
}

/// Walk up from `start` to find `game.toml`. Returns the directory containing it.
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("game.toml").exists() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("no game.toml found -- run this command from inside a silmaril project");
        }
    }
}

/// Resolve the crate directory, error if it doesn't exist.
pub fn crate_dir(project_root: &Path, target: Target) -> Result<PathBuf> {
    let dir = project_root.join(target.crate_subdir());
    if !dir.is_dir() {
        bail!(
            "target crate '{}/' not found -- is this project set up correctly?",
            target.crate_subdir()
        );
    }
    Ok(dir)
}

/// Resolve wiring target: `<crate>/src/lib.rs` or `<crate>/src/main.rs`.
pub fn wiring_target(crate_root: &Path, target: Target) -> PathBuf {
    crate_root.join("src").join(target.entry_file())
}

/// Write `content` to `path` atomically (temp file -> rename).
/// Creates parent directories if needed.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

// -- game.toml string helpers -------------------------------------------------

/// Return true if `[modules]` already has an entry for `name`.
pub fn game_toml_has_module(content: &str, name: &str) -> bool {
    let prefix = format!("{} = {{", name);
    content.lines().any(|l| l.trim_start().starts_with(&prefix))
}

/// Append `name = { <fields> }` inside the `[modules]` section.
pub fn append_module_to_game_toml(content: &str, name: &str, fields: &str) -> String {
    let entry = format!("{} = {{ {} }}", name, fields);
    if let Some(mod_pos) = content.find("[modules]") {
        let after_modules = &content[mod_pos + "[modules]".len()..];
        let insert_offset = after_modules
            .find("\n[")
            .map(|i| i + 1)
            .unwrap_or(after_modules.len());
        let insert_at = mod_pos + "[modules]".len() + insert_offset;
        let (before, after) = content.split_at(insert_at);
        let before = before.trim_end_matches('\n');
        format!("{}\n{}\n{}", before, entry, after.trim_start_matches('\n'))
    } else {
        format!("{}\n[modules]\n{}\n", content.trim_end(), entry)
    }
}

/// Remove the line `<name> = { ... }` from `[modules]` section.
pub fn remove_module_from_game_toml(content: &str, name: &str) -> String {
    let prefix = format!("{} = {{", name);
    let mut in_modules = false;
    let mut lines_out: Vec<&str> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_modules = trimmed == "[modules]";
        }
        if in_modules && line.trim_start().starts_with(&prefix) {
            continue;
        }
        lines_out.push(line);
    }
    lines_out.join("\n") + "\n"
}

// -- Cargo.toml string helpers ------------------------------------------------

/// Return true if `[dependencies]` already has an entry for `crate_name`.
pub fn cargo_toml_has_dep(content: &str, crate_name: &str) -> bool {
    let prefix = format!("{} =", crate_name);
    let prefix2 = format!("\"{}\"", crate_name);
    content.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with(&prefix) || t.contains(&prefix2)
    })
}

/// Append a dep line into `[dependencies]`.
pub fn append_dep_to_cargo_toml(content: &str, crate_name: &str, dep_value: &str) -> String {
    let entry = format!("{} = {}", crate_name, dep_value);
    if let Some(dep_pos) = content.find("[dependencies]") {
        let after = &content[dep_pos + "[dependencies]".len()..];
        let insert_offset = after
            .find("\n[")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let insert_at = dep_pos + "[dependencies]".len() + insert_offset;
        let (before, after_sec) = content.split_at(insert_at);
        let before = before.trim_end_matches('\n');
        format!("{}\n{}\n{}", before, entry, after_sec.trim_start_matches('\n'))
    } else {
        format!("{}\n[dependencies]\n{}\n", content.trim_end(), entry)
    }
}

/// Remove the dep line for `crate_name` from `[dependencies]`.
pub fn remove_dep_from_cargo_toml(content: &str, crate_name: &str) -> String {
    let prefix = format!("{} =", crate_name);
    let mut in_deps = false;
    let mut lines_out: Vec<&str> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
        }
        if in_deps && line.trim_start().starts_with(&prefix) {
            continue;
        }
        lines_out.push(line);
    }
    lines_out.join("\n") + "\n"
}

/// Add `"<member_path>"` to `[workspace] members = [...]` array.
pub fn add_workspace_member(content: &str, member_path: &str) -> String {
    let entry = format!("    \"{}\",", member_path);
    if let Some(members_pos) = content.find("members = [") {
        let after = &content[members_pos..];
        if let Some(close) = after.find(']') {
            let insert_at = members_pos + close;
            let (before, after_bracket) = content.split_at(insert_at);
            let before = before.trim_end_matches('\n');
            return format!(
                "{}\n{}\n{}",
                before,
                entry,
                after_bracket.trim_start_matches('\n')
            );
        }
    }
    content.to_string()
}

/// Remove `"<member_path>"` from `[workspace] members = [...]` array.
pub fn remove_workspace_member(content: &str, member_path: &str) -> String {
    let pattern = format!("\"{}\"", member_path);
    content
        .lines()
        .filter(|l| !l.contains(&pattern))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// -- Module wiring / codegen helpers ------------------------------------------

/// Metadata declared under `[package.metadata.silmaril]` in a module's Cargo.toml.
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub module_type: String,
    pub target: String,
    pub init: String,
}

#[derive(serde::Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(serde::Deserialize)]
struct CargoPackage {
    metadata: Option<CargoMetadata>,
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    silmaril: Option<SilmarilMeta>,
}

#[derive(serde::Deserialize)]
struct SilmarilMeta {
    module_type: String,
    #[allow(dead_code)]
    target: String,
    init: String,
}

/// Convert snake_case module name to the conventional Rust type name.
///
/// # Examples
///
/// ```
/// use engine_ops::module::module_type_from_name;
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
///
/// ```
/// use engine_ops::module::crate_name_from_module_name;
/// assert_eq!(crate_name_from_module_name("combat"), "silmaril-module-combat");
/// assert_eq!(crate_name_from_module_name("health_regen"), "silmaril-module-health-regen");
/// ```
pub fn crate_name_from_module_name(name: &str) -> String {
    format!("silmaril-module-{}", name.replace('_', "-"))
}

/// Parse `[package.metadata.silmaril]` from a Cargo.toml string.
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

/// Parse the `[package] name` field from a Cargo.toml string.
pub fn crate_name_from_cargo_toml(content: &str) -> anyhow::Result<String> {
    #[derive(serde::Deserialize)]
    struct Pkg {
        package: PkgInner,
    }
    #[derive(serde::Deserialize)]
    struct PkgInner {
        name: String,
    }
    let p: Pkg =
        toml::from_str(content).map_err(|e| anyhow::anyhow!("invalid Cargo.toml: {}", e))?;
    Ok(p.package.name)
}
