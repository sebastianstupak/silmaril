//! Stress tests for the native Vulkan viewport.
//!
//! These tests exercise edge cases in the Vulkan child-window lifecycle:
//! create/destroy cycles, rapid resizing, zero-size viewports, etc.
//!
//! All tests are `#[ignore]` because they require a GPU with Vulkan support
//! and a Win32 desktop session.  Run them with:
//!
//!   cargo test --package silmaril-editor -- --ignored

#![cfg(windows)]

use silmaril_editor::viewport::native_viewport::{NativeViewport, ViewportBounds};
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

// ---------------------------------------------------------------------------
// Helper: create a hidden top-level window that acts as the parent for the
// child viewport, similar to what Tauri provides.
// ---------------------------------------------------------------------------

fn create_hidden_parent() -> HWND {
    unsafe {
        let class_name = windows::core::w!("SilmarilTestParent");
        let hinstance: HINSTANCE = GetModuleHandleW(None).unwrap().into();

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_OWNDC,
            lpfnWndProc: Some(test_wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            windows::core::w!("TestParent"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            1024,
            768,
            None,
            None,
            Some(hinstance),
            None,
        )
        .expect("Failed to create test parent window");

        // Do NOT call ShowWindow — keep hidden for headless testing
        hwnd
    }
}

unsafe extern "system" fn test_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn destroy_parent(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn default_bounds() -> ViewportBounds {
    ViewportBounds {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Basic lifecycle: create viewport, render a few frames, destroy.
#[test]
#[ignore]
fn test_viewport_create_render_destroy() {
    let parent = create_hidden_parent();
    let mut vp = NativeViewport::new(parent, default_bounds()).expect("create viewport");
    vp.start_rendering().expect("start rendering");

    // Let the render thread run for ~5 frames
    std::thread::sleep(std::time::Duration::from_millis(100));

    vp.destroy();
    destroy_parent(parent);
}

/// Create and destroy the viewport multiple times without leaking resources.
#[test]
#[ignore]
fn test_viewport_create_destroy_cycles() {
    let parent = create_hidden_parent();

    for i in 0..10 {
        let mut vp =
            NativeViewport::new(parent, default_bounds()).unwrap_or_else(|e| {
                panic!("create viewport (cycle {i}): {e}")
            });
        vp.start_rendering()
            .unwrap_or_else(|e| panic!("start rendering (cycle {i}): {e}"));

        // Let it render a few frames
        std::thread::sleep(std::time::Duration::from_millis(80));

        vp.destroy();
    }

    destroy_parent(parent);
}

/// Rapidly resize the viewport (simulating panel drag).
#[test]
#[ignore]
fn test_viewport_rapid_resize() {
    let parent = create_hidden_parent();
    let mut vp = NativeViewport::new(parent, default_bounds()).expect("create viewport");
    vp.start_rendering().expect("start rendering");

    // Wait for Vulkan init
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Fire 50 resize events in quick succession
    for i in 0..50 {
        let w = 200 + (i * 10);
        let h = 150 + (i * 8);
        vp.set_bounds(ViewportBounds {
            x: 0,
            y: 0,
            width: w,
            height: h,
        });
        // Small delay to let a frame or two render at each size
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Let it stabilise
    std::thread::sleep(std::time::Duration::from_millis(200));

    vp.destroy();
    destroy_parent(parent);
}

/// Handle zero-size gracefully (should clamp to 1x1 internally).
#[test]
#[ignore]
fn test_viewport_zero_size() {
    let parent = create_hidden_parent();
    let bounds = ViewportBounds {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let mut vp = NativeViewport::new(parent, bounds).expect("create viewport with zero size");
    vp.start_rendering().expect("start rendering");

    std::thread::sleep(std::time::Duration::from_millis(100));

    vp.destroy();
    destroy_parent(parent);
}

/// Very small viewport (1x1).
#[test]
#[ignore]
fn test_viewport_one_pixel() {
    let parent = create_hidden_parent();
    let bounds = ViewportBounds {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    let mut vp = NativeViewport::new(parent, bounds).expect("create 1x1 viewport");
    vp.start_rendering().expect("start rendering");

    std::thread::sleep(std::time::Duration::from_millis(100));

    vp.destroy();
    destroy_parent(parent);
}

/// Large viewport (4K resolution).
#[test]
#[ignore]
fn test_viewport_4k() {
    let parent = create_hidden_parent();
    let bounds = ViewportBounds {
        x: 0,
        y: 0,
        width: 3840,
        height: 2160,
    };
    let mut vp = NativeViewport::new(parent, bounds).expect("create 4K viewport");
    vp.start_rendering().expect("start rendering");

    // Let it render several frames at 4K
    std::thread::sleep(std::time::Duration::from_millis(200));

    vp.destroy();
    destroy_parent(parent);
}

/// Stress test: create, render 10 frames, destroy -- repeat 50 times.
#[test]
#[ignore]
fn test_viewport_lifecycle_stress() {
    let parent = create_hidden_parent();

    for cycle in 0..50 {
        let mut vp = NativeViewport::new(parent, default_bounds())
            .unwrap_or_else(|e| panic!("cycle {cycle}: create: {e}"));
        vp.start_rendering()
            .unwrap_or_else(|e| panic!("cycle {cycle}: start: {e}"));

        // ~10 frames at 16ms each
        std::thread::sleep(std::time::Duration::from_millis(160));

        vp.destroy();
    }

    destroy_parent(parent);
}

/// Resize to zero then back to normal size.
#[test]
#[ignore]
fn test_viewport_resize_zero_then_restore() {
    let parent = create_hidden_parent();
    let mut vp = NativeViewport::new(parent, default_bounds()).expect("create viewport");
    vp.start_rendering().expect("start rendering");

    std::thread::sleep(std::time::Duration::from_millis(100));

    // Shrink to zero
    vp.set_bounds(ViewportBounds {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Restore to normal
    vp.set_bounds(default_bounds());
    std::thread::sleep(std::time::Duration::from_millis(100));

    vp.destroy();
    destroy_parent(parent);
}

/// Drop without explicit destroy() -- tests Drop impl.
#[test]
#[ignore]
fn test_viewport_drop_cleanup() {
    let parent = create_hidden_parent();

    {
        let mut vp = NativeViewport::new(parent, default_bounds()).expect("create viewport");
        vp.start_rendering().expect("start rendering");
        std::thread::sleep(std::time::Duration::from_millis(100));
        // vp goes out of scope here -- Drop should clean up
    }

    // If we get here without hanging or crashing, cleanup worked
    destroy_parent(parent);
}

/// Multiple viewports simultaneously (different child windows of same parent).
#[test]
#[ignore]
fn test_viewport_multiple_simultaneous() {
    let parent = create_hidden_parent();

    let mut viewports = Vec::new();
    for i in 0..3 {
        let bounds = ViewportBounds {
            x: (i as i32) * 200,
            y: 0,
            width: 200,
            height: 200,
        };
        let mut vp = NativeViewport::new(parent, bounds)
            .unwrap_or_else(|e| panic!("create viewport {i}: {e}"));
        vp.start_rendering()
            .unwrap_or_else(|e| panic!("start viewport {i}: {e}"));
        viewports.push(vp);
    }

    // Let them all render
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Destroy in reverse order
    for mut vp in viewports.into_iter().rev() {
        vp.destroy();
    }

    destroy_parent(parent);
}

/// Resize with extreme aspect ratios.
#[test]
#[ignore]
fn test_viewport_extreme_aspect_ratios() {
    let parent = create_hidden_parent();
    let mut vp = NativeViewport::new(parent, default_bounds()).expect("create viewport");
    vp.start_rendering().expect("start rendering");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Very wide
    vp.set_bounds(ViewportBounds {
        x: 0,
        y: 0,
        width: 2000,
        height: 10,
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Very tall
    vp.set_bounds(ViewportBounds {
        x: 0,
        y: 0,
        width: 10,
        height: 2000,
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Back to normal
    vp.set_bounds(default_bounds());
    std::thread::sleep(std::time::Duration::from_millis(100));

    vp.destroy();
    destroy_parent(parent);
}
