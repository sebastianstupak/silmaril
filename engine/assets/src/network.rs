//! Network asset transfer protocol for client-server asset distribution.
//!
//! This module provides efficient asset transfer over TCP with:
//! - Chunked transfer for large assets (1MB chunks)
//! - Resumable downloads with range requests
//! - Blake3 checksum validation
//! - LZ4 compression for compressible assets (shaders, fonts)
//! - Priority-based transfer queuing
//! - Deduplication based on AssetId

use crate::{AssetError, AssetId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use tracing::{debug, info, instrument, warn};

/// Default chunk size for large asset transfers (1MB).
const DEFAULT_CHUNK_SIZE: u64 = 1024 * 1024;

/// Maximum asset size for single transfer (100MB).
const MAX_ASSET_SIZE: u64 = 100 * 1024 * 1024;

/// Network message types for asset transfer protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetNetworkMessage {
    /// Client requests an asset by ID.
    Request {
        /// Asset ID to request.
        asset_id: AssetId,
        /// Optional: starting offset for resumable downloads (bytes).
        resume_offset: Option<u64>,
    },

    /// Server responds with complete asset data (for small assets < 1MB).
    Response {
        /// Asset ID.
        asset_id: AssetId,
        /// Compressed asset data.
        data: Vec<u8>,
        /// Blake3 checksum (32 bytes).
        checksum: [u8; 32],
        /// Whether data is LZ4 compressed.
        compressed: bool,
    },

    /// Server sends a chunk of a large asset.
    Chunk {
        /// Asset ID.
        asset_id: AssetId,
        /// Byte offset in the asset.
        offset: u64,
        /// Total asset size in bytes.
        total_size: u64,
        /// Chunk data (compressed if applicable).
        data: Vec<u8>,
        /// Whether this chunk is LZ4 compressed.
        compressed: bool,
    },

    /// Server indicates transfer is complete.
    Complete {
        /// Asset ID.
        asset_id: AssetId,
        /// Blake3 checksum of complete asset.
        checksum: [u8; 32],
    },

    /// Server reports error loading asset.
    Error {
        /// Asset ID.
        asset_id: AssetId,
        /// Error message.
        error: String,
    },
}

/// Priority level for asset transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferPriority {
    /// Critical assets (player model, UI).
    Critical = 3,
    /// High priority (nearby NPCs, weapons).
    High = 2,
    /// Normal priority (background objects).
    Normal = 1,
    /// Low priority (distant terrain, decorations).
    Low = 0,
}

/// Status of an asset transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
    /// Queued for transfer.
    Queued,
    /// Currently transferring.
    InProgress {
        /// Bytes received so far.
        bytes_received: u64,
        /// Total bytes to receive.
        total_bytes: u64,
    },
    /// Transfer completed successfully.
    Completed,
    /// Transfer failed.
    Failed {
        /// Error message.
        error: String,
    },
}

/// Asset transfer request with priority.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TransferRequest {
    asset_id: AssetId,
    priority: TransferPriority,
    resume_offset: u64,
}

/// Client-side asset fetcher.
///
/// Manages asset requests, receives data from server, validates checksums,
/// and caches received assets.
pub struct AssetNetworkClient {
    /// Pending transfer requests (priority-ordered).
    pending_requests: BTreeMap<TransferPriority, VecDeque<AssetId>>,
    /// Active transfers.
    active_transfers: HashMap<AssetId, TransferStatus>,
    /// Received asset chunks (for resumable downloads).
    /// Public for testing and benchmarking.
    #[doc(hidden)]
    pub chunk_buffers: HashMap<AssetId, Vec<u8>>,
    /// Completed assets waiting for validation.
    completed_assets: HashMap<AssetId, Vec<u8>>,
    /// Maximum concurrent transfers.
    max_concurrent: usize,
}

