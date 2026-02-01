//! Error handling macro checking for build scripts
//!
//! Ensures all error types use the `define_error!` macro for consistency

use crate::scanner::{is_test_file, scan_directory};
use std::path::PathBuf;
use std::process;

/// Configuration for error type checking
#[derive(Debug, Clone)]
pub struct ErrorCheckConfig {
    /// Directory to scan (default: "src")
    pub src_dir: PathBuf,
    /// Whether to fail the build on violations (default: true)
    pub fail_on_violation: bool,
    /// Files to skip (e.g., "error.rs" foundation file)
    pub skip_files: Vec<String>,
}

impl Default for ErrorCheckConfig {
    fn default() -> Self {
        Self {
            src_dir: PathBuf::from("src"),
            fail_on_violation: true,
            skip_files: vec!["error.rs".to_string()],
        }
    }
}

impl ErrorCheckConfig {
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

    /// Set files to skip during checking
    #[must_use]
    pub fn skip_files(mut self, files: Vec<String>) -> Self {
        self.skip_files = files;
        self
    }
}

/// Check that all error types use the `define_error!` macro
///
/// This enforces CLAUDE.md requirement for structured error handling.
pub fn check_error_types_use_macro(config: &ErrorCheckConfig) {
    let mut violations = Vec::new();

    scan_directory(&config.src_dir, &mut |path, content| {
        // Skip configured files
        if let Some(filename) = path.file_name() {
            let name = filename.to_string_lossy();
            for skip_file in &config.skip_files {
                if name.contains(skip_file) {
                    return;
                }
            }
        }

        // Skip test files
        if is_test_file(path) {
            return;
        }

        let mut in_define_error_block = false;
        let mut line_num = 0;
        let mut brace_depth = 0;

        for line in content.lines() {
            line_num += 1;
            let trimmed = line.trim();

            // Track if we're inside a define_error! block
            if trimmed.contains("define_error!") {
                in_define_error_block = true;
                brace_depth = 0;
            }

            if in_define_error_block {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                {
                    brace_depth += line.matches('{').count() as i32;
                    brace_depth -= line.matches('}').count() as i32;
                }

                if brace_depth <= 0 {
                    in_define_error_block = false;
                }
            }

            // Check for error enum definitions outside define_error! blocks
            if !in_define_error_block
                && (trimmed.contains("pub enum") || trimmed.contains("pub(crate) enum"))
                && (trimmed.contains("Error {")
                    || (trimmed.contains("Error") && trimmed.ends_with('{')))
            {
                // Found a potential error enum not in define_error! block
                // Make sure it actually ends with "Error"
                if let Some(enum_name) = extract_enum_name(trimmed) {
                    if enum_name.ends_with("Error") {
                        violations.push(format!(
                            "{}:{}: Error type '{}' must use define_error! macro",
                            path.display(),
                            line_num,
                            enum_name
                        ));
                    }
                }
            }
        }
    });

    if !violations.is_empty() {
        eprintln!("\n❌ ARCHITECTURE VIOLATION: Error types not using define_error! macro\n");
        eprintln!(
            "CLAUDE.md mandates all error types use the define_error! macro for consistency.\n"
        );
        eprintln!("Violations found:");
        for violation in &violations {
            eprintln!("  {violation}");
        }
        eprintln!("\n✅ Fix: Use the define_error! macro:");
        eprintln!("  define_error! {{");
        eprintln!("      pub enum MyError {{");
        eprintln!("          Variant {{ field: Type }} = ErrorCode::Code, ErrorSeverity::Level,");
        eprintln!("      }}");
        eprintln!("  }}\n");
        eprintln!("See: docs/error-handling.md\n");

        if config.fail_on_violation {
            process::exit(1);
        }
    }
}

/// Extract enum name from an enum declaration line
fn extract_enum_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();

    // Skip comments
    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
        return None;
    }

    // Looking for patterns like "pub enum ErrorName {" or "pub(crate) enum ErrorName {"
    let parts: Vec<&str> = line.split_whitespace().collect();

    for (i, part) in parts.iter().enumerate() {
        if *part == "enum" && i + 1 < parts.len() {
            let name = parts[i + 1].trim_end_matches('{').trim();
            return Some(name.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_enum_name() {
        assert_eq!(extract_enum_name("pub enum MyError {"), Some("MyError".to_string()));
        assert_eq!(
            extract_enum_name("pub(crate) enum InternalError {"),
            Some("InternalError".to_string())
        );
        assert_eq!(extract_enum_name("    pub enum TestError {"), Some("TestError".to_string()));
        assert_eq!(extract_enum_name("pub enum Result {"), Some("Result".to_string()));

        assert_eq!(extract_enum_name("struct NotAnEnum"), None);
        assert_eq!(extract_enum_name("// enum Comment"), None);
    }

    #[test]
    fn test_config_builder() {
        let config = ErrorCheckConfig::new("custom/src")
            .fail_on_violation(false)
            .skip_files(vec!["foundation.rs".to_string()]);

        assert_eq!(config.src_dir, PathBuf::from("custom/src"));
        assert!(!config.fail_on_violation);
        assert_eq!(config.skip_files, vec!["foundation.rs"]);
    }
}
