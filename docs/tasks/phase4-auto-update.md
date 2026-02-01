# Phase 4.1: Auto-Update System

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (production feature)

---

## 🎯 **Objective**

Implement automatic update system with delta patching, version checking, download management, and customizable UI. Enable seamless updates without full re-downloads.

**Must support:**
- Delta patching with xdelta3
- Version manifest checking
- Incremental downloads with resume
- Customizable update UI
- Rollback on failure

---

## 📋 **Detailed Tasks**

### **1. Version Manifest System** (Day 1)

**File:** `engine/update/src/manifest.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

/// Version manifest describing an update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub version: String,
    pub build_number: u64,
    pub release_date: String,
    pub files: Vec<FileEntry>,
    pub delta_patches: Vec<DeltaPatch>,
    pub required_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub hash: String, // SHA-256
    pub compressed_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPatch {
    pub from_version: String,
    pub to_version: String,
    pub patch_file: String,
    pub size: u64,
    pub hash: String,
}

impl VersionManifest {
    /// Load manifest from URL
    pub async fn fetch(url: &str) -> Result<Self, UpdateError> {
        let response = reqwest::get(url).await.map_err(|e| UpdateError::NetworkError {
            details: format!("Failed to fetch manifest: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(UpdateError::ManifestError {
                details: format!("HTTP {}", response.status()),
            });
        }

        let manifest: VersionManifest = response
            .json()
            .await
            .map_err(|e| UpdateError::ManifestError {
                details: format!("Failed to parse manifest: {}", e),
            })?;

        tracing::info!("Loaded manifest: version {}", manifest.version);
        Ok(manifest)
    }

    /// Load local manifest
    pub fn load_local(path: &PathBuf) -> Result<Self, UpdateError> {
        let content = std::fs::read_to_string(path).map_err(|e| UpdateError::IoError {
            details: format!("Failed to read manifest: {}", e),
        })?;

        let manifest: VersionManifest = serde_json::from_str(&content).map_err(|e| {
            UpdateError::ManifestError {
                details: format!("Failed to parse manifest: {}", e),
            }
        })?;

        Ok(manifest)
    }

    /// Save manifest to disk
    pub fn save(&self, path: &PathBuf) -> Result<(), UpdateError> {
        let content = serde_json::to_string_pretty(self).map_err(|e| UpdateError::IoError {
            details: format!("Failed to serialize manifest: {}", e),
        })?;

        std::fs::write(path, content).map_err(|e| UpdateError::IoError {
            details: format!("Failed to write manifest: {}", e),
        })?;

        Ok(())
    }

    /// Find delta patch from version
    pub fn find_delta_patch(&self, from_version: &str) -> Option<&DeltaPatch> {
        self.delta_patches
            .iter()
            .find(|p| p.from_version == from_version)
    }

    /// Calculate update size (delta if available, else full)
    pub fn calculate_update_size(&self, current_version: Option<&str>) -> u64 {
        if let Some(current) = current_version {
            if let Some(patch) = self.find_delta_patch(current) {
                return patch.size;
            }
        }

        // Full download size
        self.files.iter().map(|f| f.compressed_size.unwrap_or(f.size)).sum()
    }
}
```

---

### **2. Delta Patching with xdelta3** (Day 1-2)

**File:** `engine/update/src/delta.rs`

```rust
use std::path::Path;
use std::process::Command;
use sha2::{Sha256, Digest};

/// Delta patcher using xdelta3
pub struct DeltaPatcher {
    xdelta_path: String,
}

impl DeltaPatcher {
    pub fn new() -> Self {
        // Look for xdelta3 in PATH or bundled
        let xdelta_path = if cfg!(windows) {
            "xdelta3.exe"
        } else {
            "xdelta3"
        };

        Self {
            xdelta_path: xdelta_path.to_string(),
        }
    }

    /// Apply delta patch to file
    pub fn apply_patch(
        &self,
        source_file: &Path,
        patch_file: &Path,
        output_file: &Path,
    ) -> Result<(), UpdateError> {
        tracing::info!("Applying patch: {:?} -> {:?}", source_file, output_file);

        let output = Command::new(&self.xdelta_path)
            .arg("-d")
            .arg("-s")
            .arg(source_file)
            .arg(patch_file)
            .arg(output_file)
            .output()
            .map_err(|e| UpdateError::PatchError {
                details: format!("Failed to run xdelta3: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UpdateError::PatchError {
                details: format!("xdelta3 failed: {}", stderr),
            });
        }

        tracing::info!("Patch applied successfully");
        Ok(())
    }

    /// Verify file hash
    pub fn verify_hash(file_path: &Path, expected_hash: &str) -> Result<bool, UpdateError> {
        let content = std::fs::read(file_path).map_err(|e| UpdateError::IoError {
            details: format!("Failed to read file: {}", e),
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = format!("{:x}", hasher.finalize());

        Ok(hash == expected_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_verification() {
        // Create temporary file with known content
        let temp_file = std::env::temp_dir().join("test_hash.txt");
        std::fs::write(&temp_file, b"test content").unwrap();

        let expected_hash = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72";
        assert!(DeltaPatcher::verify_hash(&temp_file, expected_hash).unwrap());

        std::fs::remove_file(temp_file).unwrap();
    }
}
```