impl AssetNetworkClient {
    /// Create a new asset network client.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::network::AssetNetworkClient;
    ///
    /// let client = AssetNetworkClient::new(4);
    /// ```
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        info!(max_concurrent, "Initializing asset network client");
        Self {
            pending_requests: BTreeMap::new(),
            active_transfers: HashMap::new(),
            chunk_buffers: HashMap::new(),
            completed_assets: HashMap::new(),
            max_concurrent,
        }
    }

    /// Request an asset with the given priority.
    ///
    /// If the asset is already queued or transferring, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::{AssetId, network::{AssetNetworkClient, TransferPriority}};
    ///
    /// let mut client = AssetNetworkClient::new(4);
    /// let id = AssetId::from_content(b"test");
    /// client.request_asset(id, TransferPriority::Critical);
    /// ```
    pub fn request_asset(&mut self, asset_id: AssetId, priority: TransferPriority) {
        // Skip if already active or queued
        if self.active_transfers.contains_key(&asset_id) {
            debug!(asset_id = %asset_id, "Asset already transferring");
            return;
        }

        if self.pending_requests.values().any(|queue| queue.contains(&asset_id)) {
            debug!(asset_id = %asset_id, "Asset already queued");
            return;
        }

        debug!(asset_id = %asset_id, priority = ?priority, "Queueing asset request");
        self.pending_requests.entry(priority).or_default().push_back(asset_id);
    }

    /// Get the next request to send to the server.
    ///
    /// Returns `None` if no requests are pending or max concurrent transfers reached.
    #[must_use]
    pub fn next_request(&mut self) -> Option<AssetNetworkMessage> {
        if self.active_transfers.len() >= self.max_concurrent {
            return None;
        }

        // Get highest priority request
        for (_priority, queue) in self.pending_requests.iter_mut().rev() {
            if let Some(asset_id) = queue.pop_front() {
                // Check for resumable download
                let resume_offset =
                    self.chunk_buffers.get(&asset_id).map(|buffer| buffer.len() as u64);

                self.active_transfers.insert(asset_id, TransferStatus::Queued);
                info!(asset_id = %asset_id, resume_offset, "Starting asset transfer");

                return Some(AssetNetworkMessage::Request { asset_id, resume_offset });
            }
        }

        None
    }

    /// Handle a message received from the server.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Checksum validation fails
    /// - Data is corrupted
    /// - Decompression fails
    #[instrument(skip(self, message))]
    pub fn handle_message(&mut self, message: AssetNetworkMessage) -> Result<(), AssetError> {
        match message {
            AssetNetworkMessage::Response { asset_id, data, checksum, compressed } => {
                debug!(
                    asset_id = %asset_id,
                    size = data.len(),
                    compressed,
                    "Received complete asset response"
                );

                let decompressed = if compressed {
                    decompress_lz4(&data).map_err(|e| {
                        AssetError::loadfailed(
                            asset_id.to_string(),
                            format!("Decompression failed: {}", e),
                        )
                    })?
                } else {
                    data
                };

                self.validate_and_store(asset_id, decompressed, checksum)?;
            }

            AssetNetworkMessage::Chunk { asset_id, offset, total_size, data, compressed } => {
                debug!(
                    asset_id = %asset_id,
                    offset,
                    total_size,
                    chunk_size = data.len(),
                    compressed,
                    "Received asset chunk"
                );

                let decompressed = if compressed {
                    decompress_lz4(&data).map_err(|e| {
                        AssetError::loadfailed(
                            asset_id.to_string(),
                            format!("Chunk decompression failed: {}", e),
                        )
                    })?
                } else {
                    data
                };

                self.handle_chunk(asset_id, offset, total_size, decompressed)?;
            }

            AssetNetworkMessage::Complete { asset_id, checksum } => {
                debug!(asset_id = %asset_id, "Received transfer complete");

                if let Some(buffer) = self.chunk_buffers.remove(&asset_id) {
                    self.validate_and_store(asset_id, buffer, checksum)?;
                } else {
                    warn!(asset_id = %asset_id, "Complete message for non-chunked transfer");
                }
            }

            AssetNetworkMessage::Error { asset_id, error } => {
                warn!(asset_id = %asset_id, error = %error, "Asset transfer error");
                self.active_transfers
                    .insert(asset_id, TransferStatus::Failed { error: error.clone() });
                self.chunk_buffers.remove(&asset_id);

                return Err(AssetError::loadfailed(asset_id.to_string(), error));
            }

            AssetNetworkMessage::Request { .. } => {
                warn!("Client received Request message (should only be sent by client)");
            }
        }

        Ok(())
    }

    fn handle_chunk(
        &mut self,
        asset_id: AssetId,
        offset: u64,
        total_size: u64,
        data: Vec<u8>,
    ) -> Result<(), AssetError> {
        let buffer = self.chunk_buffers.entry(asset_id).or_default();

        // Verify offset matches buffer size (sequential chunks)
        if offset != buffer.len() as u64 {
            return Err(AssetError::loadfailed(
                asset_id.to_string(),
                format!("Chunk offset mismatch: expected {}, got {}", buffer.len(), offset),
            ));
        }

        buffer.extend_from_slice(&data);

        // Update status
        self.active_transfers.insert(
            asset_id,
            TransferStatus::InProgress {
                bytes_received: buffer.len() as u64,
                total_bytes: total_size,
            },
        );

        debug!(
            asset_id = %asset_id,
            progress = format!("{}/{}", buffer.len(), total_size),
            "Chunk received"
        );

        Ok(())
    }

    fn validate_and_store(
        &mut self,
        asset_id: AssetId,
        data: Vec<u8>,
        expected_checksum: [u8; 32],
    ) -> Result<(), AssetError> {
        // Validate checksum
        let actual_checksum = *blake3::hash(&data).as_bytes();
        if actual_checksum != expected_checksum {
            return Err(AssetError::loadfailed(
                asset_id.to_string(),
                format!(
                    "Checksum mismatch: expected {:?}, got {:?}",
                    expected_checksum, actual_checksum
                ),
            ));
        }

        info!(
            asset_id = %asset_id,
            size = data.len(),
            "Asset validated and stored"
        );

        self.completed_assets.insert(asset_id, data);
        self.active_transfers.insert(asset_id, TransferStatus::Completed);
        self.chunk_buffers.remove(&asset_id);

        Ok(())
    }

    /// Take a completed asset from the client.
    ///
    /// Returns `None` if the asset is not yet completed.
    #[must_use]
    pub fn take_completed(&mut self, asset_id: AssetId) -> Option<Vec<u8>> {
        self.completed_assets.remove(&asset_id)
    }

    /// Get the status of an asset transfer.
    #[must_use]
    pub fn status(&self, asset_id: &AssetId) -> Option<&TransferStatus> {
        self.active_transfers.get(asset_id)
    }

    /// Get the number of active transfers.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_transfers.len()
    }

    /// Get the number of pending requests.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_requests.values().map(|q| q.len()).sum()
    }
}

