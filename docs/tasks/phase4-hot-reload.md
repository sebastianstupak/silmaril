# Phase 4.5: Hot-Reload System

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (developer productivity)

---

## 🎯 **Objective**

Implement hot-reload system for assets, shaders, and code (if possible). Enable rapid iteration without restarting the application. Watch file changes and automatically reload modified resources.

**Must support:**
- Asset hot-reload (textures, models, materials)
- Shader hot-reload with recompilation
- File watching with debouncing
- Automatic rebuild on save
- Error handling (fallback to previous version)

---

## 📋 **Detailed Tasks**

### **1. File Watcher** (Day 1)

**File:** `engine/hot_reload/src/watcher.rs`

```rust
use notify::{Watcher, RecursiveMode, Result as NotifyResult, Event, EventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub change_type: FileChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
}

/// File watcher with debouncing
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    receiver: Receiver<NotifyResult<Event>>,
    pending_changes: HashMap<PathBuf, (FileChangeType, Instant)>,
    debounce_duration: Duration,
}

impl FileWatcher {
    /// Create new file watcher
    pub fn new(debounce_ms: u64) -> Result<Self, HotReloadError> {
        let (tx, rx) = channel();

        let watcher = notify::recommended_watcher(move |res| {
            tx.send(res).unwrap();
        })
        .map_err(|e| HotReloadError::WatcherInitFailed {
            details: format!("Failed to create watcher: {}", e),
        })?;

        Ok(Self {
            watcher,
            receiver: rx,
            pending_changes: HashMap::new(),
            debounce_duration: Duration::from_millis(debounce_ms),
        })
    }

    /// Watch directory
    pub fn watch(&mut self, path: &Path) -> Result<(), HotReloadError> {
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| HotReloadError::WatchFailed {
                details: format!("Failed to watch {:?}: {}", path, e),
            })?;

        tracing::info!("Watching directory: {:?}", path);
        Ok(())
    }

    /// Unwatch directory
    pub fn unwatch(&mut self, path: &Path) -> Result<(), HotReloadError> {
        self.watcher
            .unwatch(path)
            .map_err(|e| HotReloadError::UnwatchFailed {
                details: format!("Failed to unwatch {:?}: {}", path, e),
            })?;

        Ok(())
    }

    /// Poll for file changes (call each frame)
    pub fn poll(&mut self) -> Vec<FileChangeEvent> {
        // Process new events
        while let Ok(Ok(event)) = self.receiver.try_recv() {
            self.process_event(event);
        }

        // Check for debounced changes
        let now = Instant::now();
        let mut ready_changes = Vec::new();

        self.pending_changes.retain(|path, (change_type, timestamp)| {
            if now.duration_since(*timestamp) >= self.debounce_duration {
                ready_changes.push(FileChangeEvent {
                    path: path.clone(),
                    change_type: *change_type,
                });
                false // Remove from pending
            } else {
                true // Keep pending
            }
        });

        ready_changes
    }

    /// Process file system event
    fn process_event(&mut self, event: Event) {
        let change_type = match event.kind {
            EventKind::Create(_) => FileChangeType::Created,
            EventKind::Modify(_) => FileChangeType::Modified,
            EventKind::Remove(_) => FileChangeType::Deleted,
            _ => return, // Ignore other events
        };

        for path in event.paths {
            // Update pending change (reset debounce timer)
            self.pending_changes.insert(path, (change_type, Instant::now()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_watcher() {
        let mut watcher = FileWatcher::new(100).unwrap();
        let temp_dir = std::env::temp_dir().join("hot_reload_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        watcher.watch(&temp_dir).unwrap();

        // Create file
        let test_file = temp_dir.join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(150));

        let changes = watcher.poll();
        assert!(changes.iter().any(|c| c.path == test_file));

        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
```

---

### **2. Asset Hot-Reload** (Day 1-2)

**File:** `engine/hot_reload/src/asset_reload.rs`

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Asset type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Texture,
    Model,
    Material,
    Audio,
    Script,
}

impl AssetType {
    /// Detect asset type from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "tga" | "bmp" => Some(AssetType::Texture),
            "gltf" | "glb" | "obj" | "fbx" => Some(AssetType::Model),
            "mat" | "json" => Some(AssetType::Material),
            "wav" | "mp3" | "ogg" => Some(AssetType::Audio),
            "lua" | "wasm" => Some(AssetType::Script),
            _ => None,
        }
    }
}

