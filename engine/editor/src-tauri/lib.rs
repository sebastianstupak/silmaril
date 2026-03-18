pub mod bridge;
pub mod plugins;
pub mod state;
pub mod viewport;
pub mod world;

use bridge::commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_editor_state,
            commands::open_project,
        ])
        .run(tauri::generate_context!())
        .expect("error running Silmaril Editor");
}
