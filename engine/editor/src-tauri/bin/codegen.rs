/// Code generation binary for Silmaril editor TypeScript bindings.
///
/// Writes `bindings.ts` to the path given as the first CLI argument, defaulting
/// to `src/lib/bindings.ts` (relative to the workspace root).
///
/// Run via:
///   cargo xtask codegen
/// Or directly:
///   cargo run -p silmaril-editor --features codegen --bin silmaril-editor-codegen
fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src/lib/bindings.ts".to_string());

    silmaril_editor::generate_bindings(&output);
    eprintln!("Generated TypeScript bindings → {output}");
}
