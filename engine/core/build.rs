/// Build script for engine-core
///
/// Enforces architectural rules at compile time
///
/// # CLAUDE.md Requirements
/// 1. No `println!`/`eprintln!`/`dbg!` in production code
/// 2. Proper module structure
/// 3. Error types must use `define_error!` macro
use engine_build_utils::{ErrorCheckConfig, ModuleCheckConfig, PrintCheckConfig};

fn main() {
    // Tell cargo to rerun if source files change
    engine_build_utils::rerun_if_src_changed();

    // Check for print statements in production code
    let print_config = PrintCheckConfig::default();
    engine_build_utils::check_no_print_statements(&print_config);

    // Check module structure
    let module_config = ModuleCheckConfig::default().required_modules(vec![
        "ecs/mod.rs".to_string(),
        "ecs/entity.rs".to_string(),
        "ecs/world.rs".to_string(),
        "ecs/storage.rs".to_string(),
        "ecs/query.rs".to_string(),
        "error.rs".to_string(),
        "platform.rs".to_string(),
    ]);
    engine_build_utils::check_module_structure(&module_config);

    // Check that error types use define_error! macro
    let error_config = ErrorCheckConfig::default().skip_files(vec!["error.rs".to_string()]);
    engine_build_utils::check_error_types_use_macro(&error_config);

    // Continue with normal build (flatbuffers compilation, etc.)
    // Add flatbuffer compilation if needed in the future
}