/// Asset reload handler trait
pub trait AssetReloadHandler: Send + Sync {
    fn reload(&mut self, path: &Path) -> Result<(), HotReloadError>;
}

/// Asset hot-reload manager
pub struct AssetReloadManager {
    handlers: HashMap<AssetType, Box<dyn AssetReloadHandler>>,
    asset_paths: HashMap<PathBuf, AssetType>,
}

impl AssetReloadManager {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            asset_paths: HashMap::new(),
        }
    }

    /// Register reload handler for asset type
    pub fn register_handler(&mut self, asset_type: AssetType, handler: Box<dyn AssetReloadHandler>) {
        self.handlers.insert(asset_type, handler);
        tracing::info!("Registered reload handler for {:?}", asset_type);
    }

    /// Register asset for hot-reload
    pub fn register_asset(&mut self, path: PathBuf, asset_type: AssetType) {
        self.asset_paths.insert(path, asset_type);
    }

    /// Handle file change
    pub fn handle_file_change(&mut self, event: &FileChangeEvent) -> Result<(), HotReloadError> {
        // Detect asset type
        let asset_type = if let Some(&existing_type) = self.asset_paths.get(&event.path) {
            existing_type
        } else if let Some(ext) = event.path.extension() {
            AssetType::from_extension(ext.to_str().unwrap_or("")).ok_or_else(|| {
                HotReloadError::UnsupportedAsset {
                    path: event.path.clone(),
                }
            })?
        } else {
            return Err(HotReloadError::UnsupportedAsset {
                path: event.path.clone(),
            });
        };

        match event.change_type {
            FileChangeType::Modified | FileChangeType::Created => {
                tracing::info!("Reloading asset: {:?} ({:?})", event.path, asset_type);

                if let Some(handler) = self.handlers.get_mut(&asset_type) {
                    handler.reload(&event.path)?;
                    tracing::info!("Asset reloaded successfully: {:?}", event.path);
                } else {
                    tracing::warn!("No handler for asset type {:?}", asset_type);
                }
            }
            FileChangeType::Deleted => {
                tracing::info!("Asset deleted: {:?}", event.path);
                self.asset_paths.remove(&event.path);
            }
        }

        Ok(())
    }
}

/// Texture reload handler
pub struct TextureReloadHandler {
    texture_manager: Arc<Mutex<TextureManager>>,
    device: Arc<VulkanDevice>,
    allocator: Arc<Mutex<VulkanAllocator>>,
}

impl TextureReloadHandler {
    pub fn new(
        texture_manager: Arc<Mutex<TextureManager>>,
        device: Arc<VulkanDevice>,
        allocator: Arc<Mutex<VulkanAllocator>>,
    ) -> Self {
        Self {
            texture_manager,
            device,
            allocator,
        }
    }
}

