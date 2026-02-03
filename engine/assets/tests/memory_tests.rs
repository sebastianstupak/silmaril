//! Integration tests for memory management and LRU eviction.
//!
//! Tests the interaction between LruCache, AssetRegistry, and memory budgets.

use engine_assets::{
    AssetId, AssetRegistry, AssetType, AudioData, LruCache, MemoryBudget, MemorySized, MeshData,
    RefType, TextureData,
};

#[test]
fn test_lru_eviction_order() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id1 = AssetId::from_content(b"mesh1");
    let id2 = AssetId::from_content(b"mesh2");
    let id3 = AssetId::from_content(b"mesh3");

    // Insert all as soft references
    let mesh = MeshData::cube();
    let _h1 = registry.insert_with_reftype(id1, mesh.clone(), RefType::Soft);
    let _h2 = registry.insert_with_reftype(id2, mesh.clone(), RefType::Soft);
    let _h3 = registry.insert_with_reftype(id3, mesh.clone(), RefType::Soft);

    // Access in order: 1, 2, 3
    cache.access(id1, AssetType::Mesh);
    cache.access(id2, AssetType::Mesh);
    cache.access(id3, AssetType::Mesh);

    // Get eviction candidates - should be in order 1, 2, 3 (oldest first)
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 3);
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], id1); // Oldest
    assert_eq!(candidates[1], id2);
    assert_eq!(candidates[2], id3); // Newest
}

#[test]
fn test_hard_references_prevent_eviction() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id1 = AssetId::from_content(b"mesh1");
    let id2 = AssetId::from_content(b"mesh2");
    let id3 = AssetId::from_content(b"mesh3");

    // Insert with mixed ref types
    let mesh = MeshData::cube();
    let _h1 = registry.insert_with_reftype(id1, mesh.clone(), RefType::Soft);
    let _h2 = registry.insert_with_reftype(id2, mesh.clone(), RefType::Hard); // Hard ref
    let _h3 = registry.insert_with_reftype(id3, mesh.clone(), RefType::Soft);

    cache.access(id1, AssetType::Mesh);
    cache.access(id2, AssetType::Mesh);
    cache.access(id3, AssetType::Mesh);

    // Get candidates - should skip hard-referenced id2
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 10);
    assert_eq!(candidates.len(), 2);
    assert!(!candidates.contains(&id2)); // Hard ref not included
}

#[test]
fn test_soft_references_allow_eviction() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id = AssetId::from_content(b"mesh");
    let mesh = MeshData::cube();
    let _handle = registry.insert_with_reftype(id, mesh, RefType::Soft);

    cache.access(id, AssetType::Mesh);

    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 1);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], id); // Soft ref can be evicted
}

#[test]
fn test_memory_tracking_accuracy() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Update mesh memory
    cache.update_memory_usage(100_000, AssetType::Mesh);
    let stats = cache.stats();
    assert_eq!(stats.mesh_memory, 100_000);
    assert_eq!(stats.total_allocated, 100_000);

    // Update texture memory
    cache.update_memory_usage(200_000, AssetType::Texture);
    let stats = cache.stats();
    assert_eq!(stats.texture_memory, 200_000);
    assert_eq!(stats.total_allocated, 300_000);
}

#[test]
fn test_budget_enforcement_triggers_eviction_warning() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Exceed mesh budget (100 MB)
    cache.update_memory_usage(150 * 1024 * 1024, AssetType::Mesh);

    assert!(cache.is_over_budget(AssetType::Mesh));
    assert_eq!(cache.memory_to_free(AssetType::Mesh), 50 * 1024 * 1024);
}

#[test]
fn test_evict_reload_cycle() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id = AssetId::from_content(b"mesh");
    let mesh = MeshData::cube();

    // Load
    let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
    cache.access(id, AssetType::Mesh);

    assert!(registry.contains(id));

    // Evict
    registry.remove(id);
    cache.remove(id, AssetType::Mesh);

    assert!(!registry.contains(id));

    // Reload
    let _handle2 = registry.insert_with_reftype(id, mesh, RefType::Soft);
    cache.access(id, AssetType::Mesh);

    assert!(registry.contains(id));
}

#[test]
fn test_hard_ref_prevents_eviction_even_when_over_budget() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id = AssetId::from_content(b"mesh");
    let mesh = MeshData::cube();

    // Insert with hard ref
    let _handle = registry.insert_with_reftype(id, mesh, RefType::Hard);
    cache.access(id, AssetType::Mesh);

    // Exceed budget
    cache.update_memory_usage(200 * 1024 * 1024, AssetType::Mesh);

    // Even though over budget, hard ref prevents eviction
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 10);
    assert!(candidates.is_empty());
}

#[test]
fn test_mixed_hard_and_soft_references() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let mesh = MeshData::cube();

    // Create multiple assets with mixed ref types
    for i in 0..10 {
        let id = AssetId::from_content(&[i]);
        let ref_type = if i % 2 == 0 { RefType::Hard } else { RefType::Soft };
        let _handle = registry.insert_with_reftype(id, mesh.clone(), ref_type);
        cache.access(id, AssetType::Mesh);
    }

    // Get candidates - should only return soft refs (5 total)
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 100);
    assert_eq!(candidates.len(), 5);

    // Verify all candidates are soft refs
    for id in candidates {
        assert!(!registry.is_hard_referenced(id));
    }
}

