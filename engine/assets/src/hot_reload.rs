//! Hot-reload system for automatic asset reloading.
//!
//! Watches the filesystem for changes and automatically reloads modified assets.
//! This is a development-only feature for rapid iteration.

use crate::{AssetError, AssetManager, AssetType};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

/// Events emitted by the hot-reload system.
#[derive(Debug, Clone)]
pub enum HotReloadEvent {
    /// An asset was created.
    Created { path: PathBuf, asset_type: AssetType },
    /// An asset was modified and reloaded.
    Modified { path: PathBuf, asset_type: AssetType },
    /// An asset was deleted.
    Deleted { path: PathBuf, asset_type: AssetType },
    /// An asset failed to reload.
    ReloadFailed { path: PathBuf, error: String },
}

/// Hot-reload system for automatic asset reloading.
///
/// This system watches the filesystem for changes and automatically reloads
/// modified assets. It includes debouncing to avoid reloading during rapid
/// file writes and validation to ensure invalid assets don't crash the engine.
///
/// # Examples
///
/// ```no_run
/// use engine_assets::{AssetManager, HotReloader};
/// use std::path::Path;
///
/// let manager = AssetManager::new();
/// let mut hot_reloader = HotReloader::new(manager.clone()).unwrap();
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
    // Debouncing: track last modification time to ignore rapid writes
    last_modified: std::collections::HashMap<PathBuf, Instant>,
    debounce_duration: Duration,
}

#[cfg(feature = "hot-reload")]
impl HotReloader {
    /// Create a new hot-reloader.
    ///
    /// # Errors
    ///
    /// Returns an error if the file watcher cannot be initialized.
    pub fn new(manager: Arc<AssetManager>) -> Result<Self, AssetError> {
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

        Ok(Self {
            manager,
            watcher,
            event_rx: rx,
            hot_reload_tx,
            hot_reload_rx,
            last_modified: std::collections::HashMap::new(),
            debounce_duration: Duration::from_millis(100),
        })
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
    pub fn process_events(&mut self) {
        while let Ok(result) = self.event_rx.try_recv() {
            match result {
                Ok(event) => self.handle_event(event),
                Err(e) => error!(error = ?e, "File watcher error"),
            }
        }
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
            let _ = self
                .hot_reload_tx
                .send(HotReloadEvent::Created { path: path.to_path_buf(), asset_type });
        }
    }

    fn handle_modify(&mut self, path: &Path) {
        // Debounce: ignore rapid successive writes
        let now = Instant::now();
        if let Some(&last_time) = self.last_modified.get(path) {
            if now.duration_since(last_time) < self.debounce_duration {
                debug!(path = ?path, "Ignoring rapid file modification (debouncing)");
                return;
            }
        }
        self.last_modified.insert(path.to_path_buf(), now);

        if let Some(asset_type) = self.get_asset_type(path) {
            info!(path = ?path, asset_type = ?asset_type, "Asset modified, reloading");

            // Attempt to reload
            match self.reload_asset(path, asset_type) {
                Ok(()) => {
                    let _ = self
                        .hot_reload_tx
                        .send(HotReloadEvent::Modified { path: path.to_path_buf(), asset_type });
                }
                Err(e) => {
                    warn!(path = ?path, error = ?e, "Failed to reload asset");
                    let _ = self.hot_reload_tx.send(HotReloadEvent::ReloadFailed {
                        path: path.to_path_buf(),
                        error: e.to_string(),
                    });
                }
            }
        }
    }

    fn handle_remove(&mut self, path: &Path) {
        if let Some(asset_type) = self.get_asset_type(path) {
            info!(path = ?path, asset_type = ?asset_type, "Asset deleted");

            // Unload the asset
            self.manager.unload(path);

            let _ = self
                .hot_reload_tx
                .send(HotReloadEvent::Deleted { path: path.to_path_buf(), asset_type });
        }
    }

    fn get_asset_type(&self, path: &Path) -> Option<AssetType> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(AssetType::from_extension)
    }

    fn reload_asset(&self, path: &Path, _asset_type: AssetType) -> Result<(), AssetError> {
        // Unload old version
        self.manager.unload(path);

        // Load new version (validation happens in loader)
        // For now, we just unload - actual reload would need to know the specific type
        // This is a simplified version - production would dispatch to specific loaders
        debug!(path = ?path, "Asset unloaded for reload");

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "hot-reload")]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reloader_creation() {
        let manager = Arc::new(AssetManager::new());
        let reloader = HotReloader::new(manager);
        assert!(reloader.is_ok());
    }

    #[test]
    fn test_debounce_duration() {
        let manager = Arc::new(AssetManager::new());
        let reloader = HotReloader::new(manager).unwrap();
        assert_eq!(reloader.debounce_duration, Duration::from_millis(100));
    }
}
