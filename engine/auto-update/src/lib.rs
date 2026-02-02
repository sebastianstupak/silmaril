//! Engine Auto-Update System
//!
//! Production-grade automatic update system with:
//! - Version management with semantic versioning
//! - Differential patching using bsdiff
//! - SHA-256 verification and Ed25519 signatures
//! - Progress tracking with ETA
//! - Resume support for interrupted downloads
//! - Rollback system for failed updates
//! - Multiple release channels (stable, beta, dev)
//! - Bandwidth throttling
//! - Background downloads
//!
//! # Examples
//!
//! ## Basic Update Check
//!
//! ```no_run
//! use engine_auto_update::{UpdateManager, UpdateConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = UpdateConfig::default();
//! let mut manager = UpdateManager::new(config)?;
//!
//! if let Some(update) = manager.check_for_updates().await? {
//!     println!("Update available: {}", update.version);
//!     manager.download_update(&update).await?;
//!     manager.install_update().await?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## With Progress Tracking
//!
//! ```no_run
//! use engine_auto_update::{UpdateManager, UpdateConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = UpdateConfig::default();
//! let mut manager = UpdateManager::new(config)?;
//!
//! if let Some(update) = manager.check_for_updates().await? {
//!     let progress = manager.get_progress_tracker();
//!
//!     // Start download in background
//!     let handle = tokio::spawn(async move {
//!         manager.download_update(&update).await
//!     });
//!
//!     // Monitor progress
//!     while !handle.is_finished() {
//!         let p = progress.get_progress();
//!         println!("Progress: {:.1}% - {} - ETA: {}",
//!             p.percentage(),
//!             p.speed_string(),
//!             p.eta_string()
//!         );
//!         tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod channels;
pub mod downloader;
pub mod error;
pub mod manifest;
pub mod patcher;
pub mod progress;
pub mod rollback;
pub mod verifier;
pub mod version;

use channels::{Channel, ChannelSubscription};
use downloader::{DownloadConfig, Downloader};
use error::UpdateError;
use manifest::{ManifestIndex, UpdateManifest};
use patcher::{apply_patch, create_patch};
use progress::{MultiFileProgressTracker, ProgressTracker};
use rollback::{RollbackManager, VersionHistory};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tracing::{debug, error, info, warn};
use version::Version;

// Re-export commonly used types
pub use channels::Channel as UpdateChannel;
pub use error::UpdateError as Error;
pub use manifest::UpdateManifest;
pub use progress::Progress;
pub use version::Version as UpdateVersion;

/// Configuration for the update system.
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Base URL for update manifest
    pub manifest_url: String,
    /// Current installed version
    pub current_version: Version,
    /// Installation root directory
    pub install_dir: PathBuf,
    /// Directory for storing backups
    pub backup_dir: PathBuf,
    /// Directory for downloading updates
    pub download_dir: PathBuf,
    /// Update channel subscription
    pub channel: ChannelSubscription,
    /// Download configuration
    pub download_config: DownloadConfig,
    /// Public key for signature verification (hex encoded)
    pub public_key: Option<String>,
    /// Whether to check for updates automatically
    pub auto_check: bool,
    /// Interval between automatic update checks
    pub check_interval: Duration,
}

impl UpdateConfig {
    /// Create a new update configuration.
    pub fn new(manifest_url: String, current_version: Version, install_dir: PathBuf) -> Self {
        let backup_dir = install_dir.join(".backups");
        let download_dir = install_dir.join(".downloads");

        Self {
            manifest_url,
            current_version,
            install_dir,
            backup_dir,
            download_dir,
            channel: ChannelSubscription::default(),
            download_config: DownloadConfig::default(),
            public_key: None,
            auto_check: true,
            check_interval: Duration::from_secs(4 * 60 * 60), // 4 hours
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self::new(
            "https://updates.example.com/manifest.json".to_string(),
            Version::new(0, 1, 0),
            PathBuf::from("."),
        )
    }
}

/// Main update manager.
pub struct UpdateManager {
    config: UpdateConfig,
    downloader: Downloader,
    rollback_manager: RollbackManager,
    version_history: VersionHistory,
    progress_tracker: Arc<ProgressTracker>,
    pending_update: Option<UpdateManifest>,
}

impl UpdateManager {
    /// Create a new update manager.
    pub fn new(config: UpdateConfig) -> Result<Self, UpdateError> {
        // Create necessary directories
        std::fs::create_dir_all(&config.backup_dir).map_err(|e| {
            UpdateError::ioerror(config.backup_dir.display().to_string(), e.to_string())
        })?;

        std::fs::create_dir_all(&config.download_dir).map_err(|e| {
            UpdateError::ioerror(config.download_dir.display().to_string(), e.to_string())
        })?;

        let downloader = Downloader::with_config(config.download_config.clone())?;
        let rollback_manager = RollbackManager::new(&config.backup_dir, &config.install_dir);
        let version_history = VersionHistory::new(config.current_version);
        let progress_tracker = Arc::new(ProgressTracker::new(0));

        Ok(Self {
            config,
            downloader,
            rollback_manager,
            version_history,
            progress_tracker,
            pending_update: None,
        })
    }

