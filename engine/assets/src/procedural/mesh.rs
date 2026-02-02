//! Procedural mesh generation
//!
//! Deterministic procedural generation of geometric meshes.

use super::{ProceduralAsset, SeededRng};
use crate::mesh::{MeshData, Vertex};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Parameters for procedural mesh generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProceduralMeshParams {
    /// Generate a cube mesh
    Cube {
        /// Size (side length)
        size: f32,
    },
    /// Generate a UV sphere mesh
    Sphere {
        /// Radius
        radius: f32,
        /// Number of latitude subdivisions
        lat_subdivisions: u32,
        /// Number of longitude subdivisions
        lon_subdivisions: u32,
    },
    /// Generate a terrain mesh using Perlin noise
    Terrain {
        /// Width in world units
        width: f32,
        /// Height in world units
        height: f32,
        /// Width resolution (vertices)
        width_res: u32,
        /// Height resolution (vertices)
        height_res: u32,
        /// Maximum terrain height
        max_height: f32,
        /// Number of noise octaves
        octaves: u32,
        /// Noise frequency
        frequency: f32,
    },
}

impl ProceduralAsset for MeshData {
    type Params = ProceduralMeshParams;

    fn generate(seed: u64, params: &Self::Params) -> Self {
        match params {
            ProceduralMeshParams::Cube { size } => generate_cube(*size),
            ProceduralMeshParams::Sphere { radius, lat_subdivisions, lon_subdivisions } => {
                generate_sphere(*radius, *lat_subdivisions, *lon_subdivisions)
            }
            ProceduralMeshParams::Terrain {
                width,
                height,
                width_res,
                height_res,
                max_height,
                octaves,
                frequency,
            } => generate_terrain(
                seed,
                *width,
                *height,
                *width_res,
                *height_res,
                *max_height,
                *octaves,
                *frequency,
            ),
        }
    }
}

/// Generate a cube mesh with the given size
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::generate_cube;
///
/// let cube = generate_cube(2.0);
/// assert_eq!(cube.vertex_count(), 24); // 6 faces * 4 vertices
/// assert_eq!(cube.triangle_count(), 12); // 6 faces * 2 triangles
/// ```
pub fn generate_cube(size: f32) -> MeshData {
    let half = size / 2.0;

    let vertices = vec![
        // Front face (Z+)
        Vertex::new(Vec3::new(-half, -half, half), Vec3::Z, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, half), Vec3::Z, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::Z, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, half), Vec3::Z, Vec2::new(0.0, 1.0)),
        // Back face (Z-)
        Vertex::new(Vec3::new(half, -half, -half), Vec3::NEG_Z, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_Z, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::NEG_Z, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::NEG_Z, Vec2::new(0.0, 1.0)),
        // Top face (Y+)
        Vertex::new(Vec3::new(-half, half, half), Vec3::Y, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::Y, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::Y, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::Y, Vec2::new(0.0, 1.0)),
        // Bottom face (Y-)
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_Y, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, -half), Vec3::NEG_Y, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, half), Vec3::NEG_Y, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, -half, half), Vec3::NEG_Y, Vec2::new(0.0, 1.0)),
        // Right face (X+)
        Vertex::new(Vec3::new(half, -half, half), Vec3::X, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, -half), Vec3::X, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::X, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::X, Vec2::new(0.0, 1.0)),
        // Left face (X-)
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_X, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(-half, -half, half), Vec3::NEG_X, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(-half, half, half), Vec3::NEG_X, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::NEG_X, Vec2::new(0.0, 1.0)),
    ];

    #[rustfmt::skip]
    let indices = vec![
        // Front
        0, 1, 2, 2, 3, 0,
        // Back
        4, 5, 6, 6, 7, 4,
        // Top
        8, 9, 10, 10, 11, 8,
        // Bottom
        12, 13, 14, 14, 15, 12,
        // Right
        16, 17, 18, 18, 19, 16,
        // Left
        20, 21, 22, 22, 23, 20,
    ];

    MeshData { vertices, indices }
}

