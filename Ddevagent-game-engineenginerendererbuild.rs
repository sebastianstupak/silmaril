//! Build script for engine-renderer
//!
//! Compiles GLSL shaders to SPIR-V at build time

use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    // Tell cargo to rerun if shaders change
    println!("cargo:rerun-if-changed=shaders/");

    // Compile shaders
    compile_shaders()?;

    Ok(())
}

fn compile_shaders() -> Result<(), Box<dyn Error>> {
    let shader_dir = Path::new("shaders");
    let out_dir = Path::new("compiled_shaders");

    // Create output directory
    fs::create_dir_all(out_dir)?;

    // Initialize shader compiler
    let mut compiler = shaderc::Compiler::new()
        .ok_or("Failed to initialize shaderc compiler")?;

    let mut options = shaderc::CompileOptions::new()
        .ok_or("Failed to create compiler options")?;

    // Optimization for release builds
    #[cfg(not(debug_assertions))]
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);

    // Find all shader files
    for entry in fs::read_dir(shader_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_str().unwrap();

            // Determine shader kind from extension
            let shader_kind = if file_name.ends_with(".vert") {
                shaderc::ShaderKind::Vertex
            } else if file_name.ends_with(".frag") {
                shaderc::ShaderKind::Fragment
            } else if file_name.ends_with(".comp") {
                shaderc::ShaderKind::Compute
            } else {
                // Skip non-shader files
                continue;
            };

            // Read source
            let source = fs::read_to_string(&path)?;

            // Compile to SPIR-V
            println!("Compiling shader: {}", file_name);
            let binary_result = compiler.compile_into_spirv(
                &source,
                shader_kind,
                file_name,
                "main",
                Some(&options),
            )?;

            // Write compiled SPIR-V
            let output_path = out_dir.join(format!("{}.spv", file_name));
            fs::write(&output_path, binary_result.as_binary_u8())?;

            println!("  -> {}", output_path.display());
        }
    }

    println!("Shader compilation complete");
    Ok(())
}