/// Server-side asset serving.
///
/// Responds to client requests, compresses assets, and streams large assets in chunks.
pub struct AssetNetworkServer {
    /// Asset provider (owned data).
    assets: HashMap<AssetId, Vec<u8>>,
    /// Chunk size for large transfers.
    chunk_size: u64,
}

impl AssetNetworkServer {
    /// Create a new asset network server.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::network::AssetNetworkServer;
    ///
    /// let server = AssetNetworkServer::new(1024 * 1024);
    /// ```
    #[must_use]
    pub fn new(chunk_size: u64) -> Self {
        info!(chunk_size, "Initializing asset network server");
        Self { assets: HashMap::new(), chunk_size }
    }

    /// Register an asset with the server.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::{AssetId, network::AssetNetworkServer};
    ///
    /// let mut server = AssetNetworkServer::new(1024 * 1024);
    /// let id = AssetId::from_content(b"test data");
    /// server.register_asset(id, b"test data".to_vec());
    /// ```
    pub fn register_asset(&mut self, asset_id: AssetId, data: Vec<u8>) {
        debug!(asset_id = %asset_id, size = data.len(), "Registering asset");
        self.assets.insert(asset_id, data);
    }

    /// Handle a client request and generate response messages.
    ///
    /// Returns a list of messages to send to the client.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::{AssetId, network::{AssetNetworkServer, AssetNetworkMessage}};
    ///
    /// let mut server = AssetNetworkServer::new(1024 * 1024);
    /// let id = AssetId::from_content(b"test");
    /// server.register_asset(id, b"test".to_vec());
    ///
    /// let request = AssetNetworkMessage::Request {
    ///     asset_id: id,
    ///     resume_offset: None,
    /// };
    ///
    /// let responses = server.handle_request(request);
    /// assert!(!responses.is_empty());
    /// ```
    #[instrument(skip(self, message))]
    pub fn handle_request(&self, message: AssetNetworkMessage) -> Vec<AssetNetworkMessage> {
        match message {
            AssetNetworkMessage::Request { asset_id, resume_offset } => {
                debug!(asset_id = %asset_id, resume_offset, "Handling asset request");

                let Some(data) = self.assets.get(&asset_id) else {
                    warn!(asset_id = %asset_id, "Asset not found");
                    return vec![AssetNetworkMessage::Error {
                        asset_id,
                        error: "Asset not found".to_string(),
                    }];
                };

                if data.len() as u64 > MAX_ASSET_SIZE {
                    warn!(asset_id = %asset_id, size = data.len(), "Asset too large");
                    return vec![AssetNetworkMessage::Error {
                        asset_id,
                        error: format!("Asset too large: {} bytes", data.len()),
                    }];
                }

                let start_offset = resume_offset.unwrap_or(0);

                // Small asset: send in one message
                if data.len() as u64 <= self.chunk_size && start_offset == 0 {
                    let (compressed_data, is_compressed) = compress_if_beneficial(data);
                    let checksum = *blake3::hash(data).as_bytes();

                    info!(
                        asset_id = %asset_id,
                        size = data.len(),
                        compressed = is_compressed,
                        "Sending complete asset"
                    );

                    vec![AssetNetworkMessage::Response {
                        asset_id,
                        data: compressed_data,
                        checksum,
                        compressed: is_compressed,
                    }]
                } else {
                    // Large asset: send in chunks
                    let mut messages = Vec::new();
                    let total_size = data.len() as u64;
                    let mut offset = start_offset;

                    while offset < total_size {
                        let chunk_end = (offset + self.chunk_size).min(total_size);
                        let chunk_data = &data[offset as usize..chunk_end as usize];
                        let (compressed_chunk, is_compressed) = compress_if_beneficial(chunk_data);

                        messages.push(AssetNetworkMessage::Chunk {
                            asset_id,
                            offset,
                            total_size,
                            data: compressed_chunk,
                            compressed: is_compressed,
                        });

                        offset = chunk_end;
                    }

                    // Send completion message with checksum
                    let checksum = *blake3::hash(data).as_bytes();
                    messages.push(AssetNetworkMessage::Complete { asset_id, checksum });

                    info!(
                        asset_id = %asset_id,
                        total_size,
                        chunks = messages.len() - 1,
                        "Sending chunked asset"
                    );

                    messages
                }
            }

            _ => {
                warn!("Server received non-Request message");
                vec![]
            }
        }
    }

