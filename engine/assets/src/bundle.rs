//! Asset bundle packing and unpacking.
//!
//! Bundles package multiple assets into a single file for efficient distribution
//! and loading. Supports compression and integrity verification.

use crate::{AssetId, AssetManifest};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use tracing::{info, warn};

define_error! {
    pub enum BundleError {
        InvalidFormat { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        CompressionFailed { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        DecompressionFailed { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        AssetNotFound { id: String } = ErrorCode::AssetNotFound, ErrorSeverity::Error,
        IoError { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        ChecksumMismatch { id: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

/// Compression format for asset bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    /// No compression (fastest, largest).
    None,
    /// LZ4 compression (fast, good ratio).
    Lz4,
    /// Zstd compression (slower, better ratio).
    Zstd,
}

impl CompressionFormat {
    /// Get the file extension for this compression format.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::None => "bundle",
            Self::Lz4 => "bundle.lz4",
            Self::Zstd => "bundle.zst",
        }
    }
}

/// Header for asset bundle files.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleHeader {
    /// Magic number for validation ("BNDL").
    magic: [u8; 4],
    /// Bundle format version.
    version: u32,
    /// Compression format used.
    compression: CompressionFormat,
    /// Number of assets in bundle.
    asset_count: u32,
    /// Total size of uncompressed data.
    uncompressed_size: u64,
    /// Total size of compressed data.
    compressed_size: u64,
}

impl BundleHeader {
    const MAGIC: [u8; 4] = *b"BNDL";
    const CURRENT_VERSION: u32 = 1;

    fn new(
        compression: CompressionFormat,
        asset_count: u32,
        uncompressed_size: u64,
        compressed_size: u64,
    ) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::CURRENT_VERSION,
            compression,
            asset_count,
            uncompressed_size,
            compressed_size,
        }
    }

    fn validate(&self) -> Result<(), BundleError> {
        if self.magic != Self::MAGIC {
            return Err(BundleError::invalidformat("Invalid magic number".to_string()));
        }

        if self.version > Self::CURRENT_VERSION {
            return Err(BundleError::invalidformat(format!(
                "Unsupported bundle version: {}",
                self.version
            )));
        }

        Ok(())
    }
}

/// Packed asset data within a bundle.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PackedAsset {
    id: AssetId,
    data: Vec<u8>,
    checksum: [u8; 32],
}

/// Asset bundle containing multiple packed assets.
///
/// # Format
///
/// ```text
/// [Header: magic, version, compression, count, sizes]
/// [Manifest: serialized AssetManifest]
/// [Asset 1: id, size, data]
/// [Asset 2: id, size, data]
/// ...
/// ```
pub struct AssetBundle {
    /// Bundle manifest describing all assets.
    manifest: AssetManifest,
    /// Packed asset data (ID -> data).
    assets: HashMap<AssetId, Vec<u8>>,
    /// Compression format.
    compression: CompressionFormat,
}

impl AssetBundle {
    /// Create a new empty bundle.
    #[must_use]
    pub fn new(compression: CompressionFormat) -> Self {
        Self { manifest: AssetManifest::new(), assets: HashMap::new(), compression }
    }

    /// Create a bundle from a manifest (data must be added separately).
    #[must_use]
    pub fn from_manifest(manifest: AssetManifest, compression: CompressionFormat) -> Self {
        Self { manifest, assets: HashMap::new(), compression }
    }

    /// Add an asset to the bundle.
    ///
    /// # Errors
    ///
    /// Returns an error if the asset ID is not in the manifest.
    pub fn add_asset(&mut self, id: AssetId, data: Vec<u8>) -> Result<(), BundleError> {
        // Verify asset exists in manifest
        let entry = self
            .manifest
            .get_asset(id)
            .ok_or_else(|| BundleError::assetnotfound(format!("{id}")))?;

        // Verify checksum
        if !entry.verify_checksum(&data) {
            warn!(id = ?id, "Checksum mismatch when adding asset to bundle");
            return Err(BundleError::checksummismatch(format!("{id}")));
        }

        self.assets.insert(id, data);
        Ok(())
    }

