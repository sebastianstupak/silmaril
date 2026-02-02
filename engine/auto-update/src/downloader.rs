//! Download manager with resume support and progress tracking.

use crate::error::UpdateError;
use crate::progress::ProgressTracker;
use crate::verifier::verify_file_hash;
use futures_util::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// Download configuration.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Maximum download speed in bytes per second (0 = unlimited)
    pub max_speed: u64,
    /// Request timeout
    pub timeout: Duration,
    /// Number of retry attempts
    pub max_retries: u32,
    /// Chunk size for downloads
    pub chunk_size: usize,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self { max_speed: 0, timeout: Duration::from_secs(30), max_retries: 3, chunk_size: 8192 }
    }
}

/// Download manager for fetching update files.
pub struct Downloader {
    client: Client,
    config: DownloadConfig,
}

impl Downloader {
    /// Create a new downloader with default configuration.
    pub fn new() -> Result<Self, UpdateError> {
        Self::with_config(DownloadConfig::default())
    }

    /// Create a new downloader with custom configuration.
    pub fn with_config(config: DownloadConfig) -> Result<Self, UpdateError> {
        let client = Client::builder().timeout(config.timeout).build().map_err(|e| {
            UpdateError::networkerror(format!("Failed to create HTTP client: {}", e))
        })?;

        Ok(Self { client, config })
    }

    /// Download a file from a URL to a local path.
    pub async fn download_file<P: AsRef<Path>>(
        &self,
        url: &str,
        destination: P,
        expected_hash: Option<&str>,
        progress: Option<Arc<ProgressTracker>>,
    ) -> Result<(), UpdateError> {
        let destination = destination.as_ref();

        for attempt in 1..=self.config.max_retries {
            match self
                .download_file_internal(url, destination, expected_hash, progress.clone())
                .await
            {
                Ok(()) => {
                    info!(
                        url = %url,
                        destination = %destination.display(),
                        "Download completed successfully"
                    );
                    return Ok(());
                }
                Err(e) if attempt < self.config.max_retries => {
                    warn!(
                        url = %url,
                        attempt = attempt,
                        max_retries = self.config.max_retries,
                        error = %e,
                        "Download attempt failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!("Retry loop should always return")
    }

    async fn download_file_internal<P: AsRef<Path>>(
        &self,
        url: &str,
        destination: P,
        expected_hash: Option<&str>,
        progress: Option<Arc<ProgressTracker>>,
    ) -> Result<(), UpdateError> {
        let destination = destination.as_ref();

        debug!(
            url = %url,
            destination = %destination.display(),
            "Starting download"
        );

        // Check if file already exists and is complete
        if let Some(hash) = expected_hash {
            if destination.exists() {
                if let Ok(()) = verify_file_hash(destination, hash) {
                    info!(
                        destination = %destination.display(),
                        "File already exists and verified, skipping download"
                    );
                    if let Some(tracker) = &progress {
                        tracker.add_bytes(tracker.get_progress().total_bytes);
                    }
                    return Ok(());
                }
            }
        }

        // Start the download
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| UpdateError::downloadfailed(url.to_string(), e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::downloadfailed(
                url.to_string(),
                format!("HTTP {}", response.status()),
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        if let Some(tracker) = &progress {
            tracker.set_total_bytes(total_size);
        }

        // Create parent directories if needed
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| UpdateError::ioerror(parent.display().to_string(), e.to_string()))?;
        }

        // Open file for writing
        let mut file = File::create(destination)
            .await
            .map_err(|e| UpdateError::ioerror(destination.display().to_string(), e.to_string()))?;

        // Download with progress tracking
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| UpdateError::downloadfailed(url.to_string(), e.to_string()))?;

            file.write_all(&chunk).await.map_err(|e| {
                UpdateError::ioerror(destination.display().to_string(), e.to_string())
            })?;

            downloaded += chunk.len() as u64;
            if let Some(tracker) = &progress {
                tracker.add_bytes(chunk.len() as u64);
            }

            // Apply bandwidth throttling if configured
            if self.config.max_speed > 0 {
                let elapsed = tracker.as_ref().map(|t| t.get_progress().speed).unwrap_or(0.0);
                if elapsed > self.config.max_speed as f64 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }

        file.flush()
            .await
            .map_err(|e| UpdateError::ioerror(destination.display().to_string(), e.to_string()))?;

        // Verify hash if provided
        if let Some(hash) = expected_hash {
            verify_file_hash(destination, hash)?;
        }

        debug!(
            url = %url,
            destination = %destination.display(),
            bytes = downloaded,
            "Download completed"
        );

        Ok(())
    }

    /// Download a file with resume support.
    pub async fn download_file_resumable<P: AsRef<Path>>(
        &self,
        url: &str,
        destination: P,
        expected_hash: Option<&str>,
        progress: Option<Arc<ProgressTracker>>,
    ) -> Result<(), UpdateError> {
        let destination = destination.as_ref();

        // Check existing file size
        let start_byte = if destination.exists() {
            tokio::fs::metadata(destination).await.map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        if start_byte > 0 {
            debug!(
                url = %url,
                destination = %destination.display(),
                resume_from = start_byte,
                "Resuming download"
            );
        }

        let response = self
            .client
            .get(url)
            .header("Range", format!("bytes={}-", start_byte))
            .send()
            .await
            .map_err(|e| UpdateError::downloadfailed(url.to_string(), e.to_string()))?;

        // Check if server supports resume
        let supports_resume = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;

        if !response.status().is_success() && !supports_resume {
            return Err(UpdateError::downloadfailed(
                url.to_string(),
                format!("HTTP {}", response.status()),
            ));
        }

        if !supports_resume && start_byte > 0 {
            warn!(
                url = %url,
                "Server does not support resume, restarting download"
            );
            return self.download_file(url, destination, expected_hash, progress).await;
        }

        // Create parent directories if needed
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| UpdateError::ioerror(parent.display().to_string(), e.to_string()))?;
        }

        // Open file for appending
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(destination)
            .await
            .map_err(|e| UpdateError::ioerror(destination.display().to_string(), e.to_string()))?;

        // Download remaining data
        let mut stream = response.bytes_stream();
        let mut downloaded = start_byte;

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| UpdateError::downloadfailed(url.to_string(), e.to_string()))?;

            file.write_all(&chunk).await.map_err(|e| {
                UpdateError::ioerror(destination.display().to_string(), e.to_string())
            })?;

            downloaded += chunk.len() as u64;
            if let Some(tracker) = &progress {
                tracker.add_bytes(chunk.len() as u64);
            }
        }

        file.flush()
            .await
            .map_err(|e| UpdateError::ioerror(destination.display().to_string(), e.to_string()))?;

        // Verify hash if provided
        if let Some(hash) = expected_hash {
            verify_file_hash(destination, hash)?;
        }

        info!(
            url = %url,
            destination = %destination.display(),
            total_bytes = downloaded,
            "Download completed (with resume)"
        );

        Ok(())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new().expect("Failed to create default downloader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_downloader_creation() {
        let downloader = Downloader::new();
        assert!(downloader.is_ok());
    }

    #[tokio::test]
    async fn test_download_config_default() {
        let config = DownloadConfig::default();
        assert_eq!(config.max_speed, 0);
        assert_eq!(config.max_retries, 3);
    }

    // Additional tests would require a mock HTTP server
    // which is implemented in integration tests
}
