# Phase 1.6: Basic Rendering Pipeline - Specification

**Status:** 📝 Specification
**Version:** 1.0
**Last Updated:** 2026-02-01

---

## 🎯 **Objective**

Implement a production-ready Vulkan rendering pipeline capable of displaying a clear color to a window. This serves as the foundation for all future rendering (meshes, textures, lighting).

**Success Criteria:**
- ✅ Window opens and displays a solid color
- ✅ Render loop maintains 60 FPS
- ✅ Proper synchronization (no tearing, no GPU crashes)
- ✅ Works on Windows, Linux, macOS
- ✅ All Vulkan validation layers pass with zero errors
- ✅ Resources properly cleaned up on exit

---

## 📚 **References & Research**

### **Industry Standards:**
- [Vulkan Tutorial - Frames in Flight](https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Frames_in_flight)
- [Vulkan Tutorial Rust Edition](https://kylemayes.github.io/vulkanalia/drawing/rendering_and_presentation.html)
- [ash-window examples](https://github.com/ash-rs/ash/blob/master/ash-window/examples/winit.rs)
- [KDAB: Synchronization in Vulkan](https://www.kdab.com/synchronization-in-vulkan/)

### **Technology Stack (Researched & Validated):**
- **winit 0.30** - Cross-platform windowing ([docs](https://docs.rs/winit/latest/winit/))
- **raw-window-handle 0.6** - Window/display handle abstraction
- **ash-window 0.13** - Vulkan surface creation helper
- **shaderc 0.8** - GLSL → SPIR-V compilation ([Google library](https://github.com/google/shaderc-rs))
- **glam** - Math library (already integrated)

---

## 🏗️ **Architecture Design**

### **Module Structure:**
```
engine/renderer/src/
├── lib.rs              # Public API
├── context.rs          # Vulkan context (already exists - Phase 1.5)
├── window.rs           # NEW: winit window management
├── surface.rs          # NEW: Vulkan surface (KHR_surface)
├── swapchain.rs        # Existing, may need updates
├── render_pass.rs      # NEW: Render pass configuration
├── framebuffer.rs      # NEW: Framebuffer management
├── command.rs          # NEW: Command pool/buffers
├── sync.rs             # NEW: Fences/semaphores
├── pipeline/
│   ├── mod.rs
│   ├── shader.rs       # NEW: SPIR-V loading
│   └── graphics.rs     # NEW: Graphics pipeline (for later)
└── renderer.rs         # NEW: Main render loop orchestration
```

### **Dependency Graph:**
```
Window (winit)
  ↓
Surface (ash-window) → Instance (Phase 1.5)
  ↓
Swapchain → Device (Phase 1.5)
  ↓
RenderPass
  ↓
Framebuffers (one per swapchain image)
  ↓
CommandBuffers + Sync Objects
  ↓
Renderer (orchestrates all above)
```

---

## 📐 **Component Specifications**

### **1. Window Management (`window.rs`)**

**Purpose:** Create and manage cross-platform window using winit

**Dependencies:**
```toml
winit = "0.30"
raw-window-handle = "0.6"
```

**Public API:**
```rust
pub struct Window {
    winit_window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<()>>,
}

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub visible: bool,
}

impl Window {
    /// Create a new window with the given configuration
    pub fn new(config: WindowConfig) -> Result<Self, WindowError>;

    /// Get the required Vulkan extensions for this window
    pub fn required_extensions(&self) -> Vec<*const c_char>;

    /// Get window size in pixels
    pub fn size(&self) -> (u32, u32);

    /// Check if window should close
    pub fn should_close(&self) -> bool;

    /// Poll events (returns immediately)
    pub fn poll_events(&mut self) -> Vec<WindowEvent>;

    /// Get raw window handle for Vulkan surface creation
    pub fn raw_window_handle(&self) -> RawWindowHandle;

    /// Get raw display handle for Vulkan surface creation
    pub fn raw_display_handle(&self) -> RawDisplayHandle;
}
```

**Test Requirements:**
1. Window creation succeeds with valid config
2. Window creation fails with invalid dimensions (0x0)
3. Required extensions include VK_KHR_surface
4. Platform-specific extensions included (VK_KHR_win32_surface, etc.)
5. Window size returns correct dimensions
6. Raw handles are valid pointers

**Implementation Notes:**
- Use winit's `EventLoop::with_user_event()`
- Store event_loop as `Option` to allow event polling
- Implement `Drop` to ensure cleanup
- Support headless mode (for testing) via visibility flag

---

### **2. Surface Creation (`surface.rs`)**

**Purpose:** Create platform-specific Vulkan surface from window

**Dependencies:**
```toml
ash-window = "0.13"
```

**Public API:**
```rust
pub struct Surface {
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl Surface {
    /// Create Vulkan surface from window
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<Self, SurfaceError>;

    /// Get surface handle
    pub fn handle(&self) -> vk::SurfaceKHR;

    /// Get surface loader (for queries)
    pub fn loader(&self) -> &ash::extensions::khr::Surface;

    /// Check if physical device supports this surface
    pub fn is_supported(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<bool, SurfaceError>;
}
```

**Test Requirements:**
1. Surface creation succeeds with valid window
2. Surface handle is non-null
3. Surface supports at least one present mode
4. Surface supports at least one format
5. Cleanup destroys surface without crashes

**Implementation Notes:**
- Use `ash_window::create_surface()` helper
- Store both surface handle AND loader (needed for queries)
- Verify surface capabilities after creation
- Log surface formats/modes in debug builds

---

### **3. Render Pass (`render_pass.rs`)**

**Purpose:** Define how rendering operations are structured

**Public API:**
```rust
pub struct RenderPass {
    render_pass: vk::RenderPass,
}

pub struct RenderPassConfig {
    pub color_format: vk::Format,
    pub depth_format: Option<vk::Format>, // None for now
    pub samples: vk::SampleCountFlags,
    pub load_op: vk::AttachmentLoadOp,
    pub store_op: vk::AttachmentStoreOp,
}

impl RenderPass {
    /// Create render pass for swapchain rendering
    pub fn new(
        device: &ash::Device,
        config: RenderPassConfig,
    ) -> Result<Self, RenderPassError>;

    /// Get render pass handle
    pub fn handle(&self) -> vk::RenderPass;
}
```

**Render Pass Structure:**
```
Attachment 0: Color (swapchain format)
  - loadOp: CLEAR
  - storeOp: STORE
  - initialLayout: UNDEFINED
  - finalLayout: PRESENT_SRC_KHR

Subpass 0: Graphics
  - pipelineBindPoint: GRAPHICS
  - colorAttachments: [0]

Dependency 0: External → Subpass 0
  - srcStageMask: COLOR_ATTACHMENT_OUTPUT
  - dstStageMask: COLOR_ATTACHMENT_OUTPUT
  - srcAccessMask: 0
  - dstAccessMask: COLOR_ATTACHMENT_WRITE
```

**Test Requirements:**
1. Render pass creation succeeds
2. Render pass handle is valid
3. Compatible with swapchain format
4. Validation layers report no errors
5. Can be used to begin/end render pass

**Implementation Notes:**
- Based on [Vulkan Tutorial - Render Pass](https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Render_passes)
- Single subpass for now (multiple for deferred rendering later)
- Subpass dependency ensures correct synchronization
- Use CLEAR for testing (will see results immediately)

---

### **4. Framebuffers (`framebuffer.rs`)**

**Purpose:** Link render pass to swapchain images

**Public API:**
```rust
pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
}

impl Framebuffer {
    /// Create framebuffer for a swapchain image view
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Result<Self, FramebufferError>;

    /// Get framebuffer handle
    pub fn handle(&self) -> vk::Framebuffer;
}

/// Helper to create framebuffers for all swapchain images
pub fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    swapchain: &Swapchain,
) -> Result<Vec<Framebuffer>, FramebufferError>;
```

**Test Requirements:**
1. Can create framebuffer for each swapchain image
2. Framebuffer dimensions match swapchain extent
3. Framebuffer count equals swapchain image count
4. Can begin render pass with created framebuffers

**Implementation Notes:**
- One framebuffer per swapchain image
- Framebuffers must be recreated on window resize
- Extent must match swapchain extent exactly

---

### **5. Command Management (`command.rs`)**

**Purpose:** Allocate and manage command buffers

**Public API:**
```rust
pub struct CommandPool {
    pool: vk::CommandPool,
}

impl CommandPool {
    /// Create command pool for a queue family
    pub fn new(
        device: &ash::Device,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self, CommandError>;

    /// Allocate command buffers
    pub fn allocate(
        &self,
        device: &ash::Device,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>, CommandError>;

    /// Reset pool (invalidates all allocated buffers)
    pub fn reset(&self, device: &ash::Device) -> Result<(), CommandError>;
}

pub struct CommandBuffer {
    buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    /// Begin recording commands
    pub fn begin(
        &self,
        device: &ash::Device,
        flags: vk::CommandBufferUsageFlags,
    ) -> Result<(), CommandError>;

    /// End recording commands
    pub fn end(&self, device: &ash::Device) -> Result<(), CommandError>;

    /// Begin render pass
    pub fn begin_render_pass(
        &self,
        device: &ash::Device,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
        clear_color: [f32; 4],
    );

    /// End render pass
    pub fn end_render_pass(&self, device: &ash::Device);

    /// Get handle
    pub fn handle(&self) -> vk::CommandBuffer;
}
```

**Test Requirements:**
1. Command pool creation succeeds
2. Can allocate PRIMARY command buffers
3. Can begin/end command buffer recording
4. Can reset pool without errors
5. Validation layers report no errors

**Implementation Notes:**
- Use RESET_COMMAND_BUFFER flag for individual buffer reset
- Allocate one buffer per frame in flight (2 typical)
- Command buffers auto-freed on pool destruction

---

### **6. Synchronization (`sync.rs`)**

**Purpose:** Manage GPU-CPU and GPU-GPU synchronization

**Public API:**
```rust
pub struct FrameSyncObjects {
    pub image_available_semaphore: vk::Semaphore,
    pub render_finished_semaphore: vk::Semaphore,
    pub in_flight_fence: vk::Fence,
}

impl FrameSyncObjects {
    /// Create sync objects for one frame
    pub fn new(device: &ash::Device) -> Result<Self, SyncError>;

    /// Wait for fence
    pub fn wait(&self, device: &ash::Device, timeout_ns: u64) -> Result<(), SyncError>;

    /// Reset fence
    pub fn reset(&self, device: &ash::Device) -> Result<(), SyncError>;
}

/// Create sync objects for multiple frames in flight
pub fn create_sync_objects(
    device: &ash::Device,
    frames_in_flight: u32,
) -> Result<Vec<FrameSyncObjects>, SyncError>;
```

**Synchronization Pattern:**
```
Frame N:
  1. wait_for_fences([in_flight_fence])
  2. acquire_next_image(..., image_available_semaphore)
  3. reset_fences([in_flight_fence])
  4. record command buffer
  5. queue_submit(
       wait: [image_available_semaphore],
       signal: [render_finished_semaphore],
       fence: in_flight_fence
     )
  6. queue_present(wait: [render_finished_semaphore])
```

**Test Requirements:**
1. Can create sync objects
2. Fence starts in signaled state
3. Can wait on fence
4. Can reset fence
5. Semaphores can be used in submit/present

**Implementation Notes:**
- Based on ["Frames in Flight" pattern](https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Frames_in_flight)
- Typical: 2-3 frames in flight (2 recommended for lower latency)
- Fences: CPU-GPU sync, Semaphores: GPU-GPU sync
- Fence created with SIGNALED flag (first frame doesn't wait)

---

### **7. Shader Module (`pipeline/shader.rs`)**

**Purpose:** Load and create Vulkan shader modules from SPIR-V

**Public API:**
```rust
pub struct ShaderModule {
    module: vk::ShaderModule,
}

impl ShaderModule {
    /// Create shader module from SPIR-V bytes
    pub fn from_spirv(
        device: &ash::Device,
        spirv_code: &[u8],
    ) -> Result<Self, ShaderError>;

    /// Create shader stage info (for pipeline creation)
    pub fn stage_info(
        &self,
        stage: vk::ShaderStageFlags,
        entry_point: &CStr,
    ) -> vk::PipelineShaderStageCreateInfo;

    /// Get module handle
    pub fn handle(&self) -> vk::ShaderModule;
}
```

**Build-time Compilation (`build.rs`):**
```rust
use shaderc::{Compiler, ShaderKind};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun if shaders change
    println!("cargo:rerun-if-changed=shaders/");

    let mut compiler = Compiler::new().unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Compile vertex shader
    compile_shader(
        &mut compiler,
        "shaders/simple.vert",
        ShaderKind::Vertex,
        &format!("{}/simple.vert.spv", out_dir),
    );

    // Compile fragment shader
    compile_shader(
        &mut compiler,
        "shaders/simple.frag",
        ShaderKind::Fragment,
        &format!("{}/simple.frag.spv", out_dir),
    );
}

fn compile_shader(
    compiler: &mut Compiler,
    src_path: &str,
    kind: ShaderKind,
    out_path: &str,
) {
    let source = fs::read_to_string(src_path).unwrap();
    let artifact = compiler
        .compile_into_spirv(&source, kind, src_path, "main", None)
        .unwrap();
    fs::write(out_path, artifact.as_binary_u8()).unwrap();
}
```

**Simple Shaders:**
```glsl
// shaders/simple.vert
#version 450

layout(location = 0) out vec3 fragColor;

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    fragColor = colors[gl_VertexIndex];
}
```

```glsl
// shaders/simple.frag
#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(fragColor, 1.0);
}
```

**Test Requirements:**
1. Can create shader module from valid SPIR-V
2. Rejects invalid SPIR-V bytes
3. Build script compiles shaders successfully
4. Changes to GLSL trigger rebuild

**Implementation Notes:**
- Based on [shaderc-rs documentation](https://github.com/google/shaderc-rs)
- Use `include_bytes!` to embed compiled shaders
- Shaders compiled at build time (not runtime)
- Keep shaders simple for Phase 1.6 (hardcoded triangle for testing)

---

### **8. Main Renderer (`renderer.rs`)**

**Purpose:** Orchestrate all components into working render loop

**Public API:**
```rust
pub struct Renderer {
    // Window & Surface
    window: Window,
    surface: Surface,

    // Vulkan Core (from Phase 1.5)
    context: VulkanContext,
    swapchain: Swapchain,

    // Rendering
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,

    // Commands
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,

    // Synchronization
    sync_objects: Vec<FrameSyncObjects>,
    current_frame: usize,
}

pub struct RendererConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub max_frames_in_flight: u32,
}

impl Renderer {
    /// Create new renderer with window
    pub fn new(config: RendererConfig) -> Result<Self, RendererError>;

    /// Render one frame
    pub fn render_frame(&mut self, clear_color: [f32; 4]) -> Result<(), RendererError>;

    /// Check if window should close
    pub fn should_close(&self) -> bool;

    /// Handle window events
    pub fn poll_events(&mut self);

    /// Wait for GPU to finish all work
    pub fn wait_idle(&self);

    /// Handle window resize
    pub fn recreate_swapchain(&mut self) -> Result<(), RendererError>;
}
```

**Render Loop Algorithm:**
```
loop {
    renderer.poll_events();

    if renderer.should_close() {
        break;
    }

    renderer.render_frame([0.1, 0.2, 0.3, 1.0])?;
}

renderer.wait_idle();
```

**Test Requirements:**
1. Renderer creation succeeds
2. Can render 1000 frames without error
3. FPS stays above 55 (target 60)
4. Validation layers report zero errors
5. Clean shutdown without leaks
6. Window resize works correctly

**Implementation Notes:**
- MAX_FRAMES_IN_FLIGHT = 2 (recommended)
- Handle VK_ERROR_OUT_OF_DATE_KHR (window resize)
- Record command buffers every frame (for now, optimize later)
- Use render pass clear color for initial testing

---

## 🧪 **Testing Strategy**

### **Test Levels:**

#### **1. Unit Tests (Per Module)**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_window_creation() { ... }

    #[test]
    fn test_surface_creation() { ... }

    #[test]
    fn test_render_pass_creation() { ... }
}
```

#### **2. Integration Tests (`tests/rendering_integration.rs`)**
```rust
#[test]
fn test_full_rendering_pipeline() {
    let renderer = Renderer::new(RendererConfig::default()).unwrap();

    // Render 100 frames
    for _ in 0..100 {
        renderer.render_frame([0.0, 0.0, 0.0, 1.0]).unwrap();
    }

    renderer.wait_idle();
}

#[test]
fn test_window_resize() {
    // Test swapchain recreation
}

#[test]
fn test_validation_layers_pass() {
    // Ensure zero validation errors
}
```

#### **3. Visual Tests (Manual/Screenshot)**
```rust
#[test]
#[ignore] // Manual test
fn test_clear_color_visual() {
    // Render with known color, save screenshot, verify pixels
}
```

#### **4. Performance Tests**
```rust
#[test]
fn test_render_loop_performance() {
    let mut renderer = Renderer::new(config).unwrap();
    let start = Instant::now();

    for _ in 0..600 {
        renderer.render_frame([0.0, 0.0, 0.0, 1.0]).unwrap();
    }

    let duration = start.elapsed();
    let fps = 600.0 / duration.as_secs_f64();

    assert!(fps >= 55.0, "FPS too low: {}", fps);
}
```

---

## 📦 **Dependencies**

### **New Cargo.toml Additions:**
```toml
[dependencies]
# Existing from Phase 1.5
ash = "0.38"
gpu-allocator = "0.28"
tracing = "0.1"
thiserror = "2.0"

# NEW for Phase 1.6
winit = "0.30"
raw-window-handle = "0.6"
ash-window = "0.13"
glam = "0.29"

[build-dependencies]
shaderc = "0.8"

[dev-dependencies]
criterion = "0.5"
image = "0.25" # For screenshot verification
```

---

## 🎯 **Acceptance Criteria**

### **Functional Requirements:**
- ✅ Window opens at 1280x720 resolution
- ✅ Window title displays "Agent Game Engine"
- ✅ Clear color renders correctly (RGB values match exactly)
- ✅ Window can be resized without crashes
- ✅ Window can be closed cleanly
- ✅ Multiple render/destroy cycles work (no leaks)

### **Performance Requirements:**
- ✅ Render loop maintains ≥55 FPS (target 60 FPS)
- ✅ Frame time ≤18ms (target 16.67ms)
- ✅ GPU utilization <10% (just clear color)
- ✅ Memory usage <100MB

### **Quality Requirements:**
- ✅ Zero Vulkan validation errors
- ✅ Zero memory leaks (valgrind/ASAN clean)
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ Works on Windows 10+, Ubuntu 22.04+, macOS 12+

### **Documentation Requirements:**
- ✅ All public APIs documented with rustdoc
- ✅ Examples included in doc comments
- ✅ README.md updated with build instructions
- ✅ This spec fully implemented

---

## 📅 **Implementation Timeline**

### **Phase 1.6.1: Window & Surface (Day 1-2)**
- Implement `window.rs`
- Implement `surface.rs`
- Write tests
- Verify window opens successfully

### **Phase 1.6.2: Render Pass & Framebuffers (Day 2-3)**
- Implement `render_pass.rs`
- Implement `framebuffer.rs`
- Write tests
- Verify framebuffers created for swapchain

### **Phase 1.6.3: Commands & Sync (Day 3-4)**
- Implement `command.rs`
- Implement `sync.rs`
- Write tests
- Verify synchronization works

### **Phase 1.6.4: Shaders & Build (Day 4)**
- Implement `shader.rs`
- Write `build.rs` for GLSL compilation
- Create simple vertex/fragment shaders
- Verify shaders compile and load

### **Phase 1.6.5: Renderer Integration (Day 5)**
- Implement `renderer.rs`
- Wire all components together
- Write integration tests
- **MILESTONE:** See clear color on screen!

### **Phase 1.6.6: Polish & Testing (Day 6)**
- Fix any bugs
- Add performance tests
- Verify on all platforms
- Update documentation

---

## 🔍 **Validation Checklist**

Before marking Phase 1.6 complete:

- [ ] Code compiles on Windows, Linux, macOS
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Performance benchmarks meet targets
- [ ] Validation layers enabled in debug builds
- [ ] Zero validation errors reported
- [ ] Memory leak check passes
- [ ] Documentation complete
- [ ] Examples added
- [ ] Clean code review (cargo clippy passes)
- [ ] Formatted (cargo fmt passes)

---

## 🎓 **Learning Resources**

**Required Reading:**
1. [Vulkan Tutorial - Drawing a Triangle](https://vulkan-tutorial.com/Drawing_a_triangle)
2. [Vulkan Tutorial (Rust Edition)](https://kylemayes.github.io/vulkanalia/)
3. [Frames in Flight Explained](https://erfan-ahmadi.github.io/blog/Nabla/fif)
4. [KDAB: Synchronization in Vulkan](https://www.kdab.com/synchronization-in-vulkan/)

**API Documentation:**
- [winit docs](https://docs.rs/winit/latest/winit/)
- [ash docs](https://docs.rs/ash/latest/ash/)
- [shaderc-rs docs](https://docs.rs/shaderc/latest/shaderc/)

---

**Status:** Ready for test-driven implementation
**Next Step:** Begin with specifications → write tests → implement
