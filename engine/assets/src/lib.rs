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
pub mod bundle;
pub mod font;
#[cfg(feature = "hot-reload")]
pub mod hot_reload;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod material;
pub mod memory;
pub mod mesh;
pub mod network;
// Temporarily disabled - module not present
// pub mod procedural;
pub mod shader;
pub mod texture;

pub use asset_id::AssetId;
#[cfg(feature = "async")]
pub use async_loader::{AsyncLoadHandle, AsyncLoader, LoadPriority, LoadStatus};
pub use audio::{AudioData, AudioError, AudioFormat};
pub use bundle::{AssetBundle, BundleError, BundleStats, CompressionFormat};
pub use font::{FontData, FontError, FontMetrics, FontStyle, FontWeight};
pub use handle::{AssetHandle, RefType};
#[cfg(feature = "hot-reload")]
pub use hot_reload::{HotReloadConfig, HotReloadEvent, HotReloadStats, HotReloader};
#[cfg(feature = "async")]
pub use loader::StreamingHandle;
pub use loader::{EnhancedLoader, LoadStrategy};
pub use manager::{AssetError, AssetLoader, AssetManager, AssetType};
pub use manifest::{AssetEntry, AssetManifest, ManifestError};
pub use material::{MaterialData, MaterialError};
pub use memory::{LruCache, MemoryBudget, MemorySized, MemoryStats};
pub use mesh::{MeshData, MeshError, Vertex};
pub use network::{
    AssetNetworkClient, AssetNetworkMessage, AssetNetworkServer, TransferPriority, TransferStatus,
};
// pub use procedural::{
//     ProceduralAssetGenerator, ProceduralAudioGenerator, ProceduralAudioParams,
//     ProceduralMeshGenerator, ProceduralMeshParams, ProceduralTextureGenerator,
//     ProceduralTextureParams,
// };
pub use registry::AssetRegistry;
pub use shader::{ShaderData, ShaderError, ShaderSource, ShaderStage};
pub use texture::{MipLevel, TextureData, TextureFormat};
pub use validation::{AssetValidator, ValidationError, ValidationReport, ValidationWarning};