---

### **3. Download Manager** (Day 2)

**File:** `engine/update/src/download.rs`

```rust
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;

/// Download progress callback
pub trait DownloadProgress: Send + Sync {
    fn on_progress(&self, downloaded: u64, total: u64);
}

/// Download manager with resume support
pub struct DownloadManager {
    client: reqwest::Client,
}

impl DownloadManager {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap();

        Self { client }
    }

    /// Download file with resume support
    pub async fn download_file(
        &self,
        url: &str,
        dest_path: &PathBuf,
        progress: Option<&dyn DownloadProgress>,
    ) -> Result<(), UpdateError> {
        tracing::info!("Downloading: {} -> {:?}", url, dest_path);

        // Check if partial download exists
        let start_byte = if dest_path.exists() {
            std::fs::metadata(dest_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        // Request with Range header
        let mut request = self.client.get(url);
        if start_byte > 0 {
            request = request.header("Range", format!("bytes={}-", start_byte));
            tracing::info!("Resuming download from byte {}", start_byte);
        }

        let response = request.send().await.map_err(|e| UpdateError::NetworkError {
            details: format!("Failed to download: {}", e),
        })?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(UpdateError::NetworkError {
                details: format!("HTTP {}", response.status()),
            });
        }

        let total_size = response.content_length().unwrap_or(0) + start_byte;

        // Open file for writing (append if resuming)
        let mut file = if start_byte > 0 {
            File::options()
                .append(true)
                .open(dest_path)
                .await
        } else {
            File::create(dest_path).await
        }
        .map_err(|e| UpdateError::IoError {
            details: format!("Failed to open file: {}", e),
        })?;

        // Download chunks
        let mut downloaded = start_byte;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| UpdateError::NetworkError {
                details: format!("Download interrupted: {}", e),
            })?;

            file.write_all(&chunk).await.map_err(|e| UpdateError::IoError {
                details: format!("Failed to write chunk: {}", e),
            })?;

            downloaded += chunk.len() as u64;

            if let Some(progress_fn) = progress {
                progress_fn.on_progress(downloaded, total_size);
            }
        }

        file.flush().await.map_err(|e| UpdateError::IoError {
            details: format!("Failed to flush file: {}", e),
        })?;

        tracing::info!("Download complete: {:?}", dest_path);
        Ok(())
    }

    /// Download with hash verification
    pub async fn download_and_verify(
        &self,
        url: &str,
        dest_path: &PathBuf,
        expected_hash: &str,
        progress: Option<&dyn DownloadProgress>,
    ) -> Result<(), UpdateError> {
        self.download_file(url, dest_path, progress).await?;

        // Verify hash
        if !DeltaPatcher::verify_hash(dest_path, expected_hash)? {
            return Err(UpdateError::HashMismatch {
                details: format!("Hash mismatch for {:?}", dest_path),
            });
        }

        Ok(())
    }
}
```

---

### **4. Update Engine** (Day 3)

**File:** `engine/update/src/updater.rs`

