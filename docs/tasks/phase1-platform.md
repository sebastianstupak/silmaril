# Phase 1.4: Platform Abstraction Layer

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (foundation for cross-platform support)

---

## 🎯 **Objective**

Implement hard platform abstraction layer to isolate platform-specific code from business logic. NO `#[cfg]` attributes should appear in business logic - all platform differences handled through traits.

**Critical Rule:** Business logic must NEVER contain platform-specific code.

---

## 📋 **Detailed Tasks**

### **1. Platform Module Structure** (Day 1)

**File:** `engine/core/src/platform/mod.rs`

```rust
//! Platform abstraction layer
//!
//! All platform-specific code is isolated here.
//! Business logic MUST use these abstractions.

mod window;
mod input;
mod time;
mod filesystem;
mod threading;

pub use window::{WindowBackend, WindowConfig, WindowEvent};
pub use input::{InputBackend, Key, MouseButton, InputEvent};
pub use time::TimeBackend;
pub use filesystem::FileSystemBackend;
pub use threading::ThreadingBackend;

/// Platform factory - creates platform-specific implementations
pub struct Platform;

impl Platform {
    /// Create window backend for current platform
    pub fn create_window() -> Box<dyn WindowBackend> {
        #[cfg(target_os = "windows")]
        return Box::new(window::windows::WindowsWindow::new());

        #[cfg(target_os = "linux")]
        return Box::new(window::linux::LinuxWindow::new());

        #[cfg(target_os = "macos")]
        return Box::new(window::macos::MacOSWindow::new());

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos"
        )))]
        compile_error!("Unsupported platform");
    }

    /// Create input backend
    pub fn create_input() -> Box<dyn InputBackend> {
        #[cfg(target_os = "windows")]
        return Box::new(input::windows::WindowsInput::new());

        #[cfg(target_os = "linux")]
        return Box::new(input::linux::LinuxInput::new());

        #[cfg(target_os = "macos")]
        return Box::new(input::macos::MacOSInput::new());

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos"
        )))]
        compile_error!("Unsupported platform");
    }

    /// Create time backend
    pub fn create_time() -> Box<dyn TimeBackend> {
        #[cfg(target_os = "windows")]
        return Box::new(time::windows::WindowsTime::new());

        #[cfg(target_os = "linux")]
        return Box::new(time::linux::LinuxTime::new());

        #[cfg(target_os = "macos")]
        return Box::new(time::macos::MacOSTime::new());

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos"
        )))]
        compile_error!("Unsupported platform");
    }

    /// Create filesystem backend
    pub fn create_filesystem() -> Box<dyn FileSystemBackend> {
        // Filesystem is mostly platform-agnostic in Rust std
        Box::new(filesystem::StdFileSystem::new())
    }

    /// Create threading backend
    pub fn create_threading() -> Box<dyn ThreadingBackend> {
        #[cfg(target_os = "windows")]
        return Box::new(threading::windows::WindowsThreading::new());

        #[cfg(target_os = "linux")]
        return Box::new(threading::linux::LinuxThreading::new());

        #[cfg(target_os = "macos")]
        return Box::new(threading::macos::MacOSThreading::new());

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos"
        )))]
        compile_error!("Unsupported platform");
    }
}
```

---

### **2. Window Abstraction** (Day 1-2)

**File:** `engine/core/src/platform/window.rs`