    /// Get the number of registered assets.
    #[must_use]
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }
}

impl Default for AssetNetworkServer {
    fn default() -> Self {
        Self::new(DEFAULT_CHUNK_SIZE)
    }
}

/// Compress data with LZ4 if it provides >10% size reduction.
///
/// Returns (data, is_compressed).
fn compress_if_beneficial(data: &[u8]) -> (Vec<u8>, bool) {
    if data.len() < 1024 {
        // Don't bother compressing very small data
        return (data.to_vec(), false);
    }

    match compress_lz4(data) {
        Ok(compressed) => {
            let ratio = compressed.len() as f32 / data.len() as f32;
            if ratio < 0.9 {
                // At least 10% reduction
                (compressed, true)
            } else {
                (data.to_vec(), false)
            }
        }
        Err(_) => (data.to_vec(), false),
    }
}

/// Compress data with LZ4.
fn compress_lz4(data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "lz4")]
    {
        Ok(lz4_flex::compress_prepend_size(data))
    }
    #[cfg(not(feature = "lz4"))]
    {
        Err("LZ4 compression not enabled".to_string())
    }
}

/// Decompress LZ4 data.
fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "lz4")]
    {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| format!("LZ4 decompression failed: {}", e))
    }
    #[cfg(not(feature = "lz4"))]
    {
        Err("LZ4 decompression not enabled".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let id = AssetId::from_content(b"test");
        let msg = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };

        let bytes = bincode::serialize(&msg).unwrap();
        let deserialized: AssetNetworkMessage = bincode::deserialize(&bytes).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_client_request_deduplication() {
        let mut client = AssetNetworkClient::new(4);
        let id = AssetId::from_content(b"test");

        client.request_asset(id, TransferPriority::Critical);
        assert_eq!(client.pending_count(), 1);

        // Duplicate request should be ignored
        client.request_asset(id, TransferPriority::Critical);
        assert_eq!(client.pending_count(), 1);
    }

    #[test]
    fn test_client_priority_ordering() {
        let mut client = AssetNetworkClient::new(4);

        let id1 = AssetId::from_content(b"low");
        let id2 = AssetId::from_content(b"critical");
        let id3 = AssetId::from_content(b"high");

        client.request_asset(id1, TransferPriority::Low);
        client.request_asset(id2, TransferPriority::Critical);
        client.request_asset(id3, TransferPriority::High);

        // Should get critical first
        let msg = client.next_request().unwrap();
        if let AssetNetworkMessage::Request { asset_id, .. } = msg {
            assert_eq!(asset_id, id2);
        } else {
            panic!("Expected Request message");
        }

        // Then high
        let msg = client.next_request().unwrap();
        if let AssetNetworkMessage::Request { asset_id, .. } = msg {
            assert_eq!(asset_id, id3);
        } else {
            panic!("Expected Request message");
        }

        // Then low
        let msg = client.next_request().unwrap();
        if let AssetNetworkMessage::Request { asset_id, .. } = msg {
            assert_eq!(asset_id, id1);
        } else {
            panic!("Expected Request message");
        }
    }

    #[test]
    fn test_server_small_asset() {
        let mut server = AssetNetworkServer::new(1024 * 1024);
        let id = AssetId::from_content(b"small asset");
        let data = b"small asset".to_vec();

        server.register_asset(id, data.clone());

        let request = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
        let responses = server.handle_request(request);

        assert_eq!(responses.len(), 1);
        if let AssetNetworkMessage::Response { asset_id, checksum, .. } = &responses[0] {
            assert_eq!(*asset_id, id);
            let expected_checksum = *blake3::hash(&data).as_bytes();
            assert_eq!(*checksum, expected_checksum);
        } else {
            panic!("Expected Response message");
        }
    }

    #[test]
    fn test_server_large_asset_chunking() {
        let mut server = AssetNetworkServer::new(100); // Small chunk size for testing
        let id = AssetId::from_content(b"large");
        let data = vec![0x42u8; 250]; // 250 bytes, should be 3 chunks

        server.register_asset(id, data.clone());

        let request = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
        let responses = server.handle_request(request);

        // Should have 3 chunks + 1 complete message
        assert_eq!(responses.len(), 4);

        // Verify chunks
        for i in 0..3 {
            if let AssetNetworkMessage::Chunk { offset, total_size, .. } = &responses[i] {
                assert_eq!(*offset, i as u64 * 100);
                assert_eq!(*total_size, 250);
            } else {
                panic!("Expected Chunk message at index {}", i);
            }
        }

        // Verify complete message
        if let AssetNetworkMessage::Complete { checksum, .. } = &responses[3] {
            let expected_checksum = *blake3::hash(&data).as_bytes();
            assert_eq!(*checksum, expected_checksum);
        } else {
            panic!("Expected Complete message");
        }
    }

    #[test]
    fn test_server_asset_not_found() {
        let server = AssetNetworkServer::new(1024 * 1024);
        let id = AssetId::from_content(b"nonexistent");

        let request = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
        let responses = server.handle_request(request);

        assert_eq!(responses.len(), 1);
        if let AssetNetworkMessage::Error { error, .. } = &responses[0] {
            assert_eq!(error, "Asset not found");
        } else {
            panic!("Expected Error message");
        }
    }

    #[test]
    fn test_client_server_roundtrip_small() {
        let mut server = AssetNetworkServer::new(1024 * 1024);
        let mut client = AssetNetworkClient::new(4);

        let id = AssetId::from_content(b"test data");
        let data = b"test data".to_vec();

        server.register_asset(id, data.clone());
        client.request_asset(id, TransferPriority::Critical);

        // Client sends request
        let request = client.next_request().unwrap();

        // Server handles request
        let responses = server.handle_request(request);

        // Client receives response
        for response in responses {
            client.handle_message(response).unwrap();
        }

        // Client should have completed asset
        let received = client.take_completed(id).unwrap();
        assert_eq!(received, data);
    }

    #[test]
    fn test_client_server_roundtrip_chunked() {
        let mut server = AssetNetworkServer::new(100);
        let mut client = AssetNetworkClient::new(4);

        let id = AssetId::from_content(b"large");
        let data = vec![0x42u8; 250];

        server.register_asset(id, data.clone());
        client.request_asset(id, TransferPriority::Critical);

        let request = client.next_request().unwrap();
        let responses = server.handle_request(request);

        for response in responses {
            client.handle_message(response).unwrap();
        }

        let received = client.take_completed(id).unwrap();
        assert_eq!(received, data);
    }

    #[test]
    fn test_checksum_validation() {
        let mut client = AssetNetworkClient::new(4);
        let id = AssetId::from_content(b"test");
        let data = b"test".to_vec();
        let bad_checksum = [0u8; 32];

        let msg = AssetNetworkMessage::Response {
            asset_id: id,
            data,
            checksum: bad_checksum,
            compressed: false,
        };

        let result = client.handle_message(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_lz4() {
        let data = b"This is a test string that should compress well. ".repeat(100);
        let (compressed, is_compressed) = compress_if_beneficial(&data);

        assert!(is_compressed);
        assert!(compressed.len() < data.len());

        let decompressed = decompress_lz4(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_not_beneficial() {
        // Random data doesn't compress well
        let data = vec![0x42u8; 100];
        let (result, is_compressed) = compress_if_beneficial(&data);

        // Might or might not compress depending on LZ4 behavior
        if is_compressed {
            // If compressed, verify it decompresses correctly
            let decompressed = decompress_lz4(&result).unwrap();
            assert_eq!(decompressed, data);
        } else {
            assert_eq!(result, data);
        }
    }

    #[test]
    fn test_max_concurrent_transfers() {
        let mut client = AssetNetworkClient::new(2);

        let id1 = AssetId::from_content(b"1");
        let id2 = AssetId::from_content(b"2");
        let id3 = AssetId::from_content(b"3");

        client.request_asset(id1, TransferPriority::Critical);
        client.request_asset(id2, TransferPriority::Critical);
        client.request_asset(id3, TransferPriority::Critical);

        // Should get first 2 requests
        assert!(client.next_request().is_some());
        assert!(client.next_request().is_some());

        // Third should be blocked
        assert!(client.next_request().is_none());
        assert_eq!(client.active_count(), 2);
        assert_eq!(client.pending_count(), 1);
    }

    #[test]
    fn test_resumable_download() {
        let mut client = AssetNetworkClient::new(4);
        let id = AssetId::from_content(b"test");

        // Simulate partial download
        client.chunk_buffers.insert(id, vec![0x42u8; 100]);
        client.request_asset(id, TransferPriority::Critical);

        let request = client.next_request().unwrap();
        if let AssetNetworkMessage::Request { resume_offset, .. } = request {
            assert_eq!(resume_offset, Some(100));
        } else {
            panic!("Expected Request with resume offset");
        }
    }
}
