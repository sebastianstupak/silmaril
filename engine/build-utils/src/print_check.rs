//! Print statement checking for build scripts
//!
//! Ensures production code uses structured logging (tracing) instead of println!

use crate::scanner::{is_test_file, scan_directory};
use std::path::PathBuf;
use std::process;

/// Configuration for print statement checking
#[derive(Debug, Clone)]
pub struct PrintCheckConfig {
    /// Directory to scan (default: "src")
    pub src_dir: PathBuf,
    /// Whether to fail the build on violations (default: true)
    pub fail_on_violation: bool,
    /// Custom error message prefix
    pub error_prefix: Option<String>,
}

impl Default for PrintCheckConfig {
    fn default() -> Self {
        Self { src_dir: PathBuf::from("src"), fail_on_violation: true, error_prefix: None }
    }
}

impl PrintCheckConfig {
    /// Create a new configuration with custom source directory
    pub fn new(src_dir: impl Into<PathBuf>) -> Self {
        Self { src_dir: src_dir.into(), ..Default::default() }
    }

    /// Set whether to fail build on violations
    #[must_use]
    pub fn fail_on_violation(mut self, fail: bool) -> Self {
        self.fail_on_violation = fail;
        self
    }

    /// Set custom error message prefix
    #[must_use]
    pub fn error_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.error_prefix = Some(prefix.into());
        self
    }
}

/// Check that production code doesn't contain println!/eprintln!/dbg!
///
/// These are only allowed in tests/, benches/, and examples/ directories.
pub fn check_no_print_statements(config: &PrintCheckConfig) {
    let mut violations = Vec::new();

    scan_directory(&config.src_dir, &mut |path, content| {
        // Skip test files
        if is_test_file(path) {
            return;
        }

        let mut line_num = 0;
        for line in content.lines() {
            line_num += 1;

            // Skip comments
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            // Check for forbidden macros
            if let Some(violation) = check_line_for_prints(line) {
                violations.push(format!("{}:{}: {}", path.display(), line_num, violation));
            }
        }
    });

    if !violations.is_empty() {
        let prefix = config.error_prefix.as_deref().unwrap_or("ARCHITECTURE VIOLATION");

        eprintln!("\n❌ {prefix}: println!/eprintln!/dbg! in production code\n");
        eprintln!("CLAUDE.md mandates structured logging via tracing crate.\n");
        eprintln!("Violations found:");
        for violation in &violations {
            eprintln!("  {violation}");
        }
        eprintln!("\n✅ Fix: Replace with tracing macros:");
        eprintln!("  println!(\"...\") → info!(\"...\")");
        eprintln!("  eprintln!(\"...\") → error!(\"...\")");
        eprintln!("  dbg!(x) → debug!(?x, \"variable name\")\n");
        eprintln!("See: docs/rules/coding-standards.md\n");

        if config.fail_on_violation {
            process::exit(1);
        }
    }
}

/// Check for println!, eprintln!, dbg! in a single line
fn check_line_for_prints(line: &str) -> Option<String> {
    // Look for println! (with potential whitespace/formatting)
    if line.contains("println!") {
        return Some("Found println! - use tracing::info! instead".to_string());
    }

    if line.contains("eprintln!") {
        return Some("Found eprintln! - use tracing::error!/warn! instead".to_string());
    }

    if line.contains("dbg!") {
        return Some("Found dbg! - use tracing::debug! instead".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_line_for_prints() {
        assert!(check_line_for_prints("println!(\"hello\");").is_some());
        assert!(check_line_for_prints("    println!(\"hello\");").is_some());
        assert!(check_line_for_prints("eprintln!(\"error\");").is_some());
        assert!(check_line_for_prints("dbg!(value);").is_some());

        // Should not trigger
        assert!(check_line_for_prints("info!(\"hello\");").is_none());
        assert!(check_line_for_prints("// println!(\"commented\");").is_none());
        assert!(check_line_for_prints("let x = \"println!\";").is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = PrintCheckConfig::new("custom/src")
            .fail_on_violation(false)
            .error_prefix("CUSTOM ERROR");

        assert_eq!(config.src_dir, PathBuf::from("custom/src"));
        assert!(!config.fail_on_violation);
        assert_eq!(config.error_prefix, Some("CUSTOM ERROR".to_string()));
    }
}
