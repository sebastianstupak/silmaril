use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::bridge::schema_registry::{ComponentSchema, ComponentSchemaRegistry};
use crate::viewport::native_viewport::{CameraState, NativeViewport, ViewportBounds};

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
    EditorStateResponse { mode: "edit".to_string(), project_name: None, project_path: None }
}

/// Returns all registered component schemas.
///
/// `ComponentSchema` derives `Serialize`, so Tauri serializes it directly —
/// no intermediate `serde_json::Value` step needed.
#[tauri::command]
pub fn get_component_schemas(
    state: tauri::State<ComponentSchemaState>,
) -> Result<Vec<ComponentSchema>, String> {
    let registry = state.0.lock().map_err(|e| e.to_string())?;
    Ok(registry.all().into_iter().cloned().collect())
}

/// Sets a single component field value for an entity.
///
/// Design-time: updates are tracked in the frontend scene state.
/// Play-time (future): this will forward to the live ECS.
#[tauri::command]
pub fn set_component_field(
    entity_id: u64,
    component: String,
    field: String,
    value: serde_json::Value,
) -> Result<(), String> {
    tracing::debug!(
        entity_id,
        component = %component,
        field = %field,
        value = %value,
        "set_component_field"
    );
    Ok(())
}

#[tauri::command]
pub fn open_project(path: String) -> Result<EditorStateResponse, String> {
    let project_root = std::path::Path::new(&path);
    if !project_root.join("game.toml").exists() {
        return Err("No game.toml found in selected directory".to_string());
    }

    let game_toml =
        std::fs::read_to_string(project_root.join("game.toml")).map_err(|e| e.to_string())?;
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

    let folder = app.dialog().file().set_title("Open Silmaril Project").blocking_pick_folder();

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
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("New Entity");
            Ok(serde_json::json!({ "ok": true, "command": "create_entity", "name": name }))
        }
        "delete_entity" => {
            let id =
                parsed.get("id").and_then(|v| v.as_u64()).ok_or("delete_entity requires 'id'")?;
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
            let id =
                parsed.get("id").and_then(|v| v.as_u64()).ok_or("move_entity requires 'id'")?;
            let x = parsed.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = parsed.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let z = parsed.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(
                serde_json::json!({ "ok": true, "command": "move_entity", "id": id, "x": x, "y": y, "z": z }),
            )
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
            let id =
                parsed.get("id").and_then(|v| v.as_u64()).ok_or("focus_entity requires 'id'")?;
            Ok(serde_json::json!({ "ok": true, "command": "focus_entity", "id": id }))
        }
        "set_tool" => {
            let tool = parsed.get("tool").and_then(|v| v.as_str()).unwrap_or("select");
            Ok(serde_json::json!({ "ok": true, "command": "set_tool", "tool": tool }))
        }
        "get_state" => Ok(serde_json::json!({ "ok": true, "command": "get_state" })),
        _ => Err(format!("Unknown scene command: {command}")),
    }
}

// ---------------------------------------------------------------------------
// Native viewport (Vulkan rendering — parent-HWND surface approach)
// ---------------------------------------------------------------------------

/// Registry of NativeViewport objects, one per OS window (HWND).
/// Tracks which `viewport_id` belongs to which HWND so commands can be
/// routed to the right Vulkan context regardless of which window they came from.
#[derive(Default)]
pub struct ViewportRegistry {
    by_hwnd: HashMap<isize, NativeViewport>,
    hwnd_by_id: HashMap<String, isize>,
    /// Camera state saved when a viewport is hidden (tab switch / panel move /
    /// pop-out).  Restored when the same ID is mounted in any window — even a
    /// different HWND — so the camera survives across pop-out and dock-back.
    saved_cameras: HashMap<String, CameraState>,
}

impl ViewportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_for_id(&self, id: &str) -> Option<&NativeViewport> {
        let hwnd = self.hwnd_by_id.get(id)?;
        self.by_hwnd.get(hwnd)
    }
}

/// Tauri managed state holding the component schema registry.
pub struct ComponentSchemaState(pub std::sync::Mutex<ComponentSchemaRegistry>);

pub struct NativeViewportState(pub Mutex<ViewportRegistry>);

impl Default for NativeViewportState {
    fn default() -> Self {
        Self(Mutex::new(ViewportRegistry::new()))
    }
}

