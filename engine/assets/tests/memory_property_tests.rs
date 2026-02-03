//! Property-based tests for memory management invariants.

use engine_assets::{AssetId, AssetRegistry, AssetType, LruCache, MemoryBudget, MeshData, RefType};
use proptest::prelude::*;

proptest! {
    /// LRU invariant: most recently accessed asset is at the back of the LRU list.
    #[test]
    fn prop_lru_most_recent_at_back(asset_count in 1usize..100) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);
        let mesh = MeshData::cube();
        let registry = AssetRegistry::<MeshData>::new();

        // Create and access assets
        let mut ids = Vec::new();
        for i in 0..asset_count {
            let id = AssetId::from_content(&i.to_le_bytes());
            let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
            cache.access(id, AssetType::Mesh);
            ids.push(id);
        }

        // The last accessed asset should be at the back
        let lru = cache.mesh_lru.read();
        let last = lru.keys().last().copied();
        prop_assert_eq!(last, Some(ids[asset_count - 1]));
    }

    /// Memory tracking invariant: sum of per-type memory equals total memory.
    #[test]
    fn prop_memory_tracking_sum_equals_total(
        mesh_mem in 0usize..1_000_000,
        texture_mem in 0usize..1_000_000,
        audio_mem in 0usize..1_000_000,
    ) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        cache.update_memory_usage(mesh_mem, AssetType::Mesh);
        cache.update_memory_usage(texture_mem, AssetType::Texture);
        cache.update_memory_usage(audio_mem, AssetType::Audio);

        let stats = cache.stats();
        let expected_total = mesh_mem + texture_mem + audio_mem;

        prop_assert_eq!(stats.total_allocated, expected_total);
        prop_assert_eq!(stats.mesh_memory, mesh_mem);
        prop_assert_eq!(stats.texture_memory, texture_mem);
        prop_assert_eq!(stats.audio_memory, audio_mem);
    }

    /// Budget invariant: eviction candidates never include hard-referenced assets.
    #[test]
    fn prop_hard_refs_never_evicted(
        soft_count in 0usize..50,
        hard_count in 0usize..50,
    ) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);
        let registry = AssetRegistry::<MeshData>::new();
        let mesh = MeshData::cube();

        let mut hard_ids = Vec::new();
        let mut soft_ids = Vec::new();

        // Create hard refs
        for i in 0..hard_count {
            let id = AssetId::from_content(&[0, i as u8]);
            let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Hard);
            cache.access(id, AssetType::Mesh);
            hard_ids.push(id);
        }

        // Create soft refs
        for i in 0..soft_count {
            let id = AssetId::from_content(&[1, i as u8]);
            let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
            cache.access(id, AssetType::Mesh);
            soft_ids.push(id);
        }

        // Get eviction candidates
        let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 100);

        // INVARIANT: No hard refs in candidates
        for id in &candidates {
            prop_assert!(!hard_ids.contains(id), "Hard ref found in eviction candidates!");
        }

        // All candidates should be soft refs
        for id in &candidates {
            prop_assert!(soft_ids.contains(id) || !registry.contains(*id));
        }
    }

    /// Access order invariant: accessing an asset moves it to the back.
    #[test]
    fn prop_access_moves_to_back(access_pattern in prop::collection::vec(0u8..10, 10..50)) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        for &byte in &access_pattern {
            let id = AssetId::from_content(&[byte]);
            cache.access(id, AssetType::Mesh);
        }

        // Last accessed asset should be at back
        if let Some(&last_byte) = access_pattern.last() {
            let expected_id = AssetId::from_content(&[last_byte]);
            let lru = cache.mesh_lru.read();
            let actual_last = lru.keys().last().copied();
            prop_assert_eq!(actual_last, Some(expected_id));
        }
    }

    /// Budget check invariant: is_over_budget returns true iff usage > budget.
    #[test]
    fn prop_budget_check_correctness(usage_mb in 0usize..500, budget_mb in 1usize..500) {
        let mut budget = MemoryBudget::default();
        budget.mesh = budget_mb * 1024 * 1024;

        let cache = LruCache::new(budget);
        cache.update_memory_usage(usage_mb * 1024 * 1024, AssetType::Mesh);

        let is_over = cache.is_over_budget(AssetType::Mesh);
        let expected_over = usage_mb > budget_mb;

        prop_assert_eq!(is_over, expected_over,
            "Budget check mismatch: usage={}MB, budget={}MB, is_over={}, expected={}",
            usage_mb, budget_mb, is_over, expected_over);
    }

    /// Eviction count invariant: candidates.len() <= min(requested, available_soft_refs).
    #[test]
    fn prop_eviction_candidate_count(
        soft_count in 0usize..100,
        hard_count in 0usize..50,
        requested in 0usize..150,
    ) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);
        let registry = AssetRegistry::<MeshData>::new();
        let mesh = MeshData::cube();

        // Create hard refs
        for i in 0..hard_count {
            let id = AssetId::from_content(&[0, i as u8]);
            let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Hard);
            cache.access(id, AssetType::Mesh);
        }

        // Create soft refs
        for i in 0..soft_count {
            let id = AssetId::from_content(&[1, i as u8]);
            let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
            cache.access(id, AssetType::Mesh);
        }

        let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, requested);

        // INVARIANT: candidates.len() <= min(requested, soft_count)
        prop_assert!(candidates.len() <= requested);
        prop_assert!(candidates.len() <= soft_count);
    }

    /// Memory calculation invariant: size_bytes is always consistent.
    #[test]
    fn prop_mesh_size_consistency(vertex_count in 0usize..1000, index_count in 0usize..3000) {
        let mut mesh = MeshData::new();

        // Add vertices
        for _ in 0..vertex_count {
            mesh.vertices.push(engine_assets::Vertex::new(
                glam::Vec3::ZERO,
                glam::Vec3::Y,
                glam::Vec2::ZERO,
            ));
        }

        // Add indices
        for i in 0..index_count {
            mesh.indices.push(i as u32);
        }

        let size1 = mesh.size_bytes();
        let size2 = mesh.size_bytes(); // Should be consistent

        prop_assert_eq!(size1, size2, "size_bytes() not consistent");

        // Size should be at least the data size
        let min_size = std::mem::size_of::<MeshData>()
            + vertex_count * std::mem::size_of::<engine_assets::Vertex>()
            + index_count * std::mem::size_of::<u32>();

        prop_assert!(size1 >= min_size, "size_bytes() too small: {} < {}", size1, min_size);
    }

    /// Remove invariant: removing an asset from LRU doesn't affect other assets.
    #[test]
    fn prop_remove_independence(count in 2usize..50, remove_idx in 0usize..49) {
        let remove_idx = remove_idx % count; // Ensure valid index

        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let mut ids = Vec::new();
        for i in 0..count {
            let id = AssetId::from_content(&i.to_le_bytes());
            cache.access(id, AssetType::Mesh);
            ids.push(id);
        }

        let before_len = cache.mesh_lru.read().len();

        // Remove one asset
        cache.remove(ids[remove_idx], AssetType::Mesh);

        let after_len = cache.mesh_lru.read().len();

        // INVARIANT: exactly one fewer asset
        prop_assert_eq!(after_len, before_len - 1);

        // INVARIANT: all other assets still present
        let lru = cache.mesh_lru.read();
        for (i, id) in ids.iter().enumerate() {
            if i != remove_idx {
                prop_assert!(lru.contains_key(id), "Asset {} missing after removing {}", i, remove_idx);
            }
        }
    }

    /// Clear invariant: clear removes all tracked assets.
    #[test]
    fn prop_clear_removes_all(counts: [u8; 6]) {
        let budget = MemoryBudget::default();
        let cache = LruCache::new(budget);

        let types = [
            AssetType::Mesh,
            AssetType::Texture,
            AssetType::Shader,
            AssetType::Material,
            AssetType::Audio,
            AssetType::Font,
        ];

        // Add assets to each type
        for (i, &count) in counts.iter().enumerate() {
            for j in 0..count {
                let id = AssetId::from_content(&[i as u8, j]);
                cache.access(id, types[i]);
            }
        }

        cache.clear();

        // INVARIANT: all LRU lists are empty
        prop_assert_eq!(cache.mesh_lru.read().len(), 0);
        prop_assert_eq!(cache.texture_lru.read().len(), 0);
        prop_assert_eq!(cache.shader_lru.read().len(), 0);
        prop_assert_eq!(cache.material_lru.read().len(), 0);
        prop_assert_eq!(cache.audio_lru.read().len(), 0);
        prop_assert_eq!(cache.font_lru.read().len(), 0);

        // INVARIANT: stats are reset
        let stats = cache.stats();
        prop_assert_eq!(stats.total_allocated, 0);
    }
}
