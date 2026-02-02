//! GPU mesh cache for efficient mesh upload and caching
//!
//! The GpuCache lazily uploads MeshData to GPU buffers and caches them by AssetId.
//! This avoids redundant uploads and manages GPU memory efficiently.

use crate::buffer::{IndexBuffer, VertexBuffer};
use crate::context::VulkanContext;
use crate::error::RendererError;
use ash::vk;
use engine_assets::{AssetId, MeshData};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

/// GPU mesh representation (vertex + index buffers)
#[derive(Debug)]
pub struct GpuCachedMesh {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
}

impl GpuCachedMesh {
    /// Get vertex buffer handle
    pub fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_buffer.handle()
    }

    /// Get index buffer handle
    pub fn index_buffer(&self) -> vk::Buffer {
        self.index_buffer.handle()
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> u32 {
        self.vertex_buffer.vertex_count()
    }

    /// Get index count
    pub fn index_count(&self) -> u32 {
        self.index_buffer.index_count()
    }
}

/// Mesh info for queries (without borrowing buffers)
#[derive(Debug, Clone, Copy)]
pub struct MeshInfo {
    /// Number of vertices
    pub vertex_count: u32,
    /// Number of indices
    pub index_count: u32,
}

/// GPU mesh cache
///
/// Lazily uploads meshes to GPU and caches them by AssetId.
/// Automatically cleans up GPU resources on drop.
#[derive(Debug)]
pub struct GpuCache {
    mesh_cache: HashMap<AssetId, GpuCachedMesh>,
}

impl GpuCache {
    /// Create a new GPU cache
    #[instrument(skip(_context))]
    pub fn new(_context: &VulkanContext) -> Result<Self, RendererError> {
        info!("Creating GPU mesh cache");
        Ok(Self { mesh_cache: HashMap::new() })
    }

    /// Upload a mesh to GPU (or return cached version)
    ///
    /// # Arguments
    /// * `context` - Vulkan context for buffer creation
    /// * `asset_id` - Unique asset identifier
    /// * `mesh_data` - Mesh data to upload
    #[instrument(skip(context, mesh_data))]
    pub fn upload_mesh(
        &mut self,
        context: &VulkanContext,
        asset_id: AssetId,
        mesh_data: &MeshData,
    ) -> Result<(), RendererError> {
        // Check if already cached
        if self.mesh_cache.contains_key(&asset_id) {
            debug!(asset_id = %asset_id, "Mesh already cached");
            return Ok(());
        }

        // Handle empty mesh
        if mesh_data.vertices.is_empty() || mesh_data.indices.is_empty() {
            warn!(asset_id = %asset_id, "Attempted to upload empty mesh");
            return Err(RendererError::invalidmeshdata(
                "Mesh has no vertices or indices".to_string(),
            ));
        }

        info!(
            asset_id = %asset_id,
            vertices = mesh_data.vertices.len(),
            indices = mesh_data.indices.len(),
            "Uploading mesh to GPU"
        );

        // Create vertex and index buffers
        let vertex_buffer = VertexBuffer::from_data(context, &mesh_data.vertices)?;
        let index_buffer = IndexBuffer::from_data(context, &mesh_data.indices)?;

        // Cache the mesh
        self.mesh_cache.insert(asset_id, GpuCachedMesh { vertex_buffer, index_buffer });

        info!(asset_id = %asset_id, "Mesh uploaded and cached");
        Ok(())
    }

    /// Check if mesh is cached
    #[inline]
    pub fn contains(&self, asset_id: AssetId) -> bool {
        self.mesh_cache.contains_key(&asset_id)
    }

    /// Get mesh info (vertex/index counts)
    pub fn get_mesh_info(&self, asset_id: AssetId) -> Option<MeshInfo> {
        self.mesh_cache.get(&asset_id).map(|mesh| MeshInfo {
            vertex_count: mesh.vertex_count(),
            index_count: mesh.index_count(),
        })
    }

    /// Get buffer handles for a cached mesh
    ///
    /// Returns (vertex_buffer, index_buffer) if mesh is cached
    pub fn get_buffers(&self, asset_id: AssetId) -> Option<(vk::Buffer, vk::Buffer)> {
        self.mesh_cache
            .get(&asset_id)
            .map(|mesh| (mesh.vertex_buffer(), mesh.index_buffer()))
    }

    /// Get cached mesh reference
    pub fn get_mesh(&self, asset_id: AssetId) -> Option<&GpuCachedMesh> {
        self.mesh_cache.get(&asset_id)
    }

    /// Evict a mesh from cache
    #[instrument(skip(self))]
    pub fn evict(&mut self, asset_id: AssetId) {
        if self.mesh_cache.remove(&asset_id).is_some() {
            info!(asset_id = %asset_id, "Mesh evicted from cache");
        }
    }

    /// Clear all cached meshes
    #[instrument(skip(self))]
    pub fn clear(&mut self) {
        let count = self.mesh_cache.len();
        self.mesh_cache.clear();
        info!(count = count, "GPU cache cleared");
    }

    /// Get number of cached meshes
    #[inline]
    pub fn len(&self) -> usize {
        self.mesh_cache.len()
    }

    /// Check if cache is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.mesh_cache.is_empty()
    }
}

// Automatic cleanup via Drop for GpuCachedMesh (buffers have their own Drop)
impl Drop for GpuCache {
    fn drop(&mut self) {
        debug!(count = self.mesh_cache.len(), "Dropping GPU cache");
        self.mesh_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_info_copy() {
        // MeshInfo should be Copy
        let info = MeshInfo { vertex_count: 100, index_count: 300 };
        let info2 = info; // Copy
        assert_eq!(info.vertex_count, info2.vertex_count);
    }

    #[test]
    fn test_cache_empty_state() {
        // Test cache state without Vulkan
        let context = VulkanContext::new("CacheEmptyTest", None, None);
        if context.is_err() {
            // Skipping test - no Vulkan support
            return;
        }

        let cache = GpuCache::new(&context.unwrap()).expect("Cache creation failed");
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }
}