impl NativeViewportState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Upsert a viewport instance for the calling window.
///
/// If no `NativeViewport` exists for the caller's HWND yet, one is created and
/// its render thread is started.  Then the named instance is added (or its
/// bounds updated).  This is the only entry-point needed for both initial
/// creation and panel-drag repositioning.
#[tauri::command]
pub fn create_native_viewport(
    window: tauri::WebviewWindow,
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let bounds = ViewportBounds { x, y, width, height };

    #[cfg(windows)]
    {
        let parent_hwnd = window.hwnd().map_err(|e| format!("Failed to get HWND: {e}"))?;
        let hwnd_isize = parent_hwnd.0 as isize;

        let mut registry = viewport_state.0.lock().unwrap();

        // Create a NativeViewport for this HWND if one doesn't exist yet.
        if let std::collections::hash_map::Entry::Vacant(e) = registry.by_hwnd.entry(hwnd_isize) {
            tracing::info!(hwnd = hwnd_isize, "Creating NativeViewport for window");
            let mut vp = NativeViewport::new(parent_hwnd).map_err(|e| {
                tracing::error!(error = %e, "NativeViewport::new failed");
                e
            })?;
            vp.start_rendering().map_err(|e| {
                tracing::error!(error = %e, "start_rendering failed");
                e
            })?;
            e.insert(vp);
        }

        // Map viewport_id → hwnd for future lookups.
        registry.hwnd_by_id.insert(viewport_id.clone(), hwnd_isize);

        // Pull saved camera before borrowing vp (avoids simultaneous mutable + immutable borrow).
        let saved_cam = registry.saved_cameras.remove(&viewport_id);

        // Upsert the instance (create or update bounds).
        if let Some(vp) = registry.by_hwnd.get(&hwnd_isize) {
            vp.upsert_instance(viewport_id.clone(), bounds);
            if let Some(cam) = saved_cam {
                vp.set_instance_camera(&viewport_id, cam);
                tracing::info!(id = %viewport_id, "Camera state restored for viewport instance");
            }
            tracing::info!(id = %viewport_id, x, y, width, height, "Viewport instance upserted");
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (window, bounds, viewport_id);
        return Err("Native viewport not yet implemented for this platform".into());
    }

    Ok(())
}

/// Update the scissor bounds for an existing viewport instance.
#[tauri::command]
pub fn resize_native_viewport(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_instance_bounds(&viewport_id, ViewportBounds { x, y, width, height });
    }
    Ok(())
}

/// Hide a viewport instance (panel unmounted — tab switch or panel drag).
///
/// Marks the instance invisible so Vulkan stops rendering it, but keeps the
/// instance alive in the registry to preserve camera state.  The HWND mapping
/// is removed so `create_native_viewport` can re-register the same ID later.
///
/// The `NativeViewport` (Vulkan context) for the window is never torn down
/// here — only when the OS window itself is closed.
#[tauri::command]
pub fn destroy_native_viewport(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
) -> Result<(), String> {
    let mut registry = viewport_state.0.lock().unwrap();
    // .copied() gives an owned isize — releases the borrow on hwnd_by_id before the block.
    if let Some(hwnd) = registry.hwnd_by_id.get(&viewport_id).copied() {
        // Save camera (owned return value, borrow of by_hwnd ends after .and_then).
        let saved_cam =
            registry.by_hwnd.get(&hwnd).and_then(|vp| vp.get_instance_camera(&viewport_id));
        if let Some(cam) = saved_cam {
            registry.saved_cameras.insert(viewport_id.clone(), cam);
        }
        // Mark invisible — keeps the Vulkan instance alive in the registry.
        if let Some(vp) = registry.by_hwnd.get(&hwnd) {
            vp.set_instance_visible(&viewport_id, false);
        }
        // Remove HWND mapping so create_native_viewport can re-register cleanly.
        registry.hwnd_by_id.remove(&viewport_id);
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

    let popout = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(&title)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .inner_size(width as f64, height as f64)
        .position(x as f64, y as f64)
        .build()
        .map_err(|e| format!("Failed to create pop-out window: {e}"))?;

    // Remove WS_CLIPCHILDREN so the Vulkan parent-HWND surface shows through
    // transparent WebView2 regions — same setup as the main window.
    // Also apply DWM rounded corners + no border to match the main window.
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::*;
        let hwnd = popout.hwnd().map_err(|e| format!("Failed to get pop-out HWND: {e}"))?;
        unsafe {
            let style = GetWindowLongW(hwnd, GWL_STYLE);
            SetWindowLongW(hwnd, GWL_STYLE, style & !(WS_CLIPCHILDREN.0 as i32));
        }
        tracing::info!(panel = %panel_id, "Removed WS_CLIPCHILDREN from pop-out window");
        crate::install_nc_subclass(hwnd);
        crate::apply_dwm_window_style(hwnd);
    }

    #[cfg(not(windows))]
    let _ = popout;

    Ok(())
}

