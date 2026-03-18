//! Add a module to a silmaril game project.

use std::fs;
use std::path::{Path, PathBuf};

use super::{
    append_dep_to_cargo_toml, append_module_to_game_toml, atomic_write,
    add_workspace_member, cargo_toml_has_dep, crate_dir, crate_name_from_cargo_toml,
    crate_name_from_module_name, find_project_root, game_toml_has_module,
    generate_wiring_block, has_wiring_block, module_type_from_name,
    parse_module_metadata, wiring_target, Target,
};

/// Add a module to the game project.
///
/// Source mode is determined by the arguments provided:
/// - `vendor=true, local_path=Some(p)` -> vendor mode (copies source to `modules/<name>/`)
/// - `local_path=Some(p)` -> path mode (relative path dep)
/// - `git_url=Some(url)` -> git mode
/// - else -> registry mode (crates.io)
///
/// `name` may contain an optional `@version` suffix, e.g. `"combat@1.2.0"`.
pub fn add_module(
    name: &str,
    git_url: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    local_path: Option<&str>,
    vendor: bool,
    target: Target,
) -> anyhow::Result<()> {
    use std::env;

    // Parse optional @version suffix from name
    let (module_name, requested_version) = if let Some(at) = name.rfind('@') {
        (&name[..at], Some(&name[at + 1..]))
    } else {
        (name, None)
    };

    // Validation: vendor mode requires --path
    if vendor && local_path.is_none() {
        anyhow::bail!(
            "--vendor requires --path: clone the module to a local directory first, \
             then vendor it with --vendor --path <dir>"
        );
    }

    // Find project root
    let cwd = env::current_dir()?;
    let project_root = find_project_root(&cwd)?;

    // Resolve consuming crate paths
    let crate_root = crate_dir(&project_root, target)?;
    let game_toml_path = project_root.join("game.toml");
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    // Read originals for rollback
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;
    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry_file = if entry_file.exists() {
        fs::read_to_string(&entry_file)?
    } else {
        String::new()
    };

    // Duplicate check via game.toml
    if game_toml_has_module(&orig_game_toml, module_name) {
        anyhow::bail!(
            "module '{}' is already installed -- use 'silm module upgrade' to update",
            module_name
        );
    }

    // Vendor mode: delegate entirely to the vendor helper
    if vendor {
        let path = local_path.unwrap(); // validated above
        return add_module_vendor_from_path(module_name, Path::new(path), target, &project_root);
    }

    // Resolve (crate_name, dep_value, game_entry_fields, module_type, init_expr)
    let (crate_name, dep_value, game_entry_fields, module_type, init_expr) =
        if let Some(path_str) = local_path {
            // -- Path mode --------------------------------------------------------
            let mod_cargo = Path::new(path_str).join("Cargo.toml");
            let mod_cargo_content = fs::read_to_string(&mod_cargo)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {}", mod_cargo.display(), e))?;

            // Read actual crate name from the module's own Cargo.toml
            let actual_crate_name = crate_name_from_cargo_toml(&mod_cargo_content)?;

            let (mt, init) = if let Some(meta) = parse_module_metadata(&mod_cargo_content) {
                (meta.module_type, meta.init)
            } else {
                let mt = module_type_from_name(module_name);
                let init = format!("{}::new()", mt);
                (mt, init)
            };

            // Canonicalise and make path relative from the consuming crate directory.
            // Strip the \\?\ extended-path prefix that Windows canonicalize() adds --
            // pathdiff treats VerbatimDisk and Disk prefixes as different roots (returns None),
            // and Cargo rejects \\?\ in path dependencies entirely.
            let abs_path = {
                let canon = Path::new(path_str)
                    .canonicalize()
                    .map_err(|_| anyhow::anyhow!("path '{}' does not exist", path_str))?;
                let s = canon.to_string_lossy();
                if let Some(stripped) = s.strip_prefix(r"\\?\") {
                    PathBuf::from(stripped)
                } else {
                    canon
                }
            };
            let rel_path = pathdiff::diff_paths(&abs_path, &crate_root)
                .unwrap_or_else(|| abs_path.clone())
                .to_string_lossy()
                .replace('\\', "/");

            let dep_val = format!("{{ path = \"{}\" }}", rel_path);
            let game_entry = format!(
                "source = \"local\", path = \"{}\", target = \"{}\", crate = \"{}\"",
                path_str,
                target.crate_subdir(),
                actual_crate_name
            );
            (actual_crate_name, dep_val, game_entry, mt, init)
        } else if let Some(url) = git_url {
            // -- Git mode ---------------------------------------------------------
            let cn = crate_name_from_module_name(module_name);
            let pin = if let Some(r) = rev {
                format!(", rev = \"{}\"", r)
            } else if let Some(t) = tag {
                format!(", tag = \"{}\"", t)
            } else {
                String::new()
            };
            let dep_val = format!("{{ git = \"{}\"{} }}", url, pin);

            let game_entry = if let Some(r) = rev {
                format!(
                    "source = \"git\", url = \"{}\", rev = \"{}\", target = \"{}\", crate = \"{}\"",
                    url, r, target.crate_subdir(), cn
                )
            } else if let Some(t) = tag {
                format!(
                    "source = \"git\", url = \"{}\", tag = \"{}\", target = \"{}\", crate = \"{}\"",
                    url, t, target.crate_subdir(), cn
                )
            } else {
                format!(
                    "source = \"git\", url = \"{}\", target = \"{}\", crate = \"{}\"",
                    url, target.crate_subdir(), cn
                )
            };

            let mt = module_type_from_name(module_name);
            let init = format!("{}::new()", mt);
            tracing::warn!(
                "[silm] module '{}' is from a git URL -- review the source before use",
                module_name
            );
            (cn, dep_val, game_entry, mt, init)
        } else {
            // -- Registry mode ----------------------------------------------------
            let cn = crate_name_from_module_name(module_name);
            let version = requested_version.unwrap_or("*");
            let dep_val = format!("\"{}\"", version);
            let game_entry = format!(
                "source = \"registry\", version = \"{}\", target = \"{}\", crate = \"{}\"",
                version,
                target.crate_subdir(),
                cn
            );
            let mt = module_type_from_name(module_name);
            let init = format!("{}::new()", mt);
            (cn, dep_val, game_entry, mt, init)
        };

    // Duplicate check via Cargo.toml
    if cargo_toml_has_dep(&orig_cargo_toml, &crate_name) {
        anyhow::bail!(
            "module '{}' is already installed -- use 'silm module upgrade' to update",
            module_name
        );
    }

    // Apply all three writes with rollback on failure
    let result = (|| -> anyhow::Result<()> {
        // 1. Add dep to consuming crate's Cargo.toml
        let new_cargo = append_dep_to_cargo_toml(&orig_cargo_toml, &crate_name, &dep_value);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 2. Append wiring block to entry file (idempotent guard)
        if !has_wiring_block(&orig_entry_file, module_name) {
            let block = generate_wiring_block(
                module_name,
                &crate_name,
                "latest",
                &module_type,
                &init_expr,
            );
            let new_entry = if orig_entry_file.is_empty() {
                block
            } else {
                format!("{}\n{}", orig_entry_file.trim_end(), block)
            };
            atomic_write(&entry_file, &new_entry)?;
        }

        // 3. Record in game.toml [modules]
        let new_game =
            append_module_to_game_toml(&orig_game_toml, module_name, &game_entry_fields);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        // Best-effort rollback
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry_file);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!(
        "[silm] added {} to {}/",
        crate_name,
        target.crate_subdir()
    );
    tracing::info!("[silm] wired: {}", entry_file.display());
    tracing::info!("[silm] tracked: game.toml [modules.{}]", module_name);
    Ok(())
}

/// Vendor mode: copy `source_path` into `modules/<name>/`, add workspace member,
/// wire dep + wiring block + game.toml entry.
///
/// Isolated code path for future license-gating.
pub fn add_module_vendor_from_path(
    module_name: &str,
    source_path: &Path,
    target: Target,
    project_root: &Path,
) -> anyhow::Result<()> {
    let modules_dir = project_root.join("modules").join(module_name);
    if modules_dir.exists() {
        anyhow::bail!(
            "modules/{} already exists -- remove it first with 'silm module remove {}'",
            module_name, module_name
        );
    }

    let crate_root = crate_dir(project_root, target)?;
    let game_toml_path = project_root.join("game.toml");
    let root_cargo_path = project_root.join("Cargo.toml");
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let entry_file = wiring_target(&crate_root, target);

    // Read originals for rollback
    let orig_game_toml = fs::read_to_string(&game_toml_path)?;
    let orig_root_cargo = fs::read_to_string(&root_cargo_path)?;
    let orig_cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let orig_entry = if entry_file.exists() {
        fs::read_to_string(&entry_file)?
    } else {
        String::new()
    };

    // Duplicate check
    if game_toml_has_module(&orig_game_toml, module_name) {
        anyhow::bail!("module '{}' is already installed", module_name);
    }

    // Copy source -> modules/<name>/
    copy_dir_all(source_path, &modules_dir)?;

    // Read crate name and metadata from the vendored Cargo.toml
    let vendored_cargo_content = fs::read_to_string(modules_dir.join("Cargo.toml"))
        .map_err(|e| anyhow::anyhow!("vendored Cargo.toml missing or unreadable: {}", e))?;

    let crate_name = crate_name_from_cargo_toml(&vendored_cargo_content)
        .map_err(|e| anyhow::anyhow!("invalid Cargo.toml in vendored module: {}", e))?;

    let (module_type, init) = if let Some(meta) = parse_module_metadata(&vendored_cargo_content) {
        (meta.module_type, meta.init)
    } else {
        let mt = module_type_from_name(module_name);
        let i = format!("{}::new()", mt);
        (mt, i)
    };

    // Path dep: from consuming crate's directory to modules/<name>/
    // e.g. shared/ -> ../../modules/combat
    let rel_path = format!("../../modules/{}", module_name);
    let dep_value = format!("{{ path = \"{}\" }}", rel_path);

    let result = (|| -> anyhow::Result<()> {
        // 1. Add workspace member to root Cargo.toml
        let new_root = add_workspace_member(
            &orig_root_cargo,
            &format!("modules/{}", module_name),
        );
        atomic_write(&root_cargo_path, &new_root)?;

        // 2. Add path dep to consuming Cargo.toml
        let new_cargo = append_dep_to_cargo_toml(&orig_cargo_toml, &crate_name, &dep_value);
        atomic_write(&cargo_toml_path, &new_cargo)?;

        // 3. Append wiring block to entry file
        if !has_wiring_block(&orig_entry, module_name) {
            let block = generate_wiring_block(
                module_name, &crate_name, "vendored", &module_type, &init,
            );
            let new_entry = if orig_entry.is_empty() {
                block
            } else {
                format!("{}\n{}", orig_entry.trim_end(), block)
            };
            atomic_write(&entry_file, &new_entry)?;
        }

        // 4. Update game.toml
        let game_entry = format!(
            "source = \"vendor\", ref = \"vendored\", target = \"{}\", crate = \"{}\"",
            target.crate_subdir(),
            crate_name
        );
        let new_game = append_module_to_game_toml(&orig_game_toml, module_name, &game_entry);
        atomic_write(&game_toml_path, &new_game)?;

        Ok(())
    })();

    if let Err(e) = result {
        // Rollback: delete copied dir, restore all files
        let _ = fs::remove_dir_all(&modules_dir);
        let _ = atomic_write(&root_cargo_path, &orig_root_cargo);
        let _ = atomic_write(&cargo_toml_path, &orig_cargo_toml);
        let _ = atomic_write(&entry_file, &orig_entry);
        let _ = atomic_write(&game_toml_path, &orig_game_toml);
        return Err(e);
    }

    tracing::info!(
        module = %module_name,
        crate_name = %crate_name,
        "vendored module into modules/{}",
        module_name
    );
    tracing::info!("[silm] wired: {}", entry_file.display());
    tracing::info!("[silm] tracked: game.toml [modules.{}]", module_name);
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
