# Platform Abstraction

> **Cross-platform abstraction strategy for Windows, Linux, and macOS**
>
> ⚠️ **MANDATORY READ** when writing platform-specific code

---

## 🎯 **Core Principle**

**Business logic MUST NEVER contain platform-specific code.**

```rust
// ❌ FORBIDDEN
fn update_game(world: &mut World) {
    #[cfg(windows)]
    { /* windows code */ }

    #[cfg(unix)]
    { /* unix code */ }
}

// ✅ CORRECT
fn update_game(world: &mut World, platform: &dyn Platform) {
    let time = platform.get_time();
    // Business logic uses trait, not platform code
}
```

---

## 🏗️ **Abstraction Layers**

### **1. Window Management**

**Location:** `engine/renderer/src/platform/window.rs`

**Trait:**
```rust
pub trait WindowBackend: Send + Sync {
    fn create(&self, config: WindowConfig) -> Result<Window, WindowError>;
    fn poll_events(&mut self) -> Vec<WindowEvent>;
    fn size(&self) -> (u32, u32);
    fn set_title(&mut self, title: &str);
    fn close(&mut self);
}

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fullscreen: bool,
}

pub enum WindowEvent {
    Resized(u32, u32),
    CloseRequested,
    Focused(bool),
}
```

**Implementations:**
- `engine/renderer/src/platform/windows/window.rs` - Win32 API
- `engine/renderer/src/platform/linux/window.rs` - X11/Wayland
- `engine/renderer/src/platform/macos/window.rs` - Cocoa

**Selection:**
```rust
// engine/renderer/src/platform/mod.rs
pub fn create_window_backend() -> Box<dyn WindowBackend> {
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsWindowBackend::new());

    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxWindowBackend::new());

    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSWindowBackend::new());
}
```

---

### **2. Vulkan Surface Creation**

**Location:** `engine/renderer/src/platform/surface.rs`

**Trait:**
```rust
pub trait SurfaceBackend: Send + Sync {
    fn create_surface(
        &self,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<vk::SurfaceKHR, SurfaceError>;

    fn required_extensions(&self) -> &[&'static str];
}
```

**Platform-Specific Extensions:**
```rust
// Windows
impl SurfaceBackend for WindowsSurfaceBackend {
    fn required_extensions(&self) -> &[&'static str] {
        &["VK_KHR_surface", "VK_KHR_win32_surface"]
    }

    fn create_surface(
        &self,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<vk::SurfaceKHR, SurfaceError> {
        let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hinstance(window.hinstance())
            .hwnd(window.hwnd());

        unsafe {
            let win32_surface = ash::extensions::khr::Win32Surface::new(&entry, instance);
            Ok(win32_surface.create_win32_surface(&create_info, None)?)
        }
    }
}

// macOS (MoltenVK)
impl SurfaceBackend for MacOSSurfaceBackend {
    fn required_extensions(&self) -> &[&'static str] {
        &["VK_KHR_surface", "VK_EXT_metal_surface"]
    }

    fn create_surface(
        &self,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<vk::SurfaceKHR, SurfaceError> {
        // MoltenVK-specific surface creation
        // ...
    }
}

// Linux
impl SurfaceBackend for LinuxSurfaceBackend {
    fn required_extensions(&self) -> &[&'static str] {
        &["VK_KHR_surface", "VK_KHR_xcb_surface", "VK_KHR_wayland_surface"]
    }

    fn create_surface(
        &self,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<vk::SurfaceKHR, SurfaceError> {
        // Try Wayland first, fall back to X11
        // ...
    }
}
```

---

### **3. Input Handling**

**Location:** `engine/core/src/platform/input.rs`

**Trait:**
```rust
pub trait InputBackend: Send + Sync {
    fn poll_input(&mut self) -> Vec<InputEvent>;
    fn mouse_position(&self) -> (f32, f32);
    fn is_key_pressed(&self, key: KeyCode) -> bool;
}

pub enum InputEvent {
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
    MouseMoved(f32, f32),
    MouseButton(MouseButton, ButtonState),
    MouseWheel(f32),
}
```

**Platform-specific implementations handle raw input, expose unified API.**

---

### **4. File System**

**Location:** `engine/core/src/platform/fs.rs`

**Trait:**
```rust
pub trait FileSystemBackend: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IoError>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), IoError>;
    fn exists(&self, path: &Path) -> bool;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>, IoError>;
}
```

**Platform differences:**
- Windows: `\\` path separators, case-insensitive
- Unix: `/` path separators, case-sensitive
- macOS: Case-insensitive by default but configurable