/// Dock a panel back from a pop-out window into the main editor.
/// Emits a `dock-panel-back` event to the main window and closes the caller.
#[tauri::command]
pub async fn dock_panel_back(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    viewport_state: tauri::State<'_, NativeViewportState>,
    panel_id: String,
    zone: Option<String>,
) -> Result<(), String> {
    use tauri::{Emitter, Manager};

    let dock_zone = zone.unwrap_or_else(|| "center".to_string());
    tracing::info!(panel = %panel_id, zone = %dock_zone, window = %window.label(), "Docking panel back");

    // Save viewport camera and deregister the ID BEFORE emitting the event.
    // This prevents a race where the main window's createNativeViewport runs
    // before the pop-out's destroyNativeViewport cleanup, missing the camera.
    // Deregistering also makes the subsequent destroyNativeViewport a no-op so
    // it can't accidentally hide the instance in the main window.
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd_isize = hwnd.0 as isize;
        let mut registry = viewport_state.0.lock().unwrap();
        let saved_cam = registry
            .by_hwnd
            .get(&hwnd_isize)
            .and_then(|vp: &NativeViewport| vp.get_instance_camera(&panel_id));
        if let Some(cam) = saved_cam {
            registry.saved_cameras.insert(panel_id.clone(), cam);
            tracing::info!(panel = %panel_id, "Camera saved for dock-back");
        }
        // Deregister — pop-out cleanup's destroyNativeViewport becomes no-op.
        registry.hwnd_by_id.remove(&panel_id);
    }

    // Emit event to the main window with zone info
    if let Some(main_window) = app.get_webview_window("main") {
        main_window
            .emit("dock-panel-back", serde_json::json!({ "panelId": panel_id, "zone": dock_zone }))
            .map_err(|e| format!("Failed to emit dock-panel-back: {e}"))?;
    } else {
        return Err("Main window not found".into());
    }

    // Close the pop-out window
    window.close().map_err(|e| format!("Failed to close pop-out window: {e}"))?;

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Pop-out window controls — called via invoke() from the pop-out webview.
//
// These exist because the JS WebviewWindow API (plugin:window|*) can fail to
// route correctly from dynamically-created webviews in Tauri 2.  Custom
// commands registered in invoke_handler receive the calling WebviewWindow
// directly from Tauri, which is always reliable.
// ──────────────────────────────────────────────────────────────────────────────

/// Minimize the window that invokes this command.
#[tauri::command]
pub fn window_minimize(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

/// Toggle maximize/restore the window that invokes this command.
#[tauri::command]
pub fn window_toggle_maximize(window: tauri::WebviewWindow) -> Result<(), String> {
    // Tauri 2 has no toggle_maximize(); check state and call the right method.
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

/// Destroy (close) the window that invokes this command.
/// Uses destroy() to avoid a Tauri/Windows bug where close() silently fails
/// after a minimize/maximize cycle (tauri-apps/tauri#9504).
#[tauri::command]
pub fn window_close(window: tauri::WebviewWindow) -> Result<(), String> {
    window.destroy().map_err(|e| e.to_string())
}

/// Start a window drag from the calling window.
///
/// On Windows this uses the standard Win32 custom-titlebar technique:
///   ReleaseCapture() frees mouse capture from WebView2, then
///   PostMessageW(WM_NCLBUTTONDOWN, HTCAPTION) tells the OS to begin a
///   window-move operation.  PostMessage returns immediately; the OS move
///   loop runs in the message pump while the user holds the button down.
///
/// On other platforms falls back to Tauri's start_dragging().
#[tauri::command]
pub fn window_start_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, HTCAPTION, WM_NCLBUTTONDOWN};
        let raw = window.hwnd().map_err(|e| format!("get HWND: {e}"))?;
        // Reconstruct HWND from the raw pointer to match our local windows crate type.
        let hwnd = HWND(raw.0);
        unsafe {
            // Release WebView2's mouse capture so the OS can take over drag tracking.
            let _ = ReleaseCapture();
            // PostMessage (async) queues WM_NCLBUTTONDOWN/HTCAPTION; returns
            // immediately while the OS handles the move loop.
            let _ =
                PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    window.start_dragging().map_err(|e| e.to_string())
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

    let main = app.get_webview_window("main").ok_or("main window not found")?;

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
        let _ = main_win.emit("popout-near", serde_json::json!({ "near": near, "zone": zone }));
    }

    Ok(serde_json::json!({ "near": near, "zone": zone }))
}

