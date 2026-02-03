//! Texture data structures (pure data, no GPU/rendering dependencies)
//!
//! Can be used by:
//! - Client for rendering (via engine-renderer)
//! - Tools for asset processing
//! - Server for procedural generation

use crate::validation::{compute_hash, AssetValidator, ValidationError};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{info, instrument};

define_error! {
    pub enum TextureError {
        InvalidFormat { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        LoadFailed { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        InvalidDimensions { width: u32, height: u32 } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        UnsupportedFormat { format: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

/// Texture format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextureFormat {
    /// RGBA 8-bit per channel, unsigned normalized
    RGBA8Unorm,
    /// RGBA 8-bit per channel, sRGB color space
    RGBA8Srgb,
    /// BC7 block compression (desktop)
    BC7Unorm,
    /// ASTC 4x4 block compression (mobile)
    ASTC4x4Unorm,
}

impl TextureFormat {
    /// Get the number of bytes per block for compressed formats
    pub fn block_size(self) -> usize {
        match self {
            Self::RGBA8Unorm | Self::RGBA8Srgb => 1, // Per pixel, not block
            Self::BC7Unorm => 16,                    // 16 bytes per 4x4 block
            Self::ASTC4x4Unorm => 16,                // 16 bytes per 4x4 block
        }
    }

    /// Get the block dimensions (width, height)
    pub fn block_dimensions(self) -> (u32, u32) {
        match self {
            Self::RGBA8Unorm | Self::RGBA8Srgb => (1, 1),
            Self::BC7Unorm | Self::ASTC4x4Unorm => (4, 4),
        }
    }

    /// Check if format is compressed
    pub fn is_compressed(self) -> bool {
        matches!(self, Self::BC7Unorm | Self::ASTC4x4Unorm)
    }

    /// Get bytes per pixel for uncompressed formats
    pub fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            Self::RGBA8Unorm | Self::RGBA8Srgb => Some(4),
            Self::BC7Unorm | Self::ASTC4x4Unorm => None, // Compressed
        }
    }
}

/// Mipmap level information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MipLevel {
    /// Width of this mip level
    pub width: u32,
    /// Height of this mip level
    pub height: u32,
    /// Offset into texture data buffer
    pub data_offset: usize,
    /// Size of data for this mip level
    pub data_size: usize,
}

impl MipLevel {
    /// Create a new mip level
    pub fn new(width: u32, height: u32, data_offset: usize, data_size: usize) -> Self {
        Self { width, height, data_offset, data_size }
    }
}

/// Texture data (CPU-side image data)
///
/// Pure data structure - no GPU textures or rendering state.
/// Rendering backends create GPU resources from this data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextureData {
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
    /// Pixel format
    pub format: TextureFormat,
    /// Mipmap levels (at least one for base level)
    pub mip_levels: Vec<MipLevel>,
    /// Raw texture data (all mip levels concatenated)
    pub data: Vec<u8>,
}

impl TextureData {
    /// Load texture from PNG or JPG image data
    ///
    /// Automatically converts to RGBA8Unorm format.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use engine_assets::TextureData;
    ///
    /// let png_data = std::fs::read("texture.png").unwrap();
    /// let texture = TextureData::from_image_bytes(&png_data).unwrap();
    /// assert_eq!(texture.format, engine_assets::TextureFormat::RGBA8Unorm);
    /// ```
    #[instrument(skip(image_data))]
    pub fn from_image_bytes(image_data: &[u8]) -> Result<Self, TextureError> {
        use image::ImageReader;
        use std::io::Cursor;

        // Load image
        let img = ImageReader::new(Cursor::new(image_data))
            .with_guessed_format()
            .map_err(|e| TextureError::loadfailed(format!("Failed to guess image format: {}", e)))?
            .decode()
            .map_err(|e| TextureError::loadfailed(format!("Failed to decode image: {}", e)))?;

        // Convert to RGBA8
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let data = rgba.into_raw();

        info!(width, height, data_size = data.len(), "Loaded image texture");

        Self::new(width, height, TextureFormat::RGBA8Unorm, data)
    }

