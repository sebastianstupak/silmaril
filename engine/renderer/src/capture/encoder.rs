//! Image encoding for frame capture.
//!
//! Handles encoding raw pixel data to PNG/JPEG formats.

use crate::RendererError;
use image::{ImageBuffer, ImageFormat, Rgba};
use tracing::info;

/// Image format for saving
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFormat {
    /// PNG format (lossless, larger files)
    Png,
    /// JPEG format with quality setting (lossy, smaller files)
    Jpeg {
        /// JPEG quality (0-100, higher = better quality)
        quality: u8,
    },
}

/// Frame encoder - converts raw RGBA data to image formats
pub struct FrameEncoder;

impl FrameEncoder {
    /// Encode raw RGBA data to PNG
    ///
    /// # Performance
    /// - Target: < 3ms for 1920x1080
    pub fn encode_png(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, RendererError> {
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, data.to_vec())
            .ok_or_else(|| {
                RendererError::imagecreationfailed(
                    width,
                    height,
                    "Failed to create image buffer from raw data".to_string(),
                )
            })?;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        img.write_to(&mut cursor, ImageFormat::Png).map_err(|e| {
            RendererError::imagecreationfailed(
                width,
                height,
                format!("PNG encoding failed: {:?}", e),
            )
        })?;

        Ok(output)
    }

    /// Encode raw RGBA data to JPEG
    ///
    /// # Performance
    /// - Target: < 2ms for 1920x1080
    ///
    /// # Note
    /// JPEG doesn't support alpha channel - alpha is discarded.
    pub fn encode_jpeg(
        data: &[u8],
        width: u32,
        height: u32,
        quality: u8,
    ) -> Result<Vec<u8>, RendererError> {
        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let rgb_data: Vec<u8> = data.chunks(4).flat_map(|rgba| &rgba[0..3]).copied().collect();

        let img: ImageBuffer<image::Rgb<u8>, _> = ImageBuffer::from_raw(width, height, rgb_data)
            .ok_or_else(|| {
                RendererError::imagecreationfailed(
                    width,
                    height,
                    "Failed to create RGB image buffer".to_string(),
                )
            })?;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
        img.write_with_encoder(encoder).map_err(|e| {
            RendererError::imagecreationfailed(
                width,
                height,
                format!("JPEG encoding failed: {:?}", e),
            )
        })?;

        Ok(output)
    }

    /// Save to file
    ///
    /// Encodes and writes image to disk.
    pub fn save_to_file(
        data: &[u8],
        width: u32,
        height: u32,
        path: &std::path::Path,
        format: CaptureFormat,
    ) -> Result<(), RendererError> {
        let encoded = match format {
            CaptureFormat::Png => Self::encode_png(data, width, height)?,
            CaptureFormat::Jpeg { quality } => Self::encode_jpeg(data, width, height, quality)?,
        };

        let size_kb = encoded.len() / 1024;

        std::fs::write(path, encoded).map_err(|e| {
            RendererError::imagecreationfailed(
                width,
                height,
                format!("Failed to write file {}: {:?}", path.display(), e),
            )
        })?;

        info!(
            path = %path.display(),
            format = ?format,
            size_kb = size_kb,
            "Frame saved"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_png_empty() {
        // 1x1 black pixel
        let data = vec![0u8; 4]; // RGBA
        let result = FrameEncoder::encode_png(&data, 1, 1);
        assert!(result.is_ok());

        let png_data = result.unwrap();
        // PNG should start with signature
        assert_eq!(&png_data[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_encode_jpeg_quality() {
        // 2x2 white pixels
        let data = vec![255u8; 16]; // RGBA * 4 pixels
        let result = FrameEncoder::encode_jpeg(&data, 2, 2, 90);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capture_format_eq() {
        assert_eq!(CaptureFormat::Png, CaptureFormat::Png);
        assert_eq!(CaptureFormat::Jpeg { quality: 90 }, CaptureFormat::Jpeg { quality: 90 });
        assert_ne!(CaptureFormat::Jpeg { quality: 90 }, CaptureFormat::Jpeg { quality: 80 });
    }
}
