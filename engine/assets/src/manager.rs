//! Central asset management system.
//!
//! The AssetManager orchestrates all asset loading, unloading, and lifecycle management.
//! It provides sync/async loading, hot-reload, memory management, and thread-safe access.

use crate::{
    AssetHandle, AssetId, AssetRegistry, AudioData, FontData, MaterialData, MeshData, RefType,
    ShaderData, ShaderSource, TextureData,
};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

define_error! {
    pub enum AssetError {
        NotFound { path: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        LoadFailed { path: String, reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        ValidationFailed { path: String, reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        IoError { path: String, error: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        TypeMismatch { expected: String, actual: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

/// Asset type enumeration for runtime identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    /// Mesh asset
    Mesh,
    /// Texture asset
    Texture,
    /// Shader asset
    Shader,
    /// Material asset
    Material,
    /// Audio asset
    Audio,
    /// Font asset
    Font,
}

impl AssetType {
    /// Get the asset type from a file extension.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "obj" | "gltf" | "glb" | "fbx" => Some(Self::Mesh),
            "png" | "jpg" | "jpeg" | "dds" | "ktx2" => Some(Self::Texture),
            "glsl" | "vert" | "frag" | "comp" | "spv" => Some(Self::Shader),
            "mtl" | "mat" => Some(Self::Material),
            "wav" | "ogg" | "mp3" => Some(Self::Audio),
            "ttf" | "otf" => Some(Self::Font),
            _ => None,
        }
    }
}

/// Central asset management system.
///
/// The AssetManager is the main entry point for all asset operations.
/// It coordinates loading, caching, hot-reload, and memory management.
///
/// # Examples
///
/// ```no_run
/// use engine_assets::{AssetManager, MeshData};
/// use std::path::Path;
///
/// let manager = AssetManager::new();
///
/// // Synchronous loading
/// let mesh_handle = manager.load_sync::<MeshData>(Path::new("assets/cube.obj")).unwrap();
///
/// // Access the asset
/// if let Some(mesh) = manager.get_mesh(mesh_handle.id()) {
///     println!("Loaded mesh with {} vertices", mesh.vertices.len());
/// }
/// ```
pub struct AssetManager {
    meshes: Arc<AssetRegistry<MeshData>>,
    textures: Arc<AssetRegistry<TextureData>>,
    shaders: Arc<AssetRegistry<ShaderData>>,
    materials: Arc<AssetRegistry<MaterialData>>,
    audio: Arc<AssetRegistry<AudioData>>,
    fonts: Arc<AssetRegistry<FontData>>,
    // Path -> AssetId mapping for hot-reload
    path_to_id: Arc<RwLock<std::collections::HashMap<PathBuf, (AssetId, AssetType)>>>,
}

impl AssetManager {
    /// Create a new asset manager.
    #[must_use]
    pub fn new() -> Self {
        info!("Initializing AssetManager");
        Self {
            meshes: Arc::new(AssetRegistry::new()),
            textures: Arc::new(AssetRegistry::new()),
            shaders: Arc::new(AssetRegistry::new()),
            materials: Arc::new(AssetRegistry::new()),
            audio: Arc::new(AssetRegistry::new()),
            fonts: Arc::new(AssetRegistry::new()),
            path_to_id: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Load a mesh asset synchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[instrument(skip(self))]
    pub fn load_sync<T: AssetLoader>(
        &self,
        path: &Path,
    ) -> Result<AssetHandle<T::Asset>, AssetError>
    where
        T::Asset: Send + Sync + 'static,
    {
        debug!(path = ?path, "Loading asset synchronously");

        // Check if already loaded
        if let Some((id, _)) = self.path_to_id.read().get(path) {
            debug!(path = ?path, id = ?id, "Asset already loaded, returning existing handle");
            return Ok(AssetHandle::new(*id, RefType::Hard));
        }

        // Load the asset
        let asset = T::load(path).map_err(|e| {
            error!(path = ?path, error = ?e, "Failed to load asset");
            AssetError::loadfailed(path.display().to_string(), e.to_string())
        })?;

        // Generate ID from content
        let id = T::generate_id(&asset);

        // Insert into registry
        let handle = T::insert(self, id, asset)?;

        // Track path -> ID mapping
        self.path_to_id.write().insert(path.to_path_buf(), (id, T::asset_type()));

        info!(path = ?path, id = ?id, "Asset loaded successfully");
        Ok(handle)
    }

    /// Load an asset asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[cfg(feature = "async")]
    #[allow(dead_code)] // May not be used if async feature is disabled
    pub async fn load_async<T: AssetLoader>(
        &self,
        path: &Path,
    ) -> Result<AssetHandle<T::Asset>, AssetError>
    where
        T::Asset: Send + Sync + 'static,
    {
        debug!(path = ?path, "Loading asset asynchronously");

        // Check if already loaded
        if let Some((id, _)) = self.path_to_id.read().get(path) {
            debug!(path = ?path, id = ?id, "Asset already loaded, returning existing handle");
            return Ok(AssetHandle::new(*id, RefType::Hard));
        }

        // Read file asynchronously
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| AssetError::ioerror(path.display().to_string(), e.to_string()))?;

        // Parse in background thread (CPU-bound operation)
        let path_clone = path.to_path_buf();
        let path_str = path.display().to_string();
        let asset = tokio::task::spawn_blocking(move || T::parse(&data).map_err(|e| e.to_string()))
            .await
            .map_err(|e| AssetError::loadfailed(path_clone.display().to_string(), e.to_string()))?
            .map_err(|e| AssetError::loadfailed(path_str, e))?;

        // Generate ID from content
        let id = T::generate_id(&asset);

        // Insert into registry
        let handle = T::insert(self, id, asset)?;

        // Track path -> ID mapping
        self.path_to_id.write().insert(path.to_path_buf(), (id, T::asset_type()));

        info!(path = ?path, id = ?id, "Asset loaded asynchronously");
        Ok(handle)
    }

    /// Get a mesh by ID.
    #[must_use]
    pub fn get_mesh(&self, id: AssetId) -> Option<impl std::ops::Deref<Target = MeshData> + '_> {
        self.meshes.get(id)
    }

    /// Get a texture by ID.
    #[must_use]
    pub fn get_texture(
        &self,
        id: AssetId,
    ) -> Option<impl std::ops::Deref<Target = TextureData> + '_> {
        self.textures.get(id)
    }

    /// Get a shader by ID.
    #[must_use]
    pub fn get_shader(
        &self,
        id: AssetId,
    ) -> Option<impl std::ops::Deref<Target = ShaderData> + '_> {
        self.shaders.get(id)
    }

    /// Get a material by ID.
    #[must_use]
    pub fn get_material(
        &self,
        id: AssetId,
    ) -> Option<impl std::ops::Deref<Target = MaterialData> + '_> {
        self.materials.get(id)
    }

    /// Get audio by ID.
    #[must_use]
    pub fn get_audio(&self, id: AssetId) -> Option<impl std::ops::Deref<Target = AudioData> + '_> {
        self.audio.get(id)
    }

    /// Get a font by ID.
    #[must_use]
    pub fn get_font(&self, id: AssetId) -> Option<impl std::ops::Deref<Target = FontData> + '_> {
        self.fonts.get(id)
    }

    /// Remove an asset by path.
    pub fn unload(&self, path: &Path) -> bool {
        if let Some((id, asset_type)) = self.path_to_id.write().remove(path) {
            match asset_type {
                AssetType::Mesh => self.meshes.remove(id).is_some(),
                AssetType::Texture => self.textures.remove(id).is_some(),
                AssetType::Shader => self.shaders.remove(id).is_some(),
                AssetType::Material => self.materials.remove(id).is_some(),
                AssetType::Audio => self.audio.remove(id).is_some(),
                AssetType::Font => self.fonts.remove(id).is_some(),
            }
        } else {
            false
        }
    }

    /// Get the total number of loaded assets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.meshes.len()
            + self.textures.len()
            + self.shaders.len()
            + self.materials.len()
            + self.audio.len()
            + self.fonts.len()
    }

    /// Check if the manager has no loaded assets.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all assets.
    pub fn clear(&self) {
        info!("Clearing all assets");
        self.meshes.clear();
        self.textures.clear();
        self.shaders.clear();
        self.materials.clear();
        self.audio.clear();
        self.fonts.clear();
        self.path_to_id.write().clear();
    }

    /// Get a reference to the mesh registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn meshes(&self) -> &Arc<AssetRegistry<MeshData>> {
        &self.meshes
    }

    /// Get a reference to the texture registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn textures(&self) -> &Arc<AssetRegistry<TextureData>> {
        &self.textures
    }

    /// Get a reference to the shader registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn shaders(&self) -> &Arc<AssetRegistry<ShaderData>> {
        &self.shaders
    }

    /// Get a reference to the material registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn materials(&self) -> &Arc<AssetRegistry<MaterialData>> {
        &self.materials
    }

    /// Get a reference to the audio registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn audio_registry(&self) -> &Arc<AssetRegistry<AudioData>> {
        &self.audio
    }

    /// Get a reference to the font registry (for testing and advanced use).
    #[doc(hidden)]
    pub fn fonts_registry(&self) -> &Arc<AssetRegistry<FontData>> {
        &self.fonts
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for asset loaders.
pub trait AssetLoader {
    /// The asset type produced by this loader.
    type Asset;

    /// Load an asset from a file path synchronously.
    fn load(path: &Path) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>>;

    /// Parse asset data from bytes (for async loading).
    fn parse(data: &[u8]) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>>;

    /// Generate a content-addressable ID for the asset.
    fn generate_id(asset: &Self::Asset) -> AssetId;

    /// Insert the asset into the manager's registry.
    fn insert(
        manager: &AssetManager,
        id: AssetId,
        asset: Self::Asset,
    ) -> Result<AssetHandle<Self::Asset>, AssetError>;

    /// Get the asset type.
    fn asset_type() -> AssetType;
}

// Implement AssetLoader for MeshData
impl AssetLoader for MeshData {
    type Asset = MeshData;

    fn load(path: &Path) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext.to_lowercase().as_str() {
            "obj" => {
                let obj_data = std::fs::read_to_string(path)?;
                MeshData::from_obj(&obj_data).map_err(Into::into)
            }
            "gltf" | "glb" => {
                let gltf_data = std::fs::read(path)?;
                MeshData::from_gltf(&gltf_data, None).map_err(Into::into)
            }
            _ => Err(format!("Unsupported mesh format: {ext}").into()),
        }
    }

    fn parse(data: &[u8]) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        // Try to detect format from data
        MeshData::from_gltf(data, None).map_err(Into::into)
    }

    fn generate_id(asset: &Self::Asset) -> AssetId {
        let mut hasher = blake3::Hasher::new();
        for vertex in &asset.vertices {
            hasher.update(
                &vertex
                    .position
                    .to_array()
                    .iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
        }
        for index in &asset.indices {
            hasher.update(&index.to_le_bytes());
        }
        AssetId::from_content(hasher.finalize().as_bytes())
    }

    fn insert(
        manager: &AssetManager,
        id: AssetId,
        asset: Self::Asset,
    ) -> Result<AssetHandle<Self::Asset>, AssetError> {
        Ok(manager.meshes.insert(id, asset))
    }

    fn asset_type() -> AssetType {
        AssetType::Mesh
    }
}

