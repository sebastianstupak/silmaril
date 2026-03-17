use anyhow::Result;
use std::fs;
use std::path::Path;
use crate::codegen::module_wiring::parse_cargo_lock_version;

pub fn list_modules(project_root: &Path) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    if !game_toml_path.exists() {
        anyhow::bail!("no game.toml found");
    }
    let content = fs::read_to_string(&game_toml_path)?;

    // Read Cargo.lock if present
    let lock_content = fs::read_to_string(project_root.join("Cargo.lock")).unwrap_or_default();

    // Parse [modules] section manually
    let modules = parse_modules_section(&content);

    if modules.is_empty() {
        tracing::info!("[silm] no modules installed");
        return Ok(());
    }

    let name_w = modules.iter().map(|(n, _)| n.len()).max().unwrap_or(4).max(4);
    let src_w = 8usize;
    let req_w = modules.iter().map(|(_, v)| {
        extract_field(v, "version").or_else(|| extract_field(v, "tag")).or_else(|| extract_field(v, "ref")).or_else(|| extract_field(v, "path")).unwrap_or_default().len()
    }).max().unwrap_or(11).max(11);

    tracing::info!("{:<nw$}  {:<sw$}  {:<rw$}  {:<8}  TARGET",
        "NAME", "SOURCE", "REQUIREMENT", "RESOLVED",
        nw = name_w, sw = src_w, rw = req_w);
    tracing::info!("{}", "-".repeat(name_w + src_w + req_w + 30));

    for (name, fields) in &modules {
        let source = extract_field(fields, "source").unwrap_or_default();
        let requirement = extract_field(fields, "version")
            .or_else(|| extract_field(fields, "tag").map(|t| format!("tag={}", t)))
            .or_else(|| extract_field(fields, "ref").map(|r| format!("ref={}", r)))
            .or_else(|| extract_field(fields, "path"))
            .unwrap_or_default();
        let target = extract_field(fields, "target").unwrap_or_default();

        // Use the stored `crate = "..."` field if present
        let crate_name = extract_field(fields, "crate")
            .unwrap_or_else(|| format!("silmaril-module-{}", name.replace('_', "-")));
        let resolved = if source == "local" || source == "vendor" {
            "(local)".to_string()
        } else {
            parse_cargo_lock_version(&lock_content, &crate_name)
                .unwrap_or_else(|| "?".to_string())
        };

        tracing::info!("{:<nw$}  {:<sw$}  {:<rw$}  {:<8}  {}",
            name, source, requirement, resolved, target,
            nw = name_w, sw = src_w, rw = req_w);
    }

    Ok(())
}

/// Parse the [modules] section into (name, fields_string) pairs.
fn parse_modules_section(content: &str) -> Vec<(String, String)> {
    let mut in_modules = false;
    let mut result = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[modules]" { in_modules = true; continue; }
        if trimmed.starts_with('[') && trimmed != "[modules]" { in_modules = false; continue; }
        if !in_modules || trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if let Some(eq) = trimmed.find(" = {") {
            let name = trimmed[..eq].trim().to_string();
            let fields = trimmed[eq + 4..].trim_end_matches('}').to_string();
            result.push((name, fields));
        }
    }
    result
}

/// Extract a field value from an inline TOML fields string.
fn extract_field(fields: &str, key: &str) -> Option<String> {
    let pattern = format!("{} = \"", key);
    if let Some(start) = fields.find(&pattern) {
        let rest = &fields[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
