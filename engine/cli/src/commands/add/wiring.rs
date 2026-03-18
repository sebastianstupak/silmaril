#![allow(dead_code)] // Many helpers now superseded by engine_ops::project

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Which crate to target
#[derive(Debug, Clone, Copy)]
pub enum Target {
    Shared,
    Server,
    Client,
}

impl Target {
    /// Subdirectory name relative to project root
    pub fn crate_subdir(&self) -> &'static str {
        match self {
            Target::Shared => "shared",
            Target::Server => "server",
            Target::Client => "client",
        }
    }

    /// Entry point file within src/ (lib.rs for shared, main.rs for server/client)
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
            bail!("no game.toml found — run this command from inside a silmaril project");
        }
    }
}

/// Resolve the crate directory, error if it doesn't exist.
pub fn crate_dir(project_root: &Path, target: Target) -> Result<PathBuf> {
    let dir = project_root.join(target.crate_subdir());
    if !dir.is_dir() {
        bail!(
            "target crate '{}/' not found — is this project set up correctly?",
            target.crate_subdir()
        );
    }
    Ok(dir)
}

/// Resolve domain module file path: `<crate>/src/<domain>/mod.rs`
pub fn domain_file(crate_root: &Path, domain: &str) -> PathBuf {
    crate_root.join("src").join(domain).join("mod.rs")
}

/// Resolve wiring target: `<crate>/src/lib.rs` or `<crate>/src/main.rs`
pub fn wiring_target(crate_root: &Path, target: Target) -> PathBuf {
    crate_root.join("src").join(target.entry_file())
}

/// Check if `pub struct <Name>` (followed by `{` with optional whitespace) exists in file.
pub fn has_duplicate_component(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub struct {}", name);
    Ok(content.lines().any(|line| {
        if let Some(rest) = line.trim_start().strip_prefix(&pattern) {
            rest.trim_start().starts_with('{')
        } else {
            false
        }
    }))
}

/// Check if `pub fn <name>_system(` exists in file.
pub fn has_duplicate_system(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub fn {}_system(", name);
    Ok(content.contains(&pattern))
}

/// Write `content` to `path` atomically (temp file → rename).
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

/// Append `content` to domain file atomically.
/// If file doesn't exist, creates it. Reads original into memory for rollback.
/// Returns the original content (None if file was new) for rollback on wiring failure.
pub fn append_to_domain_file(file: &Path, content: &str) -> Result<Option<String>> {
    let original = if file.exists() { Some(fs::read_to_string(file)?) } else { None };

    let new_content = match &original {
        Some(existing) => format!("{}\n{}", existing, content),
        None => content.to_string(),
    };

    atomic_write(file, &new_content)?;
    Ok(original)
}

/// Add `pub mod <domain>;` to the wiring target file if not already present.
/// Uses atomic write. Returns the original content for rollback.
pub fn wire_module_declaration(target_file: &Path, domain: &str) -> Result<String> {
    let original = if target_file.exists() {
        fs::read_to_string(target_file)?
    } else {
        String::new()
    };

    let declaration = format!("pub mod {};", domain);
    if original.contains(&declaration) {
        return Ok(original); // already wired, nothing to do
    }

    let new_content = format!("{}\n{}\n", original.trim_end(), declaration);
    atomic_write(target_file, &new_content)?;
    Ok(original)
}

/// Rollback domain file: restore original content, or delete if it was newly created.
pub fn rollback_domain_file(file: &Path, original: Option<String>) -> Result<()> {
    match original {
        Some(content) => atomic_write(file, &content),
        None => {
            if file.exists() {
                fs::remove_file(file)?;
            }
            Ok(())
        }
    }
}

/// Rollback wiring target to original content.
#[allow(dead_code)]
pub fn rollback_wiring_target(file: &Path, original: &str) -> Result<()> {
    atomic_write(file, original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_project(tmp: &TempDir) -> PathBuf {
        let root = tmp.path().to_path_buf();
        fs::write(root.join("game.toml"), "[game]\nname = \"test\"").unwrap();
        fs::create_dir_all(root.join("shared/src")).unwrap();
        fs::write(root.join("shared/src/lib.rs"), "").unwrap();
        root
    }

    #[test]
    fn test_find_project_root_from_same_dir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let found = find_project_root(&root).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_from_subdir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let subdir = root.join("shared/src/health");
        fs::create_dir_all(&subdir).unwrap();
        let found = find_project_root(&subdir).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_not_found() {
        let tmp = TempDir::new().unwrap();
        // No game.toml
        let result = find_project_root(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no game.toml found"));
    }

    #[test]
    fn test_crate_dir_ok() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let dir = crate_dir(&root, Target::Shared).unwrap();
        assert_eq!(dir, root.join("shared"));
    }

    #[test]
    fn test_crate_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let result = crate_dir(&root, Target::Server);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("server/"));
    }

    #[test]
    fn test_has_duplicate_component_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub struct Health {\n    pub current: f32,\n}\n").unwrap();
        assert!(has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_component_not_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub struct Damage {\n    pub amount: f32,\n}\n").unwrap();
        assert!(!has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_component_no_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("nonexistent.rs");
        assert!(!has_duplicate_component(&file, "Health").unwrap());
    }

    #[test]
    fn test_has_duplicate_system_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub fn health_regen_system(world: &mut World, dt: f32) {\n}\n").unwrap();
        assert!(has_duplicate_system(&file, "health_regen").unwrap());
    }

    #[test]
    fn test_has_duplicate_system_not_found() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "pub fn other_system(world: &mut World, dt: f32) {\n}\n").unwrap();
        assert!(!has_duplicate_system(&file, "health_regen").unwrap());
    }

    #[test]
    fn test_wire_module_declaration_adds() {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path().join("lib.rs");
        fs::write(&lib, "// empty\n").unwrap();
        wire_module_declaration(&lib, "health").unwrap();
        let content = fs::read_to_string(&lib).unwrap();
        assert!(content.contains("pub mod health;"));
    }

    #[test]
    fn test_wire_module_declaration_idempotent() {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path().join("lib.rs");
        fs::write(&lib, "pub mod health;\n").unwrap();
        wire_module_declaration(&lib, "health").unwrap();
        let content = fs::read_to_string(&lib).unwrap();
        // Should appear exactly once
        assert_eq!(content.matches("pub mod health;").count(), 1);
    }

    #[test]
    fn test_append_to_domain_file_new() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("health").join("mod.rs");
        let original = append_to_domain_file(&file, "// new content\n").unwrap();
        assert!(original.is_none()); // was new
        assert_eq!(fs::read_to_string(&file).unwrap(), "// new content\n");
    }

    #[test]
    fn test_append_to_domain_file_existing() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "// existing\n").unwrap();
        let original = append_to_domain_file(&file, "// appended\n").unwrap();
        assert_eq!(original.as_deref(), Some("// existing\n"));
        let content = fs::read_to_string(&file).unwrap();
        assert!(content.contains("// existing\n"));
        assert!(content.contains("// appended\n"));
    }

    #[test]
    fn test_rollback_domain_file_new() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("health").join("mod.rs");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "// something\n").unwrap();
        rollback_domain_file(&file, None).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn test_rollback_domain_file_existing() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("mod.rs");
        fs::write(&file, "// original\n").unwrap();
        rollback_domain_file(&file, Some("// original\n".to_string())).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "// original\n");
    }
}