/// Show or hide a viewport instance.
#[tauri::command]
pub fn set_viewport_visible(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    visible: bool,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_instance_visible(&viewport_id, visible);
    }
    Ok(())
}

/// Orbit the camera for a specific viewport instance.
#[tauri::command]
pub fn viewport_camera_orbit(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    dx: f32,
    dy: f32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_orbit(&viewport_id, dx, dy);
    }
    Ok(())
}

/// Pan the camera for a specific viewport instance.
#[tauri::command]
pub fn viewport_camera_pan(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    dx: f32,
    dy: f32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_pan(&viewport_id, dx, dy);
    }
    Ok(())
}

/// Zoom the camera for a specific viewport instance.
#[tauri::command]
pub fn viewport_camera_zoom(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    delta: f32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_zoom(&viewport_id, delta);
    }
    Ok(())
}

/// Reset the camera for a specific viewport instance to its default state.
#[tauri::command]
pub fn viewport_camera_reset(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_reset(&viewport_id);
    }
    Ok(())
}

/// Show or hide the grid for a specific viewport instance.
#[tauri::command]
pub fn viewport_set_grid_visible(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    visible: bool,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_grid_visible(&viewport_id, visible);
    }
    Ok(())
}

/// Set absolute camera yaw and pitch for a specific viewport instance.
/// Used for snap-to-axis from the gizmo — bypasses the pixel-delta scaling
/// of `viewport_camera_orbit`.
#[tauri::command]
pub fn viewport_camera_set_orientation(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    yaw: f32,
    pitch: f32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_set_orientation(&viewport_id, yaw, pitch);
    }
    Ok(())
}

/// Switch between perspective and orthographic projection for a viewport instance.
#[tauri::command]
pub fn viewport_set_projection(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    is_ortho: bool,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_projection(&viewport_id, is_ortho);
    }
    Ok(())
}

/// Begin monitoring a pop-out window drag for dock-back gesture.
///
/// Spawns a background thread that polls `GetAsyncKeyState(VK_LBUTTON)` to
/// detect button release — JS `mouseup` does not fire during the OS window-move
/// loop started by `PostMessage(WM_NCLBUTTONDOWN, HTCAPTION)`.
///
/// While the button is held the thread emits `popout-near` events to the main
/// window so it can show the dock-zone overlay.  On release:
///   • cursor over main editor → save camera, emit `dock-panel-back`, destroy pop-out
///   • cursor elsewhere        → clear the overlay
#[tauri::command]
pub fn start_dock_drag(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    panel_id: String,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        let popout_hwnd_raw =
            window.hwnd().map_err(|e| format!("get pop-out HWND: {e}"))?.0 as isize;
        let window_label = window.label().to_string();

        // Obtain the main HWND here on the command thread (Tauri runtime), not
        // inside the background polling thread where hwnd() may be unsafe to call.
        let main_hwnd_raw = app
            .get_webview_window("main")
            .ok_or("main window not found")?
            .hwnd()
            .map_err(|e| format!("get main HWND: {e}"))?
            .0 as isize;

        std::thread::spawn(move || {
            dock_drag_thread(app, window_label, panel_id, popout_hwnd_raw, main_hwnd_raw);
        });
    }
    #[cfg(not(windows))]
    let _ = (app, window, panel_id);
    Ok(())
}