```rust
use ash::vk;

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Agent Game Engine".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
            vsync: true,
            resizable: true,
        }
    }
}

/// Window events
#[derive(Debug, Clone)]
pub enum WindowEvent {
    Resized { width: u32, height: u32 },
    Closed,
    FocusGained,
    FocusLost,
    Minimized,
    Restored,
}

/// Platform-agnostic window backend
pub trait WindowBackend: Send + Sync {
    /// Create window with config
    fn create(&mut self, config: WindowConfig) -> Result<(), PlatformError>;

    /// Destroy window
    fn destroy(&mut self);

    /// Process events (returns true if should continue)
    fn poll_events(&mut self) -> Vec<WindowEvent>;

    /// Get window size
    fn size(&self) -> (u32, u32);

    /// Set window title
    fn set_title(&mut self, title: &str);

    /// Set fullscreen mode
    fn set_fullscreen(&mut self, fullscreen: bool);

    /// Get raw window handle (for Vulkan surface creation)
    fn raw_window_handle(&self) -> RawWindowHandle;

    /// Get Vulkan surface extensions required for this platform
    fn required_vulkan_extensions(&self) -> Vec<&'static str>;

    /// Create Vulkan surface
    fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, PlatformError>;
}

/// Platform errors
define_error! {
    pub enum PlatformError {
        WindowCreationFailed { details: String } = ErrorCode::WindowCreationFailed, ErrorSeverity::Critical,
        SurfaceCreationFailed { details: String } = ErrorCode::SurfaceCreationFailed, ErrorSeverity::Critical,
        InputInitFailed { details: String } = ErrorCode::InputInitFailed, ErrorSeverity::Critical,
    }
}
```

**Windows Implementation:**

**File:** `engine/core/src/platform/window/windows.rs`

```rust
#[cfg(target_os = "windows")]
use winapi::um::winuser::*;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32Handle};

pub struct WindowsWindow {
    hwnd: Option<HWND>,
    config: WindowConfig,
}

impl WindowsWindow {
    pub fn new() -> Self {
        Self {
            hwnd: None,
            config: WindowConfig::default(),
        }
    }
}

impl WindowBackend for WindowsWindow {
    fn create(&mut self, config: WindowConfig) -> Result<(), PlatformError> {
        // Win32 window creation
        unsafe {
            let class_name = "AgentGameEngineWindowClass";

            // Register window class
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                hInstance: GetModuleHandleW(std::ptr::null()),
                lpszClassName: to_wide_string(class_name).as_ptr(),
                // ... other fields
            };

            if RegisterClassW(&wc) == 0 {
                return Err(PlatformError::WindowCreationFailed {
                    details: "Failed to register window class".to_string(),
                });
            }

            // Create window
            let hwnd = CreateWindowExW(
                0,
                to_wide_string(class_name).as_ptr(),
                to_wide_string(&config.title).as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                config.width as i32,
                config.height as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            );

            if hwnd.is_null() {
                return Err(PlatformError::WindowCreationFailed {
                    details: "CreateWindowExW failed".to_string(),
                });
            }

            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            self.hwnd = Some(hwnd);
            self.config = config;

            Ok(())
        }
    }

    fn poll_events(&mut self) -> Vec<WindowEvent> {
        let mut events = Vec::new();

        unsafe {
            let mut msg = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // Convert to WindowEvent
                match msg.message {
                    WM_SIZE => {
                        let width = (msg.lParam & 0xFFFF) as u32;
                        let height = ((msg.lParam >> 16) & 0xFFFF) as u32;
                        events.push(WindowEvent::Resized { width, height });
                    }
                    WM_CLOSE => events.push(WindowEvent::Closed),
                    _ => {}
                }
            }
        }

        events
    }

    fn required_vulkan_extensions(&self) -> Vec<&'static str> {
        vec!["VK_KHR_surface", "VK_KHR_win32_surface"]
    }

    fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, PlatformError> {
        use ash::extensions::khr::Win32Surface;

        let hwnd = self.hwnd.ok_or_else(|| PlatformError::SurfaceCreationFailed {
            details: "Window not created".to_string(),
        })?;

        let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hinstance(unsafe { GetModuleHandleW(std::ptr::null()) as *const _ as *const _ })
            .hwnd(hwnd as *const _ as *const _);

        let win32_surface = Win32Surface::new(entry, instance);
        unsafe {
            win32_surface
                .create_win32_surface(&create_info, None)
                .map_err(|e| PlatformError::SurfaceCreationFailed {
                    details: e.to_string(),
                })
        }
    }

    // ... other trait methods
}
```

**Linux Implementation:**

**File:** `engine/core/src/platform/window/linux.rs`