```rust
use std::path::PathBuf;

/// Update configuration
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub manifest_url: String,
    pub update_url_base: String,
    pub install_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub current_version: String,
}

/// Update state
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    CheckingForUpdates,
    UpdateAvailable { version: String, size: u64 },
    Downloading { progress: f32 },
    Applying,
    Completed,
    Failed { error: String },
}

/// Update engine
pub struct Updater {
    config: UpdateConfig,
    state: UpdateState,
    download_manager: DownloadManager,
    patcher: DeltaPatcher,
}

impl Updater {
    pub fn new(config: UpdateConfig) -> Self {
        Self {
            config,
            state: UpdateState::Idle,
            download_manager: DownloadManager::new(),
            patcher: DeltaPatcher::new(),
        }
    }

    /// Check for updates
    pub async fn check_for_updates(&mut self) -> Result<Option<VersionManifest>, UpdateError> {
        self.state = UpdateState::CheckingForUpdates;

        let manifest = VersionManifest::fetch(&self.config.manifest_url).await?;

        if manifest.version != self.config.current_version {
            let size = manifest.calculate_update_size(Some(&self.config.current_version));
            self.state = UpdateState::UpdateAvailable {
                version: manifest.version.clone(),
                size,
            };
            Ok(Some(manifest))
        } else {
            self.state = UpdateState::Idle;
            Ok(None)
        }
    }

    /// Download and apply update
    pub async fn apply_update(
        &mut self,
        manifest: &VersionManifest,
        progress: Option<&dyn DownloadProgress>,
    ) -> Result<(), UpdateError> {
        self.state = UpdateState::Downloading { progress: 0.0 };

        // Check if delta patch available
        if let Some(patch) = manifest.find_delta_patch(&self.config.current_version) {
            self.apply_delta_update(manifest, patch, progress).await?;
        } else {
            self.apply_full_update(manifest, progress).await?;
        }

        self.state = UpdateState::Completed;
        Ok(())
    }

    /// Apply delta update
    async fn apply_delta_update(
        &mut self,
        manifest: &VersionManifest,
        patch: &DeltaPatch,
        progress: Option<&dyn DownloadProgress>,
    ) -> Result<(), UpdateError> {
        tracing::info!("Applying delta update: {} -> {}", patch.from_version, patch.to_version);

        // Download patch file
        let patch_path = self.config.temp_dir.join(&patch.patch_file);
        let patch_url = format!("{}/{}", self.config.update_url_base, patch.patch_file);

        self.download_manager
            .download_and_verify(&patch_url, &patch_path, &patch.hash, progress)
            .await?;

        self.state = UpdateState::Applying;

        // Apply patches to each file
        for file_entry in &manifest.files {
            let source_path = self.config.install_dir.join(&file_entry.path);
            let output_path = self.config.temp_dir.join(&file_entry.path);

            // Create parent directory
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| UpdateError::IoError {
                    details: format!("Failed to create directory: {}", e),
                })?;
            }

            self.patcher
                .apply_patch(&source_path, &patch_path, &output_path)?;

            // Verify hash
            if !DeltaPatcher::verify_hash(&output_path, &file_entry.hash)? {
                return Err(UpdateError::HashMismatch {
                    details: format!("Hash mismatch for {:?}", output_path),
                });
            }
        }

        // Move updated files to install directory
        self.finalize_update(manifest)?;

        Ok(())
    }

    /// Apply full update
    async fn apply_full_update(
        &mut self,
        manifest: &VersionManifest,
        progress: Option<&dyn DownloadProgress>,
    ) -> Result<(), UpdateError> {
        tracing::info!("Applying full update");

        self.state = UpdateState::Downloading { progress: 0.0 };

        // Download all files
        for file_entry in &manifest.files {
            let file_path = self.config.temp_dir.join(&file_entry.path);
            let file_url = format!("{}/{}", self.config.update_url_base, file_entry.path);

            // Create parent directory
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| UpdateError::IoError {
                    details: format!("Failed to create directory: {}", e),
                })?;
            }

            self.download_manager
                .download_and_verify(&file_url, &file_path, &file_entry.hash, progress)
                .await?;
        }

        self.state = UpdateState::Applying;

        // Move files to install directory
        self.finalize_update(manifest)?;

        Ok(())
    }

    /// Finalize update (move files, update manifest)
    fn finalize_update(&self, manifest: &VersionManifest) -> Result<(), UpdateError> {
        tracing::info!("Finalizing update");

        // Backup current version (for rollback)
        let backup_dir = self.config.temp_dir.join("backup");
        std::fs::create_dir_all(&backup_dir).map_err(|e| UpdateError::IoError {
            details: format!("Failed to create backup directory: {}", e),
        })?;

        // Move updated files
        for file_entry in &manifest.files {
            let source = self.config.temp_dir.join(&file_entry.path);
            let dest = self.config.install_dir.join(&file_entry.path);

            // Backup original if exists
            if dest.exists() {
                let backup = backup_dir.join(&file_entry.path);
                if let Some(parent) = backup.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::rename(&dest, &backup).ok();
            }

            // Move new file
            std::fs::rename(&source, &dest).map_err(|e| UpdateError::IoError {
                details: format!("Failed to move file: {}", e),
            })?;
        }

        // Save new manifest
        let manifest_path = self.config.install_dir.join("version.json");
        manifest.save(&manifest_path)?;

        tracing::info!("Update finalized");
        Ok(())
    }

    pub fn state(&self) -> &UpdateState {
        &self.state
    }
}
```

---

### **5. Customizable Update UI** (Day 4)

**File:** `engine/update/src/ui.rs`