    /// Load texture from DDS file data
    ///
    /// Supports compressed formats (BC7, etc.) and mipmaps.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use engine_assets::TextureData;
    ///
    /// let dds_data = std::fs::read("texture.dds").unwrap();
    /// let texture = TextureData::from_dds_bytes(&dds_data).unwrap();
    /// ```
    #[instrument(skip(dds_data))]
    pub fn from_dds_bytes(dds_data: &[u8]) -> Result<Self, TextureError> {
        use ddsfile::Dds;

        let dds = Dds::read(&mut std::io::Cursor::new(dds_data))
            .map_err(|e| TextureError::loadfailed(format!("Failed to parse DDS: {}", e)))?;

        let width = dds.get_width();
        let height = dds.get_height();
        let mip_count = dds.get_num_mipmap_levels();

        // Determine format (currently only support BC7)
        let format = match dds.get_dxgi_format() {
            Some(ddsfile::DxgiFormat::BC7_UNorm) => TextureFormat::BC7Unorm,
            Some(ddsfile::DxgiFormat::R8G8B8A8_UNorm) => TextureFormat::RGBA8Unorm,
            Some(ddsfile::DxgiFormat::R8G8B8A8_UNorm_sRGB) => TextureFormat::RGBA8Srgb,
            other => {
                return Err(TextureError::unsupportedformat(format!(
                    "Unsupported DDS format: {:?}",
                    other
                )))
            }
        };

        // Extract mipmap data
        let mut mip_levels = Vec::with_capacity(mip_count as usize);
        let mut all_data = Vec::new();
        let mut offset = 0;

        for mip_idx in 0..mip_count {
            let mip_width = std::cmp::max(1, width >> mip_idx);
            let mip_height = std::cmp::max(1, height >> mip_idx);

            let mip_data = dds.get_data(mip_idx).map_err(|e| {
                TextureError::loadfailed(format!("Failed to get mip level {}: {}", mip_idx, e))
            })?;

            let data_size = mip_data.len();
            all_data.extend_from_slice(mip_data);

            mip_levels.push(MipLevel::new(mip_width, mip_height, offset, data_size));
            offset += data_size;
        }

        info!(
            width,
            height,
            mip_count,
            format = ?format,
            total_size = all_data.len(),
            "Loaded DDS texture"
        );

        Ok(Self { width, height, format, mip_levels, data: all_data })
    }

    /// Generate mipmaps for this texture
    ///
    /// Creates a mipmap chain down to 1x1.
    /// Only works for uncompressed RGBA8 textures.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::{TextureData, TextureFormat};
    ///
    /// let data = vec![255u8; 256 * 256 * 4];
    /// let texture = TextureData::new(256, 256, TextureFormat::RGBA8Unorm, data).unwrap();
    /// let with_mips = texture.generate_mipmaps().unwrap();
    /// assert!(with_mips.mip_count() > 1);
    /// ```
    #[instrument(skip(self))]
    pub fn generate_mipmaps(self) -> Result<Self, TextureError> {
        use image::{imageops, RgbaImage};

        // Only support RGBA8 for now
        if self.format != TextureFormat::RGBA8Unorm && self.format != TextureFormat::RGBA8Srgb {
            return Err(TextureError::unsupportedformat(
                "Mipmap generation only supports RGBA8 formats".to_string(),
            ));
        }

        // Check dimensions are power of 2
        if !self.width.is_power_of_two() || !self.height.is_power_of_two() {
            return Err(TextureError::invaliddimensions(self.width, self.height));
        }

        let mut mip_levels = Vec::new();
        let mut all_data = Vec::new();
        let mut offset = 0;

        // Calculate number of mip levels
        let max_mips =
            std::cmp::max((self.width as f32).log2() as u32, (self.height as f32).log2() as u32)
                + 1;

        let mut current_width = self.width;
        let mut current_height = self.height;
        let mut current_img = RgbaImage::from_raw(self.width, self.height, self.data.clone())
            .ok_or_else(|| TextureError::invalidformat("Invalid image data".to_string()))?;

        for _mip in 0..max_mips {
            let raw_data = current_img.as_raw().clone();
            let data_size = raw_data.len();

            mip_levels.push(MipLevel::new(current_width, current_height, offset, data_size));
            all_data.extend_from_slice(&raw_data);
            offset += data_size;

            // Stop if we've reached 1x1
            if current_width == 1 && current_height == 1 {
                break;
            }

            // Downscale for next mip level
            current_width = std::cmp::max(1, current_width / 2);
            current_height = std::cmp::max(1, current_height / 2);

            current_img = imageops::resize(
                &current_img,
                current_width,
                current_height,
                imageops::FilterType::Lanczos3,
            );
        }

        info!(
            original_size = self.width,
            mip_count = mip_levels.len(),
            total_size = all_data.len(),
            "Generated mipmaps"
        );

        Ok(Self {
            width: self.width,
            height: self.height,
            format: self.format,
            mip_levels,
            data: all_data,
        })
    }