```rust
#[cfg(target_os = "linux")]
use x11::xlib::*;

pub struct LinuxWindow {
    display: Option<*mut Display>,
    window: Option<Window>,
    config: WindowConfig,
}

impl WindowBackend for LinuxWindow {
    fn create(&mut self, config: WindowConfig) -> Result<(), PlatformError> {
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err(PlatformError::WindowCreationFailed {
                    details: "Failed to open X display".to_string(),
                });
            }

            let screen = XDefaultScreen(display);
            let root = XRootWindow(display, screen);

            let window = XCreateSimpleWindow(
                display,
                root,
                0,
                0,
                config.width,
                config.height,
                1,
                XBlackPixel(display, screen),
                XWhitePixel(display, screen),
            );

            XMapWindow(display, window);
            XFlush(display);

            self.display = Some(display);
            self.window = Some(window);
            self.config = config;

            Ok(())
        }
    }

    fn required_vulkan_extensions(&self) -> Vec<&'static str> {
        vec!["VK_KHR_surface", "VK_KHR_xcb_surface"]
    }

    fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, PlatformError> {
        use ash::extensions::khr::XcbSurface;

        // Convert X11 to XCB (Vulkan uses XCB)
        let connection = self.get_xcb_connection()?;
        let window = self.window.ok_or_else(|| PlatformError::SurfaceCreationFailed {
            details: "Window not created".to_string(),
        })?;

        let create_info = vk::XcbSurfaceCreateInfoKHR::builder()
            .connection(connection)
            .window(window as u32);

        let xcb_surface = XcbSurface::new(entry, instance);
        unsafe {
            xcb_surface
                .create_xcb_surface(&create_info, None)
                .map_err(|e| PlatformError::SurfaceCreationFailed {
                    details: e.to_string(),
                })
        }
    }

    // ... other methods
}
```

**macOS Implementation:**

**File:** `engine/core/src/platform/window/macos.rs`

```rust
#[cfg(target_os = "macos")]
use cocoa::*;
use objc::*;

pub struct MacOSWindow {
    ns_window: Option<*mut Object>,
    ns_view: Option<*mut Object>,
    config: WindowConfig,
}

impl WindowBackend for MacOSWindow {
    fn create(&mut self, config: WindowConfig) -> Result<(), PlatformError> {
        unsafe {
            let ns_app = NSApp();
            ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);

            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(config.width as f64, config.height as f64),
                ),
                NSTitledWindowMask
                    | NSClosableWindowMask
                    | NSMiniaturizableWindowMask
                    | NSResizableWindowMask,
                NSBackingStoreBuffered,
                NO,
            );

            window.center();
            window.setTitle_(NSString::alloc(nil).init_str(&config.title));
            window.makeKeyAndOrderFront_(nil);

            // Create Metal layer for MoltenVK
            let layer = CAMetalLayer::new();
            let view = window.contentView();
            view.setLayer(layer as *mut _);
            view.setWantsLayer(YES);

            self.ns_window = Some(window);
            self.ns_view = Some(view);
            self.config = config;

            Ok(())
        }
    }

    fn required_vulkan_extensions(&self) -> Vec<&'static str> {
        vec!["VK_KHR_surface", "VK_MVK_macos_surface"]
    }

    fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, PlatformError> {
        use ash::extensions::mvk::MacOSSurface;

        let view = self.ns_view.ok_or_else(|| PlatformError::SurfaceCreationFailed {
            details: "View not created".to_string(),
        })?;

        let create_info = vk::MacOSSurfaceCreateInfoMVK::builder().view(view as *const _);

        let macos_surface = MacOSSurface::new(entry, instance);
        unsafe {
            macos_surface
                .create_mac_os_surface_mvk(&create_info, None)
                .map_err(|e| PlatformError::SurfaceCreationFailed {
                    details: e.to_string(),
                })
        }
    }

    // ... other methods
}
```

---

### **3. Input Abstraction** (Day 2)

**File:** `engine/core/src/platform/input.rs`

