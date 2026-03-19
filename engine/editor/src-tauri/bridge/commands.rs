use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

use crate::viewport::native_viewport::{NativeViewport, ViewportBounds};

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

// ---------------------------------------------------------------------------
// Scene commands (AI agent API)
// ---------------------------------------------------------------------------

/// Unified scene command dispatcher.
///
/// AI agents (via MCP) and the frontend can call this with a command name and
/// a JSON-encoded argument object.  For MVP the backend simply validates and
/// echoes the command; the real scene state lives in the frontend.  When the
/// ECS backend is wired, this will mutate server-side state.
#[tauri::command]
pub fn scene_command(command: String, args: String) -> Result<serde_json::Value, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(&args).map_err(|e| format!("Invalid args JSON: {e}"))?;

    match command.as_str() {
        "select_entity" => {
            let _id = parsed.get("id");
            Ok(serde_json::json!({ "ok": true, "command": "select_entity" }))
        }
        "create_entity" => {
            let name = parsed
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("New Entity");
            Ok(serde_json::json!({ "ok": true, "command": "create_entity", "name": name }))
        }
        "delete_entity" => {
            let id = parsed
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("delete_entity requires 'id'")?;
            Ok(serde_json::json!({ "ok": true, "command": "delete_entity", "id": id }))
        }
        "duplicate_entity" => {
            let id = parsed
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("duplicate_entity requires 'id'")?;
            Ok(serde_json::json!({ "ok": true, "command": "duplicate_entity", "id": id }))
        }
        "move_entity" => {
            let id = parsed.get("id").and_then(|v| v.as_u64()).ok_or("move_entity requires 'id'")?;
            let x = parsed.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = parsed.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let z = parsed.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!({ "ok": true, "command": "move_entity", "id": id, "x": x, "y": y, "z": z }))
        }
        "pan_camera" => {
            let dx = parsed.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let dy = parsed.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!({ "ok": true, "command": "pan_camera", "dx": dx, "dy": dy }))
        }
        "zoom_camera" => {
            let delta = parsed.get("delta").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!({ "ok": true, "command": "zoom_camera", "delta": delta }))
        }
        "focus_entity" => {
            let id = parsed
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("focus_entity requires 'id'")?;
            Ok(serde_json::json!({ "ok": true, "command": "focus_entity", "id": id }))
        }
        "set_tool" => {
            let tool = parsed
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("select");
            Ok(serde_json::json!({ "ok": true, "command": "set_tool", "tool": tool }))
        }
        "get_state" => {
            Ok(serde_json::json!({ "ok": true, "command": "get_state" }))
        }
        _ => Err(format!("Unknown scene command: {command}")),
    }
}

// ---------------------------------------------------------------------------
// Native viewport (child window for Vulkan rendering)
// ---------------------------------------------------------------------------

/// Managed state holding the optional native viewport.
pub struct NativeViewportState(pub Mutex<Option<NativeViewport>>);

/// Create a native child window for the Vulkan viewport.
///
/// The child window is parented to the Tauri main window.  Svelte passes in
/// the desired bounds (in physical/device pixels).  Once created, a render
/// thread starts drawing into the child window.
#[tauri::command]
pub fn create_native_viewport(
    app: tauri::AppHandle,
    viewport_state: tauri::State<NativeViewportState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    // Don't create a second viewport if one already exists.
    {
        let guard = viewport_state.0.lock().unwrap();
        if guard.is_some() {
            tracing::warn!("Native viewport already exists; ignoring create_native_viewport");
            return Ok(());
        }
    }

    let bounds = ViewportBounds {
        x,
        y,
        width,
        height,
    };

    #[cfg(windows)]
    {
        use tauri::Manager;

        let window = app
            .get_webview_window("main")
            .ok_or("main window not found")?;
        let parent_hwnd = window.hwnd().map_err(|e| format!("Failed to get HWND: {e}"))?;

        tracing::info!(hwnd = ?parent_hwnd, x, y, width, height, "Creating native viewport");

        let mut vp = NativeViewport::new(parent_hwnd, bounds).map_err(|e| {
            tracing::error!(error = %e, "NativeViewport::new failed");
            e
        })?;
        vp.start_rendering().map_err(|e| {
            tracing::error!(error = %e, "start_rendering failed");
            e
        })?;

        tracing::info!("Native viewport created and rendering started");
        *viewport_state.0.lock().unwrap() = Some(vp);
    }

    #[cfg(not(windows))]
    {
        let _ = (app, bounds);
        return Err("Native viewport not yet implemented for this platform".into());
    }

    Ok(())
}

