//! Memory management and LRU eviction for assets.
//!
//! Tracks memory usage and automatically evicts least-recently-used assets
//! when memory budgets are exceeded.

use crate::{
    AssetId, AssetRegistry, AssetType, AudioData, FontData, MaterialData, MeshData, ShaderData,
    TextureData,
};
use linked_hash_map::LinkedHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Trait for types that can report their memory usage.
pub trait MemorySized {
    /// Return the size of this asset in bytes (including heap allocations).
    fn size_bytes(&self) -> usize;
}

/// Memory statistics for asset tracking.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Total memory allocated for assets (bytes).
    pub total_allocated: usize,
    /// Memory used by mesh assets (bytes).
    pub mesh_memory: usize,
    /// Memory used by texture assets (bytes).
    pub texture_memory: usize,
    /// Memory used by shader assets (bytes).
    pub shader_memory: usize,
    /// Memory used by material assets (bytes).
    pub material_memory: usize,
    /// Memory used by audio assets (bytes).
    pub audio_memory: usize,
    /// Memory used by font assets (bytes).
    pub font_memory: usize,
}

impl MemoryStats {
    /// Create new zero-initialized stats.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total_allocated: 0,
            mesh_memory: 0,
            texture_memory: 0,
            shader_memory: 0,
            material_memory: 0,
            audio_memory: 0,
            font_memory: 0,
        }
    }

    /// Get memory usage for a specific asset type.
    #[must_use]
    pub const fn get_type_memory(&self, asset_type: AssetType) -> usize {
        match asset_type {
            AssetType::Mesh => self.mesh_memory,
            AssetType::Texture => self.texture_memory,
            AssetType::Shader => self.shader_memory,
            AssetType::Material => self.material_memory,
            AssetType::Audio => self.audio_memory,
            AssetType::Font => self.font_memory,
        }
    }

    /// Update memory usage for a specific asset type.
    pub fn set_type_memory(&mut self, asset_type: AssetType, bytes: usize) {
        match asset_type {
            AssetType::Mesh => self.mesh_memory = bytes,
            AssetType::Texture => self.texture_memory = bytes,
            AssetType::Shader => self.shader_memory = bytes,
            AssetType::Material => self.material_memory = bytes,
            AssetType::Audio => self.audio_memory = bytes,
            AssetType::Font => self.font_memory = bytes,
        }
        self.total_allocated = self.mesh_memory
            + self.texture_memory
            + self.shader_memory
            + self.material_memory
            + self.audio_memory
            + self.font_memory;
    }
}

/// Memory budget configuration.
#[derive(Debug, Clone, Copy)]
pub struct MemoryBudget {
    /// Total memory budget for all assets (bytes).
    pub total: usize,
    /// Per-type budgets (bytes).
    pub mesh: usize,
    /// Texture budget (bytes).
    pub texture: usize,
    /// Shader budget (bytes).
    pub shader: usize,
    /// Material budget (bytes).
    pub material: usize,
    /// Audio budget (bytes).
    pub audio: usize,
    /// Font budget (bytes).
    pub font: usize,
}

impl MemoryBudget {
    /// Create a default budget (1 GB total).
    #[must_use]
    pub const fn default_budget() -> Self {
        const MB: usize = 1024 * 1024;
        const GB: usize = 1024 * MB;

        Self {
            total: GB,         // 1 GB total
            mesh: 100 * MB,    // 100 MB for meshes
            texture: 500 * MB, // 500 MB for textures
            shader: 10 * MB,   // 10 MB for shaders
            material: 50 * MB, // 50 MB for materials
            audio: 200 * MB,   // 200 MB for audio
            font: 50 * MB,     // 50 MB for fonts
        }
    }

    /// Get the budget for a specific asset type.
    #[must_use]
    pub const fn get_type_budget(&self, asset_type: AssetType) -> usize {
        match asset_type {
            AssetType::Mesh => self.mesh,
            AssetType::Texture => self.texture,
            AssetType::Shader => self.shader,
            AssetType::Material => self.material,
            AssetType::Audio => self.audio,
            AssetType::Font => self.font,
        }
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::default_budget()
    }
}

/// LRU (Least Recently Used) cache with memory budget enforcement.
///
/// Tracks asset access patterns and automatically evicts least-recently-used
/// soft-referenced assets when memory budgets are exceeded.
///
/// # Examples
///
/// ```
/// use engine_assets::{LruCache, MemoryBudget, AssetId, AssetType};
///
/// let budget = MemoryBudget::default();
/// let mut cache = LruCache::new(budget);
///
/// // Track asset access
/// let id = AssetId::from_content(b"test");
/// cache.access(id, AssetType::Mesh);
///
/// // Check if eviction is needed
/// cache.update_memory_usage(150_000_000); // 150 MB meshes
/// ```
pub struct LruCache {
    budget: MemoryBudget,
    stats: Arc<RwLock<MemoryStats>>,
    // Track access order per asset type
    mesh_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    texture_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    shader_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    material_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    audio_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
    font_lru: Arc<RwLock<LinkedHashMap<AssetId, ()>>>,
}