    /// Check for available updates.
    pub async fn check_for_updates(&mut self) -> Result<Option<UpdateManifest>, UpdateError> {
        info!(
            current_version = %self.config.current_version,
            channel = %self.config.channel.channel,
            "Checking for updates"
        );

        // Download manifest index
        let response = reqwest::get(&self.config.manifest_url)
            .await
            .map_err(|e| UpdateError::checkfailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::checkfailed(format!("HTTP {}", response.status())));
        }

        let index: ManifestIndex =
            response.json().await.map_err(|e| UpdateError::invalidmanifest(e.to_string()))?;

        // Get latest version for our channel
        let latest_version = index.get_latest_version(self.config.channel.channel.as_str())?;

        if !latest_version.is_newer_than(&self.config.current_version) {
            info!("No update available");
            return Ok(None);
        }

        info!(
            current = %self.config.current_version,
            latest = %latest_version,
            "Update available"
        );

        // Download full manifest for the latest version
        let manifest_url =
            index.get_manifest_url(self.config.channel.channel.as_str(), latest_version);

        let response = reqwest::get(&manifest_url)
            .await
            .map_err(|e| UpdateError::checkfailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::checkfailed(format!(
                "HTTP {} when fetching manifest",
                response.status()
            )));
        }

        let manifest: UpdateManifest =
            response.json().await.map_err(|e| UpdateError::invalidmanifest(e.to_string()))?;

        manifest.validate()?;

        // Verify compatibility
        if !manifest.is_compatible_with(&self.config.current_version) {
            return Err(UpdateError::checkfailed(format!(
                "Update requires minimum version {:?}",
                manifest.min_version
            )));
        }

        // Verify signature if public key is provided
        if let Some(ref public_key) = self.config.public_key {
            if let Some(ref signature) = manifest.signature {
                let manifest_json = serde_json::to_string(&manifest)
                    .map_err(|e| UpdateError::invalidmanifest(e.to_string()))?;
                verifier::verify_signature(manifest_json.as_bytes(), signature, public_key)?;
            } else {
                warn!("Public key configured but manifest has no signature");
            }
        }