impl AssetReloadHandler for TextureReloadHandler {
    fn reload(&mut self, path: &Path) -> Result<(), HotReloadError> {
        let mut texture_manager = self.texture_manager.lock().unwrap();
        let mut allocator = self.allocator.lock().unwrap();

        // Reload texture
        let texture_data = TextureData::load_from_file(path, TextureFormat::Rgba8Srgb)
            .map_err(|e| HotReloadError::AssetLoadFailed {
                path: path.to_path_buf(),
                details: e.to_string(),
            })?;

        // TODO: Find existing texture handle and replace data
        // For now, just load as new
        texture_manager
            .create_texture(&self.device, &mut allocator, texture_data)
            .map_err(|e| HotReloadError::AssetLoadFailed {
                path: path.to_path_buf(),
                details: e.to_string(),
            })?;

        Ok(())
    }
}
```

---

### **3. Shader Hot-Reload** (Day 2-3)

**File:** `engine/hot_reload/src/shader_reload.rs`

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

/// Shader compiler
pub struct ShaderCompiler {
    glslc_path: PathBuf,
}

impl ShaderCompiler {
    pub fn new() -> Self {
        // Look for glslc in PATH or Vulkan SDK
        let glslc_path = if cfg!(windows) {
            "glslc.exe"
        } else {
            "glslc"
        };

        Self {
            glslc_path: PathBuf::from(glslc_path),
        }
    }

    /// Compile GLSL to SPIR-V
    pub fn compile(
        &self,
        input: &Path,
        output: &Path,
    ) -> Result<(), HotReloadError> {
        tracing::info!("Compiling shader: {:?} -> {:?}", input, output);

        let output_result = Command::new(&self.glslc_path)
            .arg(input)
            .arg("-o")
            .arg(output)
            .arg("-O") // Optimize
            .output()
            .map_err(|e| HotReloadError::ShaderCompileFailed {
                path: input.to_path_buf(),
                details: format!("Failed to run glslc: {}", e),
            })?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(HotReloadError::ShaderCompileFailed {
                path: input.to_path_buf(),
                details: stderr.to_string(),
            });
        }

        tracing::info!("Shader compiled successfully");
        Ok(())
    }

    /// Compile and validate shader
    pub fn compile_and_validate(
        &self,
        input: &Path,
        output: &Path,
    ) -> Result<Vec<u32>, HotReloadError> {
        self.compile(input, output)?;

        // Load SPIR-V
        let spirv_bytes = std::fs::read(output).map_err(|e| HotReloadError::ShaderLoadFailed {
            path: output.to_path_buf(),
            details: e.to_string(),
        })?;

        // Convert to u32 array
        let spirv = spirv_bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        Ok(spirv)
    }
}

/// Shader reload handler
pub struct ShaderReloadHandler {
    shader_compiler: ShaderCompiler,
    device: Arc<VulkanDevice>,
    shader_cache: HashMap<PathBuf, vk::ShaderModule>,
    output_dir: PathBuf,
}

impl ShaderReloadHandler {
    pub fn new(device: Arc<VulkanDevice>, output_dir: PathBuf) -> Self {
        Self {
            shader_compiler: ShaderCompiler::new(),
            device,
            shader_cache: HashMap::new(),
            output_dir,
        }
    }

    /// Reload shader module
    fn reload_shader_module(&mut self, path: &Path) -> Result<vk::ShaderModule, HotReloadError> {
        // Compile shader
        let output_path = self.output_dir.join(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".glsl", ".spv"),
        );

        let spirv = self
            .shader_compiler
            .compile_and_validate(path, &output_path)?;

        // Create shader module
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&spirv);

        let shader_module = unsafe {
            self.device
                .device()
                .create_shader_module(&create_info, None)
                .map_err(|e| HotReloadError::ShaderCreateFailed {
                    path: path.to_path_buf(),
                    details: e.to_string(),
                })?
        };

        // Destroy old shader module if exists
        if let Some(old_module) = self.shader_cache.insert(path.to_path_buf(), shader_module) {
            unsafe {
                self.device.device().destroy_shader_module(old_module, None);
            }
        }

        tracing::info!("Shader module reloaded: {:?}", path);
        Ok(shader_module)
    }
}

impl AssetReloadHandler for ShaderReloadHandler {
    fn reload(&mut self, path: &Path) -> Result<(), HotReloadError> {
        self.reload_shader_module(path)?;

        // TODO: Recreate pipeline using new shader module
        // This requires tracking which pipelines use this shader

        Ok(())
    }
}
```

---

### **4. Hot-Reload System** (Day 3-4)

**File:** `engine/hot_reload/src/lib.rs`

```rust
mod watcher;
mod asset_reload;
mod shader_reload;

pub use watcher::{FileWatcher, FileChangeEvent, FileChangeType};
pub use asset_reload::{AssetReloadManager, AssetType, AssetReloadHandler};
pub use shader_reload::{ShaderReloadHandler, ShaderCompiler};

use std::path::PathBuf;

/// Hot-reload system
pub struct HotReloadSystem {
    file_watcher: FileWatcher,
    asset_manager: AssetReloadManager,
    enabled: bool,
}

impl HotReloadSystem {
    /// Create new hot-reload system
    pub fn new() -> Result<Self, HotReloadError> {
        Ok(Self {
            file_watcher: FileWatcher::new(200)?, // 200ms debounce
            asset_manager: AssetReloadManager::new(),
            enabled: true,
        })
    }

    /// Enable/disable hot-reload
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        tracing::info!("Hot-reload {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Watch directory for asset changes
    pub fn watch_directory(&mut self, path: &PathBuf) -> Result<(), HotReloadError> {
        self.file_watcher.watch(path)
    }

    /// Register asset reload handler
    pub fn register_handler(&mut self, asset_type: AssetType, handler: Box<dyn AssetReloadHandler>) {
        self.asset_manager.register_handler(asset_type, handler);
    }

    /// Update (call each frame)
    pub fn update(&mut self) {
        if !self.enabled {
            return;
        }

        let changes = self.file_watcher.poll();

        for change in changes {
            tracing::debug!("File changed: {:?} ({:?})", change.path, change.change_type);

            if let Err(e) = self.asset_manager.handle_file_change(&change) {
                tracing::error!("Failed to reload asset {:?}: {:?}", change.path, e);
            }
        }
    }
}

/// Hot-reload error types
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    #[error("Failed to initialize file watcher: {details}")]
    WatcherInitFailed { details: String },

    #[error("Failed to watch path: {details}")]
    WatchFailed { details: String },

    #[error("Failed to unwatch path: {details}")]
    UnwatchFailed { details: String },

    #[error("Unsupported asset: {path:?}")]
    UnsupportedAsset { path: PathBuf },

    #[error("Failed to load asset {path:?}: {details}")]
    AssetLoadFailed { path: PathBuf, details: String },

    #[error("Failed to compile shader {path:?}: {details}")]
    ShaderCompileFailed { path: PathBuf, details: String },

    #[error("Failed to load shader {path:?}: {details}")]
    ShaderLoadFailed { path: PathBuf, details: String },

    #[error("Failed to create shader module {path:?}: {details}")]
    ShaderCreateFailed { path: PathBuf, details: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_system() {
        let mut system = HotReloadSystem::new().unwrap();

        let temp_dir = std::env::temp_dir().join("hot_reload_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        system.watch_directory(&temp_dir).unwrap();

        // Create test asset
        let test_file = temp_dir.join("test.png");
        std::fs::write(&test_file, b"fake png data").unwrap();

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(250));

        system.update();

        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
```

