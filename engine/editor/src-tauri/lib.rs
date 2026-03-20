pub mod bridge;
pub mod plugins;
pub mod state;
pub mod viewport;
pub mod world;

use bridge::commands;

/// Apply DWM styling to a frameless window on Windows 11:
///   - Rounded corners (DWMWCP_ROUND) matching the OS default
///   - No 1px DWM border (DWMWA_COLOR_NONE)
///
/// Must be called after the window HWND is valid. Safe to call on older Windows
/// versions — DwmSetWindowAttribute is a no-op for unknown attribute IDs.
#[cfg(windows)]
pub(crate) fn apply_dwm_window_style(hwnd: windows::Win32::Foundation::HWND) {
    use std::mem::size_of;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute,
        DWMWA_BORDER_COLOR,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND,
    };

    unsafe {
        // Round corners — Windows 11 DWM DWMWCP_ROUND (value 2)
        let corner: u32 = DWMWCP_ROUND.0 as u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const u32 as *const _,
            size_of::<u32>() as u32,
        );

        // Remove the 1px DWM border (DWMWA_COLOR_NONE = 0xFFFFFFFE)
        let no_border: u32 = 0xFFFFFFFE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &no_border as *const u32 as *const _,
            size_of::<u32>() as u32,
        );
    }

    tracing::debug!("Applied DWM rounded corners + no border");
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "silmaril_editor=debug,engine_renderer=debug,warn".into()),
        )
        .init();

    // Install a panic hook that logs to tracing before the default handler runs
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".into()
        };
        tracing::error!(location = %location, payload = %payload, "PANIC in Silmaril Editor");
        // Also write to stderr in case tracing subscriber is broken
        eprintln!("[CRASH] PANIC at {location}: {payload}");
        default_hook(info);
    }));

    // On Windows, install a vectored exception handler to catch SEH
    // exceptions (access violations, etc.) that bypass Rust's panic
    // mechanism.  This logs the crash before the process terminates.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Diagnostics::Debug::{
            AddVectoredExceptionHandler, EXCEPTION_POINTERS,
        };
        unsafe extern "system" fn crash_handler(
            info: *mut EXCEPTION_POINTERS,
        ) -> i32 {
            if info.is_null() {
                return 0; // EXCEPTION_CONTINUE_SEARCH
            }
            let record = (*info).ExceptionRecord;
            if record.is_null() {
                return 0;
            }
            let code = (*record).ExceptionCode.0 as u32;
            // Only log fatal exceptions, not benign ones
            if code == 0xC0000005  // ACCESS_VIOLATION
                || code == 0xC0000409  // STATUS_STACK_BUFFER_OVERRUN
                || code == 0xC0000374  // HEAP_CORRUPTION
                || code == 0xC00000FD  // STACK_OVERFLOW
            {
                let addr = (*record).ExceptionAddress as usize;
                eprintln!(
                    "[CRASH] SEH exception 0x{code:08X} at address 0x{addr:X}"
                );
            }
            0 // EXCEPTION_CONTINUE_SEARCH
        }
        AddVectoredExceptionHandler(0, Some(crash_handler));
    }

    tracing::info!("Silmaril Editor starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::NativeViewportState::new())
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
            commands::viewport_camera_orbit,
            commands::viewport_camera_pan,
            commands::viewport_camera_zoom,
            commands::viewport_camera_reset,
            commands::viewport_set_grid_visible,
            commands::viewport_camera_set_orientation,
            commands::create_popout_window,
            commands::dock_panel_back,
            commands::check_dock_proximity,
            commands::window_minimize,
            commands::window_toggle_maximize,
            commands::window_close,
            commands::window_start_drag,
            commands::start_dock_drag,
            commands::broadcast_settings,
        ])
        .setup(|app| {
            use tauri::Manager;
            if let Some(main_window) = app.get_webview_window("main") {
                // Set window + webview background to fully transparent
                let transparent = tauri::utils::config::Color(0, 0, 0, 0);
                let _ = main_window.set_background_color(Some(transparent));

                // On Windows, disable clip-children so the Vulkan child window
                // (behind the webview) can show through transparent regions.
                // Also apply DWM styling: rounded corners + no border.
                #[cfg(windows)]
                {
                    use windows::Win32::UI::WindowsAndMessaging::*;

                    let hwnd = main_window.hwnd().unwrap();
                    unsafe {
                        // Remove WS_CLIPCHILDREN so the parent HWND's Vulkan DXGI
                        // swapchain is NOT clipped by the WebView2 child window bounds.
                        // With clip_children=false the Vulkan surface paints everywhere,
                        // and transparent CSS regions in WebView2 show it through.
                        let style = GetWindowLongW(hwnd, GWL_STYLE);
                        SetWindowLongW(hwnd, GWL_STYLE, style & !(WS_CLIPCHILDREN.0 as i32));
                        tracing::info!("Removed WS_CLIPCHILDREN from Tauri window");
                    }
                    apply_dwm_window_style(hwnd);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Silmaril Editor");
}