/// Generate a UV sphere mesh
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::generate_sphere;
///
/// let sphere = generate_sphere(1.0, 16, 32);
/// assert!(sphere.vertex_count() > 0);
/// ```
pub fn generate_sphere(radius: f32, lat_subdivisions: u32, lon_subdivisions: u32) -> MeshData {
    use std::f32::consts::PI;

    let lat_subdivisions = lat_subdivisions.max(3);
    let lon_subdivisions = lon_subdivisions.max(3);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for lat in 0..=lat_subdivisions {
        let theta = (lat as f32 / lat_subdivisions as f32) * PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_subdivisions {
            let phi = (lon as f32 / lon_subdivisions as f32) * 2.0 * PI;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            // Spherical to Cartesian coordinates
            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            let position = Vec3::new(x, y, z);
            let normal = position.normalize();
            let uv = Vec2::new(
                lon as f32 / lon_subdivisions as f32,
                lat as f32 / lat_subdivisions as f32,
            );

            vertices.push(Vertex::new(position, normal, uv));
        }
    }

    // Generate indices
    for lat in 0..lat_subdivisions {
        for lon in 0..lon_subdivisions {
            let first = lat * (lon_subdivisions + 1) + lon;
            let second = first + lon_subdivisions + 1;

            // Two triangles per quad
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    MeshData { vertices, indices }
}

/// Generate a terrain mesh using Perlin noise
///
/// # Determinism
///
/// This function is fully deterministic - the same seed produces identical terrain
/// on all platforms (Windows, Linux, macOS, ARM, x64).
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::generate_terrain;
///
/// let terrain = generate_terrain(12345, 100.0, 100.0, 64, 64, 10.0, 4, 0.05);
/// assert_eq!(terrain.vertex_count(), 64 * 64);
/// ```
pub fn generate_terrain(
    seed: u64,
    width: f32,
    height: f32,
    width_res: u32,
    height_res: u32,
    max_height: f32,
    octaves: u32,
    frequency: f32,
) -> MeshData {
    let width_res = width_res.max(2);
    let height_res = height_res.max(2);
    let octaves = octaves.max(1).min(8);

    let mut rng = SeededRng::new(seed);

    // Generate octave offsets for Perlin noise (deterministic)
    let mut octave_offsets = Vec::with_capacity(octaves as usize);
    for _ in 0..octaves {
        let offset_x = rng.next_range(-1000.0, 1000.0);
        let offset_y = rng.next_range(-1000.0, 1000.0);
        octave_offsets.push((offset_x, offset_y));
    }

    let mut vertices = Vec::with_capacity((width_res * height_res) as usize);
    let mut indices = Vec::new();

    // Generate height map
    let mut heights = vec![vec![0.0f32; width_res as usize]; height_res as usize];

    for z in 0..height_res {
        for x in 0..width_res {
            let sample_x = (x as f32 / width_res as f32) * width;
            let sample_z = (z as f32 / height_res as f32) * height;

            let mut amplitude = 1.0;
            let mut freq = frequency;
            let mut noise_height = 0.0;

            // Combine octaves
            for octave in 0..octaves {
                let (offset_x, offset_y) = octave_offsets[octave as usize];

                let sample_nx = sample_x * freq + offset_x;
                let sample_nz = sample_z * freq + offset_y;

                // Simple Perlin-like noise (deterministic)
                let noise_value = perlin_noise(&mut rng, sample_nx, sample_nz);
                noise_height += noise_value * amplitude;

                amplitude *= 0.5;
                freq *= 2.0;
            }

            heights[z as usize][x as usize] = noise_height * max_height;
        }
    }

    // Generate vertices with computed normals
    for z in 0..height_res {
        for x in 0..width_res {
            let x_pos = (x as f32 / (width_res - 1) as f32) * width - width / 2.0;
            let z_pos = (z as f32 / (height_res - 1) as f32) * height - height / 2.0;
            let y_pos = heights[z as usize][x as usize];

            // Calculate normal from neighboring heights
            let normal =
                calculate_terrain_normal(&heights, x, z, width_res, height_res, width, height);

            let uv =
                Vec2::new(x as f32 / (width_res - 1) as f32, z as f32 / (height_res - 1) as f32);

            vertices.push(Vertex::new(Vec3::new(x_pos, y_pos, z_pos), normal, uv));
        }
    }

    // Generate indices
    for z in 0..(height_res - 1) {
        for x in 0..(width_res - 1) {
            let top_left = z * width_res + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * width_res + x;
            let bottom_right = bottom_left + 1;

            // Two triangles per quad
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    MeshData { vertices, indices }
}

/// Simple Perlin-like noise function (deterministic)
///
/// This is a simplified Perlin noise implementation that is fully deterministic
/// and doesn't rely on precomputed permutation tables.
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

/// Calculate terrain normal from neighboring heights
fn calculate_terrain_normal(
    heights: &[Vec<f32>],
    x: u32,
    z: u32,
    width_res: u32,
    height_res: u32,
    width: f32,
    height: f32,
) -> Vec3 {
    let x = x as usize;
    let z = z as usize;

    // Get neighboring heights (with bounds checking)
    let left = if x > 0 { heights[z][x - 1] } else { heights[z][x] };
    let right = if x < width_res as usize - 1 { heights[z][x + 1] } else { heights[z][x] };

    let down = if z > 0 { heights[z - 1][x] } else { heights[z][x] };
    let up = if z < height_res as usize - 1 { heights[z + 1][x] } else { heights[z][x] };

    // Calculate normal using finite differences
    let dx = (right - left) / (2.0 * width / width_res as f32);
    let dz = (up - down) / (2.0 * height / height_res as f32);

    Vec3::new(-dx, 1.0, -dz).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cube() {
        let cube = generate_cube(2.0);
        assert_eq!(cube.vertex_count(), 24);
        assert_eq!(cube.triangle_count(), 12);
    }

    #[test]
    fn test_generate_cube_size() {
        let cube = generate_cube(4.0);
        let (min, max) = cube.bounding_box();

        assert_eq!(min, Vec3::new(-2.0, -2.0, -2.0));
        assert_eq!(max, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_generate_sphere() {
        let sphere = generate_sphere(1.0, 16, 32);
        assert!(sphere.vertex_count() > 0);
        assert!(sphere.triangle_count() > 0);
    }

    #[test]
    fn test_generate_sphere_radius() {
        let sphere = generate_sphere(5.0, 16, 32);
        let (min, max) = sphere.bounding_box();

        // Bounding box should be approximately [-5, -5, -5] to [5, 5, 5]
        assert!((min.length() - 5.0).abs() < 0.5);
        assert!((max.length() - 5.0).abs() < 0.5);
    }

    #[test]
    fn test_generate_terrain_determinism() {
        let seed = 12345;
        let terrain1 = generate_terrain(seed, 100.0, 100.0, 32, 32, 10.0, 4, 0.05);
        let terrain2 = generate_terrain(seed, 100.0, 100.0, 32, 32, 10.0, 4, 0.05);

        // Same seed should produce identical terrain
        assert_eq!(terrain1.vertex_count(), terrain2.vertex_count());
        assert_eq!(terrain1.index_count(), terrain2.index_count());

        for (v1, v2) in terrain1.vertices.iter().zip(terrain2.vertices.iter()) {
            assert_eq!(v1.position, v2.position);
            assert_eq!(v1.normal, v2.normal);
            assert_eq!(v1.uv, v2.uv);
        }

        for (i1, i2) in terrain1.indices.iter().zip(terrain2.indices.iter()) {
            assert_eq!(i1, i2);
        }
    }

    #[test]
    fn test_generate_terrain_different_seeds() {
        let terrain1 = generate_terrain(111, 100.0, 100.0, 32, 32, 10.0, 4, 0.05);
        let terrain2 = generate_terrain(222, 100.0, 100.0, 32, 32, 10.0, 4, 0.05);

        // Different seeds should produce different terrain
        let mut different_positions = false;
        for (v1, v2) in terrain1.vertices.iter().zip(terrain2.vertices.iter()) {
            if v1.position != v2.position {
                different_positions = true;
                break;
            }
        }

        assert!(different_positions, "Different seeds should produce different terrain");
    }

    #[test]
    fn test_generate_terrain_vertex_count() {
        let terrain = generate_terrain(12345, 100.0, 100.0, 64, 48, 10.0, 4, 0.05);
        assert_eq!(terrain.vertex_count(), 64 * 48);
    }

    #[test]
    fn test_generate_terrain_triangle_count() {
        let width_res = 32u32;
        let height_res = 24u32;
        let terrain = generate_terrain(12345, 100.0, 100.0, width_res, height_res, 10.0, 4, 0.05);

        // Each quad = 2 triangles
        let expected_triangles = (width_res - 1) * (height_res - 1) * 2;
        assert_eq!(terrain.triangle_count(), expected_triangles as usize);
    }

    #[test]
    fn test_procedural_asset_trait_cube() {
        let params = ProceduralMeshParams::Cube { size: 3.0 };
        let mesh = MeshData::generate(12345, &params);

        assert_eq!(mesh.vertex_count(), 24);
    }

    #[test]
    fn test_procedural_asset_trait_sphere() {
        let params = ProceduralMeshParams::Sphere {
            radius: 2.0,
            lat_subdivisions: 16,
            lon_subdivisions: 32,
        };
        let mesh = MeshData::generate(12345, &params);

        assert!(mesh.vertex_count() > 0);
    }

    #[test]
    fn test_procedural_asset_trait_terrain() {
        let params = ProceduralMeshParams::Terrain {
            width: 50.0,
            height: 50.0,
            width_res: 32,
            height_res: 32,
            max_height: 5.0,
            octaves: 3,
            frequency: 0.1,
        };
        let mesh = MeshData::generate(54321, &params);

        assert_eq!(mesh.vertex_count(), 32 * 32);
    }

    #[test]
    fn test_params_serialization() {
        let params = ProceduralMeshParams::Terrain {
            width: 100.0,
            height: 100.0,
            width_res: 64,
            height_res: 64,
            max_height: 10.0,
            octaves: 4,
            frequency: 0.05,
        };

        let serialized = bincode::serialize(&params).unwrap();
        let deserialized: ProceduralMeshParams = bincode::deserialize(&serialized).unwrap();

        assert_eq!(params, deserialized);
    }
}
