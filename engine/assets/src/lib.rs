//! Asset data structures and loaders
//!
//! Pure data structures for game assets (meshes, textures, materials).
//! No rendering or GPU dependencies - can be used by server, tools, or client.

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]

mod asset_id;
mod handle;
mod registry;
mod validation;

pub mod audio;
pub mod font;
pub mod material;
pub mod mesh;
// Temporarily disabled due to compilation errors - pending fixes
// pub mod procedural;
pub mod shader;
pub mod texture;

pub use asset_id::AssetId;
pub use audio::{AudioData, AudioError, AudioFormat};
pub use font::{FontData, FontError, FontMetrics, FontStyle, FontWeight};
pub use handle::{AssetHandle, RefType};
pub use material::{MaterialData, MaterialError};
pub use mesh::{MeshData, MeshError, Vertex};
// pub use procedural::{ProceduralAsset, ProceduralMeshParams, ProceduralTextureParams, SeededRng};
pub use registry::AssetRegistry;
pub use shader::{ShaderData, ShaderError, ShaderSource, ShaderStage};
pub use texture::{MipLevel, TextureData, TextureFormat};
pub use validation::{AssetValidator, ValidationError, ValidationReport, ValidationWarning};