```rust
use egui::{Context, Window, ProgressBar, RichText, Color32};

/// Update UI widget
pub struct UpdateUI {
    show_window: bool,
    state: UpdateState,
    manifest: Option<VersionManifest>,
}

impl UpdateUI {
    pub fn new() -> Self {
        Self {
            show_window: false,
            state: UpdateState::Idle,
            manifest: None,
        }
    }

    /// Update state
    pub fn update_state(&mut self, state: UpdateState, manifest: Option<VersionManifest>) {
        self.state = state;
        self.manifest = manifest;

        // Show window on update available
        if matches!(self.state, UpdateState::UpdateAvailable { .. }) {
            self.show_window = true;
        }
    }

    /// Render UI
    pub fn render(&mut self, ctx: &Context) -> UpdateUIAction {
        let mut action = UpdateUIAction::None;

        if !self.show_window {
            return action;
        }

        Window::new("Update Available")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                match &self.state {
                    UpdateState::UpdateAvailable { version, size } => {
                        ui.heading(RichText::new("New Version Available!").color(Color32::GREEN));
                        ui.separator();

                        ui.label(format!("Version: {}", version));
                        ui.label(format!("Size: {:.2} MB", *size as f64 / 1_048_576.0));

                        if let Some(manifest) = &self.manifest {
                            ui.label(format!("Release Date: {}", manifest.release_date));
                        }

                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("Update Now").clicked() {
                                action = UpdateUIAction::StartUpdate;
                            }
                            if ui.button("Later").clicked() {
                                self.show_window = false;
                            }
                        });
                    }
                    UpdateState::Downloading { progress } => {
                        ui.heading("Downloading Update");
                        ui.separator();

                        ui.add(ProgressBar::new(*progress).show_percentage());
                    }
                    UpdateState::Applying => {
                        ui.heading("Applying Update");
                        ui.separator();

                        ui.spinner();
                        ui.label("Please wait...");
                    }
                    UpdateState::Completed => {
                        ui.heading(RichText::new("Update Complete!").color(Color32::GREEN));
                        ui.separator();

                        ui.label("Please restart the application.");

                        if ui.button("Restart Now").clicked() {
                            action = UpdateUIAction::Restart;
                        }
                    }
                    UpdateState::Failed { error } => {
                        ui.heading(RichText::new("Update Failed").color(Color32::RED));
                        ui.separator();

                        ui.label(error);

                        if ui.button("Close").clicked() {
                            self.show_window = false;
                        }
                    }
                    _ => {}
                }
            });

        action
    }
}

/// UI action
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateUIAction {
    None,
    StartUpdate,
    Restart,
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Version manifest system with delta patch support
- [ ] Delta patching with xdelta3 integration
- [ ] Download manager with resume capability
- [ ] Update engine (delta and full updates)
- [ ] Hash verification for all downloads
- [ ] Customizable update UI with egui
- [ ] Rollback on failed updates
- [ ] <10% CPU usage during downloads
- [ ] Bandwidth throttling option
- [ ] Cross-platform support (Windows, Linux, macOS)

---

## 🧪 **Tests**

```rust
#[tokio::test]
async fn test_manifest_fetch() {
    let manifest = VersionManifest::fetch("https://example.com/manifest.json")
        .await
        .unwrap();
    assert!(!manifest.version.is_empty());
}

#[test]
fn test_delta_patch_application() {
    let patcher = DeltaPatcher::new();
    // Create test files and apply patch
    // Verify output matches expected
}

#[tokio::test]
async fn test_download_resume() {
    let manager = DownloadManager::new();
    let dest = PathBuf::from("/tmp/test_download");

    // Download partial
    // Interrupt
    // Resume and verify complete
}

#[tokio::test]
async fn test_full_update_flow() {
    let config = UpdateConfig {
        manifest_url: "https://example.com/manifest.json".to_string(),
        update_url_base: "https://example.com/updates".to_string(),
        install_dir: PathBuf::from("/tmp/install"),
        temp_dir: PathBuf::from("/tmp/update_temp"),
        current_version: "1.0.0".to_string(),
    };

    let mut updater = Updater::new(config);
    let manifest = updater.check_for_updates().await.unwrap();

    if let Some(manifest) = manifest {
        updater.apply_update(&manifest, None).await.unwrap();
        assert_eq!(updater.state(), &UpdateState::Completed);
    }
}
```

---

## ⚡ **Performance Targets**

- **Download Speed:** Limited only by network bandwidth
- **CPU Usage:** <10% during downloads, <50% during patching
- **Memory Usage:** <100 MB during operation
- **Patch Application:** <30 seconds for typical game update (100 MB)
- **Resume Overhead:** <1 second to resume download

---

## 📚 **Dependencies**

```toml
[dependencies]
reqwest = { version = "0.11", features = ["stream"] }
tokio = { version = "1.0", features = ["fs", "io-util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
futures-util = "0.3"
egui = "0.28"
tracing = "0.1"
```

**External:** xdelta3 binary (bundled or system)

---

**Dependencies:** [phase3-scripting.md](phase3-scripting.md)
**Next:** [phase4-pbr-materials.md](phase4-pbr-materials.md)