    /// Get an asset from the bundle.
    #[must_use]
    pub fn get_asset(&self, id: AssetId) -> Option<&[u8]> {
        self.assets.get(&id).map(|v| v.as_slice())
    }

    /// Get the manifest.
    #[must_use]
    pub fn manifest(&self) -> &AssetManifest {
        &self.manifest
    }

    /// Get the compression format.
    #[must_use]
    pub fn compression(&self) -> CompressionFormat {
        self.compression
    }

    /// Pack the bundle into bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or compression fails.
    pub fn pack(&self) -> Result<Vec<u8>, BundleError> {
        info!(
            asset_count = self.assets.len(),
            compression = ?self.compression,
            "Packing asset bundle"
        );

        // Serialize manifest
        let manifest_bytes = self
            .manifest
            .to_bincode()
            .map_err(|e| BundleError::ioerror(format!("Failed to serialize manifest: {e}")))?;

        // Calculate uncompressed size
        let mut uncompressed_size = manifest_bytes.len() as u64;
        for data in self.assets.values() {
            uncompressed_size += 32 + 4 + data.len() as u64; // ID + size + data
        }

        // Pack data
        let mut packed_data = Vec::new();

        // Write manifest length and data
        packed_data
            .write_all(&(manifest_bytes.len() as u32).to_le_bytes())
            .map_err(|e| BundleError::ioerror(format!("Failed to write manifest length: {e}")))?;
        packed_data
            .write_all(&manifest_bytes)
            .map_err(|e| BundleError::ioerror(format!("Failed to write manifest: {e}")))?;

        // Write assets in manifest order for predictable packing
        for entry in &self.manifest.assets {
            if let Some(data) = self.assets.get(&entry.id) {
                // Write asset ID (32 bytes)
                packed_data
                    .write_all(entry.id.as_bytes())
                    .map_err(|e| BundleError::ioerror(format!("Failed to write asset ID: {e}")))?;

                // Write data length (4 bytes)
                packed_data.write_all(&(data.len() as u32).to_le_bytes()).map_err(|e| {
                    BundleError::ioerror(format!("Failed to write data length: {e}"))
                })?;

                // Write data
                packed_data.write_all(data).map_err(|e| {
                    BundleError::ioerror(format!("Failed to write asset data: {e}"))
                })?;
            }
        }

        // Compress if requested
        let compressed_data = match self.compression {
            CompressionFormat::None => packed_data,
            CompressionFormat::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    lz4_flex::compress_prepend_size(&packed_data)
                }
                #[cfg(not(feature = "lz4"))]
                {
                    return Err(BundleError::compressionfailed(
                        "LZ4 support not enabled".to_string(),
                    ));
                }
            }
            CompressionFormat::Zstd => {
                #[cfg(feature = "zstd")]
                {
                    zstd::encode_all(&packed_data[..], 3)
                        .map_err(|e| BundleError::compressionfailed(format!("Zstd: {e}")))?
                }
                #[cfg(not(feature = "zstd"))]
                {
                    return Err(BundleError::compressionfailed(
                        "Zstd support not enabled".to_string(),
                    ));
                }
            }
        };

        let compressed_size = compressed_data.len() as u64;

        // Create header
        let header = BundleHeader::new(
            self.compression,
            self.assets.len() as u32,
            uncompressed_size,
            compressed_size,
        );

        // Serialize header
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| BundleError::ioerror(format!("Failed to serialize header: {e}")))?;

        // Combine header + compressed data
        let mut result = Vec::new();
        result
            .write_all(&header_bytes)
            .map_err(|e| BundleError::ioerror(format!("Failed to write header: {e}")))?;
        result
            .write_all(&compressed_data)
            .map_err(|e| BundleError::ioerror(format!("Failed to write compressed data: {e}")))?;

        info!(
            uncompressed_size,
            compressed_size,
            ratio = (compressed_size as f64 / uncompressed_size as f64),
            "Bundle packed successfully"
        );

        Ok(result)
    }

    /// Unpack a bundle from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization or decompression fails.
    pub fn unpack(data: &[u8]) -> Result<Self, BundleError> {
        info!(size_bytes = data.len(), "Unpacking asset bundle");

        // Deserialize header (first part of data)
        // We need to figure out header size - bincode doesn't have fixed size
        // For safety, assume header is first 256 bytes max
        if data.len() < 64 {
            return Err(BundleError::invalidformat("Bundle too small".to_string()));
        }

        // Try to deserialize header
        let header: BundleHeader = bincode::deserialize(data).map_err(|e| {
            BundleError::invalidformat(format!("Failed to deserialize header: {e}"))
        })?;

        header.validate()?;

        // Calculate header size by serializing it again
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| BundleError::ioerror(format!("Failed to serialize header: {e}")))?;
        let header_size = header_bytes.len();

        if data.len() < header_size {
            return Err(BundleError::invalidformat("Incomplete bundle".to_string()));
        }

        let compressed_data = &data[header_size..];

        // Decompress data
        let decompressed_data = match header.compression {
            CompressionFormat::None => compressed_data.to_vec(),
            CompressionFormat::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    lz4_flex::decompress_size_prepended(compressed_data)
                        .map_err(|e| BundleError::decompressionfailed(format!("LZ4: {e}")))?
                }
                #[cfg(not(feature = "lz4"))]
                {
                    return Err(BundleError::decompressionfailed(
                        "LZ4 support not enabled".to_string(),
                    ));
                }
            }
            CompressionFormat::Zstd => {
                #[cfg(feature = "zstd")]
                {
                    zstd::decode_all(compressed_data)
                        .map_err(|e| BundleError::decompressionfailed(format!("Zstd: {e}")))?
                }
                #[cfg(not(feature = "zstd"))]
                {
                    return Err(BundleError::decompressionfailed(
                        "Zstd support not enabled".to_string(),
                    ));
                }
            }
        };

        // Read manifest
        let mut cursor = &decompressed_data[..];
        let mut manifest_len_bytes = [0u8; 4];
        cursor
            .read_exact(&mut manifest_len_bytes)
            .map_err(|e| BundleError::ioerror(format!("Failed to read manifest length: {e}")))?;
        let manifest_len = u32::from_le_bytes(manifest_len_bytes) as usize;

        let mut manifest_bytes = vec![0u8; manifest_len];
        cursor
            .read_exact(&mut manifest_bytes)
            .map_err(|e| BundleError::ioerror(format!("Failed to read manifest: {e}")))?;

        let manifest = AssetManifest::from_bincode(&manifest_bytes)
            .map_err(|e| BundleError::ioerror(format!("Failed to deserialize manifest: {e}")))?;

        // Read assets
        let mut assets = HashMap::new();
        while !cursor.is_empty() {
            // Read asset ID (32 bytes)
            let mut id_bytes = [0u8; 32];
            if cursor.read_exact(&mut id_bytes).is_err() {
                break; // End of data
            }
            let id = AssetId::from_bytes(id_bytes);

            // Read data length (4 bytes)
            let mut len_bytes = [0u8; 4];
            cursor
                .read_exact(&mut len_bytes)
                .map_err(|e| BundleError::ioerror(format!("Failed to read data length: {e}")))?;
            let len = u32::from_le_bytes(len_bytes) as usize;

            // Read data
            let mut asset_data = vec![0u8; len];
            cursor
                .read_exact(&mut asset_data)
                .map_err(|e| BundleError::ioerror(format!("Failed to read asset data: {e}")))?;

            // Verify checksum
            if let Some(entry) = manifest.get_asset(id) {
                if !entry.verify_checksum(&asset_data) {
                    warn!(id = ?id, "Checksum mismatch when unpacking asset");
                    return Err(BundleError::checksummismatch(format!("{id}")));
                }
            }

            assets.insert(id, asset_data);
        }

        info!(
            asset_count = assets.len(),
            compression = ?header.compression,
            "Bundle unpacked successfully"
        );

        Ok(Self { manifest, assets, compression: header.compression })
    }

    /// Get statistics about the bundle.
    #[must_use]
    pub fn stats(&self) -> BundleStats {
        let total_size = self.assets.values().map(|v| v.len() as u64).sum();

        BundleStats { asset_count: self.assets.len(), total_size, compression: self.compression }
    }
}

