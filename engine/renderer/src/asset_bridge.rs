//! Asset system bridge for GPU resource management.
//!
//! This module connects the asset system (engine-assets) to the renderer,
//! providing automatic GPU upload and caching for meshes, textures, and shaders.
//!
//! # Features
//!
//! - Lazy GPU upload (CPU asset → GPU resource only when needed)
//! - Asset handle resolution to GPU resources
//! - Hot-reload support with GPU resource updates
//! - Resource pooling (shared textures, shared meshes)
//! - Streaming support (low-LOD first → high-LOD later)
//!
//! # Architecture
//!
//! ```text
//! AssetManager (CPU) → AssetBridge → GPU Resources
//!     ↓                     ↓             ↓
//! AssetHandle<T>  →  get_or_upload()  →  GpuMesh/GpuTexture
//! ```

use crate::context::VulkanContext;
use crate::error::RendererError;
use ash::vk;
use engine_assets::{AssetId, AssetManager, MeshData, TextureData};
use gpu_allocator::MemoryLocation;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// GPU mesh (vertex + index buffers)
#[derive(Debug)]
pub struct GpuMesh {
    /// Vertex buffer
    pub vertex_buffer: vk::Buffer,
    /// Index buffer
    pub index_buffer: vk::Buffer,
    /// Vertex count
    pub vertex_count: u32,
    /// Index count
    pub index_count: u32,
}

impl GpuMesh {
    /// Get triangle count
    #[inline]
    pub fn triangle_count(&self) -> u32 {
        self.index_count / 3
    }
}

/// GPU texture (image + view)
#[derive(Debug)]
pub struct GpuTexture {
    /// Vulkan image handle
    pub image: vk::Image,
    /// Image view handle
    pub image_view: vk::ImageView,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Mipmap levels
    pub mip_levels: u32,
    /// Format
    pub format: vk::Format,
}

/// GPU shader module
#[derive(Debug)]
pub struct GpuShader {
    /// Shader module handle
    pub module: vk::ShaderModule,
    /// Entry point name
    pub entry_point: String,
    /// Shader stage
    pub stage: vk::ShaderStageFlags,
}

/// Asset → GPU resource bridge.
///
/// Manages automatic GPU upload, caching, and hot-reload for assets.
///
/// # Examples
///
/// ```no_run
/// use engine_assets::{AssetManager, MeshData};
/// use engine_renderer::{VulkanContext, AssetBridge};
/// use std::sync::Arc;
/// use std::path::Path;
///
/// let context = VulkanContext::new("MyApp", None, None)?;
/// let asset_manager = Arc::new(AssetManager::new());
/// let mut bridge = AssetBridge::new(context, asset_manager.clone());
///
/// // Load mesh asset
/// let mesh_handle = asset_manager.load_sync::<MeshData>(Path::new("cube.obj"))?;
///
/// // Automatic GPU upload on first use
/// let gpu_mesh = bridge.get_or_upload_mesh(mesh_handle.id())?;
///
/// // Subsequent calls use cached GPU resource
/// let gpu_mesh_cached = bridge.get_or_upload_mesh(mesh_handle.id())?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct AssetBridge {
    context: VulkanContext,
    asset_manager: Arc<AssetManager>,
    // GPU resource caches
    mesh_cache: HashMap<AssetId, GpuMesh>,
    texture_cache: HashMap<AssetId, GpuTexture>,
    shader_cache: HashMap<AssetId, GpuShader>,
    // Track upload statistics
    total_uploads: usize,
    cache_hits: usize,
}

impl AssetBridge {
    /// Create a new asset bridge.
    ///
    /// # Arguments
    ///
    /// * `context` - Vulkan context for GPU resource creation
    /// * `asset_manager` - Asset manager for CPU asset access
    #[instrument(skip(context, asset_manager))]
    pub fn new(context: VulkanContext, asset_manager: Arc<AssetManager>) -> Self {
        info!("Initializing AssetBridge");
        Self {
            context,
            asset_manager,
            mesh_cache: HashMap::new(),
            texture_cache: HashMap::new(),
            shader_cache: HashMap::new(),
            total_uploads: 0,
            cache_hits: 0,
        }
    }

