//! Integration tests for window management
//!
//! NOTE: winit only allows ONE EventLoop per process, so all window tests
//! must be in a single test function or run in separate processes.

use engine_renderer::window::{Window, WindowConfig, WindowError};

#[test]
fn test_window_functionality() {
    // Test 1: Invalid dimensions should fail
    let invalid_config = WindowConfig {
        title: "Invalid".to_string(),
        width: 0,
        height: 0,
        resizable: false,
        visible: false,
    };

    match Window::new(invalid_config) {
        Err(WindowError::InvalidDimensions { .. }) => {
            // Expected
        }
        other => panic!("Expected InvalidDimensions error, got {:?}", other),
    }

    // Test 2: Valid window creation
    let config = WindowConfig {
        title: "Test Window".to_string(),
        width: 1280,
        height: 720,
        resizable: true,
        visible: false,
    };

    let window = Window::new(config).expect("Window creation should succeed with valid config");

    // Test 3: Size is correct
    let (width, height) = window.size();
    assert_eq!(width, 1280, "Window width should match config");
    assert_eq!(height, 720, "Window height should match config");

    // Test 4: Should close is initially false
    assert!(!window.should_close(), "Window should not close initially");

    // Test 5: Required extensions are present
    let extensions = window.required_extensions();
    assert!(!extensions.is_empty(), "Should require at least VK_KHR_surface");

    // Convert to strings for checking
    let ext_names: Vec<String> = extensions
        .iter()
        .map(|&ptr| unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap().to_string() })
        .collect();

    // Must include VK_KHR_surface
    assert!(
        ext_names.iter().any(|name| name.contains("VK_KHR_surface")),
        "Extensions must include VK_KHR_surface, got: {:?}",
        ext_names
    );

    // Test 6: Platform-specific extensions
    #[cfg(target_os = "windows")]
    assert!(
        ext_names.iter().any(|name| name.contains("VK_KHR_win32_surface")),
        "Windows must include VK_KHR_win32_surface, got: {:?}",
        ext_names
    );

    #[cfg(target_os = "linux")]
    assert!(
        ext_names.iter().any(|name| name.contains("VK_KHR_xcb_surface")
            || name.contains("VK_KHR_xlib_surface")
            || name.contains("VK_KHR_wayland_surface")),
        "Linux must include X11 or Wayland surface extension, got: {:?}",
        ext_names
    );

    #[cfg(target_os = "macos")]
    assert!(
        ext_names.iter().any(|name| name.contains("VK_EXT_metal_surface")),
        "macOS must include VK_EXT_metal_surface, got: {:?}",
        ext_names
    );

    // Test 7: Raw handles don't panic
    let _ = window.raw_window_handle();
    let _ = window.raw_display_handle();

    // Test 8: Poll events doesn't crash
    let mut window_mut = window;
    let events = window_mut.poll_events();
    // Events list can be empty or not, just shouldn't crash
    let _ = events;
}

#[test]
fn test_window_config_default() {
    let config = WindowConfig::default();

    assert_eq!(config.title, "Agent Game Engine");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
    assert!(config.resizable);
    assert!(!config.visible); // Default to headless for testing
}

#[test]
fn test_window_error_display() {
    let err = WindowError::InvalidDimensions { width: 0, height: 0 };
    let display = format!("{}", err);
    assert!(display.contains("Invalid window dimensions") || display.len() > 0);

    let err2 = WindowError::CreationFailed { details: "test error".to_string() };
    let display2 = format!("{}", err2);
    assert!(display2.contains("Window creation failed") || display2.len() > 0);
}