/// Statistics about an asset bundle.
#[derive(Debug, Clone)]
pub struct BundleStats {
    /// Number of assets in bundle.
    pub asset_count: usize,
    /// Total uncompressed size in bytes.
    pub total_size: u64,
    /// Compression format.
    pub compression: CompressionFormat,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetEntry, AssetType};
    use std::path::PathBuf;

    fn create_test_manifest() -> AssetManifest {
        let mut manifest = AssetManifest::new();

        let id1 = AssetId::from_content(b"asset1");
        let data1 = b"test data 1";
        let entry1 = AssetEntry::new(
            id1,
            PathBuf::from("asset1.dat"),
            AssetType::Mesh,
            data1.len() as u64,
            *blake3::hash(data1).as_bytes(),
        );

        let id2 = AssetId::from_content(b"asset2");
        let data2 = b"test data 2";
        let entry2 = AssetEntry::new(
            id2,
            PathBuf::from("asset2.dat"),
            AssetType::Texture,
            data2.len() as u64,
            *blake3::hash(data2).as_bytes(),
        );

        manifest.add_asset(entry1);
        manifest.add_asset(entry2);

        manifest
    }

    #[test]
    fn test_bundle_creation() {
        let bundle = AssetBundle::new(CompressionFormat::None);
        assert_eq!(bundle.assets.len(), 0);
    }

    #[test]
    fn test_add_asset() {
        let manifest = create_test_manifest();
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        let id = AssetId::from_content(b"asset1");
        let data = b"test data 1".to_vec();

        assert!(bundle.add_asset(id, data).is_ok());
        assert_eq!(bundle.assets.len(), 1);
    }

    #[test]
    fn test_add_asset_checksum_mismatch() {
        let manifest = create_test_manifest();
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        let id = AssetId::from_content(b"asset1");
        let wrong_data = b"wrong data".to_vec();

        let result = bundle.add_asset(id, wrong_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BundleError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_pack_unpack_no_compression() {
        let manifest = create_test_manifest();
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        // Add assets
        let id1 = AssetId::from_content(b"asset1");
        let data1 = b"test data 1".to_vec();
        bundle.add_asset(id1, data1.clone()).unwrap();

        let id2 = AssetId::from_content(b"asset2");
        let data2 = b"test data 2".to_vec();
        bundle.add_asset(id2, data2.clone()).unwrap();

        // Pack
        let packed = bundle.pack().unwrap();

        // Unpack
        let unpacked = AssetBundle::unpack(&packed).unwrap();

        // Verify
        assert_eq!(unpacked.assets.len(), 2);
        assert_eq!(unpacked.get_asset(id1).unwrap(), data1.as_slice());
        assert_eq!(unpacked.get_asset(id2).unwrap(), data2.as_slice());
    }

    #[test]
    fn test_bundle_stats() {
        let manifest = create_test_manifest();
        let mut bundle = AssetBundle::from_manifest(manifest, CompressionFormat::None);

        let id = AssetId::from_content(b"asset1");
        let data = b"test data 1".to_vec();
        bundle.add_asset(id, data).unwrap();

        let stats = bundle.stats();
        assert_eq!(stats.asset_count, 1);
        assert_eq!(stats.total_size, 11);
        assert_eq!(stats.compression, CompressionFormat::None);
    }

    #[test]
    fn test_compression_format_extension() {
        assert_eq!(CompressionFormat::None.extension(), "bundle");
        assert_eq!(CompressionFormat::Lz4.extension(), "bundle.lz4");
        assert_eq!(CompressionFormat::Zstd.extension(), "bundle.zst");
    }

    #[test]
    fn test_bundle_with_empty_manifest() {
        let bundle = AssetBundle::new(CompressionFormat::None);
        let packed = bundle.pack().unwrap();
        let unpacked = AssetBundle::unpack(&packed).unwrap();

        assert_eq!(unpacked.assets.len(), 0);
    }

    #[test]
    fn test_get_nonexistent_asset() {
        let bundle = AssetBundle::new(CompressionFormat::None);
        let id = AssetId::from_content(b"nonexistent");

        assert!(bundle.get_asset(id).is_none());
    }

    #[test]
    fn test_invalid_bundle_data() {
        let invalid_data = b"not a bundle";
        let result = AssetBundle::unpack(invalid_data);

        assert!(result.is_err());
    }
}