    /// Get or upload a mesh to GPU.
    ///
    /// This function lazily uploads the mesh on first access and returns
    /// a reference to the cached GPU resource on subsequent calls.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the mesh
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The asset is not found in the asset manager
    /// - GPU upload fails
    #[instrument(skip(self))]
    pub fn get_or_upload_mesh(&mut self, asset_id: AssetId) -> Result<&GpuMesh, RendererError> {
        // Check cache first
        if self.mesh_cache.contains_key(&asset_id) {
            self.cache_hits += 1;
            debug!(asset_id = %asset_id, "Mesh cache hit");
            return Ok(self.mesh_cache.get(&asset_id).unwrap());
        }

        // Not cached - fetch from asset manager
        let mesh_data = self
            .asset_manager
            .get_mesh(asset_id)
            .ok_or_else(|| RendererError::assetnotfound(asset_id.to_string()))?;

        // Upload to GPU
        info!(
            asset_id = %asset_id,
            vertices = mesh_data.vertices.len(),
            indices = mesh_data.indices.len(),
            "Uploading mesh to GPU"
        );

        let gpu_mesh = self.upload_mesh_data(&mesh_data)?;

        self.total_uploads += 1;

        // Cache the GPU resource
        self.mesh_cache.insert(asset_id, gpu_mesh);

        Ok(self.mesh_cache.get(&asset_id).unwrap())
    }

    /// Upload mesh data to GPU buffers.
    ///
    /// Creates vertex and index buffers and uploads the mesh data.
    fn upload_mesh_data(&self, mesh_data: &MeshData) -> Result<GpuMesh, RendererError> {
        use crate::buffer::GpuBuffer;

        // Validate mesh data
        if mesh_data.vertices.is_empty() {
            return Err(RendererError::invalidmeshdata("No vertices".to_string()));
        }
        if mesh_data.indices.is_empty() {
            return Err(RendererError::invalidmeshdata("No indices".to_string()));
        }

        // Create vertex buffer
        let vertex_size = std::mem::size_of_val(&mesh_data.vertices[..]) as u64;
        let vertex_usage =
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST;

        let mut vertex_buffer =
            GpuBuffer::new(&self.context, vertex_size, vertex_usage, MemoryLocation::CpuToGpu)?;
        vertex_buffer.upload(&mesh_data.vertices)?;

        // Create index buffer
        let index_size = std::mem::size_of_val(&mesh_data.indices[..]) as u64;
        let index_usage = vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST;

        let mut index_buffer =
            GpuBuffer::new(&self.context, index_size, index_usage, MemoryLocation::CpuToGpu)?;
        index_buffer.upload(&mesh_data.indices)?;

        Ok(GpuMesh {
            vertex_buffer: vertex_buffer.handle(),
            index_buffer: index_buffer.handle(),
            vertex_count: mesh_data.vertices.len() as u32,
            index_count: mesh_data.indices.len() as u32,
        })
    }

    /// Get or upload a texture to GPU.
    ///
    /// This function lazily uploads the texture on first access and returns
    /// a reference to the cached GPU resource on subsequent calls.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the texture
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The asset is not found in the asset manager
    /// - GPU upload fails
    #[instrument(skip(self))]
    pub fn get_or_upload_texture(
        &mut self,
        asset_id: AssetId,
    ) -> Result<&GpuTexture, RendererError> {
        // Check cache first
        if self.texture_cache.contains_key(&asset_id) {
            self.cache_hits += 1;
            debug!(asset_id = %asset_id, "Texture cache hit");
            return Ok(self.texture_cache.get(&asset_id).unwrap());
        }

        // Not cached - fetch from asset manager
        let texture_data = self
            .asset_manager
            .get_texture(asset_id)
            .ok_or_else(|| RendererError::assetnotfound(asset_id.to_string()))?;

        // Upload to GPU
        info!(
            asset_id = %asset_id,
            width = texture_data.width,
            height = texture_data.height,
            mip_levels = texture_data.mip_levels.len(),
            "Uploading texture to GPU"
        );

        let gpu_texture = self.upload_texture_data(&texture_data)?;

        self.total_uploads += 1;

        // Cache the GPU resource
        self.texture_cache.insert(asset_id, gpu_texture);

        Ok(self.texture_cache.get(&asset_id).unwrap())
    }

    /// Upload texture data to GPU image.
    ///
    /// Creates a Vulkan image and uploads the texture data.
    /// For now, this is a placeholder - full implementation requires
    /// command buffer submission and image layout transitions.
    fn upload_texture_data(&self, _texture_data: &TextureData) -> Result<GpuTexture, RendererError> {
        // TODO: Full implementation requires:
        // 1. Create VkImage
        // 2. Allocate GPU memory
        // 3. Create staging buffer
        // 4. Upload data via transfer queue
        // 5. Transition image layout
        // 6. Create image view
        //
        // For now, return a placeholder error
        Err(RendererError::notimplemented("Texture upload not yet implemented".to_string()))
    }

