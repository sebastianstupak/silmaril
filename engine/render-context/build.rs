/// Build script for engine-render-context
/// Sets up OUT_DIR for shader loading and compiles any shaders in the shaders/ directory.
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
    let error_config = ErrorCheckConfig::default().skip_files(vec![
        "error.rs".to_string(),
    ]);
    engine_build_utils::check_error_types_use_macro(&error_config);

    // Compile shaders (gracefully handles missing compilers or missing directory)
    compile_shaders();
}

fn compile_shaders() {
    let shader_dir = Path::new("shaders");
    if !shader_dir.exists() {
        println!("cargo:warning=No shaders directory found in render-context, skipping shader compilation");
        // Still create the output directory so env!("OUT_DIR") paths don't panic at runtime
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let spv_dir = Path::new(&out_dir).join("shaders");
        fs::create_dir_all(&spv_dir).expect("Failed to create output directory");
        return;
    }

    println!("cargo:rerun-if-changed=shaders/");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let spv_dir = Path::new(&out_dir).join("shaders");
    fs::create_dir_all(&spv_dir).expect("Failed to create output directory");

    let compiler = find_shader_compiler();
    if compiler.is_none() {
        println!("cargo:warning=No shader compiler found (glslc or glslangValidator)");
        println!("cargo:warning=Shaders will not be compiled. Install Vulkan SDK for shader compilation support.");
        return;
    }

    let (compiler_cmd, compiler_type) = compiler.unwrap();

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

        if !matches!(ext, "vert" | "frag" | "comp" | "geom" | "tesc" | "tese") {
            continue;
        }

        let file_name = path.file_name().unwrap();
        let output_path = spv_dir.join(format!("{}.spv", file_name.to_str().unwrap()));

        let result = if compiler_type == 0 {
            Command::new(&compiler_cmd).arg(&path).arg("-o").arg(&output_path).output()
        } else {
            Command::new(&compiler_cmd).arg("-V").arg(&path).arg("-o").arg(&output_path).output()
        };

        match result {
            Ok(output) if output.status.success() => {
                println!("cargo:warning=Compiled shader: {:?}", file_name);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("cargo:warning=Failed to compile shader {:?}: {}", file_name, stderr);
            }
            Err(e) => {
                println!("cargo:warning=Failed to execute shader compiler: {}", e);
            }
        }
    }
}

fn find_shader_compiler() -> Option<(String, u8)> {
    if command_exists("glslc") {
        return Some(("glslc".to_string(), 0));
    }
    if command_exists("glslangValidator") {
        return Some(("glslangValidator".to_string(), 1));
    }
    None
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
