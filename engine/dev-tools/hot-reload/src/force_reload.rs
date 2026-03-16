//! Bridge between the async `DevReloadServer` TCP task and the synchronous
//! `HotReloader` owned by the game loop thread.
//!
//! The game loop calls `HotReloader::process_events()` each frame. When
//! `silm dev` sends a `reload_asset` message, `ForceReloader::reload` calls
//! `HotReloader::force_reload` which queues the path for the next frame.

use crate::error::DevError;
use engine_assets::hot_reload::HotReloader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Drives `HotReloader` from outside the game loop (e.g. from an async TCP handler).
///
/// Thread-safe: wraps `HotReloader` in `Arc<Mutex<_>>`.
///
/// # Examples
///
/// ```no_run
/// use engine_dev_tools_hot_reload::force_reload::ForceReloader;
/// use engine_assets::{AssetManager, hot_reload::{HotReloader, HotReloadConfig}};
/// use std::sync::{Arc, Mutex};
///
/// let manager = Arc::new(AssetManager::new());
/// let hot_reloader = Arc::new(Mutex::new(
///     HotReloader::new(manager, HotReloadConfig::default()).unwrap(),
/// ));
/// let force = ForceReloader::new(hot_reloader);
/// let _ = force.reload("assets/textures/diffuse.png");
/// ```
#[derive(Clone)]
pub struct ForceReloader {
    inner: Arc<Mutex<HotReloader>>,
}

impl ForceReloader {
    /// Create a new `ForceReloader` wrapping an existing `HotReloader`.
    pub fn new(hot_reloader: Arc<Mutex<HotReloader>>) -> Self {
        Self {
            inner: hot_reloader,
        }
    }

    /// Queue an immediate reload of the asset at `path_str` (project-relative string).
    ///
    /// Returns `DevError::ReloadFailed` if:
    /// - the path is not registered in `HotReloader`, or
    /// - the internal mutex has been poisoned.
    ///
    /// # Errors
    ///
    /// Returns [`DevError::ReloadFailed`] on failure.
    pub fn reload(&self, path_str: &str) -> Result<(), DevError> {
        let path = Path::new(path_str);
        let guard = self.inner.lock().map_err(|e| {
            warn!(
                path = path_str,
                reason = %e,
                "ForceReloader: mutex poisoned"
            );
            DevError::ReloadFailed {
                path: path_str.to_string(),
                reason: format!("mutex poisoned: {e}"),
            }
        })?;
        guard.force_reload(path).map_err(|e| {
            warn!(
                path = path_str,
                error = ?e,
                "ForceReloader: force_reload failed"
            );
            DevError::ReloadFailed {
                path: path_str.to_string(),
                reason: format!("{e:?}"),
            }
        })?;
        Ok(())
    }
}