#[test]
fn test_bulk_eviction() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let mesh = MeshData::cube();

    // Load 100 assets
    let mut ids = Vec::new();
    for i in 0..100 {
        let id = AssetId::from_content(&i.to_le_bytes());
        let _handle = registry.insert_with_reftype(id, mesh.clone(), RefType::Soft);
        cache.access(id, AssetType::Mesh);
        ids.push(id);
    }

    assert_eq!(registry.len(), 100);

    // Get candidates for bulk eviction
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 50);
    assert_eq!(candidates.len(), 50);

    // Evict
    for id in &candidates {
        registry.remove(*id);
        cache.remove(*id, AssetType::Mesh);
    }

    assert_eq!(registry.len(), 50);
}

#[test]
fn test_memory_leak_detection() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let id = AssetId::from_content(b"mesh");
    let mesh = MeshData::cube();

    {
        let _handle = registry.insert_with_reftype(id, mesh, RefType::Soft);
        cache.access(id, AssetType::Mesh);
        // Handle dropped here
    }

    // Asset should still exist in registry (handles don't auto-cleanup)
    assert!(registry.contains(id));
}

#[test]
fn test_lru_access_operation_performance() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    let id = AssetId::from_content(b"mesh");

    // Access should be fast (< 1us typically)
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        cache.access(id, AssetType::Mesh);
    }
    let elapsed = start.elapsed();

    // 1000 accesses should be < 1ms
    assert!(elapsed.as_millis() < 10, "LRU access too slow: {:?}", elapsed);
}

#[test]
fn test_memory_usage_calculation() {
    let mesh = MeshData::cube();
    let size = mesh.size_bytes();

    // Cube has 24 vertices, 36 indices
    let expected_min = std::mem::size_of::<MeshData>()
        + 24 * std::mem::size_of::<engine_assets::Vertex>()
        + 36 * std::mem::size_of::<u32>();

    assert!(size >= expected_min, "Memory size calculation incorrect");
}

#[test]
fn test_texture_memory_sizing() {
    let texture = TextureData::solid_color([255, 0, 0, 255], 1024, 1024);
    let size = texture.size_bytes();

    // 1024x1024 RGBA8 should be at least 4MB
    assert!(size >= 1024 * 1024 * 4);
}

#[test]
fn test_audio_memory_sizing() {
    use engine_assets::AudioFormat;

    // 1 second of 16-bit PCM audio at 44.1kHz mono
    let data = vec![0u8; 44100 * 2]; // 2 bytes per sample
    let audio = AudioData::new(44100, 1, AudioFormat::PCM16, data);
    let size = audio.size_bytes();

    // Should include sample data
    assert!(size >= 44100 * 2);
}

#[test]
fn test_per_type_lru_isolation() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    let mesh_id = AssetId::from_content(b"mesh");
    let texture_id = AssetId::from_content(b"texture");

    cache.access(mesh_id, AssetType::Mesh);
    cache.access(texture_id, AssetType::Texture);

    // Mesh LRU should only have mesh
    let mesh_lru = cache.mesh_lru.read();
    assert_eq!(mesh_lru.len(), 1);
    assert!(mesh_lru.contains_key(&mesh_id));
    assert!(!mesh_lru.contains_key(&texture_id));

    // Texture LRU should only have texture
    let texture_lru = cache.texture_lru.read();
    assert_eq!(texture_lru.len(), 1);
    assert!(texture_lru.contains_key(&texture_id));
    assert!(!texture_lru.contains_key(&mesh_id));
}

#[test]
fn test_budget_check_operation_performance() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    cache.update_memory_usage(50 * 1024 * 1024, AssetType::Mesh);

    let start = std::time::Instant::now();
    for _ in 0..100_000 {
        let _ = cache.is_over_budget(AssetType::Mesh);
    }
    let elapsed = start.elapsed();

    // 100k checks should be < 10ms
    assert!(elapsed.as_millis() < 10, "Budget check too slow: {:?}", elapsed);
}

#[test]
fn test_eviction_candidate_ordering() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);
    let registry = AssetRegistry::<MeshData>::new();

    let mesh = MeshData::cube();
    let ids: Vec<AssetId> = (0..5).map(|i| AssetId::from_content(&[i])).collect();

    // Insert and access in order
    for id in &ids {
        let _handle = registry.insert_with_reftype(*id, mesh.clone(), RefType::Soft);
        cache.access(*id, AssetType::Mesh);
    }

    // Access middle one again
    cache.access(ids[2], AssetType::Mesh);

    // Get candidates - ids[2] should be last (most recent)
    let candidates = cache.eviction_candidates(AssetType::Mesh, &registry, 5);
    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[4], ids[2]); // Most recently accessed last
}
