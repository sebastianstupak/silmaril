//! Shared build-time utilities for engine modules
//!
//! Provides reusable build script functionality for enforcing
//! architectural rules and code quality standards across all engine crates.

mod error_check;
mod module_check;
mod print_check;
mod scanner;

pub use error_check::{check_error_types_use_macro, ErrorCheckConfig};
pub use module_check::{check_module_structure, ModuleCheckConfig};
pub use print_check::{check_no_print_statements, PrintCheckConfig};
pub use scanner::scan_directory;

/// Standard cargo rerun-if-changed for source files
pub fn rerun_if_src_changed() {
    println!("cargo:rerun-if-changed=src/");
}

/// Run all standard architectural checks with default configuration
pub fn run_standard_checks() {
    rerun_if_src_changed();

    let print_config = PrintCheckConfig::default();
    check_no_print_statements(&print_config);

    let error_config = ErrorCheckConfig::default();
    check_error_types_use_macro(&error_config);
}

/// Run all checks with custom configuration
pub fn run_checks(
    print_config: &PrintCheckConfig,
    module_config: Option<&ModuleCheckConfig>,
    error_config: Option<&ErrorCheckConfig>,
) {
    rerun_if_src_changed();

    check_no_print_statements(print_config);

    if let Some(config) = module_config {
        check_module_structure(config);
    }

    if let Some(config) = error_config {
        check_error_types_use_macro(config);
    }
}
