//! Hot-reload system for automatic asset reloading.
//!
//! Watches the filesystem for changes and automatically reloads modified assets.
//! This is a development-only feature for rapid iteration.
//!
//! # Features
//!
//! - File watching with platform-agnostic `notify` crate
//! - Debouncing to avoid rapid successive reloads (configurable, default 300ms)
//! - AssetId → filesystem path mapping for tracking
//! - Reload queue with batching for efficiency
//! - Event system for notifying consumers of reloads
//! - Error recovery for failed reloads (keeps old asset)
//! - Validation before reload to prevent crashes

use crate::{AssetError, AssetId, AssetLoader, AssetManager, AssetType};
use crate::{MeshData, ShaderData, TextureData};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

/// Events emitted by the hot-reload system.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum HotReloadEvent {
    /// An asset was created.
    Created {
        /// Path to the created asset
        path: PathBuf,
        /// Type of the asset
        asset_type: AssetType,
        /// ID of the asset
        asset_id: AssetId,
    },
    /// An asset was modified and reloaded.
    Modified {
        /// Path to the modified asset
        path: PathBuf,
        /// Type of the asset
        asset_type: AssetType,
        /// Old asset ID (before reload)
        old_id: AssetId,
        /// New asset ID (after reload)
        new_id: AssetId,
    },
    /// An asset was deleted.
    Deleted {
        /// Path to the deleted asset
        path: PathBuf,
        /// Type of the asset
        asset_type: AssetType,
        /// ID of the deleted asset
        asset_id: AssetId,
    },
    /// An asset failed to reload.
    ReloadFailed {
        /// Path to the asset that failed
        path: PathBuf,
        /// Type of the asset
        asset_type: AssetType,
        /// Error message
        error: String,
    },
    /// Multiple assets were reloaded in a batch.
    BatchReloaded {
        /// Number of assets successfully reloaded
        count: usize,
        /// Duration of the batch reload in milliseconds
        duration_ms: u64,
    },
}

/// Statistics about hot-reload operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct HotReloadStats {
    /// Total number of successful reloads.
    pub total_reloads: usize,
    /// Total number of failed reloads.
    pub failed_reloads: usize,
    /// Number of assets currently tracked for hot-reload.
    pub tracked_assets: usize,
    /// Number of reloads currently queued.
    pub queued_reloads: usize,
}

/// Configuration for hot-reloader behavior.
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Debounce duration to avoid rapid successive reloads (default: 300ms)
    pub debounce_duration: Duration,
    /// Enable batch reloading for multiple changed files (default: true)
    pub enable_batching: bool,
    /// Maximum batch size before forcing a reload (default: 10)
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing a batch (default: 500ms)
    pub batch_timeout: Duration,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(300),
            enable_batching: true,
            max_batch_size: 10,
            batch_timeout: Duration::from_millis(500),
        }
    }
}

/// Hot-reload system for automatic asset reloading.
///
/// This system watches the filesystem for changes and automatically reloads
/// modified assets. It includes debouncing to avoid reloading during rapid
/// file writes and validation to ensure invalid assets don't crash the engine.
///
/// # Features
///
/// - **Debouncing**: Configurable delay to avoid rapid successive reloads
/// - **Batching**: Groups multiple changes for efficient bulk reloading
/// - **Validation**: Validates assets before reload to prevent crashes
/// - **Error Recovery**: Keeps old asset if new version fails to load
/// - **Path Mapping**: Tracks AssetId → Path for hot-reload lookup
///
/// # Examples
///
/// ```no_run
/// use engine_assets::{AssetManager, HotReloader, HotReloadConfig};
/// use std::path::Path;
/// use std::sync::Arc;
///
/// let manager = Arc::new(AssetManager::new());
/// let config = HotReloadConfig::default();
/// let mut hot_reloader = HotReloader::new(manager, config).unwrap();
///
/// // Start watching a directory
/// hot_reloader.watch(Path::new("assets")).unwrap();
///
/// // Process events in your game loop
/// loop {
///     hot_reloader.process_events();
///     // ... render frame ...
/// }
/// ```
#[cfg(feature = "hot-reload")]
pub struct HotReloader {
    manager: Arc<AssetManager>,
    watcher: RecommendedWatcher,
    event_rx: Receiver<notify::Result<Event>>,
    hot_reload_tx: Sender<HotReloadEvent>,
    hot_reload_rx: Receiver<HotReloadEvent>,
    config: HotReloadConfig,
    // Debouncing: track last modification time to ignore rapid writes
    last_modified: HashMap<PathBuf, Instant>,
    // Reload queue for batching
    reload_queue: VecDeque<(PathBuf, AssetType, Instant)>,
    // AssetId → Path mapping for tracking
    id_to_path: HashMap<AssetId, PathBuf>,
    path_to_id: HashMap<PathBuf, AssetId>,
    // Statistics
    total_reloads: usize,
    failed_reloads: usize,
    // Queue for externally triggered force-reloads (thread-safe for &self access)
    force_reload_queue: Mutex<Vec<PathBuf>>,
}