impl LruCache {
    /// Create a new LRU cache with the given budget.
    #[must_use]
    pub fn new(budget: MemoryBudget) -> Self {
        info!(total_budget_mb = budget.total / (1024 * 1024), "Initializing LRU cache");
        Self {
            budget,
            stats: Arc::new(RwLock::new(MemoryStats::new())),
            mesh_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
            texture_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
            shader_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
            material_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
            audio_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
            font_lru: Arc::new(RwLock::new(LinkedHashMap::new())),
        }
    }

    /// Record an asset access (moves it to front of LRU list).
    #[instrument(skip(self))]
    pub fn access(&self, id: AssetId, asset_type: AssetType) {
        debug!(id = ?id, asset_type = ?asset_type, "Asset accessed");

        let lru = match asset_type {
            AssetType::Mesh => &self.mesh_lru,
            AssetType::Texture => &self.texture_lru,
            AssetType::Shader => &self.shader_lru,
            AssetType::Material => &self.material_lru,
            AssetType::Audio => &self.audio_lru,
            AssetType::Font => &self.font_lru,
        };

        let mut lru = lru.write();
        // Remove and re-insert to move to front (most recently used)
        lru.remove(&id);
        lru.insert(id, ());
    }

    /// Update memory usage statistics.
    pub fn update_memory_usage(&self, bytes: usize, asset_type: AssetType) {
        let mut stats = self.stats.write();
        stats.set_type_memory(asset_type, bytes);

        // Check if we need to evict
        let type_budget = self.budget.get_type_budget(asset_type);
        if bytes > type_budget {
            warn!(
                asset_type = ?asset_type,
                usage_mb = bytes / (1024 * 1024),
                budget_mb = type_budget / (1024 * 1024),
                "Memory budget exceeded for asset type"
            );
        }

        if stats.total_allocated > self.budget.total {
            warn!(
                usage_mb = stats.total_allocated / (1024 * 1024),
                budget_mb = self.budget.total / (1024 * 1024),
                "Total memory budget exceeded"
            );
        }
    }

    /// Get current memory statistics.
    #[must_use]
    pub fn stats(&self) -> MemoryStats {
        *self.stats.read()
    }

    /// Get the memory budget.
    #[must_use]
    pub const fn budget(&self) -> MemoryBudget {
        self.budget
    }

    /// Find candidates for eviction (least recently used soft-referenced assets).
    ///
    /// Returns asset IDs in order from least to most recently used.
    #[must_use]
    pub fn eviction_candidates<T>(
        &self,
        asset_type: AssetType,
        registry: &AssetRegistry<T>,
        count: usize,
    ) -> Vec<AssetId> {
        let lru = match asset_type {
            AssetType::Mesh => &self.mesh_lru,
            AssetType::Texture => &self.texture_lru,
            AssetType::Shader => &self.shader_lru,
            AssetType::Material => &self.material_lru,
            AssetType::Audio => &self.audio_lru,
            AssetType::Font => &self.font_lru,
        };

        let lru = lru.read();
        lru.iter()
            .filter(|(id, _)| !registry.is_hard_referenced(**id))
            .take(count)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Remove an asset from the LRU tracking.
    pub fn remove(&self, id: AssetId, asset_type: AssetType) {
        let lru = match asset_type {
            AssetType::Mesh => &self.mesh_lru,
            AssetType::Texture => &self.texture_lru,
            AssetType::Shader => &self.shader_lru,
            AssetType::Material => &self.material_lru,
            AssetType::Audio => &self.audio_lru,
            AssetType::Font => &self.font_lru,
        };

        lru.write().remove(&id);
    }

    /// Clear all LRU tracking.
    pub fn clear(&self) {
        self.mesh_lru.write().clear();
        self.texture_lru.write().clear();
        self.shader_lru.write().clear();
        self.material_lru.write().clear();
        self.audio_lru.write().clear();
        self.font_lru.write().clear();
        *self.stats.write() = MemoryStats::new();
    }

    /// Check if we're over budget for a specific type.
    #[must_use]
    pub fn is_over_budget(&self, asset_type: AssetType) -> bool {
        let stats = self.stats.read();
        let usage = stats.get_type_memory(asset_type);
        let budget = self.budget.get_type_budget(asset_type);
        usage > budget
    }

    /// Check if we're over the total budget.
    #[must_use]
    pub fn is_over_total_budget(&self) -> bool {
        let stats = self.stats.read();
        stats.total_allocated > self.budget.total
    }

    /// Get the amount of memory to free for a specific type.
    #[must_use]
    pub fn memory_to_free(&self, asset_type: AssetType) -> usize {
        let stats = self.stats.read();
        let usage = stats.get_type_memory(asset_type);
        let budget = self.budget.get_type_budget(asset_type);
        usage.saturating_sub(budget)
    }
}

// Implement MemorySized for all asset types

impl MemorySized for MeshData {
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.vertices.len() * std::mem::size_of::<crate::Vertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }
}

