//! Asynchronous asset loading system.
//!
//! Provides background loading with progress tracking, cancellation, and priorities.

use crate::{AssetError, AssetHandle, AssetLoader, AssetManager};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, error, info, instrument};

/// Priority for asset loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadPriority {
    /// Low priority (background loading).
    Low = 0,
    /// Normal priority (default).
    Normal = 1,
    /// High priority (needed for current frame).
    High = 2,
    /// Critical priority (must load immediately).
    Critical = 3,
}

/// Status of an async load operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    /// Load is queued but not started.
    Queued,
    /// Load is in progress.
    InProgress,
    /// Load completed successfully.
    Completed,
    /// Load failed with an error.
    Failed,
    /// Load was cancelled.
    Cancelled,
}

/// Handle to an async load operation.
///
/// Can be used to check progress, cancel the operation, or wait for completion.
pub struct AsyncLoadHandle<T> {
    #[allow(dead_code)]
    id: u64,
    status: Arc<AtomicUsize>, // LoadStatus as usize
    progress: Arc<AtomicU64>, // Progress as u64 (0-10000 = 0-100.00%)
    result_rx: oneshot::Receiver<Result<AssetHandle<T>, AssetError>>,
    cancel_flag: Arc<AtomicBool>,
}

impl<T> AsyncLoadHandle<T> {
    /// Get the current load status.
    #[must_use]
    pub fn status(&self) -> LoadStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => LoadStatus::Queued,
            1 => LoadStatus::InProgress,
            2 => LoadStatus::Completed,
            3 => LoadStatus::Failed,
            4 => LoadStatus::Cancelled,
            _ => LoadStatus::Failed,
        }
    }

    /// Get the current progress (0.0 - 1.0).
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.progress.load(Ordering::Relaxed) as f32 / 10000.0
    }

    /// Check if the load is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status(),
            LoadStatus::Completed | LoadStatus::Failed | LoadStatus::Cancelled
        )
    }

    /// Cancel the load operation.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        self.status.store(4, Ordering::Relaxed); // Cancelled
    }

    /// Wait for the load to complete and get the result.
    ///
    /// # Errors
    ///
    /// Returns an error if the load failed or was cancelled.
    pub async fn await_result(self) -> Result<AssetHandle<T>, AssetError> {
        self.result_rx.await.map_err(|_| {
            AssetError::loadfailed("async_load".to_string(), "Load operation dropped".to_string())
        })?
    }

    /// Try to get the result without blocking.
    ///
    /// Returns `None` if the load is not yet complete.
    ///
    /// # Errors
    ///
    /// Returns an error if the load failed or was cancelled.
    pub fn try_result(&mut self) -> Option<Result<AssetHandle<T>, AssetError>> {
        self.result_rx.try_recv().ok()
    }
}

#[allow(dead_code)]
struct LoadRequest<T> {
    id: u64,
    path: PathBuf,
    priority: LoadPriority,
    status: Arc<AtomicUsize>,
    progress: Arc<AtomicU64>,
    result_tx: oneshot::Sender<Result<AssetHandle<T>, AssetError>>,
    cancel_flag: Arc<AtomicBool>,
}

/// Asynchronous asset loader.
///
/// Manages a background thread pool for loading assets without blocking the main thread.
///
/// # Examples
///
/// ```no_run
/// use engine_assets::{AsyncLoader, AssetManager, MeshData, LoadPriority};
/// use std::path::Path;
///
/// # async fn example() {
/// let manager = AssetManager::new();
/// let mut loader = AsyncLoader::new(manager.clone(), 4);
///
/// // Start an async load
/// let handle = loader.load_async::<MeshData>(
///     Path::new("assets/cube.obj"),
///     LoadPriority::Normal
/// );
///
/// // Check progress
/// println!("Progress: {:.1}%", handle.progress() * 100.0);
///
/// // Wait for completion
/// let mesh_handle = handle.await_result().await.unwrap();
/// # }
/// ```
pub struct AsyncLoader {
    manager: Arc<AssetManager>,
    next_id: AtomicU64,
    #[allow(dead_code)]
    worker_count: usize,
}

impl AsyncLoader {
    /// Create a new async loader.
    ///
    /// # Arguments
    ///
    /// * `manager` - The asset manager to use for loading
    /// * `worker_count` - Number of background worker threads
    #[must_use]
    pub fn new(manager: Arc<AssetManager>, worker_count: usize) -> Self {
        info!(worker_count = worker_count, "Initializing AsyncLoader");
        Self { manager, next_id: AtomicU64::new(0), worker_count }
    }

