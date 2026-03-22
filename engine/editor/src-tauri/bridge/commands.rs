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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u64>,
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
/// Updates both the frontend scene state (via Tauri event) and the live ECS
/// world.  For Transform fields the ECS component is mutated in-place and an
/// `entity-transform-changed` event is emitted so the frontend stays in sync.
#[tauri::command]
pub fn set_component_field(
    entity_id: u64,
    component: String,
    field: String,
    value: serde_json::Value,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tracing::debug!(
        entity_id,
        component = %component,
        field = %field,
        value = %value,
        "set_component_field"
    );

    // For Transform component fields, also update the live ECS world.
    if component == "Transform" {
        debug_assert!(entity_id <= u32::MAX as u64, "entity_id truncation");
        let entity = engine_core::Entity::new(entity_id as u32, 0); // FIXME: hardcodes generation 0 — will break after any entity slot reuse
        let val = value.as_f64().ok_or("value must be a number")? as f32;
        let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
        if let Some(t) = world.get_mut::<engine_core::Transform>(entity) {
            match field.as_str() {
                "position.x" => t.position.x = val,
                "position.y" => t.position.y = val,
                "position.z" => t.position.z = val,
                "rotation.x" => t.rotation.x = val,
                "rotation.y" => t.rotation.y = val,
                "rotation.z" => t.rotation.z = val,
                "rotation.w" => t.rotation.w = val,
                "scale.x" => t.scale.x = val,
                "scale.y" => t.scale.y = val,
                "scale.z" => t.scale.z = val,
                _ => {}
            }
        } else {
            return Err(format!("Entity {entity_id} has no Transform component"));
        }
        // Read back the full transform to emit the event.
        if let Some(t) = world.get::<engine_core::Transform>(entity) {
            let pos = [t.position.x, t.position.y, t.position.z];
            let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
            let scl = [t.scale.x, t.scale.y, t.scale.z];
            use tauri::Emitter;
            app.emit(
                "entity-transform-changed",
                serde_json::json!({
                    "id": entity_id, "position": pos, "rotation": rot, "scale": scl
                }),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Create a new entity in the live ECS world with a default Transform.
///
/// Emits `entity-created` so the frontend can add it to the scene graph.
#[tauri::command]
pub fn create_entity(
    name: Option<String>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<u64, String> {
    use tauri::Emitter;

    let entity_id = {
        let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
        let entity = world.spawn();
        world.add(entity, engine_core::Transform::default());
        entity.id() as u64
    };
    let entity_name = name.unwrap_or_else(|| format!("Entity {entity_id}"));
    app.emit(
        "entity-created",
        serde_json::json!({ "id": entity_id, "name": entity_name }),
    )
    .map_err(|e| e.to_string())?;
    tracing::info!(entity_id, name = %entity_name, "Entity created");
    Ok(entity_id)
}

/// Remove an entity from the live ECS world.
///
/// Emits `entity-deleted` so the frontend can remove it from the scene graph.
#[tauri::command]
pub fn delete_entity(
    entity_id: u64,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
    debug_assert!(entity_id <= u32::MAX as u64, "entity_id truncation");
    world.despawn(engine_core::Entity::new(entity_id as u32, 0)); // FIXME: hardcodes generation 0 — will break after any entity slot reuse
    drop(world);

    app.emit(
        "entity-deleted",
        serde_json::json!({ "id": entity_id }),
    )
    .map_err(|e| e.to_string())?;
    tracing::info!(entity_id, "Entity deleted");
    Ok(())
}

/// Create a new child entity under an existing parent.
///
/// Spawns the entity in the ECS world and emits `entity-created` with a
/// `parentId` field so the frontend can place it under the right node.
#[tauri::command]
pub fn create_entity_child(
    parent_id: u64,
    name: Option<String>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<u64, String> {
    use tauri::Emitter;

    // Ensure the parent exists before creating the child.
    {
        debug_assert!(parent_id <= u32::MAX as u64, "parent_id truncation");
        let parent_entity = engine_core::Entity::new(parent_id as u32, 0);
        let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
        if !world.is_alive(parent_entity) {
            world.spawn_with_id(parent_entity);
            world.add(parent_entity, engine_core::Transform::default());
        }
    }

    let entity_id = {
        let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
        let entity = world.spawn();
        world.add(entity, engine_core::Transform::default());
        entity.id() as u64
    };
    let entity_name = name.unwrap_or_else(|| format!("Entity {entity_id}"));
    app.emit(
        "entity-created",
        serde_json::json!({ "id": entity_id, "name": entity_name, "parentId": parent_id }),
    )
    .map_err(|e| e.to_string())?;
    tracing::info!(entity_id, parent_id, name = %entity_name, "Child entity created");
    Ok(entity_id)
}

#[tauri::command]
pub fn open_project(
    project_state: tauri::State<ProjectState>,
    path: String,
) -> Result<EditorStateResponse, String> {
    let project_root = std::path::Path::new(&path);
    if !project_root.join("game.toml").exists() {
        return Err("No game.toml found in selected directory".to_string());
    }

    let game_toml =
        std::fs::read_to_string(project_root.join("game.toml")).map_err(|e| e.to_string())?;
    let name = engine_ops::build::parse_project_name(&game_toml)
        .unwrap_or_else(|| "Unknown Project".to_string());

    // Store the project path so component commands can find the scene file.
    *project_state.0.lock().map_err(|e| e.to_string())? =
        Some(std::path::PathBuf::from(&path));

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
            parent_id: None,
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

    pub fn get_for_id(&self, id: &str) -> Option<&NativeViewport> {
        let hwnd = self.hwnd_by_id.get(id)?;
        self.by_hwnd.get(hwnd)
    }

    /// Return any one active viewport (used for screenshot capture).
    pub fn first_viewport(&self) -> Option<&NativeViewport> {
        self.by_hwnd.values().next()
    }
}

/// Tauri managed state holding the component schema registry.
pub struct ComponentSchemaState(pub std::sync::Mutex<ComponentSchemaRegistry>);

/// Tauri managed state holding the currently open project path.
pub struct ProjectState(pub std::sync::Mutex<Option<std::path::PathBuf>>);

impl ProjectState {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }
}

pub struct NativeViewportState {
    pub registry: Mutex<ViewportRegistry>,
    /// Active gizmo drag operation, if any. Cleared on `gizmo_drag_end`.
    pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,
    /// Current gizmo mode: 0 = Move, 1 = Rotate, 2 = Scale.
    /// Stored as `Arc<AtomicU8>` so it can be cloned into the render thread.
    pub gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    /// Which gizmo axis is currently hovered (0 = none, 1..=6 = axes).
    /// Stored as `Arc<AtomicU8>` so it can be cloned into the render thread.
    pub hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    /// The entity currently selected in the hierarchy, or `None`.
    /// Stored as `Arc<Mutex<Option<u64>>>` so the render thread can read it.
    pub selected_entity_id: std::sync::Arc<Mutex<Option<u64>>>,
}

impl Default for NativeViewportState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ViewportRegistry::new()),
            drag_state: Mutex::new(None),
            gizmo_mode: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            hovered_gizmo_axis: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            selected_entity_id: std::sync::Arc::new(Mutex::new(None)),
        }
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
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    asset_manager_state: tauri::State<'_, crate::state::AssetManagerState>,
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

        let mut registry = viewport_state.registry.lock().unwrap();

        // Create a NativeViewport for this HWND if one doesn't exist yet.
        if let std::collections::hash_map::Entry::Vacant(e) = registry.by_hwnd.entry(hwnd_isize) {
            tracing::info!(hwnd = hwnd_isize, "Creating NativeViewport for window");
            let selected_entity_id = std::sync::Arc::clone(&viewport_state.selected_entity_id);
            let gizmo_mode = std::sync::Arc::clone(&viewport_state.gizmo_mode);
            let hovered_gizmo_axis = std::sync::Arc::clone(&viewport_state.hovered_gizmo_axis);
            let asset_manager = asset_manager_state.0.clone();
            let mut vp = NativeViewport::new(parent_hwnd, world_state.inner().0.clone(), selected_entity_id, gizmo_mode, hovered_gizmo_axis, asset_manager).map_err(|e| {
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let mut registry = viewport_state.registry.lock().unwrap();
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
        let mut registry = viewport_state.registry.lock().unwrap();
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

/// Begin a window resize from JavaScript.
///
/// `direction` is one of: "n", "s", "e", "w", "ne", "nw", "se", "sw".
///
/// Uses the same WM_NCLBUTTONDOWN technique as window_start_drag but with
/// the appropriate resize hit code instead of HTCAPTION.  This is needed
/// because WebView2 intercepts WM_NCHITTEST before it reaches the parent
/// HWND, so cursor-based hit testing in the WNDPROC never fires.
#[tauri::command]
pub fn window_start_resize(window: tauri::WebviewWindow, direction: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
        use windows::Win32::UI::WindowsAndMessaging::{
            PostMessageW, WM_NCLBUTTONDOWN,
            HTLEFT, HTRIGHT, HTTOP, HTBOTTOM,
            HTTOPLEFT, HTTOPRIGHT, HTBOTTOMLEFT, HTBOTTOMRIGHT,
        };
        let hit: usize = match direction.as_str() {
            "n"  => HTTOP as usize,
            "s"  => HTBOTTOM as usize,
            "e"  => HTRIGHT as usize,
            "w"  => HTLEFT as usize,
            "ne" => HTTOPRIGHT as usize,
            "nw" => HTTOPLEFT as usize,
            "se" => HTBOTTOMRIGHT as usize,
            "sw" => HTBOTTOMLEFT as usize,
            other => return Err(format!("unknown resize direction: {other}")),
        };
        let raw = window.hwnd().map_err(|e| format!("get HWND: {e}"))?;
        let hwnd = HWND(raw.0);
        unsafe {
            let _ = ReleaseCapture();
            let _ = PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(hit), LPARAM(0));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        use tauri::window::ResizeDirection;
        let dir = match direction.as_str() {
            "n"  => ResizeDirection::North,
            "s"  => ResizeDirection::South,
            "e"  => ResizeDirection::East,
            "w"  => ResizeDirection::West,
            "ne" => ResizeDirection::NorthEast,
            "nw" => ResizeDirection::NorthWest,
            "se" => ResizeDirection::SouthEast,
            "sw" => ResizeDirection::SouthWest,
            other => return Err(format!("unknown resize direction: {other}")),
        };
        window.start_resizing(dir).map_err(|e| e.to_string())
    }
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
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
    let registry = viewport_state.registry.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_projection(&viewport_id, is_ortho);
    }
    Ok(())
}

/// Update which entity is selected in the viewport gizmo renderer.
///
/// Called by the frontend whenever `selectedEntityId` changes in the hierarchy.
/// Pass `None` to deselect.
///
/// Also ensures the entity exists in the ECS world (the frontend creates
/// entities optimistically without always going through Tauri IPC) and focuses
/// every active viewport camera on the entity's Transform position.
#[tauri::command]
pub fn set_selected_entity(
    entity_id: Option<u64>,
    viewport_state: tauri::State<'_, NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
) -> Result<(), String> {
    tracing::debug!(entity_id = ?entity_id, "set_selected_entity");
    *viewport_state
        .selected_entity_id
        .lock()
        .map_err(|e| e.to_string())? = entity_id;

    if let Some(id) = entity_id {
        debug_assert!(id <= u32::MAX as u64, "entity_id truncation");
        let entity = engine_core::Entity::new(id as u32, 0);

        // Ensure entity exists in the ECS world.
        let pos = {
            let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
            if !world.is_alive(entity) {
                world.spawn_with_id(entity);
                world.add(entity, engine_core::Transform::default());
            }
            world
                .get::<engine_core::Transform>(entity)
                .map(|t| [t.position.x, t.position.y, t.position.z])
                .unwrap_or([0.0, 0.0, 0.0])
        };

        // Focus every active viewport camera to the entity's position.
        let registry = viewport_state.registry.lock().map_err(|e| e.to_string())?;
        let ids: Vec<String> = registry.hwnd_by_id.keys().cloned().collect();
        for vid in ids {
            if let Some(vp) = registry.get_for_id(&vid) {
                vp.camera_focus(&vid, pos);
            }
        }
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
                    let mut registry = vp_state.registry.lock().unwrap();
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

// ── Asset scanning ────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct AssetInfo {
    pub path: String,
    pub asset_type: String,
}

fn ext_to_asset_type(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "webp" => "texture",
        "gltf" | "glb" => "mesh",
        "wav" | "ogg" | "mp3" => "audio",
        "toml" => "config",
        _ => "unknown",
    }
}

#[tauri::command]
pub fn scan_assets(project_path: String) -> Result<Vec<AssetInfo>, String> {
    let root = std::path::Path::new(&project_path);
    if !root.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    const ASSET_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gltf", "glb", "wav", "ogg", "mp3", "toml"];
    let mut assets = Vec::new();

    fn walk(dir: &std::path::Path, exts: &[&str], out: &mut Vec<AssetInfo>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, exts, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if exts.contains(&ext.to_lowercase().as_str()) {
                    out.push(AssetInfo {
                        path: path.to_string_lossy().to_string(),
                        asset_type: ext_to_asset_type(&ext.to_lowercase()).to_string(),
                    });
                }
            }
        }
    }

    walk(root, ASSET_EXTS, &mut assets);
    Ok(assets)
}

// ── Minimal scene representation for editor persistence ───────────────────
//
// Mirrors engine_ops::scene but lives here to avoid pulling serde_yaml /
// bincode transitive deps into the editor binary.  Persisted as JSON in
// `<project>/scenes/main.scene.json`.

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SceneFile {
    name: String,
    entities: Vec<SceneFileEntity>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SceneFileEntity {
    id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    components: Vec<SceneFileComponent>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SceneFileComponent {
    type_name: String,
    #[serde(default)]
    data: serde_json::Value,
}

/// Load `<project>/scenes/main.scene.json`, or return an empty scene.
fn load_or_create_scene(project_path: &std::path::Path) -> Result<SceneFile, String> {
    let scene_path = project_path.join("scenes").join("main.scene.json");
    if scene_path.exists() {
        let raw = std::fs::read_to_string(&scene_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    } else {
        Ok(SceneFile { name: "main".into(), entities: Vec::new() })
    }
}

/// Save to `<project>/scenes/main.scene.json`, creating the directory if needed.
fn save_scene(project_path: &std::path::Path, scene: &SceneFile) -> Result<(), String> {
    let scene_path = project_path.join("scenes").join("main.scene.json");
    if let Some(dir) = scene_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(scene).map_err(|e| e.to_string())?;
    std::fs::write(&scene_path, json).map_err(|e| e.to_string())
}

/// Add a component to an entity in the persisted scene.
///
/// Loads `<project>/scenes/main.scene.json`, inserts the component (with empty
/// data object) into the matching entity (or creates the entity if missing),
/// then saves the file back.  No-op when no project is open (browser mode).
#[tauri::command]
pub fn add_component(
    project_state: tauri::State<ProjectState>,
    entity_id: u64,
    component: String,
) -> Result<(), String> {
    if component.is_empty() {
        return Err("component name required".into());
    }
    let guard = project_state.0.lock().map_err(|e| e.to_string())?;
    let Some(ref project_path) = *guard else {
        tracing::debug!(entity_id, component = %component, "add_component (no project open)");
        return Ok(());
    };

    let mut scene = load_or_create_scene(project_path)?;

    if let Some(entity) = scene.entities.iter_mut().find(|e| e.id == entity_id) {
        if !entity.components.iter().any(|c| c.type_name == component) {
            entity.components.push(SceneFileComponent {
                type_name: component.clone(),
                data: serde_json::Value::Object(Default::default()),
            });
        }
    } else {
        scene.entities.push(SceneFileEntity {
            id: entity_id,
            name: None,
            components: vec![SceneFileComponent {
                type_name: component.clone(),
                data: serde_json::Value::Object(Default::default()),
            }],
        });
    }

    save_scene(project_path, &scene)?;
    tracing::info!(entity_id, component = %component, "add_component");
    Ok(())
}

/// Remove a component from an entity in the persisted scene.
///
/// Loads `<project>/scenes/main.scene.json`, removes the named component from
/// the matching entity, then saves the file back.  No-op when no project is
/// open (browser mode).
#[tauri::command]
pub fn remove_component(
    project_state: tauri::State<ProjectState>,
    entity_id: u64,
    component: String,
) -> Result<(), String> {
    if component.is_empty() {
        return Err("component name required".into());
    }
    let guard = project_state.0.lock().map_err(|e| e.to_string())?;
    let Some(ref project_path) = *guard else {
        tracing::debug!(entity_id, component = %component, "remove_component (no project open)");
        return Ok(());
    };

    let mut scene = load_or_create_scene(project_path)?;

    if let Some(entity) = scene.entities.iter_mut().find(|e| e.id == entity_id) {
        entity.components.retain(|c| c.type_name != component);
    }

    save_scene(project_path, &scene)?;
    tracing::info!(entity_id, component = %component, "remove_component");
    Ok(())
}

// ---------------------------------------------------------------------------
// Mesh assignment
// ---------------------------------------------------------------------------

/// Assign a mesh to an entity by writing to its template and syncing to ECS.
///
/// `mesh_path` is either `"builtin://cube"` (etc.) or a project-relative path
/// like `"assets/models/robot.glb"`.
///
/// Executes a `SetComponent { type_name: "MeshRenderer", data: { mesh_path } }`
/// command via the template's `CommandProcessor`, then calls
/// [`crate::bridge::template_commands::sync_mesh_renderer_to_ecs`] to push the
/// change into the live ECS world.
#[tauri::command]
pub fn assign_mesh(
    entity_id: u64,
    template_path: String,
    mesh_path: String,
    template_state: tauri::State<'_, std::sync::Mutex<crate::bridge::template_commands::EditorState>>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    asset_state: tauri::State<'_, crate::state::AssetManagerState>,
    project_state: tauri::State<'_, ProjectState>,
) -> Result<(), String> {
    use crate::bridge::template_commands::{sync_mesh_renderer_to_ecs, template_execute_inner};
    use engine_ops::command::TemplateCommand;

    // Build SetComponent payload for MeshRenderer
    let component_data = serde_json::json!({ "mesh_path": mesh_path });

    // Write to template via CommandProcessor
    let new_template_state = {
        let cmd = TemplateCommand::SetComponent {
            id: entity_id,
            type_name: "MeshRenderer".to_string(),
            data: component_data,
        };
        template_execute_inner(&template_state, template_path, cmd)
            .map_err(|e| e.message)?
            .new_state
    };

    // Derive project root from ProjectState
    let project_root_buf = {
        let guard = project_state.0.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .cloned()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };

    // Sync MeshRenderer component to the live ECS world
    sync_mesh_renderer_to_ecs(
        entity_id,
        &new_template_state,
        &world_state,
        &asset_state.0,
        &project_root_buf,
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Scene undo / redo IPC
// ---------------------------------------------------------------------------


#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::{Transform, World};
    use std::sync::{Arc, RwLock};

    #[test]
    fn create_entity_adds_transform_to_world() {
        let world = Arc::new(RwLock::new(World::new()));
        let entity = {
            let mut w = world.write().unwrap();
            w.register::<Transform>();
            let e = w.spawn();
            w.add(e, Transform::default());
            e
        };
        let w = world.read().unwrap();
        assert!(w.get::<Transform>(entity).is_some());
    }

    #[test]
    fn set_component_field_updates_transform_position() {
        let world = Arc::new(RwLock::new(World::new()));
        let entity = {
            let mut w = world.write().unwrap();
            w.register::<Transform>();
            let e = w.spawn();
            w.add(e, Transform::default());
            e
        };
        {
            let mut w = world.write().unwrap();
            if let Some(t) = w.get_mut::<Transform>(entity) {
                t.position.x = 5.0;
            }
        }
        let w = world.read().unwrap();
        let t = w.get::<Transform>(entity).unwrap();
        assert_eq!(t.position.x, 5.0);
    }

    #[test]
    fn despawn_removes_entity_from_world() {
        let world = Arc::new(RwLock::new(World::new()));
        let entity = {
            let mut w = world.write().unwrap();
            w.register::<Transform>();
            let e = w.spawn();
            w.add(e, Transform::default());
            e
        };
        {
            let mut w = world.write().unwrap();
            w.despawn(entity);
        }
        let w = world.read().unwrap();
        assert!(w.get::<Transform>(entity).is_none());
    }

    #[test]
    fn selected_entity_id_default_is_none() {
        let state = NativeViewportState::new();
        assert_eq!(*state.selected_entity_id.lock().unwrap(), None);
    }

    #[test]
    fn selected_entity_id_can_be_set_and_cleared() {
        let state = NativeViewportState::new();
        *state.selected_entity_id.lock().unwrap() = Some(42);
        assert_eq!(*state.selected_entity_id.lock().unwrap(), Some(42));
        *state.selected_entity_id.lock().unwrap() = None;
        assert_eq!(*state.selected_entity_id.lock().unwrap(), None);
    }
}