---

### **5. Integration Example** (Day 4)

**File:** `examples/hot_reload.rs`

```rust
use engine::hot_reload::{HotReloadSystem, AssetType, TextureReloadHandler, ShaderReloadHandler};
use std::path::PathBuf;

fn main() {
    // Initialize hot-reload system
    let mut hot_reload = HotReloadSystem::new().unwrap();

    // Watch assets directory
    hot_reload.watch_directory(&PathBuf::from("assets")).unwrap();

    // Register handlers
    let texture_handler = TextureReloadHandler::new(
        texture_manager.clone(),
        device.clone(),
        allocator.clone(),
    );
    hot_reload.register_handler(AssetType::Texture, Box::new(texture_handler));

    let shader_handler = ShaderReloadHandler::new(
        device.clone(),
        PathBuf::from("shaders/compiled"),
    );
    hot_reload.register_handler(AssetType::Shader, Box::new(shader_handler));

    // Main loop
    loop {
        // Update hot-reload system
        hot_reload.update();

        // Render...
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] File watcher with debouncing (200ms)
- [ ] Asset hot-reload (textures, models, materials)
- [ ] Shader hot-reload with automatic recompilation
- [ ] Error handling (fallback on failure)
- [ ] Support for multiple asset types
- [ ] Extensible handler system
- [ ] Watch multiple directories
- [ ] <1ms overhead per frame
- [ ] Cross-platform file watching

---

## 🧪 **Tests**

```rust
#[test]
fn test_file_watcher_debounce() {
    let mut watcher = FileWatcher::new(100).unwrap();
    let temp_dir = std::env::temp_dir().join("debounce_test");
    std::fs::create_dir_all(&temp_dir).unwrap();

    watcher.watch(&temp_dir).unwrap();

    let test_file = temp_dir.join("test.txt");

    // Multiple writes
    for i in 0..10 {
        std::fs::write(&test_file, format!("write {}", i)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }

    // Should only trigger once after debounce
    std::thread::sleep(Duration::from_millis(150));

    let changes = watcher.poll();
    assert_eq!(changes.len(), 1);

    std::fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_shader_compilation() {
    let compiler = ShaderCompiler::new();

    let input = PathBuf::from("shaders/test.frag");
    let output = PathBuf::from("shaders/test.spv");

    std::fs::write(&input, "
        #version 450
        layout(location = 0) out vec4 color;
        void main() { color = vec4(1.0); }
    ").unwrap();

    compiler.compile(&input, &output).unwrap();
    assert!(output.exists());

    std::fs::remove_file(input).unwrap();
    std::fs::remove_file(output).unwrap();
}
```

---

## ⚡ **Performance Targets**

- **File Watching Overhead:** <0.1ms per frame
- **Debounce Time:** 200ms (configurable)
- **Shader Recompilation:** <500ms for typical shader
- **Texture Reload:** <100ms for 2K texture
- **Memory Overhead:** <5 MB

---

## 📚 **Dependencies**

```toml
[dependencies]
notify = "6.0"
thiserror = "1.0"
```

---

**Dependencies:** [phase4-profiling-integration.md](phase4-profiling-integration.md)
**Next:** [phase4-save-load.md](phase4-save-load.md)
