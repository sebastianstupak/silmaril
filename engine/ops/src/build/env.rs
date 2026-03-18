//! Environment variable parsing and merging for build commands.
//!
//! Supports three layers of environment configuration:
//! 1. `[build.env]` section in game.toml (lowest priority)
//! 2. `.env` file (medium priority)
//! 3. Explicit env file path (highest priority)
//!
//! Shell environment variables always win over all layers.

use std::collections::HashMap;

/// Parse a `.env` file content into key-value pairs.
///
/// Rules:
/// - One `KEY=VALUE` per line
/// - Lines starting with `#` are comments
/// - Blank lines are skipped
/// - Values are NOT trimmed (preserves whitespace)
/// - Lines without `=` are skipped
/// - Duplicate keys: last occurrence wins (in returned Vec order)
pub fn parse_env_file(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].to_string();
            if !key.is_empty() {
                entries.push((key, value));
            }
        }
    }
    entries
}

/// Parse the `[build.env]` section from game.toml content.
///
/// Returns key-value pairs from the `[build.env]` table.
/// Only string values are extracted; non-string values are skipped.
/// Returns empty Vec if the section is absent.
pub fn parse_build_env(game_toml_content: &str) -> Vec<(String, String)> {
    let table: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let env_table = table
        .get("build")
        .and_then(|b| b.get("env"))
        .and_then(|e| e.as_table());

    match env_table {
        Some(t) => t
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        None => Vec::new(),
    }
}

/// Parse the `[build]` section's `platforms` array from game.toml content.
///
/// Returns `Some(vec)` with platform names if present and non-empty,
/// `None` if the `[build]` section or `platforms` key is absent or
/// the array is empty.
pub fn parse_build_section(game_toml_content: &str) -> Option<Vec<String>> {
    let table: toml::Value = game_toml_content.parse().ok()?;

    let platforms = table
        .get("build")
        .and_then(|b| b.get("platforms"))
        .and_then(|p| p.as_array())?;

    let result: Vec<String> = platforms
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Merge environment variable layers with priority:
/// `build_env` (lowest) < `dotenv` < `env_file` (highest).
///
/// After merging, any key that already exists in the current shell
/// environment (`std::env::var`) is removed — the shell always wins.
pub fn merge_env(
    build_env: &[(String, String)],
    dotenv: &[(String, String)],
    env_file: &[(String, String)],
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    // Lowest priority first
    for (k, v) in build_env {
        merged.insert(k.clone(), v.clone());
    }
    for (k, v) in dotenv {
        merged.insert(k.clone(), v.clone());
    }
    // Highest priority last
    for (k, v) in env_file {
        merged.insert(k.clone(), v.clone());
    }

    // Shell environment always wins — remove keys already set
    merged.retain(|k, _| std::env::var(k).is_err());

    merged
}
