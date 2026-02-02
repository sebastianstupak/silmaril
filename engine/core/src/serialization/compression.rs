//! Compression support for serialized data
//!
//! Supports:
//! - LZ4: Fast compression for network transmission
//! - Zstd: High compression ratio for save files
//!
//! Only available with `compression` feature flag.

use super::SerializationError;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// LZ4 - Fast compression/decompression (good for network)
    #[cfg(feature = "compression")]
    Lz4,
    /// Zstd - High compression ratio (good for save files)
    #[cfg(feature = "compression")]
    Zstd,
}

/// Compress data using specified algorithm
pub fn compress(
    data: &[u8],
    algorithm: CompressionAlgorithm,
) -> Result<Vec<u8>, SerializationError> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),

        #[cfg(feature = "compression")]
        CompressionAlgorithm::Lz4 => lz4::block::compress(data, None, false)
            .map_err(|e| SerializationError::compressionerror(e.to_string())),

        #[cfg(feature = "compression")]
        CompressionAlgorithm::Zstd => {
            zstd::bulk::compress(data, 3) // Level 3 balances speed/ratio
                .map_err(|e| SerializationError::compressionerror(e.to_string()))
        }
    }
}

/// Decompress data
pub fn decompress(
    data: &[u8],
    algorithm: CompressionAlgorithm,
    _original_size: Option<usize>,
) -> Result<Vec<u8>, SerializationError> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),

        #[cfg(feature = "compression")]
        CompressionAlgorithm::Lz4 => {
            let size = _original_size.ok_or_else(|| {
                SerializationError::decompressionerror("LZ4 requires original size".into())
            })?;
            lz4::block::decompress(data, Some(size as i32))
                .map_err(|e| SerializationError::decompressionerror(e.to_string()))
        }

        #[cfg(feature = "compression")]
        CompressionAlgorithm::Zstd => {
            let size = _original_size.unwrap_or(data.len() * 4); // Estimate if not provided
            zstd::bulk::decompress(data, size)
                .map_err(|e| SerializationError::decompressionerror(e.to_string()))
        }
    }
}

/// Compressed data with metadata
#[derive(Debug, Clone)]
pub struct CompressedData {
    /// Compressed bytes
    pub data: Vec<u8>,
    /// Original size before compression
    pub original_size: usize,
    /// Compression algorithm used
    pub algorithm: CompressionAlgorithm,
    /// Compression ratio (compressed / original)
    pub compression_ratio: f32,
}

impl CompressedData {
    /// Compress data and create metadata
    pub fn compress(
        data: &[u8],
        algorithm: CompressionAlgorithm,
    ) -> Result<Self, SerializationError> {
        let original_size = data.len();
        let compressed = compress(data, algorithm)?;
        let compression_ratio = compressed.len() as f32 / original_size as f32;

        Ok(Self { data: compressed, original_size, algorithm, compression_ratio })
    }

    /// Decompress data
    pub fn decompress(&self) -> Result<Vec<u8>, SerializationError> {
        decompress(&self.data, self.algorithm, Some(self.original_size))
    }

    /// Size savings in bytes
    pub fn size_savings(&self) -> isize {
        self.original_size as isize - self.data.len() as isize
    }

    /// Size savings as percentage (0-100)
    pub fn size_savings_percent(&self) -> f32 {
        (1.0 - self.compression_ratio) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compression() {
        let data = b"Hello, World!";
        let compressed = compress(data, CompressionAlgorithm::None).unwrap();
        assert_eq!(&compressed, data);

        let decompressed = decompress(&compressed, CompressionAlgorithm::None, None).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_lz4_compression() {
        let data = b"Hello, World! This is a test of LZ4 compression.";
        let compressed = compress(data, CompressionAlgorithm::Lz4).unwrap();
        assert!(compressed.len() > 0);

        let decompressed =
            decompress(&compressed, CompressionAlgorithm::Lz4, Some(data.len())).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_zstd_compression() {
        let data = b"Hello, World! This is a test of Zstd compression.";
        let compressed = compress(data, CompressionAlgorithm::Zstd).unwrap();
        assert!(compressed.len() > 0);
        assert!(compressed.len() < data.len()); // Should compress small data

        let decompressed =
            decompress(&compressed, CompressionAlgorithm::Zstd, Some(data.len())).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_compressed_data() {
        let data = vec![42u8; 1000]; // Highly compressible
        let compressed = CompressedData::compress(&data, CompressionAlgorithm::Zstd).unwrap();

        assert!(compressed.compression_ratio < 0.1); // Should compress very well
        assert!(compressed.size_savings() > 900); // Should save >900 bytes
        assert!(compressed.size_savings_percent() > 90.0); // Should save >90%

        let decompressed = compressed.decompress().unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_compression_ratio() {
        // Test with realistic game data
        let mut data = Vec::new();
        for i in 0..1000 {
            data.extend_from_slice(&i.to_le_bytes());
            data.extend_from_slice(&[0u8; 12]); // Padding (compressible)
        }

        let lz4 = CompressedData::compress(&data, CompressionAlgorithm::Lz4).unwrap();
        let zstd = CompressedData::compress(&data, CompressionAlgorithm::Zstd).unwrap();

        // Log compression results
        tracing::debug!(
            original_bytes = data.len(),
            lz4_bytes = lz4.data.len(),
            lz4_savings = lz4.size_savings_percent(),
            zstd_bytes = zstd.data.len(),
            zstd_savings = zstd.size_savings_percent(),
            "Compression comparison"
        );

        // Zstd should have better compression ratio
        assert!(zstd.compression_ratio < lz4.compression_ratio);
    }
}
