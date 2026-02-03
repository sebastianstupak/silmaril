//! Asset loading strategies: sync, async, and streaming.
//!
//! This module provides three loading strategies for different use cases:
//! - **Sync**: Blocking load for small assets (< 1MB)
//! - **Async**: Non-blocking load for large assets (> 1MB)
//! - **Streaming**: Progressive LOD loading for maximum responsiveness

use crate::{AssetError, AssetHandle, AssetLoader, AssetManager};
use std::path::Path;
#[cfg(feature = "async")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

#[cfg(feature = "async")]
use tokio::sync::RwLock as AsyncRwLock;

/// Loader strategy for different asset loading modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStrategy {
    /// Synchronous blocking load (use for small assets < 1MB).
    Sync,
    /// Asynchronous non-blocking load (use for large assets > 1MB).
    Async,
    /// Progressive streaming load with LOD (use for very large assets).
    Streaming,
}

/// Handle to a streaming asset load with progressive LOD levels.
///
/// Provides immediate access to low-resolution LOD 0 while higher LODs
/// stream in the background.
#[cfg(feature = "async")]
pub struct StreamingHandle<T> {
    /// Available LOD levels (0 = lowest res, higher = better quality).
    lod_levels: Arc<AsyncRwLock<Vec<Option<AssetHandle<T>>>>>,
    /// Current highest available LOD.
    current_lod: Arc<AtomicUsize>,
    /// Total number of LOD levels expected.
    total_lods: usize,
}

#[cfg(feature = "async")]
impl<T> StreamingHandle<T> {
    /// Create a new streaming handle.
    fn new(total_lods: usize) -> Self {
        let mut lods = Vec::with_capacity(total_lods);
        for _ in 0..total_lods {
            lods.push(None);
        }

        Self {
            lod_levels: Arc::new(AsyncRwLock::new(lods)),
            current_lod: Arc::new(AtomicUsize::new(0)),
            total_lods,
        }
    }

    /// Get the current highest available LOD level.
    #[must_use]
    pub fn current_lod(&self) -> usize {
        self.current_lod.load(Ordering::Acquire)
    }

    /// Get the total number of LOD levels.
    #[must_use]
    pub fn total_lods(&self) -> usize {
        self.total_lods
    }

    /// Check if all LOD levels have been loaded.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.current_lod() + 1 >= self.total_lods
    }

    /// Get the handle for a specific LOD level.
    ///
    /// Returns `None` if the LOD is not yet loaded.
    pub async fn get_lod(&self, level: usize) -> Option<AssetHandle<T>>
    where
        T: Clone,
    {
        if level >= self.total_lods {
            return None;
        }

        let lods = self.lod_levels.read().await;
        lods[level].clone()
    }

    /// Get the best available LOD level (highest loaded so far).
    pub async fn get_best(&self) -> Option<AssetHandle<T>>
    where
        T: Clone,
    {
        let current = self.current_lod();
        self.get_lod(current).await
    }

    /// Set a LOD level (internal use only).
    pub(crate) async fn set_lod(&self, level: usize, handle: AssetHandle<T>) {
        if level >= self.total_lods {
            warn!(level, total_lods = self.total_lods, "LOD level out of range");
            return;
        }

        {
            let mut lods = self.lod_levels.write().await;
            lods[level] = Some(handle);
        }

        // Update current LOD if this is higher
        loop {
            let current = self.current_lod.load(Ordering::Acquire);
            if level <= current {
                break;
            }
            if self
                .current_lod
                .compare_exchange(current, level, Ordering::Release, Ordering::Acquire)
                .is_ok()
            {
                debug!(level, "Updated current LOD");
                break;
            }
        }
    }
}

/// Enhanced asset loader with multiple strategies.
pub struct EnhancedLoader {
    manager: Arc<AssetManager>,
}

impl EnhancedLoader {
    /// Create a new enhanced loader.
    #[must_use]
    pub fn new(manager: Arc<AssetManager>) -> Self {
        info!("Initializing EnhancedLoader");
        Self { manager }
    }

    /// Load an asset synchronously (blocking).
    ///
    /// Use for small assets (< 1MB) that need to be available immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[instrument(skip(self))]
    pub fn load_sync<T: AssetLoader>(
        &self,
        path: &Path,
    ) -> Result<AssetHandle<T::Asset>, AssetError>
    where
        T::Asset: Send + Sync + 'static,
    {
        debug!(path = ?path, "Loading asset synchronously");
        self.manager.load_sync::<T>(path)
    }

    /// Load an asset asynchronously (non-blocking).
    ///
    /// Use for large assets (> 1MB) that can load in the background.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[cfg(feature = "async")]
    #[instrument(skip(self))]
    pub async fn load_async<T: AssetLoader>(
        &self,
        path: &Path,
    ) -> Result<AssetHandle<T::Asset>, AssetError>
    where
        T::Asset: Send + Sync + 'static,
    {
        debug!(path = ?path, "Loading asset asynchronously");
        self.manager.load_async::<T>(path).await
    }