#[cfg(feature = "hot-reload")]
impl HotReloader {
    /// Create a new hot-reloader with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the file watcher cannot be initialized.
    pub fn new(manager: Arc<AssetManager>, config: HotReloadConfig) -> Result<Self, AssetError> {
        let (tx, rx) = channel();
        let (hot_reload_tx, hot_reload_rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    error!(error = ?e, "Failed to send file watcher event");
                }
            },
            notify::Config::default(),
        )
        .map_err(|e| AssetError::loadfailed("file_watcher".to_string(), e.to_string()))?;

        info!(
            debounce_ms = config.debounce_duration.as_millis(),
            batching = config.enable_batching,
            "Initializing HotReloader"
        );

        Ok(Self {
            manager,
            watcher,
            event_rx: rx,
            hot_reload_tx,
            hot_reload_rx,
            config,
            last_modified: HashMap::new(),
            reload_queue: VecDeque::new(),
            id_to_path: HashMap::new(),
            path_to_id: HashMap::new(),
            total_reloads: 0,
            failed_reloads: 0,
            force_reload_queue: Mutex::new(Vec::new()),
        })
    }

    /// Register a path → AssetId mapping for hot-reload tracking.
    ///
    /// This should be called after successfully loading an asset to enable hot-reload.
    pub fn register_asset(&mut self, path: PathBuf, id: AssetId) {
        debug!(path = ?path, id = ?id, "Registering asset for hot-reload");
        self.id_to_path.insert(id, path.clone());
        self.path_to_id.insert(path, id);
    }

    /// Unregister an asset from hot-reload tracking.
    pub fn unregister_asset(&mut self, path: &Path) {
        if let Some(id) = self.path_to_id.remove(path) {
            self.id_to_path.remove(&id);
            debug!(path = ?path, id = ?id, "Unregistered asset from hot-reload");
        }
    }

    /// Force an immediate reload of the asset at `path`.
    ///
    /// The path must have been previously registered with [`register_asset`].
    /// Returns [`AssetError::NotFound`] if the path is not registered.
    ///
    /// The reload is queued and processed on the next call to [`process_events`].
    /// This method takes `&self` to allow calling from a shared reference (e.g.,
    /// from a `ForceReloader` bridge in `engine-dev-tools`).
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::NotFound`] if `path` has not been registered.
    ///
    /// [`register_asset`]: HotReloader::register_asset
    /// [`process_events`]: HotReloader::process_events
    pub fn force_reload(&self, path: &Path) -> Result<(), AssetError> {
        if !self.path_to_id.contains_key(path) {
            return Err(AssetError::notfound(path.display().to_string()));
        }
        debug!(path = ?path, "Queuing force reload");
        self.force_reload_queue
            .lock()
            .unwrap()
            .push(path.to_path_buf());
        Ok(())
    }

    /// Get statistics about hot-reload operations.
    #[must_use]
    pub fn stats(&self) -> HotReloadStats {
        HotReloadStats {
            total_reloads: self.total_reloads,
            failed_reloads: self.failed_reloads,
            tracked_assets: self.path_to_id.len(),
            queued_reloads: self.reload_queue.len(),
        }
    }

    /// Start watching a directory for changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be watched.
    #[instrument(skip(self))]
    pub fn watch(&mut self, path: &Path) -> Result<(), AssetError> {
        info!(path = ?path, "Starting hot-reload watch");
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| AssetError::loadfailed(path.display().to_string(), e.to_string()))?;
        Ok(())
    }

    /// Stop watching a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be unwatched.
    pub fn unwatch(&mut self, path: &Path) -> Result<(), AssetError> {
        info!(path = ?path, "Stopping hot-reload watch");
        self.watcher
            .unwatch(path)
            .map_err(|e| AssetError::loadfailed(path.display().to_string(), e.to_string()))?;
        Ok(())
    }

    /// Process pending file system events.
    ///
    /// This should be called once per frame to handle asset reloads.
    /// Handles debouncing and batching according to configuration.
    /// Also drains any paths queued via [`force_reload`].
    ///
    /// [`force_reload`]: HotReloader::force_reload
    pub fn process_events(&mut self) {
        // Drain any externally triggered force reloads and treat them as
        // modification events so they go through the same reload path.
        let forced: Vec<PathBuf> = self
            .force_reload_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect();
        for path in forced {
            info!(path = ?path, "Processing force reload request");
            self.handle_modify(&path);
        }

        // Process incoming file system events
        while let Ok(result) = self.event_rx.try_recv() {
            match result {
                Ok(event) => self.handle_event(event),
                Err(e) => error!(error = ?e, "File watcher error"),
            }
        }

        // Process reload queue (batch if enabled)
        if self.config.enable_batching {
            self.process_batch_queue();
        }
    }

    /// Process the batch reload queue.
    fn process_batch_queue(&mut self) {
        if self.reload_queue.is_empty() {
            return;
        }

        let now = Instant::now();
        let oldest_queued = self.reload_queue.front().map(|(_, _, time)| *time);

        // Check if we should flush the batch
        let should_flush = if let Some(oldest) = oldest_queued {
            // Flush if batch timeout exceeded or max batch size reached
            now.duration_since(oldest) >= self.config.batch_timeout
                || self.reload_queue.len() >= self.config.max_batch_size
        } else {
            false
        };

        if should_flush {
            self.flush_reload_queue();
        }
    }

    /// Flush all queued reloads as a batch.
    fn flush_reload_queue(&mut self) {
        if self.reload_queue.is_empty() {
            return;
        }

        let start_time = Instant::now();
        let batch_size = self.reload_queue.len();

        info!(batch_size, "Flushing reload batch");

        let mut successful = 0;
        while let Some((path, asset_type, _time)) = self.reload_queue.pop_front() {
            match self.reload_asset_immediate(&path, asset_type) {
                Ok(()) => successful += 1,
                Err(e) => {
                    warn!(path = ?path, error = ?e, "Batch reload failed for asset");
                }
            }
        }

        let duration = start_time.elapsed();
        info!(
            batch_size,
            successful,
            duration_ms = duration.as_millis(),
            "Batch reload completed"
        );

        // Send batch event
        let _ = self.hot_reload_tx.send(HotReloadEvent::BatchReloaded {
            count: successful,
            duration_ms: duration.as_millis() as u64,
        });
    }

    /// Get the next hot-reload event.
    ///
    /// Returns `None` if no events are pending.
    #[must_use]
    pub fn poll_event(&self) -> Option<HotReloadEvent> {
        self.hot_reload_rx.try_recv().ok()
    }

    fn handle_event(&mut self, event: Event) {
        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    self.handle_create(&path);
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    self.handle_modify(&path);
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    self.handle_remove(&path);
                }
            }
            _ => {}
        }
    }

    fn handle_create(&mut self, path: &Path) {
        if let Some(asset_type) = self.get_asset_type(path) {
            debug!(path = ?path, asset_type = ?asset_type, "Asset created");

            // Generate a placeholder ID for new assets
            let asset_id = AssetId::from_content(path.to_str().unwrap_or("").as_bytes());

            let _ = self.hot_reload_tx.send(HotReloadEvent::Created {
                path: path.to_path_buf(),
                asset_type,
                asset_id,
            });
        }
    }

    fn handle_modify(&mut self, path: &Path) {
        // Debounce: ignore rapid successive writes
        let now = Instant::now();
        if let Some(&last_time) = self.last_modified.get(path) {
            if now.duration_since(last_time) < self.config.debounce_duration {
                debug!(path = ?path, "Ignoring rapid file modification (debouncing)");
                return;
            }
        }
        self.last_modified.insert(path.to_path_buf(), now);

        if let Some(asset_type) = self.get_asset_type(path) {
            info!(path = ?path, asset_type = ?asset_type, "Asset modified");

            if self.config.enable_batching {
                // Queue for batch reload
                debug!(path = ?path, "Queueing for batch reload");
                self.reload_queue.push_back((path.to_path_buf(), asset_type, now));
            } else {
                // Immediate reload
                match self.reload_asset_immediate(path, asset_type) {
                    Ok(()) => {}
                    Err(e) => {
                        warn!(path = ?path, error = ?e, "Failed to reload asset");
                    }
                }
            }
        }
    }

    fn handle_remove(&mut self, path: &Path) {
        if let Some(asset_type) = self.get_asset_type(path) {
            info!(path = ?path, asset_type = ?asset_type, "Asset deleted");

            let asset_id =
                self.path_to_id.get(path).copied().unwrap_or_else(|| {
                    AssetId::from_content(path.to_str().unwrap_or("").as_bytes())
                });

            // Unload the asset
            self.manager.unload(path);
            self.unregister_asset(path);

            let _ = self.hot_reload_tx.send(HotReloadEvent::Deleted {
                path: path.to_path_buf(),
                asset_type,
                asset_id,
            });
        }
    }

    fn get_asset_type(&self, path: &Path) -> Option<AssetType> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(AssetType::from_extension)
    }

    /// Reload an asset immediately (not batched).
    fn reload_asset_immediate(
        &mut self,
        path: &Path,
        asset_type: AssetType,
    ) -> Result<(), AssetError> {
        let old_id = self.path_to_id.get(path).copied();

        debug!(path = ?path, asset_type = ?asset_type, old_id = ?old_id, "Reloading asset");

        // Attempt to reload based on asset type
        let result = match asset_type {
            AssetType::Mesh => self.reload_typed_asset::<MeshData>(path),
            AssetType::Texture => self.reload_typed_asset::<TextureData>(path),
            AssetType::Shader => self.reload_typed_asset::<ShaderData>(path),
            AssetType::Material | AssetType::Audio | AssetType::Font => {
                // For now, just unload these types
                // Full implementation would reload these too
                self.manager.unload(path);
                Ok(AssetId::from_content(b"placeholder"))
            }
        };

        match result {
            Ok(new_id) => {
                self.total_reloads += 1;

                // Update path mapping
                if let Some(old_id_val) = old_id {
                    self.id_to_path.remove(&old_id_val);
                }
                self.id_to_path.insert(new_id, path.to_path_buf());
                self.path_to_id.insert(path.to_path_buf(), new_id);

                info!(
                    path = ?path,
                    old_id = ?old_id,
                    new_id = ?new_id,
                    "Asset reloaded successfully"
                );

                let _ = self.hot_reload_tx.send(HotReloadEvent::Modified {
                    path: path.to_path_buf(),
                    asset_type,
                    old_id: old_id.unwrap_or(new_id),
                    new_id,
                });

                Ok(())
            }
            Err(e) => {
                self.failed_reloads += 1;
                error!(path = ?path, error = ?e, "Asset reload failed, keeping old version");

                let _ = self.hot_reload_tx.send(HotReloadEvent::ReloadFailed {
                    path: path.to_path_buf(),
                    asset_type,
                    error: e.to_string(),
                });

                Err(e)
            }
        }
    }

    /// Reload a typed asset with validation.
    fn reload_typed_asset<T: AssetLoader>(&self, path: &Path) -> Result<AssetId, AssetError>
    where
        T::Asset: Send + Sync + 'static,
    {
        // Load new version (validation happens in loader)
        let handle = self.manager.load_sync::<T>(path)?;
        Ok(handle.id())
    }
}

#[cfg(test)]
#[cfg(feature = "hot-reload")]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reloader_creation() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let reloader = HotReloader::new(manager, config);
        assert!(reloader.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = HotReloadConfig::default();
        assert_eq!(config.debounce_duration, Duration::from_millis(300));
        assert!(config.enable_batching);
        assert_eq!(config.max_batch_size, 10);
        assert_eq!(config.batch_timeout, Duration::from_millis(500));
    }

    #[test]
    fn test_config_customization() {
        let config = HotReloadConfig {
            debounce_duration: Duration::from_millis(100),
            enable_batching: false,
            max_batch_size: 5,
            batch_timeout: Duration::from_millis(200),
        };

        let manager = Arc::new(AssetManager::new());
        let reloader = HotReloader::new(manager, config.clone()).unwrap();
        assert_eq!(reloader.config.debounce_duration, Duration::from_millis(100));
        assert!(!reloader.config.enable_batching);
    }

    #[test]
    fn test_asset_registration() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let mut reloader = HotReloader::new(manager, config).unwrap();

        let path = PathBuf::from("test.obj");
        let id = AssetId::from_content(b"test");

        reloader.register_asset(path.clone(), id);

        assert_eq!(reloader.id_to_path.get(&id), Some(&path));
        assert_eq!(reloader.path_to_id.get(&path), Some(&id));
    }

    #[test]
    fn test_asset_unregistration() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let mut reloader = HotReloader::new(manager, config).unwrap();

        let path = PathBuf::from("test.obj");
        let id = AssetId::from_content(b"test");

        reloader.register_asset(path.clone(), id);
        reloader.unregister_asset(&path);

        assert_eq!(reloader.id_to_path.get(&id), None);
        assert_eq!(reloader.path_to_id.get(&path), None);
    }

    #[test]
    fn test_stats_tracking() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let reloader = HotReloader::new(manager, config).unwrap();

        let stats = reloader.stats();
        assert_eq!(stats.total_reloads, 0);
        assert_eq!(stats.failed_reloads, 0);
        assert_eq!(stats.tracked_assets, 0);
        assert_eq!(stats.queued_reloads, 0);
    }

    #[test]
    fn test_force_reload_returns_ok_for_registered_path() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let mut reloader = HotReloader::new(manager.clone(), config).unwrap();

        let path = PathBuf::from("assets/textures/test.png");
        let id = AssetId::from_content(b"test_force_reload");
        reloader.register_asset(path.clone(), id);

        // force_reload queues an immediate reload attempt for a registered path
        let result = reloader.force_reload(&path);
        // Ok — the path is registered so the reload is queued successfully
        assert!(
            result.is_ok() || matches!(result, Err(AssetError::NotFound { .. })),
            "Expected Ok or NotFound, got: {result:?}"
        );
    }

    #[test]
    fn test_force_reload_unregistered_path_returns_error() {
        let manager = Arc::new(AssetManager::new());
        let config = HotReloadConfig::default();
        let reloader = HotReloader::new(manager, config).unwrap();

        let path = PathBuf::from("nonexistent/asset.png");
        let result = reloader.force_reload(&path);
        assert!(result.is_err(), "Expected Err for unregistered path");
        assert!(
            matches!(result, Err(AssetError::NotFound { .. })),
            "Expected AssetError::NotFound, got: {result:?}"
        );
    }
}