**Abstraction handles normalization:**
```rust
impl FileSystemBackend for NativeFileSystem {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        let normalized = self.normalize_path(path);
        std::fs::read(normalized)
    }

    #[cfg(windows)]
    fn normalize_path(&self, path: &Path) -> PathBuf {
        path.to_str()
            .unwrap()
            .replace('/', "\\")
            .into()
    }

    #[cfg(unix)]
    fn normalize_path(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }
}
```

---

### **5. Time & Clock**

**Location:** `engine/core/src/platform/time.rs`

**Trait:**
```rust
pub trait ClockBackend: Send + Sync {
    fn now(&self) -> Instant;
    fn elapsed(&self, since: Instant) -> Duration;
    fn sleep(&self, duration: Duration);
}
```

**Platform-specific implementations:**
- Windows: `QueryPerformanceCounter`
- Unix: `clock_gettime(CLOCK_MONOTONIC)`
- macOS: `mach_absolute_time`

---

### **6. Threading**

**Location:** `engine/core/src/platform/thread.rs`

**Trait:**
```rust
pub trait ThreadBackend: Send + Sync {
    fn spawn<F>(&self, f: F) -> ThreadHandle
    where
        F: FnOnce() + Send + 'static;

    fn set_thread_priority(&self, priority: ThreadPriority);
    fn current_thread_id(&self) -> ThreadId;
}

pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}
```

**Platform-specific thread priorities differ - abstraction normalizes.**

---

### **7. Networking**

**Location:** `engine/networking/src/platform/socket.rs`

**Trait:**
```rust
pub trait SocketBackend: Send + Sync {
    fn bind_tcp(&self, addr: SocketAddr) -> Result<TcpListener, IoError>;
    fn bind_udp(&self, addr: SocketAddr) -> Result<UdpSocket, IoError>;
    fn set_nonblocking(&self, socket: &impl Socket, nonblocking: bool) -> Result<(), IoError>;
}
```

**Platform differences:**
- Windows: Winsock2
- Unix: POSIX sockets
- Error codes differ

---

## 🔧 **Implementation Guidelines**

### **1. Define Trait First**

Before writing platform code, define the abstraction:

```rust
// Step 1: Define trait in platform/trait_name.rs
pub trait MyFeature: Send + Sync {
    fn do_thing(&self) -> Result<Output, Error>;
}

// Step 2: Implement for each platform
#[cfg(windows)]
mod windows_impl;

#[cfg(unix)]
mod unix_impl;

// Step 3: Factory function
pub fn create_my_feature() -> Box<dyn MyFeature> {
    #[cfg(windows)]
    return Box::new(windows_impl::WindowsMyFeature::new());

    #[cfg(unix)]
    return Box::new(unix_impl::UnixMyFeature::new());
}
```

---

### **2. Minimize Platform-Specific Code**

Keep platform implementations **small and focused**:

```rust
// ✅ GOOD: Minimal platform code
impl ClockBackend for WindowsClock {
    fn now(&self) -> Instant {
        unsafe {
            let mut counter: i64 = 0;
            QueryPerformanceCounter(&mut counter);
            Instant(counter as u64)
        }
    }
}

// ❌ BAD: Business logic in platform code
impl ClockBackend for WindowsClock {
    fn now(&self) -> Instant {
        let instant = /* get time */;

        // DON'T: Game logic in platform code
        if self.game_paused {
            return self.pause_start_time;
        }

        instant
    }
}
```

---

### **3. Test Each Platform**

Every abstraction MUST have tests on all platforms:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_monotonic() {
        let clock = create_clock_backend();
        let t1 = clock.now();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = clock.now();
        assert!(clock.elapsed(t1) >= Duration::from_millis(10));
    }

    // This test runs on Windows, Linux, macOS
}
```

**CI runs tests on all platforms** - must pass everywhere.

---

### **4. Document Platform Quirks**

When platform behavior differs, document it:

```rust
/// Returns the current mouse position.
///
/// # Platform-Specific Behavior
///
/// - **Windows**: Coordinates relative to window client area
/// - **macOS**: Coordinates may have sub-pixel precision due to Retina displays
/// - **Linux**: Behavior depends on X11 vs Wayland
pub fn mouse_position(&self) -> (f32, f32);
```

---

## 🧪 **Testing Strategy**

### **Unit Tests**

Test trait implementations independently:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_window_creation() {
        let backend = create_window_backend();
        let config = WindowConfig {
            title: "Test".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            fullscreen: false,
        };

        let window = backend.create(&config).unwrap();
        assert_eq!(window.size(), (800, 600));
    }
}
```

### **Integration Tests**

Test cross-platform behavior:

```rust
#[test]
fn test_file_path_normalization() {
    let fs = NativeFileSystem::new();

    // Works on all platforms
    let path = Path::new("assets/textures/player.png");
    assert!(fs.exists(path));
}
```

### **CI Matrix**

```yaml
strategy:
  matrix:
    os: [windows-latest, ubuntu-latest, macos-13, macos-14]
    include:
      - os: windows-latest
        target: x86_64-pc-windows-msvc
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: macos-13
        target: x86_64-apple-darwin
      - os: macos-14
        target: aarch64-apple-darwin
```

---

## 📊 **Platform-Specific Considerations**

### **Windows**

**Challenges:**
- Win32 API is C-based (unsafe)
- Wide strings (UTF-16) vs Rust strings (UTF-8)
- COM initialization for some APIs

**Solutions:**
- Use `winapi` crate for FFI
- Convert strings at boundary
- RAII wrappers for cleanup

**Example:**
```rust
use winapi::um::winuser::*;

fn create_window_windows() -> HWND {
    unsafe {
        let wide_title: Vec<u16> = "Title".encode_utf16().collect();
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide_title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            // ...
        )
    }
}
```

---

### **Linux**

**Challenges:**
- X11 vs Wayland (two different window systems)
- Distribution differences (package names, paths)
- Vulkan drivers vary

**Solutions:**
- Support both X11 and Wayland
- Use `pkg-config` for library discovery
- Test on multiple distros (Ubuntu, Fedora, Arch)

**Example:**
```rust
fn create_surface_linux(window: &Window) -> vk::SurfaceKHR {
    // Try Wayland first
    if let Ok(surface) = create_wayland_surface(window) {
        return surface;
    }

    // Fall back to X11
    create_x11_surface(window)
}
```

---

### **macOS**

**Challenges:**
- No native Vulkan (requires MoltenVK)
- Metal API is primary graphics API
- App sandboxing restrictions
- Retina displays (HiDPI scaling)

**Solutions:**
- Bundle MoltenVK with app
- Use Metal surface extensions
- Handle display scaling factor
- Test on both Intel and Apple Silicon

**Example:**
```rust
// macOS requires MoltenVK
fn create_instance_macos() -> ash::Instance {
    let extensions = [
        "VK_KHR_surface",
        "VK_EXT_metal_surface",  // MoltenVK
        "VK_KHR_portability_subset", // Required by MoltenVK
    ];

    // Enable portability feature
    let instance_info = vk::InstanceCreateInfo::builder()
        .enabled_extension_names(&extensions)
        .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

    // ...
}
```

---

## 📋 **Checklist for New Platform Code**

When adding platform-specific functionality:

- [ ] Define trait in `platform/trait_name.rs`
- [ ] Implement for Windows in `platform/windows/`
- [ ] Implement for Linux in `platform/linux/`
- [ ] Implement for macOS in `platform/macos/`
- [ ] Write unit tests for each implementation
- [ ] Write integration tests (cross-platform)
- [ ] Update CI to test on all platforms
- [ ] Document platform differences (if any)
- [ ] Verify no `#[cfg]` in business logic

---

## 🚫 **Anti-Patterns to Avoid**

### **❌ Platform Checks in Business Logic**

```rust
// WRONG
fn update_game(world: &mut World) {
    #[cfg(windows)]
    let delta_time = windows_get_time();

    #[cfg(unix)]
    let delta_time = unix_get_time();

    world.update(delta_time);
}

// RIGHT
fn update_game(world: &mut World, clock: &dyn ClockBackend) {
    let delta_time = clock.get_delta();
    world.update(delta_time);
}
```

---

### **❌ Platform-Specific Dependencies in Core**

```toml
# WRONG - Core crate depends on platform libs
[dependencies]
winapi = "0.3"  # Windows-only

# RIGHT - Platform dependencies in platform modules
[target.'cfg(windows)'.dependencies]
winapi = "0.3"
```

---

### **❌ Untested Platform Code**

```rust
// WRONG - No tests
#[cfg(windows)]
fn do_windows_thing() {
    // Complex platform code
}

// RIGHT - Test all platform paths
#[test]
fn test_platform_thing() {
    let platform = create_platform_backend();
    assert!(platform.do_thing().is_ok());
}
```

---

## 📚 **Related Documentation**

- [docs/testing-strategy.md](docs/testing-strategy.md) - Cross-platform testing
- [docs/rules/coding-standards.md](docs/rules/coding-standards.md) - Code style
- [docs/architecture.md](docs/architecture.md) - Overall architecture

---

**Last Updated:** 2026-01-31