    /// Create a new texture with single mip level
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        data: Vec<u8>,
    ) -> Result<Self, TextureError> {
        let expected_size = Self::calculate_data_size(width, height, format);
        if data.len() != expected_size {
            return Err(TextureError::invalidformat(format!(
                "Data size mismatch: expected {} bytes, got {} bytes",
                expected_size,
                data.len()
            )));
        }

        let mip_level = MipLevel::new(width, height, 0, data.len());

        Ok(Self { width, height, format, mip_levels: vec![mip_level], data })
    }

    /// Calculate required data size for a texture
    fn calculate_data_size(width: u32, height: u32, format: TextureFormat) -> usize {
        if let Some(bpp) = format.bytes_per_pixel() {
            // Uncompressed format
            (width * height) as usize * bpp
        } else {
            // Compressed format
            let (block_w, block_h) = format.block_dimensions();
            let blocks_x = width.div_ceil(block_w);
            let blocks_y = height.div_ceil(block_h);
            blocks_x as usize * blocks_y as usize * format.block_size()
        }
    }

    /// Get the number of mipmap levels
    pub fn mip_count(&self) -> usize {
        self.mip_levels.len()
    }

    /// Get data for a specific mip level
    pub fn mip_data(&self, level: usize) -> Option<&[u8]> {
        self.mip_levels.get(level).map(|mip| {
            let start = mip.data_offset;
            let end = start + mip.data_size;
            &self.data[start..end]
        })
    }

    /// Calculate total memory size in bytes
    pub fn memory_size(&self) -> usize {
        self.data.len()
    }
}

// ============================================================================
// Validation Implementation
// ============================================================================

