#![allow(dead_code)]

// ── game.toml string helpers ──────────────────────────────────────────────────

/// Return true if `[modules]` already has an entry for `name`.
/// Matches lines that look like `name = { ...}`.
pub fn game_toml_has_module(content: &str, name: &str) -> bool {
    let prefix = format!("{} = {{", name);
    content.lines().any(|l| l.trim_start().starts_with(&prefix))
}

/// Append `name = { <fields> }` inside the `[modules]` section.
/// Inserts just before the next section header after `[modules]`, or at EOF.
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
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with(&prefix))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// ── Cargo.toml string helpers ─────────────────────────────────────────────────

/// Return true if `[dependencies]` already has an entry for `crate_name`.
pub fn cargo_toml_has_dep(content: &str, crate_name: &str) -> bool {
    let prefix = format!("{} =", crate_name);
    let prefix2 = format!("\"{}\"", crate_name); // table format: [dependencies.crate-name]
    content.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with(&prefix) || t.contains(&prefix2)
    })
}

/// Append a dep line into `[dependencies]`.
/// `dep_value` is the RHS: e.g. `"^1.2.0"` or `{ git = "...", tag = "v1.0" }`.
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
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with(&prefix))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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
            return format!("{}\n{}\n{}", before, entry, after_bracket.trim_start_matches('\n'));
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