// Implement AssetLoader for TextureData
impl AssetLoader for TextureData {
    type Asset = TextureData;

    fn load(path: &Path) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        let data = std::fs::read(path)?;
        Self::parse(&data)
    }

    fn parse(data: &[u8]) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        // Try DDS first (has magic number), then fall back to image
        if data.len() >= 4 && &data[0..4] == b"DDS " {
            TextureData::from_dds_bytes(data).map_err(Into::into)
        } else {
            TextureData::from_image_bytes(data).map_err(Into::into)
        }
    }

    fn generate_id(asset: &Self::Asset) -> AssetId {
        AssetId::from_content(&asset.data)
    }

    fn insert(
        manager: &AssetManager,
        id: AssetId,
        asset: Self::Asset,
    ) -> Result<AssetHandle<Self::Asset>, AssetError> {
        Ok(manager.textures.insert(id, asset))
    }

    fn asset_type() -> AssetType {
        AssetType::Texture
    }
}

// Implement AssetLoader for ShaderData
impl AssetLoader for ShaderData {
    type Asset = ShaderData;

    fn load(path: &Path) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        let data = std::fs::read(path)?;
        Self::parse(&data)
    }

    fn parse(data: &[u8]) -> Result<Self::Asset, Box<dyn std::error::Error + Send + Sync>> {
        use crate::ShaderStage;

        // Simple heuristic: if starts with SPIR-V magic number, it's SPIR-V
        if data.len() >= 4 && u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == 0x07230203
        {
            // SPIR-V binary
            let spirv: Vec<u32> = data
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            Ok(ShaderData::from_spirv(ShaderStage::Vertex, spirv, None)?)
        } else {
            // Assume GLSL text
            let source = String::from_utf8_lossy(data).to_string();
            Ok(ShaderData::from_glsl(ShaderStage::Vertex, source, None)?)
        }
    }

    fn generate_id(asset: &Self::Asset) -> AssetId {
        // Use a simple hash of stage + source
        let mut hasher = blake3::Hasher::new();
        hasher.update(asset.stage.as_str().as_bytes());
        hasher.update(asset.entry_point.as_bytes());
        match &asset.source {
            ShaderSource::Glsl(source) => {
                hasher.update(source.as_bytes());
            }
            ShaderSource::Spirv(spirv) => {
                for &word in spirv {
                    hasher.update(&word.to_le_bytes());
                }
            }
        }
        AssetId::from_content(hasher.finalize().as_bytes())
    }

    fn insert(
        manager: &AssetManager,
        id: AssetId,
        asset: Self::Asset,
    ) -> Result<AssetHandle<Self::Asset>, AssetError> {
        Ok(manager.shaders.insert(id, asset))
    }

    fn asset_type() -> AssetType {
        AssetType::Shader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("obj"), Some(AssetType::Mesh));
        assert_eq!(AssetType::from_extension("OBJ"), Some(AssetType::Mesh));
        assert_eq!(AssetType::from_extension("png"), Some(AssetType::Texture));
        assert_eq!(AssetType::from_extension("glsl"), Some(AssetType::Shader));
        assert_eq!(AssetType::from_extension("ttf"), Some(AssetType::Font));
        assert_eq!(AssetType::from_extension("unknown"), None);
    }

    #[test]
    fn test_manager_creation() {
        let manager = AssetManager::new();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_manager_clear() {
        let manager = AssetManager::new();

        // Insert a test mesh directly
        let mesh = MeshData::cube();
        let id = AssetId::from_content(b"test");
        let _handle = manager.meshes.insert(id, mesh);

        assert_eq!(manager.len(), 1);

        manager.clear();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }
}
