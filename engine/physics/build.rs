/// Build script for engine-physics
/// Enforces architectural rules at compile time
///
/// CLAUDE.md Requirements:
/// 1. No println!/eprintln!/dbg! in production code
/// 2. Error types must use define_error! macro
use engine_build_utils::{ErrorCheckConfig, PrintCheckConfig};

fn main() {
    // Tell cargo to rerun if source files change
    engine_build_utils::rerun_if_src_changed();

    // Check for print statements in production code
    let print_config = PrintCheckConfig::default();
    engine_build_utils::check_no_print_statements(&print_config);

    // Check that error types use define_error! macro
    let error_config = ErrorCheckConfig::default().skip_files(vec!["error.rs".to_string()]);
    engine_build_utils::check_error_types_use_macro(&error_config);
}
