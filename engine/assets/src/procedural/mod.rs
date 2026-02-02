//! Procedural asset generation with deterministic RNG
//!
//! This module provides deterministic procedural generation for meshes and textures.
//! All generation is cross-platform deterministic - the same seed produces identical
//! output on Windows, Linux, and macOS.

use serde::{Deserialize, Serialize};

pub mod mesh;
pub mod rng;
pub mod texture;

pub use mesh::{generate_cube, generate_sphere, generate_terrain, ProceduralMeshParams};
pub use rng::SeededRng;
pub use texture::{generate_checkerboard, generate_noise, ProceduralTextureParams};

/// Trait for procedurally generated assets
///
/// All implementors must guarantee determinism:
/// - Same seed + params = identical output on all platforms
/// - Must not use platform-specific RNG (std::rand)
/// - Must use SeededRng for all randomness
pub trait ProceduralAsset: Sized + Serialize + for<'de> Deserialize<'de> {
    /// Parameters type for generation
    type Params: Serialize + for<'de> Deserialize<'de>;

    /// Generate asset from seed and parameters
    ///
    /// # Determinism Guarantee
    ///
    /// This method MUST produce identical output for the same seed and params
    /// across all platforms (Windows, Linux, macOS, ARM, x64).
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::{ProceduralAsset, ProceduralMeshParams};
    /// use engine_assets::MeshData;
    ///
    /// let params = ProceduralMeshParams::Cube { size: 2.0 };
    /// let mesh1 = MeshData::generate(12345, &params);
    /// let mesh2 = MeshData::generate(12345, &params);
    ///
    /// // Same seed + params = identical output
    /// assert_eq!(mesh1.vertex_count(), mesh2.vertex_count());
    /// ```
    fn generate(seed: u64, params: &Self::Params) -> Self;
}