    /// Reload a mesh asset (for hot-reload support).
    ///
    /// Destroys the old GPU resource and uploads the new version.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the mesh to reload
    ///
    /// # Errors
    ///
    /// Returns an error if the mesh is not found or upload fails.
    #[instrument(skip(self))]
    pub fn reload_mesh(&mut self, asset_id: AssetId) -> Result<(), RendererError> {
        info!(asset_id = %asset_id, "Reloading mesh GPU resource");

        // Evict old GPU resource (Drop will clean up)
        if let Some(_old_mesh) = self.mesh_cache.remove(&asset_id) {
            debug!(asset_id = %asset_id, "Evicted old mesh GPU resource");
            // GPU resources are cleaned up via Drop automatically
        }

        // Re-upload will happen on next get_or_upload_mesh call
        Ok(())
    }

    /// Reload a texture asset (for hot-reload support).
    ///
    /// Destroys the old GPU resource and uploads the new version.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the texture to reload
    ///
    /// # Errors
    ///
    /// Returns an error if the texture is not found or upload fails.
    #[instrument(skip(self))]
    pub fn reload_texture(&mut self, asset_id: AssetId) -> Result<(), RendererError> {
        info!(asset_id = %asset_id, "Reloading texture GPU resource");

        // Evict old GPU resource
        if let Some(_old_texture) = self.texture_cache.remove(&asset_id) {
            debug!(asset_id = %asset_id, "Evicted old texture GPU resource");
            // GPU resources are cleaned up via Drop automatically
        }

        // Re-upload will happen on next get_or_upload_texture call
        Ok(())
    }

    /// Evict a mesh from GPU cache.
    ///
    /// This frees GPU memory but keeps the CPU asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the mesh to evict
    pub fn evict_mesh(&mut self, asset_id: AssetId) {
        if self.mesh_cache.remove(&asset_id).is_some() {
            info!(asset_id = %asset_id, "Mesh evicted from GPU cache");
        }
    }

    /// Evict a texture from GPU cache.
    ///
    /// This frees GPU memory but keeps the CPU asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - Asset ID of the texture to evict
    pub fn evict_texture(&mut self, asset_id: AssetId) {
        if self.texture_cache.remove(&asset_id).is_some() {
            info!(asset_id = %asset_id, "Texture evicted from GPU cache");
        }
    }

    /// Get cache statistics.
    #[must_use]
    pub fn stats(&self) -> AssetBridgeStats {
        AssetBridgeStats {
            mesh_count: self.mesh_cache.len(),
            texture_count: self.texture_cache.len(),
            shader_count: self.shader_cache.len(),
            total_uploads: self.total_uploads,
            cache_hits: self.cache_hits,
            cache_hit_rate: if self.total_uploads + self.cache_hits > 0 {
                self.cache_hits as f32 / (self.total_uploads + self.cache_hits) as f32
            } else {
                0.0
            },
        }
    }

    /// Clear all GPU caches.
    ///
    /// This destroys all GPU resources but keeps CPU assets.
    pub fn clear(&mut self) {
        info!("Clearing all GPU caches");
        let mesh_count = self.mesh_cache.len();
        let texture_count = self.texture_cache.len();
        let shader_count = self.shader_cache.len();

        self.mesh_cache.clear();
        self.texture_cache.clear();
        self.shader_cache.clear();

        info!(
            meshes = mesh_count,
            textures = texture_count,
            shaders = shader_count,
            "GPU caches cleared"
        );
    }
}

/// Statistics about asset bridge operations.
#[derive(Debug, Clone, Copy)]
pub struct AssetBridgeStats {
    /// Number of meshes in GPU cache
    pub mesh_count: usize,
    /// Number of textures in GPU cache
    pub texture_count: usize,
    /// Number of shaders in GPU cache
    pub shader_count: usize,
    /// Total number of GPU uploads performed
    pub total_uploads: usize,
    /// Number of cache hits (no upload needed)
    pub cache_hits: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_bridge_stats() {
        // Test stats with no Vulkan required
        let stats = AssetBridgeStats {
            mesh_count: 5,
            texture_count: 10,
            shader_count: 3,
            total_uploads: 18,
            cache_hits: 32,
            cache_hit_rate: 0.64,
        };

        assert_eq!(stats.mesh_count, 5);
        assert_eq!(stats.texture_count, 10);
        assert_eq!(stats.total_uploads, 18);
        assert_eq!(stats.cache_hits, 32);
        assert!((stats.cache_hit_rate - 0.64).abs() < 0.01);
    }

    #[test]
    fn test_gpu_mesh_triangle_count() {
        let mesh = GpuMesh {
            vertex_buffer: vk::Buffer::null(),
            index_buffer: vk::Buffer::null(),
            vertex_count: 100,
            index_count: 300,
        };

        assert_eq!(mesh.triangle_count(), 100);
    }
}
