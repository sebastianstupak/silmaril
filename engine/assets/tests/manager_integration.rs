//! Integration tests for AssetManager.

use engine_assets::{AssetId, AssetManager, LruCache, MemoryBudget, MeshData};
use std::sync::Arc;

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
    let id = AssetId::from_content(b"test_cube");
    let _handle = manager.meshes().insert(id, mesh);

    assert_eq!(manager.len(), 1);
    assert!(!manager.is_empty());

    manager.clear();
    assert_eq!(manager.len(), 0);
    assert!(manager.is_empty());
}

#[test]
fn test_mesh_insertion_and_retrieval() {
    let manager = AssetManager::new();

    // Create and insert a mesh
    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"cube");
    let handle = manager.meshes().insert(id, mesh);

    // Retrieve the mesh
    let retrieved = manager.get_mesh(handle.id()).expect("Mesh should exist");
    assert_eq!(retrieved.vertices.len(), 24); // Cube has 24 vertices (6 faces × 4 vertices)
}

#[test]
fn test_multiple_asset_types() {
    let manager = AssetManager::new();

    // Insert different asset types
    let mesh = MeshData::cube();
    let mesh_id = AssetId::from_content(b"mesh");
    let _mesh_handle = manager.meshes().insert(mesh_id, mesh);

    assert_eq!(manager.len(), 1);
}

#[test]
fn test_handle_refcounting() {
    let manager = AssetManager::new();

    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"cube");
    let handle1 = manager.meshes().insert(id, mesh);

    // Clone the handle
    let handle2 = handle1.clone();

    // Both handles should point to the same asset
    assert_eq!(handle1.id(), handle2.id());
    assert_eq!(handle1.refcount(), 2);
    assert_eq!(handle2.refcount(), 2);

    drop(handle2);
    assert_eq!(handle1.refcount(), 1);
}

#[test]
fn test_lru_cache_basic() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    let id1 = AssetId::from_content(b"asset1");
    let id2 = AssetId::from_content(b"asset2");

    // Access assets in order
    cache.access(id1, engine_assets::AssetType::Mesh);
    cache.access(id2, engine_assets::AssetType::Mesh);

    let stats = cache.stats();
    assert_eq!(stats.total_allocated, 0); // No memory usage tracked yet
}

#[test]
fn test_lru_cache_memory_tracking() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Update memory usage
    cache.update_memory_usage(1_000_000, engine_assets::AssetType::Mesh);

    let stats = cache.stats();
    assert_eq!(stats.mesh_memory, 1_000_000);
    assert_eq!(stats.total_allocated, 1_000_000);
}

#[test]
fn test_lru_access_order() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    let id1 = AssetId::from_content(b"asset1");
    let id2 = AssetId::from_content(b"asset2");
    let id3 = AssetId::from_content(b"asset3");

    // Access in order: 1, 2, 3
    cache.access(id1, engine_assets::AssetType::Mesh);
    cache.access(id2, engine_assets::AssetType::Mesh);
    cache.access(id3, engine_assets::AssetType::Mesh);

    // Access 1 again (should move to front)
    cache.access(id1, engine_assets::AssetType::Mesh);

    // id2 should be the least recently used now
}

#[test]
fn test_manager_with_lru() {
    let manager = Arc::new(AssetManager::new());
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Insert some meshes
    let mesh1 = MeshData::cube();
    let id1 = AssetId::from_content(b"cube1");
    let handle1 = manager.meshes().insert(id1, mesh1);

    let mesh2 = MeshData::cube();
    let id2 = AssetId::from_content(b"cube2");
    let handle2 = manager.meshes().insert(id2, mesh2);

    // Track access
    cache.access(handle1.id(), engine_assets::AssetType::Mesh);
    cache.access(handle2.id(), engine_assets::AssetType::Mesh);

    // Verify both are tracked
    assert_eq!(manager.len(), 2);
}

#[test]
fn test_concurrent_access() {
    use std::thread;

    let manager = Arc::new(AssetManager::new());
    let mut handles = vec![];

    // Spawn multiple threads to access assets concurrently
    for i in 0u64..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mesh = MeshData::cube();
            let id = AssetId::from_content(&i.to_le_bytes());
            manager_clone.meshes().insert(id, mesh);
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(manager.len(), 10);
}

#[test]
fn test_memory_budget_exceeded_warning() {
    let budget = MemoryBudget {
        total: 1024,  // 1 KB total
        mesh: 512,    // 512 bytes for meshes
        texture: 512, // 512 bytes for textures
        shader: 100,
        material: 100,
        audio: 100,
        font: 100,
    };

    let cache = LruCache::new(budget);

    // Exceed mesh budget
    cache.update_memory_usage(1000, engine_assets::AssetType::Mesh);

    let stats = cache.stats();
    assert_eq!(stats.mesh_memory, 1000);
    assert!(stats.mesh_memory > budget.mesh);
}

#[test]
fn test_lru_clear() {
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    let id1 = AssetId::from_content(b"asset1");
    cache.access(id1, engine_assets::AssetType::Mesh);
    cache.update_memory_usage(1000, engine_assets::AssetType::Mesh);

    cache.clear();

    let stats = cache.stats();
    assert_eq!(stats.total_allocated, 0);
}

#[test]
fn test_asset_type_from_extension() {
    use engine_assets::AssetType;

    assert_eq!(AssetType::from_extension("obj"), Some(AssetType::Mesh));
    assert_eq!(AssetType::from_extension("gltf"), Some(AssetType::Mesh));
    assert_eq!(AssetType::from_extension("png"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("jpg"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("glsl"), Some(AssetType::Shader));
    assert_eq!(AssetType::from_extension("wav"), Some(AssetType::Audio));
    assert_eq!(AssetType::from_extension("ttf"), Some(AssetType::Font));
    assert_eq!(AssetType::from_extension("unknown"), None);
}

#[test]
fn test_manager_default() {
    let manager1 = AssetManager::new();
    let manager2 = AssetManager::default();

    assert_eq!(manager1.len(), manager2.len());
}

#[test]
fn test_hard_reference_prevents_eviction() {
    use engine_assets::RefType;

    let manager = AssetManager::new();
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Insert with hard reference
    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"hard_ref_mesh");
    let handle = manager.meshes().insert_with_reftype(id, mesh, RefType::Hard);

    // Track in LRU
    cache.access(handle.id(), engine_assets::AssetType::Mesh);

    // Hard reference should prevent eviction
    assert!(manager.meshes().is_hard_referenced(handle.id()));

    // Try to get eviction candidates (should be empty for hard refs)
    let candidates =
        cache.eviction_candidates(engine_assets::AssetType::Mesh, manager.meshes().as_ref(), 10);
    assert!(candidates.is_empty());
}

#[test]
fn test_soft_reference_allows_eviction() {
    use engine_assets::RefType;

    let manager = AssetManager::new();
    let budget = MemoryBudget::default();
    let cache = LruCache::new(budget);

    // Insert with soft reference
    let mesh = MeshData::cube();
    let id = AssetId::from_content(b"soft_ref_mesh");
    let handle = manager.meshes().insert_with_reftype(id, mesh, RefType::Soft);

    // Track in LRU
    cache.access(handle.id(), engine_assets::AssetType::Mesh);

    // Soft reference should allow eviction
    assert!(!manager.meshes().is_hard_referenced(handle.id()));

    // Should appear in eviction candidates
    let candidates =
        cache.eviction_candidates(engine_assets::AssetType::Mesh, manager.meshes().as_ref(), 10);
    assert!(candidates.contains(&handle.id()));
}