impl MemorySized for TextureData {
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.data.len()
            + self.mip_levels.len() * std::mem::size_of::<crate::MipLevel>()
    }
}

impl MemorySized for ShaderData {
    fn size_bytes(&self) -> usize {
        let source_size = match &self.source {
            crate::ShaderSource::Glsl(s) => s.len(),
            crate::ShaderSource::Spirv(v) => v.len() * std::mem::size_of::<u32>(),
        };
        std::mem::size_of::<Self>() + source_size + self.entry_point.len()
    }
}

impl MemorySized for MaterialData {
    fn size_bytes(&self) -> usize {
        // MaterialData is relatively small (mostly IDs and scalars)
        std::mem::size_of::<Self>()
    }
}

impl MemorySized for AudioData {
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.data.len()
    }
}

impl MemorySized for FontData {
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats_creation() {
        let stats = MemoryStats::new();
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.mesh_memory, 0);
    }

    #[test]
    fn test_memory_stats_update() {
        let mut stats = MemoryStats::new();
        stats.set_type_memory(AssetType::Mesh, 1000);
        assert_eq!(stats.mesh_memory, 1000);
        assert_eq!(stats.total_allocated, 1000);

        stats.set_type_memory(AssetType::Texture, 2000);
        assert_eq!(stats.texture_memory, 2000);
        assert_eq!(stats.total_allocated, 3000);
    }

    #[test]
    fn test_memory_budget_default() {
        let budget = MemoryBudget::default();
        assert_eq!(budget.total, 1024 * 1024 * 1024); // 1 GB
        assert_eq!(budget.mesh, 100 * 1024 * 1024); // 100 MB
    }

    #[test]
    fn test_lru_cache_creation() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);
        let stats = cache.stats();
        assert_eq!(stats.total_allocated, 0);
    }

    #[test]
    fn test_lru_access() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id1 = AssetId::from_content(b"asset1");
        let id2 = AssetId::from_content(b"asset2");

        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Mesh);

        // Both should be tracked
        let lru = cache.mesh_lru.read();
        assert_eq!(lru.len(), 2);
    }

    #[test]
    fn test_lru_access_order() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id1 = AssetId::from_content(b"asset1");
        let id2 = AssetId::from_content(b"asset2");
        let id3 = AssetId::from_content(b"asset3");

        // Access in order: 1, 2, 3
        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Mesh);
        cache.access(id3, AssetType::Mesh);

        // Access 1 again (should move to front)
        cache.access(id1, AssetType::Mesh);

        // Order should now be: 2, 3, 1 (least to most recent)
        let lru = cache.mesh_lru.read();
        let ids: Vec<AssetId> = lru.keys().copied().collect();
        assert_eq!(ids, vec![id2, id3, id1]);
    }

    #[test]
    fn test_lru_remove() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id = AssetId::from_content(b"asset");
        cache.access(id, AssetType::Mesh);

        assert_eq!(cache.mesh_lru.read().len(), 1);

        cache.remove(id, AssetType::Mesh);
        assert_eq!(cache.mesh_lru.read().len(), 0);
    }

    #[test]
    fn test_lru_clear() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id1 = AssetId::from_content(b"asset1");
        let id2 = AssetId::from_content(b"asset2");

        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Texture);

        cache.clear();

        assert_eq!(cache.mesh_lru.read().len(), 0);
        assert_eq!(cache.texture_lru.read().len(), 0);
        assert_eq!(cache.stats().total_allocated, 0);
    }

    #[test]
    fn test_memory_budget_exceeded() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        // Update with usage over budget
        cache.update_memory_usage(200 * 1024 * 1024, AssetType::Mesh); // 200 MB > 100 MB budget

        assert!(cache.is_over_budget(AssetType::Mesh));
        assert_eq!(cache.memory_to_free(AssetType::Mesh), 100 * 1024 * 1024);
    }

    #[test]
    fn test_total_budget_exceeded() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        // Fill all types to exceed total budget
        cache.update_memory_usage(500 * 1024 * 1024, AssetType::Mesh);
        cache.update_memory_usage(500 * 1024 * 1024, AssetType::Texture);

        assert!(cache.is_over_total_budget());
    }

    #[test]
    fn test_eviction_candidates_with_registry() {
        use crate::{AssetRegistry, RefType};

        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);
        let registry = AssetRegistry::<MeshData>::new();

        let id1 = AssetId::from_content(b"asset1");
        let id2 = AssetId::from_content(b"asset2");
        let id3 = AssetId::from_content(b"asset3");

        // Insert assets with different ref types
        let mesh = MeshData::cube();
        let _h1 = registry.insert_with_reftype(id1, mesh.clone(), RefType::Soft);
        let _h2 = registry.insert_with_reftype(id2, mesh.clone(), RefType::Hard);
        let _h3 = registry.insert_with_reftype(id3, mesh.clone(), RefType::Soft);

        // Track access
        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Mesh);
        cache.access(id3, AssetType::Mesh);

        // Get eviction candidates (should skip hard-referenced id2)
        let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 10);

        assert_eq!(candidates.len(), 2);
        assert!(candidates.contains(&id1));
        assert!(!candidates.contains(&id2)); // Hard ref should be skipped
        assert!(candidates.contains(&id3));
    }

    #[test]
    fn test_memory_sized_mesh() {
        let mesh = MeshData::cube();
        let size = mesh.size_bytes();

        // Should include struct size + vertices + indices
        let expected_min = std::mem::size_of::<MeshData>()
            + mesh.vertices.len() * std::mem::size_of::<crate::Vertex>();
        assert!(size >= expected_min);
    }

    #[test]
    fn test_memory_sized_texture() {
        use crate::TextureFormat;

        let data = vec![255u8; 256 * 256 * 4]; // Red solid color
        let texture = TextureData::new(256, 256, TextureFormat::RGBA8Unorm, data).unwrap();
        let size = texture.size_bytes();

        // Should include struct size + pixel data
        assert!(size > 256 * 256 * 4); // At least the pixel data
    }

    #[test]
    fn test_lru_respects_access_order() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id1 = AssetId::from_content(b"oldest");
        let id2 = AssetId::from_content(b"middle");
        let id3 = AssetId::from_content(b"newest");

        // Access in order: 1, 2, 3
        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Mesh);
        cache.access(id3, AssetType::Mesh);

        // id1 should be first (oldest)
        let lru = cache.mesh_lru.read();
        let first = lru.keys().next().copied();
        assert_eq!(first, Some(id1));
    }

    #[test]
    fn test_multiple_access_moves_to_front() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let id1 = AssetId::from_content(b"asset1");
        let id2 = AssetId::from_content(b"asset2");
        let id3 = AssetId::from_content(b"asset3");

        cache.access(id1, AssetType::Mesh);
        cache.access(id2, AssetType::Mesh);
        cache.access(id3, AssetType::Mesh);

        // Now access id1 again - should move to back
        cache.access(id1, AssetType::Mesh);

        let lru = cache.mesh_lru.read();
        let last = lru.keys().last().copied();
        assert_eq!(last, Some(id1));
    }

    #[test]
    fn test_per_type_budgets() {
        let budget = MemoryBudget::default();

        assert_eq!(budget.get_type_budget(AssetType::Mesh), 100 * 1024 * 1024);
        assert_eq!(budget.get_type_budget(AssetType::Texture), 500 * 1024 * 1024);
        assert_eq!(budget.get_type_budget(AssetType::Audio), 200 * 1024 * 1024);
    }

    #[test]
    fn test_stats_per_type_tracking() {
        let mut stats = MemoryStats::new();

        stats.set_type_memory(AssetType::Mesh, 1000);
        stats.set_type_memory(AssetType::Texture, 2000);

        assert_eq!(stats.get_type_memory(AssetType::Mesh), 1000);
        assert_eq!(stats.get_type_memory(AssetType::Texture), 2000);
        assert_eq!(stats.total_allocated, 3000);
    }

    #[test]
    fn test_memory_to_free_calculation() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        cache.update_memory_usage(150 * 1024 * 1024, AssetType::Mesh); // 150 MB, budget 100 MB

        let to_free = cache.memory_to_free(AssetType::Mesh);
        assert_eq!(to_free, 50 * 1024 * 1024); // Need to free 50 MB
    }

    #[test]
    fn test_memory_to_free_under_budget() {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        cache.update_memory_usage(50 * 1024 * 1024, AssetType::Mesh); // 50 MB, budget 100 MB

        let to_free = cache.memory_to_free(AssetType::Mesh);
        assert_eq!(to_free, 0); // Under budget, nothing to free
    }
}
