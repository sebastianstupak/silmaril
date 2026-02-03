/// Build script for engine-renderer
/// Enforces architectural rules at compile time and compiles GLSL shaders to SPIR-V
///
/// CLAUDE.md Requirements:
/// 1. No println!/eprintln!/dbg! in production code
/// 2. Error types must use define_error! macro
/// 3. Compile GLSL shaders to SPIR-V at build time
///
/// Shader Compilation:
/// - Gracefully handles missing glslc/glslangValidator (warns but doesn't fail build)
/// - Compiles all *.vert, *.frag, *.comp shaders in shaders/ directory
/// - Outputs SPIR-V to $OUT_DIR/shaders/*.spv
use engine_build_utils::{ErrorCheckConfig, PrintCheckConfig};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell cargo to rerun if source files change
    engine_build_utils::rerun_if_src_changed();

    // Check for print statements in production code
    let print_config = PrintCheckConfig::default();
    engine_build_utils::check_no_print_statements(&print_config);

    // Check that error types use define_error! macro
    // All error types in renderer must use define_error! for consistency
    let error_config = ErrorCheckConfig::default().skip_files(vec![
        "error.rs".to_string(), // Defines error codes themselves
    ]);
    engine_build_utils::check_error_types_use_macro(&error_config);

    // Compile shaders (gracefully handles missing compilers)
    compile_shaders();
}

fn compile_shaders() {
    // Get shader directory
    let shader_dir = Path::new("shaders");
    if !shader_dir.exists() {
        println!("cargo:warning=No shaders directory found, skipping shader compilation");
        return;
    }

    // Tell Cargo to rerun if shaders change
    println!("cargo:rerun-if-changed=shaders/");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let spv_dir = Path::new(&out_dir).join("shaders");
    fs::create_dir_all(&spv_dir).expect("Failed to create output directory");

    // Check if shader compiler is available
    let compiler = find_shader_compiler();
    if compiler.is_none() {
        println!("cargo:warning=No shader compiler found (glslc or glslangValidator)");
        println!("cargo:warning=Shaders will not be compiled. Install Vulkan SDK for shader compilation support.");
        println!("cargo:warning=Application will need to load pre-compiled SPIR-V shaders.");
        return;
    }

    let (compiler_cmd, compiler_type) = compiler.unwrap();
    println!("cargo:warning=Using {} for shader compilation", compiler_cmd);

    // Compile all shader files
    let entries = match fs::read_dir(shader_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("cargo:warning=Failed to read shader directory: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e,
            None => continue,
        };

        // Only compile shader files (vert, frag, comp, geom, tesc, tese)
        if !matches!(ext, "vert" | "frag" | "comp" | "geom" | "tesc" | "tese") {
            continue;
        }

        let file_name = path.file_name().unwrap();
        let output_path = spv_dir.join(format!("{}.spv", file_name.to_str().unwrap()));

        println!("cargo:warning=Compiling shader: {:?} -> {:?}", path, output_path);

        let success = compile_shader(&path, &output_path, &compiler_cmd, compiler_type);
        if !success {
            println!(
                "cargo:warning=Failed to compile shader {:?}, continuing with other shaders",
                file_name
            );
        }
    }
}

/// Find available shader compiler
/// Returns (command, compiler_type) where compiler_type is 0 for glslc, 1 for glslangValidator
fn find_shader_compiler() -> Option<(String, u8)> {
    // Try glslc first (preferred, simpler output)
    if command_exists("glslc") {
        return Some(("glslc".to_string(), 0));
    }

    // Try glslangValidator as fallback
    if command_exists("glslangValidator") {
        return Some(("glslangValidator".to_string(), 1));
    }

    None
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Compile a single shader file
/// Returns true on success, false on failure
fn compile_shader(input: &Path, output: &Path, compiler: &str, compiler_type: u8) -> bool {
    let result = if compiler_type == 0 {
        // glslc
        Command::new(compiler).arg(input).arg("-o").arg(output).output()
    } else {
        // glslangValidator
        Command::new(compiler).arg("-V").arg(input).arg("-o").arg(output).output()
    };

    match result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Successfully compiled {:?}", input.file_name().unwrap());
            true
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "cargo:warning=Shader compilation failed for {:?}",
                input.file_name().unwrap()
            );
            println!("cargo:warning={}", stderr);
            false
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute shader compiler: {}", e);
            false
        }
    }
}