/// Broadcast a settings change to every open window.
///
/// JS `emit()` from `@tauri-apps/api/event` only delivers events to the current
/// window's own IPC channel.  `AppHandle::emit()` here broadcasts to the Rust
/// event bus which re-delivers to ALL registered frontend listeners, including
/// pop-out windows.
#[tauri::command]
pub fn broadcast_settings(
    app: tauri::AppHandle,
    theme: String,
    font_size: f64,
    language: String,
) -> Result<(), String> {
    use tauri::Emitter;
    app.emit(
        "settings-changed",
        serde_json::json!({ "theme": theme, "fontSize": font_size, "language": language }),
    )
    .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn dock_drag_thread(
    app: tauri::AppHandle,
    window_label: String,
    panel_id: String,
    popout_hwnd_raw: isize,
    main_hwnd_raw: isize,
) {
    use tauri::{Emitter, Manager};
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

    // Brief delay so the OS drag loop is running before we start polling.
    std::thread::sleep(std::time::Duration::from_millis(60));

    let main_win = match app.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };

    // HWND was obtained on the command thread and passed in — safe to use here.
    let main_hwnd = HWND(main_hwnd_raw as *mut _);

    tracing::debug!(panel = %panel_id, "dock_drag_thread: polling started");

    loop {
        std::thread::sleep(std::time::Duration::from_millis(33)); // ~30 fps polling

        // Bit 15: key currently pressed (GetAsyncKeyState is thread-safe).
        let button_down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 };

        let mut cursor = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut cursor);
        }
        let cx = cursor.x;
        let cy = cursor.y;

        // GetWindowRect uses the same Win32 screen coordinate system as GetCursorPos,
        // avoiding any Tauri PhysicalPosition / DPI conversion mismatch.
        let mut main_rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(main_hwnd, &mut main_rect);
        }

        let near = cx >= main_rect.left
            && cx <= main_rect.right
            && cy >= main_rect.top
            && cy <= main_rect.bottom;

        let mw = (main_rect.right - main_rect.left) as f64;
        let mh = (main_rect.bottom - main_rect.top) as f64;
        let rel_x = if mw > 0.0 { (cx - main_rect.left) as f64 / mw } else { 0.0 };
        let rel_y = if mh > 0.0 { (cy - main_rect.top) as f64 / mh } else { 0.0 };

        let zone = if near {
            if rel_x < 0.2 {
                "left"
            } else if rel_x > 0.8 {
                "right"
            } else if rel_y < 0.15 {
                "top"
            } else if rel_y > 0.85 {
                "bottom"
            } else {
                "center"
            }
        } else {
            "none"
        };

        let _ = main_win.emit(
            "popout-near",
            serde_json::json!({
                "near": near,
                "zone": zone,
                "panelId": panel_id,
                "relX": rel_x,
                "relY": rel_y,
            }),
        );

        if !button_down {
            tracing::debug!(panel = %panel_id, near, zone, "dock_drag_thread: button released");
            if near {
                // Save viewport camera before notifying main so it's available when
                // createNativeViewport is called after the dock-panel-back event.
                let vp_state = app.state::<NativeViewportState>();
                {
                    let mut registry = vp_state.0.lock().unwrap();
                    let saved_cam = registry
                        .by_hwnd
                        .get(&popout_hwnd_raw)
                        .and_then(|vp| vp.get_instance_camera(&panel_id));
                    if let Some(cam) = saved_cam {
                        registry.saved_cameras.insert(panel_id.clone(), cam);
                        tracing::info!(panel = %panel_id, "Camera saved via drag-dock");
                    }
                    // Deregister so the pop-out's destroyNativeViewport cleanup is a no-op.
                    registry.hwnd_by_id.remove(&panel_id);
                }

                let _ = main_win.emit(
                    "dock-panel-back",
                    serde_json::json!({ "panelId": panel_id, "zone": zone }),
                );
                if let Some(popout_win) = app.get_webview_window(&window_label) {
                    let _ = popout_win.destroy();
                }
            } else {
                // Drag ended away from main window — clear the overlay.
                let _ = main_win
                    .emit("popout-near", serde_json::json!({ "near": false, "zone": "none" }));
            }
            break;
        }
    }
}
