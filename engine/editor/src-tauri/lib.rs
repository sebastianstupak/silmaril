pub mod bridge;
pub mod plugins;
pub mod state;
pub mod viewport;
pub mod world;

use bridge::commands;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "silmaril_editor=debug,engine_renderer=debug,warn".into()),
        )
        .init();

    tracing::info!("Silmaril Editor starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::NativeViewportState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::get_editor_state,
            commands::open_project,
            commands::open_project_dialog,
            commands::scan_project_entities,
            commands::scene_command,
            commands::create_native_viewport,
            commands::resize_native_viewport,
            commands::destroy_native_viewport,
            commands::set_viewport_visible,
            commands::create_popout_window,
            commands::dock_panel_back,
            commands::check_dock_proximity,
        ])
        .setup(|app| {
            use tauri::Manager;
            if let Some(main_window) = app.get_webview_window("main") {
                // Set window + webview background to fully transparent
                let transparent = tauri::utils::config::Color(0, 0, 0, 0);
                let _ = main_window.set_background_color(Some(transparent));

                // On Windows, disable clip-children so the Vulkan child window
                // (behind the webview) can show through transparent regions.
                #[cfg(windows)]
                {
                    use windows::Win32::UI::WindowsAndMessaging::*;

                    let hwnd = main_window.hwnd().unwrap();
                    unsafe {
                        // Remove WS_CLIPCHILDREN from the main window style.
                        // This allows child windows to render over each other
                        // and lets the transparent webview show the Vulkan
                        // child window behind it.
                        let style = GetWindowLongW(hwnd, GWL_STYLE);
                        let new_style = style & !(WS_CLIPCHILDREN.0 as i32);
                        SetWindowLongW(hwnd, GWL_STYLE, new_style);
                        tracing::info!("Removed WS_CLIPCHILDREN from main window");
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Silmaril Editor");
}
