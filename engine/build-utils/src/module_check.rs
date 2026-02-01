//! Module structure checking for build scripts
//!
//! Validates that required modules exist according to architectural guidelines

use std::path::PathBuf;
use std::process;

/// Configuration for module structure checking
#[derive(Debug, Clone)]
pub struct ModuleCheckConfig {
    /// Directory to scan (default: "src")
    pub src_dir: PathBuf,
    /// List of required module paths (relative to `src_dir`)
    pub required_modules: Vec<String>,
    /// Whether to fail the build if modules are missing (default: false, just warn)
    pub fail_on_missing: bool,
    /// Custom warning prefix
    pub warning_prefix: Option<String>,
}

impl Default for ModuleCheckConfig {
    fn default() -> Self {
        Self {
            src_dir: PathBuf::from("src"),
            required_modules: Vec::new(),
            fail_on_missing: false,
            warning_prefix: None,
        }
    }
}

impl ModuleCheckConfig {
    /// Create a new configuration with custom source directory
    pub fn new(src_dir: impl Into<PathBuf>) -> Self {
        Self { src_dir: src_dir.into(), ..Default::default() }
    }

    /// Set required modules to check for
    #[must_use]
    pub fn required_modules(mut self, modules: Vec<String>) -> Self {
        self.required_modules = modules;
        self
    }

    /// Set whether to fail build on missing modules
    #[must_use]
    pub fn fail_on_missing(mut self, fail: bool) -> Self {
        self.fail_on_missing = fail;
        self
    }

    /// Set custom warning prefix
    #[must_use]
    pub fn warning_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.warning_prefix = Some(prefix.into());
        self
    }
}

/// Check that required modules exist according to configuration
pub fn check_module_structure(config: &ModuleCheckConfig) {
    let mut missing_modules = Vec::new();

    for module in &config.required_modules {
        let path = config.src_dir.join(module);
        if !path.exists() {
            missing_modules.push(module.clone());
        }
    }

    if !missing_modules.is_empty() {
        let prefix =
            config.warning_prefix.as_deref().unwrap_or("WARNING: Missing expected modules");

        eprintln!("\n⚠️  {prefix}:");
        for module in &missing_modules {
            eprintln!("  src/{module}");
        }
        eprintln!("\nThis may be expected if modules are still being implemented.");
        eprintln!("See: docs/architecture.md for module structure\n");

        if config.fail_on_missing {
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = ModuleCheckConfig::new("custom/src")
            .required_modules(vec!["ecs/mod.rs".to_string(), "math.rs".to_string()])
            .fail_on_missing(true)
            .warning_prefix("CRITICAL");

        assert_eq!(config.src_dir, PathBuf::from("custom/src"));
        assert_eq!(config.required_modules.len(), 2);
        assert!(config.fail_on_missing);
        assert_eq!(config.warning_prefix, Some("CRITICAL".to_string()));
    }
}
