pub mod bridge;
pub mod plugins;
pub mod state;
pub mod viewport;
pub mod world;

use bridge::commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::NativeViewportState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::get_editor_state,
            commands::open_project,
            commands::open_project_dialog,
            commands::scan_project_entities,
            commands::get_viewport_frame,
            commands::pick_viewport_entity,
            commands::scene_command,
            commands::create_native_viewport,
            commands::resize_native_viewport,
            commands::destroy_native_viewport,
            commands::set_viewport_visible,
            commands::create_popout_window,
        ])
        .run(tauri::generate_context!())
        .expect("error running Silmaril Editor");
}
