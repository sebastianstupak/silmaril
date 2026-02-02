//! Procedural texture generation
//!
//! Deterministic procedural generation of textures.

use super::{ProceduralAsset, SeededRng};
use crate::texture::{TextureData, TextureFormat};
use serde::{Deserialize, Serialize};

/// Parameters for procedural texture generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProceduralTextureParams {
    /// Generate Perlin noise texture
    Noise {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
        /// Number of noise octaves
        octaves: u32,
        /// Noise frequency
        frequency: f32,
    },
    /// Generate checkerboard pattern
    Checkerboard {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
        /// Square size in pixels
        square_size: u32,
        /// First color (RGBA)
        color1: [u8; 4],
        /// Second color (RGBA)
        color2: [u8; 4],
    },
}

impl ProceduralAsset for TextureData {
    type Params = ProceduralTextureParams;

    fn generate(seed: u64, params: &Self::Params) -> Self {
        match params {
            ProceduralTextureParams::Noise { width, height, octaves, frequency } => {
                generate_noise(seed, *width, *height, *octaves, *frequency)
            }
            ProceduralTextureParams::Checkerboard {
                width,
                height,
                square_size,
                color1,
                color2,
            } => generate_checkerboard(*width, *height, *square_size, *color1, *color2),
        }
    }
}

/// Generate a noise texture using Perlin noise
///
/// # Determinism
///
/// This function is fully deterministic - the same seed produces identical textures
/// on all platforms (Windows, Linux, macOS, ARM, x64).
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::generate_noise;
///
/// let texture = generate_noise(12345, 256, 256, 4, 0.05);
/// assert_eq!(texture.width, 256);
/// assert_eq!(texture.height, 256);
/// ```
pub fn generate_noise(
    seed: u64,
    width: u32,
    height: u32,
    octaves: u32,
    frequency: f32,
) -> TextureData {
    let width = width.max(1);
    let height = height.max(1);
    let octaves = octaves.max(1).min(8);

    let mut rng = SeededRng::new(seed);

    // Generate octave offsets (deterministic)
    let mut octave_offsets = Vec::with_capacity(octaves as usize);
    for _ in 0..octaves {
        let offset_x = rng.next_range(-1000.0, 1000.0);
        let offset_y = rng.next_range(-1000.0, 1000.0);
        octave_offsets.push((offset_x, offset_y));
    }

    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let sample_x = (x as f32 / width as f32) * width as f32;
            let sample_y = (y as f32 / height as f32) * height as f32;

            let mut amplitude = 1.0;
            let mut freq = frequency;
            let mut noise_value = 0.0;

            // Combine octaves
            for octave in 0..octaves {
                let (offset_x, offset_y) = octave_offsets[octave as usize];

                let sample_nx = sample_x * freq + offset_x;
                let sample_ny = sample_y * freq + offset_y;

                // Simple Perlin-like noise
                let value = perlin_noise(&mut rng, sample_nx, sample_ny);
                noise_value += value * amplitude;

                amplitude *= 0.5;
                freq *= 2.0;
            }

            // Normalize to [0, 1] and convert to grayscale RGBA
            let normalized = ((noise_value + 1.0) / 2.0).clamp(0.0, 1.0);
            let gray = (normalized * 255.0) as u8;

            data.push(gray); // R
            data.push(gray); // G
            data.push(gray); // B
            data.push(255); // A
        }
    }

    TextureData::new(width, height, TextureFormat::RGBA8Unorm, data)
        .expect("Failed to create noise texture")
}

/// Generate a checkerboard texture
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::generate_checkerboard;
///
/// let texture = generate_checkerboard(256, 256, 32, [255, 0, 0, 255], [0, 255, 0, 255]);
/// assert_eq!(texture.width, 256);
/// assert_eq!(texture.height, 256);
/// ```
pub fn generate_checkerboard(
    width: u32,
    height: u32,
    square_size: u32,
    color1: [u8; 4],
    color2: [u8; 4],
) -> TextureData {
    let width = width.max(1);
    let height = height.max(1);
    let square_size = square_size.max(1);

    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Determine which square we're in
            let square_x = x / square_size;
            let square_y = y / square_size;

            // Checkerboard pattern
            let color = if (square_x + square_y) % 2 == 0 { color1 } else { color2 };

            data.extend_from_slice(&color);
        }
    }

    TextureData::new(width, height, TextureFormat::RGBA8Unorm, data)
        .expect("Failed to create checkerboard texture")
}

/// Simple Perlin-like noise function (deterministic)
fn perlin_noise(rng: &mut SeededRng, x: f32, y: f32) -> f32 {
    // Get integer and fractional parts
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    // Smooth interpolation (fade function)
    let u = fade(fx);
    let v = fade(fy);

    // Hash coordinates to get gradients (deterministic)
    let aa = hash_coords(rng, x0, y0);
    let ab = hash_coords(rng, x0, y0 + 1);
    let ba = hash_coords(rng, x0 + 1, y0);
    let bb = hash_coords(rng, x0 + 1, y0 + 1);

    // Interpolate
    let x1 = lerp(grad(aa, fx, fy), grad(ba, fx - 1.0, fy), u);
    let x2 = lerp(grad(ab, fx, fy - 1.0), grad(bb, fx - 1.0, fy - 1.0), u);

    lerp(x1, x2, v)
}