impl AssetValidator for TextureData {
    /// Validate texture format (dimensions, mipmap chain)
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        // For raw texture data, we can't validate without metadata
        // This would be implemented for specific file formats (DDS, etc.)
        if data.is_empty() {
            return Err(ValidationError::emptydata());
        }
        Ok(())
    }

    /// Validate texture data integrity
    fn validate_data(&self) -> Result<(), ValidationError> {
        // Check dimensions
        if self.width == 0 || self.height == 0 {
            return Err(ValidationError::invaliddimensions(format!(
                "Zero dimensions: {}x{}",
                self.width, self.height
            )));
        }

        // Check reasonable maximum size (16K x 16K)
        const MAX_DIMENSION: u32 = 16384;
        if self.width > MAX_DIMENSION || self.height > MAX_DIMENSION {
            return Err(ValidationError::invaliddimensions(format!(
                "Dimensions too large: {}x{} (max {}x{})",
                self.width, self.height, MAX_DIMENSION, MAX_DIMENSION
            )));
        }

        // Check mipmap chain consistency
        if self.mip_levels.is_empty() {
            return Err(ValidationError::mipmapchaininconsistent(
                "No mip levels defined".to_string(),
            ));
        }

        // Validate first mip level matches base dimensions
        let base_mip = &self.mip_levels[0];
        if base_mip.width != self.width || base_mip.height != self.height {
            return Err(ValidationError::mipmapchaininconsistent(format!(
                "Base mip dimensions ({} x {}) don't match texture dimensions ({} x {})",
                base_mip.width, base_mip.height, self.width, self.height
            )));
        }

        // Validate mipmap chain dimensions
        for (i, mip) in self.mip_levels.iter().enumerate() {
            let expected_width = std::cmp::max(1, self.width >> i);
            let expected_height = std::cmp::max(1, self.height >> i);

            if mip.width != expected_width || mip.height != expected_height {
                return Err(ValidationError::mipmapchaininconsistent(format!(
                    "Mip level {} has wrong dimensions: got {}x{}, expected {}x{}",
                    i, mip.width, mip.height, expected_width, expected_height
                )));
            }

            // Validate data offset and size
            if mip.data_offset + mip.data_size > self.data.len() {
                return Err(ValidationError::mipmapchaininconsistent(format!(
                    "Mip level {} data extends beyond buffer: offset={}, size={}, buffer={}",
                    i,
                    mip.data_offset,
                    mip.data_size,
                    self.data.len()
                )));
            }
        }

        Ok(())
    }

    /// Validate checksum
    fn validate_checksum(&self, expected: &[u8; 32]) -> Result<(), ValidationError> {
        let actual = self.compute_checksum();
        if &actual != expected {
            return Err(ValidationError::checksummismatch(*expected, actual));
        }
        Ok(())
    }

    /// Compute Blake3 checksum of texture data
    fn compute_checksum(&self) -> [u8; 32] {
        compute_hash(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_format_block_size() {
        assert_eq!(TextureFormat::RGBA8Unorm.block_size(), 1);
        assert_eq!(TextureFormat::RGBA8Srgb.block_size(), 1);
        assert_eq!(TextureFormat::BC7Unorm.block_size(), 16);
        assert_eq!(TextureFormat::ASTC4x4Unorm.block_size(), 16);
    }

    #[test]
    fn test_texture_format_block_dimensions() {
        assert_eq!(TextureFormat::RGBA8Unorm.block_dimensions(), (1, 1));
        assert_eq!(TextureFormat::BC7Unorm.block_dimensions(), (4, 4));
        assert_eq!(TextureFormat::ASTC4x4Unorm.block_dimensions(), (4, 4));
    }

    #[test]
    fn test_texture_format_is_compressed() {
        assert!(!TextureFormat::RGBA8Unorm.is_compressed());
        assert!(!TextureFormat::RGBA8Srgb.is_compressed());
        assert!(TextureFormat::BC7Unorm.is_compressed());
        assert!(TextureFormat::ASTC4x4Unorm.is_compressed());
    }

    #[test]
    fn test_texture_format_bytes_per_pixel() {
        assert_eq!(TextureFormat::RGBA8Unorm.bytes_per_pixel(), Some(4));
        assert_eq!(TextureFormat::RGBA8Srgb.bytes_per_pixel(), Some(4));
        assert_eq!(TextureFormat::BC7Unorm.bytes_per_pixel(), None);
        assert_eq!(TextureFormat::ASTC4x4Unorm.bytes_per_pixel(), None);
    }

    #[test]
    fn test_mip_level_creation() {
        let mip = MipLevel::new(256, 256, 0, 262144);
        assert_eq!(mip.width, 256);
        assert_eq!(mip.height, 256);
        assert_eq!(mip.data_offset, 0);
        assert_eq!(mip.data_size, 262144);
    }

    #[test]
    fn test_texture_data_creation_rgba8() {
        // 2x2 RGBA8 texture = 16 bytes
        let data = vec![0u8; 16];
        let texture = TextureData::new(2, 2, TextureFormat::RGBA8Unorm, data).unwrap();

        assert_eq!(texture.width, 2);
        assert_eq!(texture.height, 2);
        assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
        assert_eq!(texture.mip_count(), 1);
        assert_eq!(texture.memory_size(), 16);
    }

    #[test]
    fn test_texture_data_creation_invalid_size() {
        // 2x2 RGBA8 texture requires 16 bytes, but we provide 10
        let data = vec![0u8; 10];
        let result = TextureData::new(2, 2, TextureFormat::RGBA8Unorm, data);

        assert!(result.is_err());
    }

    #[test]
    fn test_texture_data_query_dimensions() {
        let data = vec![0u8; 64];
        let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, data).unwrap();

        assert_eq!(texture.width, 4);
        assert_eq!(texture.height, 4);
    }

    #[test]
    fn test_texture_data_mip_access() {
        let data = vec![1u8; 16];
        let texture = TextureData::new(2, 2, TextureFormat::RGBA8Unorm, data).unwrap();

        let mip0 = texture.mip_data(0).unwrap();
        assert_eq!(mip0.len(), 16);
        assert_eq!(mip0[0], 1);

        // No mip level 1
        assert!(texture.mip_data(1).is_none());
    }

    #[test]
    fn test_texture_data_memory_size() {
        let data = vec![0u8; 256];
        let texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, data).unwrap();

        assert_eq!(texture.memory_size(), 256);
    }

    #[test]
    fn test_calculate_data_size_uncompressed() {
        // 4x4 RGBA8 = 64 bytes
        let size = TextureData::calculate_data_size(4, 4, TextureFormat::RGBA8Unorm);
        assert_eq!(size, 64);

        // 256x256 RGBA8 = 262144 bytes
        let size = TextureData::calculate_data_size(256, 256, TextureFormat::RGBA8Unorm);
        assert_eq!(size, 262144);
    }

    #[test]
    fn test_calculate_data_size_compressed() {
        // 4x4 BC7 = 1 block = 16 bytes
        let size = TextureData::calculate_data_size(4, 4, TextureFormat::BC7Unorm);
        assert_eq!(size, 16);

        // 8x8 BC7 = 4 blocks (2x2) = 64 bytes
        let size = TextureData::calculate_data_size(8, 8, TextureFormat::BC7Unorm);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_from_image_bytes_png() {
        use image::{ImageBuffer, Rgba};

        // Create a simple 4x4 RGBA image
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(4, 4, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 0, 0, 255]) // Red
            } else {
                Rgba([0, 255, 0, 255]) // Green
            }
        });

        // Encode to PNG
        let mut png_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
            .unwrap();

        // Load back
        let texture = TextureData::from_image_bytes(&png_data).unwrap();
        assert_eq!(texture.width, 4);
        assert_eq!(texture.height, 4);
        assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
        assert_eq!(texture.mip_count(), 1);
        assert_eq!(texture.data.len(), 64); // 4x4 * 4 bytes
    }

    #[test]
    fn test_from_image_bytes_jpg() {
        use image::{ImageBuffer, Rgb};

        // Create a simple 8x8 RGB image
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 32) as u8, (y * 32) as u8, 128]));

        // Encode to JPEG
        let mut jpg_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut jpg_data), image::ImageFormat::Jpeg)
            .unwrap();

        // Load back (JPEG is lossy, so we just check dimensions)
        let texture = TextureData::from_image_bytes(&jpg_data).unwrap();
        assert_eq!(texture.width, 8);
        assert_eq!(texture.height, 8);
        assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
        assert_eq!(texture.mip_count(), 1);
        assert_eq!(texture.data.len(), 256); // 8x8 * 4 bytes (converted to RGBA)
    }

    #[test]
    fn test_generate_mipmaps_power_of_two() {
        // Create 256x256 texture
        let data = vec![128u8; 256 * 256 * 4];
        let texture = TextureData::new(256, 256, TextureFormat::RGBA8Unorm, data).unwrap();

        // Generate mipmaps
        let with_mips = texture.generate_mipmaps().unwrap();

        // Should have 9 mip levels (256 -> 128 -> 64 -> 32 -> 16 -> 8 -> 4 -> 2 -> 1)
        assert_eq!(with_mips.mip_count(), 9);

        // Check first mip is original size
        let mip0 = &with_mips.mip_levels[0];
        assert_eq!(mip0.width, 256);
        assert_eq!(mip0.height, 256);

        // Check last mip is 1x1
        let last_mip = with_mips.mip_levels.last().unwrap();
        assert_eq!(last_mip.width, 1);
        assert_eq!(last_mip.height, 1);
    }

    #[test]
    fn test_generate_mipmaps_non_power_of_two_fails() {
        // Create 100x100 texture (not power of 2)
        let data = vec![128u8; 100 * 100 * 4];
        let texture = TextureData::new(100, 100, TextureFormat::RGBA8Unorm, data).unwrap();

        // Should fail to generate mipmaps
        let result = texture.generate_mipmaps();
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_mipmaps_validates_each_level() {
        // Create 4x4 texture
        let data = vec![200u8; 4 * 4 * 4];
        let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, data).unwrap();

        // Generate mipmaps
        let with_mips = texture.generate_mipmaps().unwrap();

        // Should have 3 mip levels (4x4, 2x2, 1x1)
        assert_eq!(with_mips.mip_count(), 3);

        // Validate each level
        assert_eq!(with_mips.mip_levels[0].width, 4);
        assert_eq!(with_mips.mip_levels[0].height, 4);
        assert_eq!(with_mips.mip_levels[0].data_size, 64);

        assert_eq!(with_mips.mip_levels[1].width, 2);
        assert_eq!(with_mips.mip_levels[1].height, 2);
        assert_eq!(with_mips.mip_levels[1].data_size, 16);

        assert_eq!(with_mips.mip_levels[2].width, 1);
        assert_eq!(with_mips.mip_levels[2].height, 1);
        assert_eq!(with_mips.mip_levels[2].data_size, 4);
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    use crate::validation::{AssetValidator, ValidationError};

    #[test]
    fn test_valid_texture_passes_validation() {
        let data = vec![255u8; 256];
        let texture = TextureData::new(8, 8, TextureFormat::RGBA8Unorm, data).unwrap();
        let report = texture.validate_all();
        assert!(report.is_valid());
    }

    #[test]
    fn test_validate_data_zero_dimensions() {
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
        texture.width = 0;

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { .. }) => {}
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_oversized_dimensions() {
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
        texture.width = 20000; // Exceeds MAX_DIMENSION

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { reason }) => {
                assert!(reason.contains("too large"));
            }
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_no_mip_levels() {
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
        texture.mip_levels.clear();

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::MipmapChainInconsistent { .. }) => {}
            _ => panic!("Expected MipmapChainInconsistent error"),
        }
    }

    #[test]
    fn test_validate_data_base_mip_dimension_mismatch() {
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
        texture.mip_levels[0].width = 4; // Wrong!

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::MipmapChainInconsistent { reason }) => {
                assert!(reason.contains("don't match"));
            }
            _ => panic!("Expected MipmapChainInconsistent error"),
        }
    }

    #[test]
    fn test_validate_data_mip_level_wrong_dimensions() {
        let data = vec![0u8; 256 + 64]; // Base + 1 mip
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();

        // Add a second mip level with wrong dimensions
        use crate::MipLevel;
        texture.mip_levels.push(MipLevel::new(5, 5, 256, 64)); // Should be 4x4
        texture.data = data;

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::MipmapChainInconsistent { reason }) => {
                assert!(reason.contains("wrong dimensions"));
            }
            _ => panic!("Expected MipmapChainInconsistent error"),
        }
    }

    #[test]
    fn test_validate_data_mip_data_out_of_bounds() {
        let mut texture =
            TextureData::new(8, 8, TextureFormat::RGBA8Unorm, vec![0u8; 256]).unwrap();
        texture.mip_levels[0].data_size = 1000; // Way too big

        let result = texture.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::MipmapChainInconsistent { reason }) => {
                assert!(reason.contains("beyond buffer"));
            }
            _ => panic!("Expected MipmapChainInconsistent error"),
        }
    }

    #[test]
    fn test_checksum_validation_passes() {
        let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, vec![128u8; 64]).unwrap();
        let checksum = texture.compute_checksum();
        assert!(texture.validate_checksum(&checksum).is_ok());
    }

    #[test]
    fn test_checksum_validation_fails() {
        let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, vec![128u8; 64]).unwrap();
        let wrong_checksum = [0u8; 32];
        let result = texture.validate_checksum(&wrong_checksum);
        assert!(result.is_err());
        match result {
            Err(ValidationError::ChecksumMismatch { .. }) => {}
            _ => panic!("Expected ChecksumMismatch error"),
        }
    }

    #[test]
    fn test_checksum_deterministic() {
        let texture = TextureData::new(4, 4, TextureFormat::RGBA8Unorm, vec![200u8; 64]).unwrap();
        let hash1 = texture.compute_checksum();
        let hash2 = texture.compute_checksum();
        assert_eq!(hash1, hash2);
    }
}