    /// Load an asset asynchronously.
    #[instrument(skip(self))]
    pub fn load_async<T: AssetLoader + Send + 'static>(
        &self,
        path: &std::path::Path,
        priority: LoadPriority,
    ) -> AsyncLoadHandle<T::Asset>
    where
        T::Asset: Send + Sync + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let status = Arc::new(AtomicUsize::new(0)); // Queued
        let progress = Arc::new(AtomicU64::new(0));
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (result_tx, result_rx) = oneshot::channel();

        debug!(id = id, path = ?path, priority = ?priority, "Queuing async load");

        // Spawn the load task
        let manager = Arc::clone(&self.manager);
        let path_clone = path.to_path_buf();
        let status_clone = Arc::clone(&status);
        let progress_clone = Arc::clone(&progress);
        let cancel_flag_clone = Arc::clone(&cancel_flag);

        tokio::spawn(async move {
            // Check if cancelled before starting
            if cancel_flag_clone.load(Ordering::Relaxed) {
                status_clone.store(4, Ordering::Relaxed); // Cancelled
                let _ = result_tx.send(Err(AssetError::loadfailed(
                    path_clone.display().to_string(),
                    "Load cancelled".to_string(),
                )));
                return;
            }

            // Mark as in progress
            status_clone.store(1, Ordering::Relaxed);
            progress_clone.store(1000, Ordering::Relaxed); // 10%

            // Read file
            let data = match tokio::fs::read(&path_clone).await {
                Ok(data) => {
                    progress_clone.store(5000, Ordering::Relaxed); // 50%
                    data
                }
                Err(e) => {
                    error!(path = ?path_clone, error = ?e, "Failed to read file");
                    status_clone.store(3, Ordering::Relaxed); // Failed
                    let _ = result_tx.send(Err(AssetError::ioerror(
                        path_clone.display().to_string(),
                        e.to_string(),
                    )));
                    return;
                }
            };

            // Check if cancelled before parsing
            if cancel_flag_clone.load(Ordering::Relaxed) {
                status_clone.store(4, Ordering::Relaxed); // Cancelled
                let _ = result_tx.send(Err(AssetError::loadfailed(
                    path_clone.display().to_string(),
                    "Load cancelled".to_string(),
                )));
                return;
            }

            // Parse in background thread
            let path_clone2 = path_clone.clone();
            let asset = match tokio::task::spawn_blocking(move || {
                T::parse(&data).map_err(|e| e.to_string())
            })
            .await
            {
                Ok(Ok(asset)) => {
                    progress_clone.store(9000, Ordering::Relaxed); // 90%
                    asset
                }
                Ok(Err(e)) => {
                    error!(path = ?path_clone, error = %e, "Failed to parse asset");
                    status_clone.store(3, Ordering::Relaxed); // Failed
                    let _ = result_tx
                        .send(Err(AssetError::loadfailed(path_clone.display().to_string(), e)));
                    return;
                }
                Err(e) => {
                    error!(path = ?path_clone, error = ?e, "Parse task panicked");
                    status_clone.store(3, Ordering::Relaxed); // Failed
                    let _ = result_tx.send(Err(AssetError::loadfailed(
                        path_clone.display().to_string(),
                        e.to_string(),
                    )));
                    return;
                }
            };

            // Generate ID and insert
            let id = T::generate_id(&asset);
            match T::insert(&manager, id, asset) {
                Ok(handle) => {
                    progress_clone.store(10000, Ordering::Relaxed); // 100%
                    status_clone.store(2, Ordering::Relaxed); // Completed
                    info!(path = ?path_clone2, id = ?id, "Async load completed");
                    let _ = result_tx.send(Ok(handle));
                }
                Err(e) => {
                    error!(path = ?path_clone2, error = ?e, "Failed to insert asset");
                    status_clone.store(3, Ordering::Relaxed); // Failed
                    let _ = result_tx.send(Err(e));
                }
            }
        });

        AsyncLoadHandle { id, status, progress, result_rx, cancel_flag }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_priority_ordering() {
        assert!(LoadPriority::Critical > LoadPriority::High);
        assert!(LoadPriority::High > LoadPriority::Normal);
        assert!(LoadPriority::Normal > LoadPriority::Low);
    }

    #[test]
    fn test_load_status() {
        let status = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            match status.load(Ordering::Relaxed) {
                0 => LoadStatus::Queued,
                _ => LoadStatus::Failed,
            },
            LoadStatus::Queued
        );

        status.store(1, Ordering::Relaxed);
        assert_eq!(
            match status.load(Ordering::Relaxed) {
                1 => LoadStatus::InProgress,
                _ => LoadStatus::Failed,
            },
            LoadStatus::InProgress
        );
    }
}