/// Fade function for smooth interpolation (6t^5 - 15t^4 + 10t^3)
#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Hash coordinates to get a deterministic gradient index
fn hash_coords(rng: &mut SeededRng, x: i32, y: i32) -> u8 {
    // Use coordinates to seed a temporary RNG
    let hash_seed = ((x as u64).wrapping_mul(374_761_393))
        .wrapping_add((y as u64).wrapping_mul(668_265_263))
        .wrapping_add(rng.next_u64());

    let mut temp_rng = SeededRng::new(hash_seed);
    (temp_rng.next_u32() & 0xFF) as u8
}

/// Gradient function for Perlin noise
fn grad(hash: u8, x: f32, y: f32) -> f32 {
    // Use hash to select gradient direction
    let h = hash & 3;
    match h {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_noise_dimensions() {
        let texture = generate_noise(12345, 256, 128, 4, 0.05);
        assert_eq!(texture.width, 256);
        assert_eq!(texture.height, 128);
        assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
    }

    #[test]
    fn test_generate_noise_determinism() {
        let seed = 54321;
        let texture1 = generate_noise(seed, 128, 128, 4, 0.05);
        let texture2 = generate_noise(seed, 128, 128, 4, 0.05);

        // Same seed should produce identical textures
        assert_eq!(texture1.data, texture2.data);
    }

    #[test]
    fn test_generate_noise_different_seeds() {
        let texture1 = generate_noise(111, 128, 128, 4, 0.05);
        let texture2 = generate_noise(222, 128, 128, 4, 0.05);

        // Different seeds should produce different textures
        assert_ne!(texture1.data, texture2.data);
    }

    #[test]
    fn test_generate_noise_data_size() {
        let width = 64;
        let height = 64;
        let texture = generate_noise(12345, width, height, 4, 0.05);

        // RGBA8 = 4 bytes per pixel
        assert_eq!(texture.data.len(), (width * height * 4) as usize);
    }

    #[test]
    fn test_generate_checkerboard_dimensions() {
        let texture = generate_checkerboard(256, 256, 32, [255, 0, 0, 255], [0, 255, 0, 255]);
        assert_eq!(texture.width, 256);
        assert_eq!(texture.height, 256);
        assert_eq!(texture.format, TextureFormat::RGBA8Unorm);
    }

    #[test]
    fn test_generate_checkerboard_pattern() {
        let width = 4;
        let height = 4;
        let square_size = 2;
        let red = [255, 0, 0, 255];
        let green = [0, 255, 0, 255];

        let texture = generate_checkerboard(width, height, square_size, red, green);

        // Check pattern (top-left should be red)
        assert_eq!(&texture.data[0..4], &red);

        // Top-right (2 pixels over) should be green
        assert_eq!(&texture.data[8..12], &green);

        // Bottom-left (2 rows down) should be green
        assert_eq!(&texture.data[32..36], &green);

        // Bottom-right should be red
        assert_eq!(&texture.data[40..44], &red);
    }

    #[test]
    fn test_generate_checkerboard_data_size() {
        let width = 64;
        let height = 64;
        let texture = generate_checkerboard(width, height, 8, [255, 255, 255, 255], [0, 0, 0, 255]);

        // RGBA8 = 4 bytes per pixel
        assert_eq!(texture.data.len(), (width * height * 4) as usize);
    }

    #[test]
    fn test_generate_checkerboard_single_pixel() {
        let texture = generate_checkerboard(1, 1, 1, [255, 0, 0, 255], [0, 255, 0, 255]);
        assert_eq!(texture.width, 1);
        assert_eq!(texture.height, 1);
        assert_eq!(texture.data.len(), 4);
    }

    #[test]
    fn test_procedural_asset_trait_noise() {
        let params =
            ProceduralTextureParams::Noise { width: 128, height: 128, octaves: 4, frequency: 0.05 };
        let texture = TextureData::generate(12345, &params);

        assert_eq!(texture.width, 128);
        assert_eq!(texture.height, 128);
    }

    #[test]
    fn test_procedural_asset_trait_checkerboard() {
        let params = ProceduralTextureParams::Checkerboard {
            width: 256,
            height: 256,
            square_size: 32,
            color1: [255, 0, 0, 255],
            color2: [0, 0, 255, 255],
        };
        let texture = TextureData::generate(12345, &params);

        assert_eq!(texture.width, 256);
        assert_eq!(texture.height, 256);
    }

    #[test]
    fn test_texture_params_serialization() {
        let params =
            ProceduralTextureParams::Noise { width: 512, height: 512, octaves: 6, frequency: 0.03 };

        let serialized = bincode::serialize(&params).unwrap();
        let deserialized: ProceduralTextureParams = bincode::deserialize(&serialized).unwrap();

        assert_eq!(params, deserialized);
    }

    #[test]
    fn test_checkerboard_params_serialization() {
        let params = ProceduralTextureParams::Checkerboard {
            width: 1024,
            height: 1024,
            square_size: 64,
            color1: [128, 64, 32, 255],
            color2: [32, 64, 128, 255],
        };

        let serialized = bincode::serialize(&params).unwrap();
        let deserialized: ProceduralTextureParams = bincode::deserialize(&serialized).unwrap();

        assert_eq!(params, deserialized);
    }

    #[test]
    fn test_noise_octaves_parameter() {
        // Test different octave counts produce different results
        let texture1 = generate_noise(12345, 128, 128, 1, 0.05);
        let texture4 = generate_noise(12345, 128, 128, 4, 0.05);

        assert_ne!(texture1.data, texture4.data);
    }

    #[test]
    fn test_noise_frequency_parameter() {
        // Test different frequencies produce different results
        let texture1 = generate_noise(12345, 128, 128, 4, 0.01);
        let texture2 = generate_noise(12345, 128, 128, 4, 0.1);

        assert_ne!(texture1.data, texture2.data);
    }
}