/// Reposition/resize the native viewport child window.
///
/// Called by Svelte whenever the viewport container's bounds change (e.g.
/// panel resize, window resize).
#[tauri::command]
pub fn resize_native_viewport(
    viewport_state: tauri::State<NativeViewportState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let guard = viewport_state.0.lock().unwrap();
    if let Some(ref vp) = *guard {
        vp.set_bounds(ViewportBounds {
            x,
            y,
            width,
            height,
        });
    }
    Ok(())
}

/// Stop the render thread and destroy the native viewport child window.
#[tauri::command]
pub fn destroy_native_viewport(
    viewport_state: tauri::State<NativeViewportState>,
) -> Result<(), String> {
    let mut guard = viewport_state.0.lock().unwrap();
    if let Some(mut vp) = guard.take() {
        vp.destroy();
    }
    Ok(())
}

/// Create a pop-out window for a panel.
#[tauri::command]
pub async fn create_popout_window(
    app: tauri::AppHandle,
    panel_id: String,
    title: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let label = format!(
        "popout-{}-{}",
        panel_id.replace(|c: char| !c.is_alphanumeric(), "-"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // Build the URL — in dev mode use the Vite dev server
    let url = format!("?panel={}", panel_id);

    tracing::info!(label = %label, panel = %panel_id, url = %url, "Creating pop-out window");

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(&title)
        .decorations(false)
        .inner_size(width as f64, height as f64)
        .position(x as f64, y as f64)
        .build()
        .map_err(|e| format!("Failed to create pop-out window: {e}"))?;

    Ok(())
}

/// Dock a panel back from a pop-out window into the main editor.
/// Emits a `dock-panel-back` event to the main window and closes the caller.
#[tauri::command]
pub async fn dock_panel_back(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    panel_id: String,
    zone: Option<String>,
) -> Result<(), String> {
    use tauri::{Emitter, Manager};

    let dock_zone = zone.unwrap_or_else(|| "center".to_string());
    tracing::info!(panel = %panel_id, zone = %dock_zone, window = %window.label(), "Docking panel back");

    // Emit event to the main window with zone info
    if let Some(main_window) = app.get_webview_window("main") {
        main_window
            .emit("dock-panel-back", serde_json::json!({ "panelId": panel_id, "zone": dock_zone }))
            .map_err(|e| format!("Failed to emit dock-panel-back: {e}"))?;
    } else {
        return Err("Main window not found".into());
    }

    // Close the pop-out window
    window
        .close()
        .map_err(|e| format!("Failed to close pop-out window: {e}"))?;

    Ok(())
}

/// Check if a pop-out window cursor is over the main editor window.
/// Returns `{ near: true, zone: "left"|"right"|"top"|"bottom"|"center" }`.
/// Also emits `popout-near` event to main window with zone info.
#[tauri::command]
pub async fn check_dock_proximity(
    app: tauri::AppHandle,
    popout_x: i32,
    popout_y: i32,
    popout_width: u32,
    popout_height: u32,
    cursor_x: Option<i32>,
    cursor_y: Option<i32>,
) -> Result<serde_json::Value, String> {
    use tauri::{Emitter, Manager};

    let main = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    let main_pos = main.outer_position().map_err(|e| e.to_string())?;
    let main_size = main.outer_size().map_err(|e| e.to_string())?;

    let mx1 = main_pos.x;
    let my1 = main_pos.y;
    let mw = main_size.width as i32;
    let mh = main_size.height as i32;
    let mx2 = mx1 + mw;
    let my2 = my1 + mh;

    // Use cursor position if provided, otherwise center of popout
    let cx = cursor_x.unwrap_or(popout_x + popout_width as i32 / 2);
    let cy = cursor_y.unwrap_or(popout_y + popout_height as i32 / 2);

    // Check if cursor is inside main window
    let near = cx >= mx1 && cx <= mx2 && cy >= my1 && cy <= my2;

    // Determine dock zone based on cursor position relative to main window
    let zone = if near {
        let rel_x = (cx - mx1) as f64 / mw as f64;
        let rel_y = (cy - my1) as f64 / mh as f64;

        if rel_x < 0.2 {
            "left"
        } else if rel_x > 0.8 {
            "right"
        } else if rel_y < 0.2 {
            "top"
        } else if rel_y > 0.8 {
            "bottom"
        } else {
            "center"
        }
    } else {
        "none"
    };

    // Emit to main window
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.emit(
            "popout-near",
            serde_json::json!({ "near": near, "zone": zone }),
        );
    }

    Ok(serde_json::json!({ "near": near, "zone": zone })
    )
}

/// Show or hide the native viewport child window.
/// Used to temporarily hide during panel drag operations so the
/// webview drop zone overlay is visible.
#[tauri::command]
pub fn set_viewport_visible(
    viewport_state: tauri::State<NativeViewportState>,
    visible: bool,
) -> Result<(), String> {
    let guard = viewport_state.0.lock().unwrap();
    if let Some(ref vp) = *guard {
        vp.set_visible(visible);
    }
    Ok(())
}