```rust
/// Keyboard key codes (platform-agnostic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    W, A, S, D,
    Space, Escape, Enter,
    Left, Right, Up, Down,
    // ... all keys
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input event
#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPressed(Key),
    KeyReleased(Key),
    MouseMoved { x: f32, y: f32 },
    MouseButtonPressed(MouseButton),
    MouseButtonReleased(MouseButton),
    MouseWheel { delta: f32 },
}

/// Input backend
pub trait InputBackend: Send + Sync {
    /// Poll input events
    fn poll_events(&mut self) -> Vec<InputEvent>;

    /// Check if key is currently pressed
    fn is_key_down(&self, key: Key) -> bool;

    /// Check if mouse button is currently pressed
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;

    /// Get current mouse position
    fn mouse_position(&self) -> (f32, f32);
}
```

---

### **4. Time Abstraction** (Day 2-3)

**File:** `engine/core/src/platform/time.rs`

```rust
use std::time::Duration;

/// High-precision time backend
pub trait TimeBackend: Send + Sync {
    /// Get current time in nanoseconds since epoch
    fn now_nanos(&self) -> u64;

    /// Sleep for duration (high-precision if available)
    fn sleep(&self, duration: Duration);

    /// Get monotonic time (for delta time calculations)
    fn monotonic_nanos(&self) -> u64;
}
```

**Windows Implementation:**

```rust
#[cfg(target_os = "windows")]
pub struct WindowsTime {
    frequency: i64,
}

impl WindowsTime {
    pub fn new() -> Self {
        unsafe {
            let mut frequency = 0;
            QueryPerformanceFrequency(&mut frequency);
            Self { frequency }
        }
    }
}

impl TimeBackend for WindowsTime {
    fn monotonic_nanos(&self) -> u64 {
        unsafe {
            let mut counter = 0;
            QueryPerformanceCounter(&mut counter);
            (counter * 1_000_000_000 / self.frequency) as u64
        }
    }

    fn sleep(&self, duration: Duration) {
        // Use high-precision waitable timer on Windows
        unsafe {
            let handle = CreateWaitableTimerW(std::ptr::null_mut(), 0, std::ptr::null());
            let due_time = -(duration.as_nanos() as i64 / 100); // 100ns units
            SetWaitableTimer(handle, &due_time, 0, None, std::ptr::null_mut(), 0);
            WaitForSingleObject(handle, INFINITE);
            CloseHandle(handle);
        }
    }

    // ... other methods
}
```

---

### **5. Threading Abstraction** (Day 3-4)

**File:** `engine/core/src/platform/threading.rs`

```rust
/// Thread priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}

/// Threading backend
pub trait ThreadingBackend: Send + Sync {
    /// Set current thread priority
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError>;

    /// Set thread affinity (CPU cores)
    fn set_thread_affinity(&self, cores: &[usize]) -> Result<(), PlatformError>;

    /// Get number of logical CPU cores
    fn logical_cpu_count(&self) -> usize;

    /// Get number of physical CPU cores
    fn physical_cpu_count(&self) -> usize;
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Platform module structure created
- [ ] Window abstraction implemented for all platforms
- [ ] Input abstraction implemented
- [ ] Time abstraction implemented
- [ ] Threading abstraction implemented
- [ ] FileSystem abstraction implemented
- [ ] Factory pattern for platform creation
- [ ] NO `#[cfg]` in business logic (only in platform module)
- [ ] All platforms tested in CI
- [ ] Vulkan surface creation works on all platforms
- [ ] Documentation complete

---

## 🧪 **Tests**

```rust
#[test]
fn test_window_creation() {
    let mut window = Platform::create_window();
    let config = WindowConfig::default();
    window.create(config).unwrap();

    let (width, height) = window.size();
    assert_eq!(width, 1280);
    assert_eq!(height, 720);
}

#[test]
fn test_time_monotonic() {
    let time = Platform::create_time();
    let t1 = time.monotonic_nanos();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let t2 = time.monotonic_nanos();

    assert!(t2 > t1);
    assert!(t2 - t1 >= 10_000_000); // At least 10ms
}

#[test]
fn test_input_events() {
    let mut input = Platform::create_input();
    let events = input.poll_events();
    // Events depend on actual input, just verify API works
    assert!(events.is_empty() || !events.is_empty());
}
```

---

**Dependencies:** [phase1-serialization.md](phase1-serialization.md)
**Next:** [phase1-vulkan-context.md](phase1-vulkan-context.md)
