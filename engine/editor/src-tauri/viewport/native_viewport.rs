//! Native child-window viewport for Vulkan rendering.
//!
//! Creates a platform-native child window parented inside the Tauri webview
//! window.  The child window is the target for a Vulkan surface; a render
//! thread draws into it at ~60 fps.
//!
//! Currently the render thread just keeps the window alive and fills it with
//! a solid colour via `FillRect`.  Once the engine renderer is wired, it
//! will create a real Vulkan swapchain and render the scene.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Viewport bounds in physical (device) pixels.
#[derive(Clone, Copy, Debug)]
pub struct ViewportBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Windows implementation
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
mod platform {
    use super::*;

    use windows::Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect,
            HBRUSH, PAINTSTRUCT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    };

    /// Wrapper around HWND that is Send+Sync.
    ///
    /// HWND itself contains a raw pointer.  We only pass it to Win32 APIs
    /// that are safe to call from any thread (SetWindowPos, InvalidateRect,
    /// DestroyWindow, message pumping) so the Send impl is sound.
    #[derive(Clone, Copy)]
    struct SendHwnd(HWND);

    // SAFETY: We restrict usage to thread-safe Win32 calls.
    unsafe impl Send for SendHwnd {}
    unsafe impl Sync for SendHwnd {}

    /// State of the native Vulkan viewport (Windows).
    pub struct NativeViewport {
        child_hwnd: SendHwnd,
        renderer_thread: Option<std::thread::JoinHandle<()>>,
        should_stop: Arc<AtomicBool>,
        bounds: Arc<Mutex<ViewportBounds>>,
    }

    impl NativeViewport {
        /// Create a new child window parented to `parent_hwnd`.
        ///
        /// `parent_hwnd` is the HWND of the Tauri main window, obtained via
        /// `tauri::WebviewWindow::hwnd()`.
        pub fn new(parent_hwnd: HWND, bounds: ViewportBounds) -> Result<Self, String> {
            unsafe {
                let class_name = windows::core::w!("SilmarilViewport");
                let hinstance: HINSTANCE = GetModuleHandleW(None)
                    .map_err(|e| format!("GetModuleHandleW failed: {e}"))?
                    .into();

                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                    lpfnWndProc: Some(viewport_wnd_proc),
                    hInstance: hinstance,
                    lpszClassName: class_name,
                    hbrBackground: HBRUSH(std::ptr::null_mut()),
                    ..Default::default()
                };

                // RegisterClassExW returns 0 on failure *unless* the class
                // already exists (in which case the previous registration is
                // reused).  We ignore the return value intentionally.
                RegisterClassExW(&wc);

                let child = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    windows::core::w!(""),
                    WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                    bounds.x,
                    bounds.y,
                    bounds.width as i32,
                    bounds.height as i32,
                    Some(parent_hwnd),
                    None, // no menu
                    Some(hinstance),
                    None, // no extra param
                )
                .map_err(|e| format!("CreateWindowExW failed: {e}"))?;

                tracing::info!(
                    x = bounds.x,
                    y = bounds.y,
                    w = bounds.width,
                    h = bounds.height,
                    "Native viewport child window created"
                );

                Ok(Self {
                    child_hwnd: SendHwnd(child),
                    renderer_thread: None,
                    should_stop: Arc::new(AtomicBool::new(false)),
                    bounds: Arc::new(Mutex::new(bounds)),
                })
            }
        }

        /// Start the render loop on a background thread.
        ///
        /// Currently fills the window with a dark colour each frame to prove
        /// the child window is alive.  Will be replaced with Vulkan rendering.
        pub fn start_rendering(&mut self) -> Result<(), String> {
            let should_stop = self.should_stop.clone();
            let bounds = self.bounds.clone();
            // Extract the raw pointer as an integer so we can send it across
            // threads without triggering the `Send` check on `*mut c_void`.
            let hwnd_raw = self.child_hwnd.0 .0 as isize;

            let handle = std::thread::Builder::new()
                .name("viewport-render".into())
                .spawn(move || {
                    let hwnd = HWND(hwnd_raw as *mut _);
                    tracing::info!("Viewport render thread started");
                    render_loop(hwnd, should_stop, bounds);
                    tracing::info!("Viewport render thread stopped");
                })
                .map_err(|e| format!("Failed to spawn render thread: {e}"))?;

            self.renderer_thread = Some(handle);
            Ok(())
        }

        /// Reposition and resize the child window (called when the Svelte
        /// container's bounds change).
        pub fn set_bounds(&self, new_bounds: ViewportBounds) {
            *self.bounds.lock().unwrap() = new_bounds;

            unsafe {
                let _ = SetWindowPos(
                    self.child_hwnd.0,
                    None,
                    new_bounds.x,
                    new_bounds.y,
                    new_bounds.width as i32,
                    new_bounds.height as i32,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }

        /// Get the child HWND (for future Vulkan surface creation).
        #[allow(dead_code)]
        pub fn hwnd(&self) -> HWND {
            self.child_hwnd.0
        }

        /// Stop the render thread and destroy the child window.
        pub fn destroy(&mut self) {
            self.should_stop.store(true, Ordering::Relaxed);
            if let Some(handle) = self.renderer_thread.take() {
                let _ = handle.join();
            }
            unsafe {
                let _ = DestroyWindow(self.child_hwnd.0);
            }
            tracing::info!("Native viewport destroyed");
        }
    }

    impl Drop for NativeViewport {
        fn drop(&mut self) {
            self.destroy();
        }
    }

    /// Minimal render loop: invalidates the child window so WM_PAINT fires,
    /// then pumps messages.  The actual painting happens in `viewport_wnd_proc`.
    fn render_loop(
        hwnd: HWND,
        should_stop: Arc<AtomicBool>,
        _bounds: Arc<Mutex<ViewportBounds>>,
    ) {
        while !should_stop.load(Ordering::Relaxed) {
            unsafe {
                // Invalidate so WM_PAINT fires
                let _ = InvalidateRect(Some(hwnd), None, false);

                // Process pending messages for this window (non-blocking)
                let mut msg = std::mem::zeroed::<MSG>();
                while PeekMessageW(&mut msg, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    /// Window procedure for the child viewport window.
    ///
    /// Handles WM_PAINT by filling with the dark background colour.
    /// All other messages are forwarded to `DefWindowProcW`.
    unsafe extern "system" fn viewport_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.is_invalid() {
                    // Fill with #1a1a2e (Win32 COLORREF is 0x00BBGGRR)
                    let brush = CreateSolidBrush(COLORREF(0x002e1a1a));
                    FillRect(hdc, &ps.rcPaint, brush);
                    let _ = DeleteObject(brush.into());
                }
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_ERASEBKGND => {
                // Prevent flicker — we handle painting in WM_PAINT
                LRESULT(1)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Platform-agnostic re-exports
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
pub use platform::NativeViewport;

// Stub for non-Windows platforms (not yet implemented)
#[cfg(not(windows))]
pub struct NativeViewport;

#[cfg(not(windows))]
impl NativeViewport {
    pub fn new(_parent: isize, _bounds: ViewportBounds) -> Result<Self, String> {
        Err("Native viewport not yet implemented for this platform".into())
    }

    pub fn start_rendering(&mut self) -> Result<(), String> {
        Err("Native viewport not yet implemented for this platform".into())
    }

    pub fn set_bounds(&self, _bounds: ViewportBounds) {}

    pub fn destroy(&mut self) {}
}