        self.pending_update = Some(manifest.clone());
        Ok(Some(manifest))
    }

    /// Download an update.
    pub async fn download_update(&mut self, manifest: &UpdateManifest) -> Result<(), UpdateError> {
        info!(
            version = %manifest.version,
            file_count = manifest.files.len(),
            total_size = manifest.total_download_size(Some(&self.config.current_version)),
            "Downloading update"
        );

        let download_size = manifest.total_download_size(Some(&self.config.current_version));
        self.progress_tracker.set_total_bytes(download_size);
        self.progress_tracker.reset();

        // Download each file
        for file_info in &manifest.files {
            let dest_path = self.config.download_dir.join(&file_info.path);

            // Try to use patch if available and applicable
            if let Some(patch_info) = &file_info.patch {
                if patch_info.from_version == self.config.current_version {
                    debug!(
                        file = %file_info.path,
                        patch_size = patch_info.size,
                        "Downloading patch"
                    );

                    let patch_path =
                        self.config.download_dir.join(format!("{}.patch", file_info.path));

                    self.downloader
                        .download_file(
                            &patch_info.url,
                            &patch_path,
                            Some(&patch_info.sha256),
                            Some(self.progress_tracker.clone()),
                        )
                        .await?;

                    // Apply patch
                    let old_file = self.config.install_dir.join(&file_info.path);
                    if old_file.exists() {
                        apply_patch(&old_file, &patch_path, &dest_path)?;
                        // Verify patched file
                        verifier::verify_file_hash(&dest_path, &file_info.sha256)?;
                        continue;
                    }
                }
            }

            // Download full file
            debug!(
                file = %file_info.path,
                size = file_info.size,
                "Downloading file"
            );

            self.downloader
                .download_file_resumable(
                    &file_info.url,
                    &dest_path,
                    Some(&file_info.sha256),
                    Some(self.progress_tracker.clone()),
                )
                .await?;
        }

        info!(
            version = %manifest.version,
            "Update downloaded successfully"
        );

        self.pending_update = Some(manifest.clone());
        Ok(())
    }

    /// Install a downloaded update.
    pub async fn install_update(&mut self) -> Result<(), UpdateError> {
        let manifest = self
            .pending_update
            .as_ref()
            .ok_or_else(|| UpdateError::installfailed("No pending update".to_string()))?;

        info!(
            version = %manifest.version,
            "Installing update"
        );

        // Create backup before installing
        let files_to_backup: Vec<String> = manifest.files.iter().map(|f| f.path.clone()).collect();

        let backup = self
            .rollback_manager
            .create_backup(&self.config.current_version, &files_to_backup)?;

        // Install files
        for file_info in &manifest.files {
            let source = self.config.download_dir.join(&file_info.path);
            let dest = self.config.install_dir.join(&file_info.path);

            if !source.exists() {
                error!(file = %file_info.path, "Downloaded file not found");
                // Rollback
                warn!("Installation failed, rolling back");
                self.rollback_manager.restore_backup(&backup)?;
                return Err(UpdateError::installfailed(format!(
                    "File not found: {}",
                    file_info.path
                )));
            }

            // Create parent directories
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    UpdateError::ioerror(parent.display().to_string(), e.to_string())
                })?;
            }

            // Copy file
            std::fs::copy(&source, &dest).map_err(|e| {
                // Rollback on failure
                warn!("File copy failed, rolling back");
                let _ = self.rollback_manager.restore_backup(&backup);
                UpdateError::installfailed(format!("Failed to copy {}: {}", file_info.path, e))
            })?;

            // Verify installed file
            if let Err(e) = verifier::verify_file_hash(&dest, &file_info.sha256) {
                error!(file = %file_info.path, "Verification failed after installation");
                // Rollback
                warn!("Verification failed, rolling back");
                self.rollback_manager.restore_backup(&backup)?;
                return Err(e);
            }
        }

        // Update version history
        self.version_history.add_backup(backup);
        self.version_history.current_version = manifest.version;

        // Cleanup old backups
        self.rollback_manager.cleanup_old_backups(2)?;

        // Clear pending update
        self.pending_update = None;

        info!(
            version = %manifest.version,
            "Update installed successfully"
        );

        Ok(())
    }

    /// Rollback to the previous version.
    pub fn rollback(&mut self) -> Result<(), UpdateError> {
        let backup = self
            .version_history
            .get_latest_backup()
            .ok_or_else(|| UpdateError::rollbackfailed("No backup available".to_string()))?;

        info!(version = %backup.version, "Rolling back to previous version");

        self.rollback_manager.restore_backup(backup)?;
        self.version_history.current_version = backup.version;

        Ok(())
    }

    /// Get the progress tracker for monitoring downloads.
    pub fn get_progress_tracker(&self) -> Arc<ProgressTracker> {
        self.progress_tracker.clone()
    }

    /// Get the current version.
    pub fn current_version(&self) -> &Version {
        &self.version_history.current_version
    }

    /// Get the version history.
    pub fn version_history(&self) -> &VersionHistory {
        &self.version_history
    }

    /// Switch to a different update channel.
    pub fn switch_channel(&mut self, channel: Channel) -> Result<(), UpdateError> {
        self.config.channel.switch_to(channel)?;
        info!(channel = %channel, "Switched to new update channel");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_config_default() {
        let config = UpdateConfig::default();
        assert_eq!(config.current_version, Version::new(0, 1, 0));
        assert_eq!(config.channel.channel, Channel::Stable);
    }

    #[test]
    fn test_update_manager_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UpdateConfig::new(
            "https://example.com/manifest.json".to_string(),
            Version::new(1, 0, 0),
            temp_dir.path().to_path_buf(),
        );

        let manager = UpdateManager::new(config);
        assert!(manager.is_ok());
    }
}
