use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct EditorStateResponse {
    pub mode: String,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EntityInfo {
    pub id: u64,
    pub name: String,
    pub components: Vec<String>,
}

#[tauri::command]
pub fn get_editor_state() -> EditorStateResponse {
    EditorStateResponse {
        mode: "edit".to_string(),
        project_name: None,
        project_path: None,
    }
}

#[tauri::command]
pub fn open_project(path: String) -> Result<EditorStateResponse, String> {
    let project_root = std::path::Path::new(&path);
    if !project_root.join("game.toml").exists() {
        return Err("No game.toml found in selected directory".to_string());
    }

    let game_toml = std::fs::read_to_string(project_root.join("game.toml"))
        .map_err(|e| e.to_string())?;
    let name = engine_ops::build::parse_project_name(&game_toml)
        .unwrap_or_else(|| "Unknown Project".to_string());

    Ok(EditorStateResponse {
        mode: "edit".to_string(),
        project_name: Some(name),
        project_path: Some(path),
    })
}

#[tauri::command]
pub async fn open_project_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app
        .dialog()
        .file()
        .set_title("Open Silmaril Project")
        .blocking_pick_folder();

    Ok(folder.map(|f| f.to_string()))
}

#[tauri::command]
pub fn scan_project_entities(project_path: String) -> Result<Vec<EntityInfo>, String> {
    let root = Path::new(&project_path);
    if !root.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let mut components: Vec<String> = Vec::new();

    // Scan for component structs in .rs files under common project dirs
    let scan_dirs = ["shared/src", "server/src", "client/src", "src"];
    for dir in &scan_dirs {
        let scan_path = root.join(dir);
        if scan_path.is_dir() {
            scan_rust_components(&scan_path, &mut components);
        }
    }

    components.sort();
    components.dedup();

    // Build mock entities from discovered components
    // For MVP, each unique component type becomes a standalone entity entry
    let entities: Vec<EntityInfo> = components
        .iter()
        .enumerate()
        .map(|(i, name)| EntityInfo {
            id: (i + 1) as u64,
            name: name.clone(),
            components: vec![name.clone()],
        })
        .collect();

    Ok(entities)
}

/// Recursively scan a directory for Rust files containing `#[derive(...Component...)]`
/// or `pub struct` patterns that look like ECS components.
fn scan_rust_components(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rust_components(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                extract_component_names(&content, out);
            }
        }
    }
}

/// Extract struct names that appear after `#[derive(...Component...)]` annotations
/// or are named with common component suffixes.
fn extract_component_names(source: &str, out: &mut Vec<String>) {
    let lines: Vec<&str> = source.lines().collect();
    let mut found_component_derive = false;

    for line in &lines {
        let trimmed = line.trim();

        // Check for derive macros containing "Component"
        if trimmed.starts_with("#[derive(") && trimmed.contains("Component") {
            found_component_derive = true;
            continue;
        }

        // If previous line had Component derive, extract the struct name
        if found_component_derive && trimmed.starts_with("pub struct ") {
            if let Some(name) = trimmed
                .strip_prefix("pub struct ")
                .and_then(|rest| rest.split(|c: char| !c.is_alphanumeric() && c != '_').next())
            {
                if !name.is_empty() {
                    out.push(name.to_string());
                }
            }
            found_component_derive = false;
            continue;
        }

        // Reset if we hit a non-attribute, non-empty line without finding struct
        if found_component_derive && !trimmed.is_empty() && !trimmed.starts_with("#[") {
            found_component_derive = false;
        }
    }
}
