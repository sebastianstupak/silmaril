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

#[cfg(feature = "async")]
pub mod async_loader;
pub mod audio;
pub mod font;
#[cfg(feature = "hot-reload")]
pub mod hot_reload;
pub mod manager;
pub mod material;
pub mod memory;
pub mod mesh;
// Temporarily disabled due to compilation errors - pending fixes
// pub mod procedural;
pub mod shader;
pub mod texture;

pub use asset_id::AssetId;
#[cfg(feature = "async")]
pub use async_loader::{AsyncLoadHandle, AsyncLoader, LoadPriority, LoadStatus};
pub use audio::{AudioData, AudioError, AudioFormat};
pub use font::{FontData, FontError, FontMetrics, FontStyle, FontWeight};
pub use handle::{AssetHandle, RefType};
#[cfg(feature = "hot-reload")]
pub use hot_reload::{HotReloadEvent, HotReloader};
pub use manager::{AssetError, AssetLoader, AssetManager, AssetType};
pub use material::{MaterialData, MaterialError};
pub use memory::{LruCache, MemoryBudget, MemoryStats};
pub use mesh::{MeshData, MeshError, Vertex};
// pub use procedural::{ProceduralAsset, ProceduralMeshParams, ProceduralTextureParams, SeededRng};
pub use registry::AssetRegistry;
pub use shader::{ShaderData, ShaderError, ShaderSource, ShaderStage};
pub use texture::{MipLevel, TextureData, TextureFormat};
pub use validation::{AssetValidator, ValidationError, ValidationReport, ValidationWarning};
