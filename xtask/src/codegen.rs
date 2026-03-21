use anyhow::Result;
use std::process::Command;

use crate::utils::*;

/// Generate TypeScript bindings from Rust Tauri command types.
///
/// Runs the `silmaril-editor-codegen` binary (gated behind `--features codegen`)
/// which calls `generate_bindings()` in `lib.rs` via `tauri-specta` to write
/// `engine/editor/src/lib/bindings.ts`.
pub fn run_codegen() -> Result<()> {
    print_section("Generating TypeScript Bindings");

    let root = project_root()?;
    let output_path = root.join("engine/editor/src/lib/bindings.ts");

    // Ensure the output directory exists.
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let cargo = cargo();
    print_info(&format!(
        "Writing bindings to {}",
        output_path.display()
    ));

    let status = Command::new(&cargo)
        .args([
            "run",
            "-p",
            "silmaril-editor",
            "--features",
            "codegen",
            "--bin",
            "silmaril-editor-codegen",
            "--",
            output_path.to_str().expect("non-UTF8 path"),
        ])
        .current_dir(&root)
        .status()?;

    if !status.success() {
        anyhow::bail!("silmaril-editor-codegen failed");
    }

    print_success("TypeScript bindings generated");
    Ok(())
}

/// Verify that the committed `bindings.ts` is up to date.
///
/// Generates bindings to a temporary file and diffs it against the committed
/// file. Exits non-zero if different, printing a hint to run `cargo xtask codegen`.
pub fn run_check_bindings() -> Result<()> {
    print_section("Checking TypeScript Bindings Are Up To Date");

    let root = project_root()?;
    let committed_path = root.join("engine/editor/src/lib/bindings.ts");

    // Write to a temp file alongside the real one so relative paths in the
    // generated header stay the same.
    let tmp_path = root.join("engine/editor/src/lib/bindings.ts.tmp");

    // Ensure the output directory exists.
    if let Some(parent) = tmp_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let cargo = cargo();
    let status = Command::new(&cargo)
        .args([
            "run",
            "-p",
            "silmaril-editor",
            "--features",
            "codegen",
            "--bin",
            "silmaril-editor-codegen",
            "--",
            tmp_path.to_str().expect("non-UTF8 path"),
        ])
        .current_dir(&root)
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::bail!("silmaril-editor-codegen failed");
    }

    let generated = std::fs::read_to_string(&tmp_path)?;
    let _ = std::fs::remove_file(&tmp_path);

    if !committed_path.exists() {
        anyhow::bail!(
            "bindings.ts does not exist. Run: cargo xtask codegen"
        );
    }

    let committed = std::fs::read_to_string(&committed_path)?;

    if generated != committed {
        anyhow::bail!(
            "TypeScript bindings are out of date. Run: cargo xtask codegen"
        );
    }

    print_success("TypeScript bindings are up to date");
    Ok(())
}
