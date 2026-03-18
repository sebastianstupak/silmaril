//! Remove a module from a silmaril game project.

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use super::{
    atomic_write, crate_dir, game_toml_has_module, remove_dep_from_cargo_toml,
    remove_module_from_game_toml, remove_workspace_member, remove_wiring_block,
    wiring_target, Target,
};

/// Remove a module from the game project.
///
/// Removes the dependency from the consuming crate's `Cargo.toml`, the wiring
/// block from the entry file, any workspace member (vendor mode), the vendored
/// directory itself, and the `game.toml` entry.
pub fn remove_module(module_name: &str, project_root: &Path) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;

    if !game_toml_has_module(&orig_game_toml, module_name) {
        bail!("module '{}' is not installed", module_name);
    }

    // Determine target and crate name from game.toml entry
    let target = detect_target_from_game_toml(&orig_game_toml, module_name)?;
    let source = detect_source_from_game_toml(&orig_game_toml, module_name);
    let crate_name = detect_crate_name_from_game_toml(&orig_game_toml, module_name)
        .unwrap_or_else(|| format!("silmaril-module-{}", module_name.replace('_', "-")));

    let crate_root = crate_dir(project_root, target)?;
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry = if entry_file.exists() {
        fs::read_to_string(&entry_file)?
    } else {
        String::new()
    };
    let orig_root_cargo =
        fs::read_to_string(project_root.join("Cargo.toml")).unwrap_or_default();

    let result = (|| -> Result<()> {
        // 1. Remove dep from consuming Cargo.toml
        let new_cargo = remove_dep_from_cargo_toml(&orig_cargo_toml, &crate_name);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 2. Remove wiring block from entry file
        let new_entry = remove_wiring_block(&orig_entry, module_name);
        atomic_write(&entry_file, &new_entry)?;

        // 3. Vendor: remove workspace member + delete modules/<name>/
        if source.as_deref() == Some("vendor") {
            let new_root = remove_workspace_member(
                &orig_root_cargo,
                &format!("modules/{}", module_name),
            );
            atomic_write(&project_root.join("Cargo.toml"), &new_root)?;
            let vendor_dir = project_root.join("modules").join(module_name);
            if vendor_dir.exists() {
                fs::remove_dir_all(&vendor_dir)?;
            }
        }

        // 4. Remove from game.toml
        let new_game = remove_module_from_game_toml(&orig_game_toml, module_name);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!("[silm] removed module '{}'", module_name);
    Ok(())
}

fn detect_target_from_game_toml(content: &str, module_name: &str) -> Result<Target> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            if line.contains("\"shared\"") {
                return Ok(Target::Shared);
            }
            if line.contains("\"server\"") {
                return Ok(Target::Server);
            }
            if line.contains("\"client\"") {
                return Ok(Target::Client);
            }
        }
    }
    bail!("cannot determine target for module '{}'", module_name);
}

fn detect_source_from_game_toml(content: &str, module_name: &str) -> Option<String> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            if line.contains("source = \"vendor\"") {
                return Some("vendor".to_string());
            }
            if line.contains("source = \"local\"") {
                return Some("local".to_string());
            }
            if line.contains("source = \"git\"") {
                return Some("git".to_string());
            }
            if line.contains("source = \"registry\"") {
                return Some("registry".to_string());
            }
        }
    }
    None
}

/// Read the actual crate name from the game.toml `crate = "..."` field.
fn detect_crate_name_from_game_toml(content: &str, module_name: &str) -> Option<String> {
    let prefix = format!("{} = {{", module_name);
    for line in content.lines() {
        if line.trim_start().starts_with(&prefix) {
            let pat = "crate = \"";
            if let Some(start) = line.find(pat) {
                let rest = &line[start + pat.len()..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}