    /// Load an asset with progressive streaming (LOD-based).
    ///
    /// Returns immediately with LOD 0 (lowest res), then progressively
    /// streams higher quality LODs in the background.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial LOD cannot be loaded.
    #[cfg(feature = "async")]
    #[instrument(skip(self))]
    pub async fn load_streaming<T: AssetLoader>(
        &self,
        path: &Path,
        lod_count: usize,
    ) -> Result<StreamingHandle<T::Asset>, AssetError>
    where
        T::Asset: Send + Sync + Clone + 'static,
    {
        debug!(path = ?path, lod_count, "Loading asset with streaming");

        if lod_count == 0 {
            return Err(AssetError::loadfailed(
                path.display().to_string(),
                "LOD count must be at least 1".to_string(),
            ));
        }

        let handle = StreamingHandle::new(lod_count);

        // Load LOD 0 immediately (lowest resolution)
        // For now, we just load the base asset as LOD 0
        // In a real implementation, we'd load a downsampled version
        let lod0_handle = self.load_async::<T>(path).await?;
        handle.set_lod(0, lod0_handle).await;

        // Spawn background tasks to load higher LODs
        if lod_count > 1 {
            let manager = Arc::clone(&self.manager);
            let path_clone = path.to_path_buf();
            let handle_clone = StreamingHandle {
                lod_levels: Arc::clone(&handle.lod_levels),
                current_lod: Arc::clone(&handle.current_lod),
                total_lods: handle.total_lods,
            };

            tokio::spawn(async move {
                for lod in 1..lod_count {
                    debug!(lod, path = ?path_clone, "Loading higher LOD");

                    // In a real implementation, we'd load a higher-res version
                    // For now, just reuse the same asset
                    match manager.load_async::<T>(&path_clone).await {
                        Ok(lod_handle) => {
                            handle_clone.set_lod(lod, lod_handle).await;
                            info!(lod, path = ?path_clone, "Higher LOD loaded");
                        }
                        Err(e) => {
                            warn!(lod, path = ?path_clone, error = ?e, "Failed to load higher LOD");
                            break;
                        }
                    }
                }
            });
        }

        info!(path = ?path, lod_count, "Streaming load initiated");
        Ok(handle)
    }

    /// Get a reference to the underlying asset manager.
    #[must_use]
    pub fn manager(&self) -> &Arc<AssetManager> {
        &self.manager
    }
}

impl Default for EnhancedLoader {
    fn default() -> Self {
        Self::new(Arc::new(AssetManager::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MeshData;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_obj() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".obj").unwrap();
        writeln!(file, "# Test OBJ file").unwrap();
        writeln!(file, "v 0.0 0.0 0.0").unwrap();
        writeln!(file, "v 1.0 0.0 0.0").unwrap();
        writeln!(file, "v 0.0 1.0 0.0").unwrap();
        writeln!(file, "vn 0.0 0.0 1.0").unwrap();
        writeln!(file, "vt 0.0 0.0").unwrap();
        writeln!(file, "vt 1.0 0.0").unwrap();
        writeln!(file, "vt 0.0 1.0").unwrap();
        writeln!(file, "f 1/1/1 2/2/1 3/3/1").unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_loader_creation() {
        let loader = EnhancedLoader::default();
        assert_eq!(loader.manager().len(), 0);
    }

    #[test]
    fn test_sync_load() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let result = loader.load_sync::<MeshData>(test_file.path());
        assert!(result.is_ok());

        let handle = result.unwrap();
        let mesh = loader.manager().get_mesh(handle.id()).unwrap();
        assert!(!mesh.vertices.is_empty());
    }

    #[test]
    fn test_sync_load_missing_file() {
        let loader = EnhancedLoader::default();
        let result = loader.load_sync::<MeshData>(Path::new("nonexistent.obj"));
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_load() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let result = loader.load_async::<MeshData>(test_file.path()).await;
        assert!(result.is_ok());

        let handle = result.unwrap();
        let mesh = loader.manager().get_mesh(handle.id()).unwrap();
        assert!(!mesh.vertices.is_empty());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_load_missing_file() {
        let loader = EnhancedLoader::default();
        let result = loader.load_async::<MeshData>(Path::new("nonexistent.obj")).await;
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_streaming_load() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let result = loader.load_streaming::<MeshData>(test_file.path(), 3).await;
        assert!(result.is_ok());

        let handle = result.unwrap();
        assert_eq!(handle.total_lods(), 3);
        assert_eq!(handle.current_lod(), 0);

        // LOD 0 should be immediately available
        let lod0 = handle.get_lod(0).await;
        assert!(lod0.is_some());

        // Wait for higher LODs to load
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Higher LODs should eventually be available
        let best = handle.get_best().await;
        assert!(best.is_some());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_streaming_lod_progression() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let handle = loader.load_streaming::<MeshData>(test_file.path(), 2).await.unwrap();

        // Initially only LOD 0
        assert_eq!(handle.current_lod(), 0);
        assert!(!handle.is_complete());

        // Wait for all LODs
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Should have loaded higher LODs
        assert!(handle.current_lod() >= 1);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_streaming_zero_lods() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let result = loader.load_streaming::<MeshData>(test_file.path(), 0).await;
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_streaming_handle_get_lod_out_of_range() {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj();

        let handle = loader.load_streaming::<MeshData>(test_file.path(), 2).await.unwrap();

        let lod = handle.get_lod(10).await;
        assert!(lod.is_none());
    }
}
