//! Thread-safe per-type asset storage with reference counting.
//!
//! AssetRegistry stores assets of a single type with automatic reference counting
//! and memory tracking.

use crate::{AssetHandle, AssetId, RefType};
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Metadata for a stored asset.
#[derive(Debug)]
struct AssetMetadata {
    hard_refcount: AtomicUsize,
    soft_refcount: AtomicUsize,
}

impl AssetMetadata {
    fn new() -> Self {
        Self { hard_refcount: AtomicUsize::new(0), soft_refcount: AtomicUsize::new(0) }
    }

    fn increment(&self, ref_type: RefType) {
        match ref_type {
            RefType::Hard => {
                self.hard_refcount.fetch_add(1, Ordering::Relaxed);
            }
            RefType::Soft => {
                self.soft_refcount.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn decrement(&self, ref_type: RefType) -> bool {
        let hard = match ref_type {
            RefType::Hard => self.hard_refcount.fetch_sub(1, Ordering::Relaxed) - 1,
            RefType::Soft => self.hard_refcount.load(Ordering::Relaxed),
        };
        let soft = match ref_type {
            RefType::Soft => self.soft_refcount.fetch_sub(1, Ordering::Relaxed) - 1,
            RefType::Hard => self.soft_refcount.load(Ordering::Relaxed),
        };

        hard == 0 && soft == 0
    }

    fn total_refcount(&self) -> usize {
        self.hard_refcount.load(Ordering::Relaxed) + self.soft_refcount.load(Ordering::Relaxed)
    }

    fn is_hard_referenced(&self) -> bool {
        self.hard_refcount.load(Ordering::Relaxed) > 0
    }
}

/// Thread-safe registry for assets of type T.
///
/// # Examples
///
/// ```
/// use engine_assets::{AssetRegistry, AssetId, RefType};
///
/// #[derive(Clone)]
/// struct MyAsset {
///     data: String,
/// }
///
/// let registry = AssetRegistry::<MyAsset>::new();
/// let id = AssetId::from_content(b"test");
///
/// let asset = MyAsset { data: "hello".to_string() };
/// let handle = registry.insert(id, asset);
///
/// assert!(registry.contains(id));
/// ```
pub struct AssetRegistry<T> {
    assets: DashMap<AssetId, (T, Arc<AssetMetadata>)>,
}

impl<T> AssetRegistry<T> {
    /// Create a new empty asset registry.
    #[must_use]
    pub fn new() -> Self {
        Self { assets: DashMap::new() }
    }

    /// Insert an asset and return a handle to it.
    ///
    /// If an asset with this ID already exists, it will be replaced.
    pub fn insert(&self, id: AssetId, asset: T) -> AssetHandle<T> {
        self.insert_with_reftype(id, asset, RefType::Hard)
    }

    /// Insert an asset with a specific reference type.
    pub fn insert_with_reftype(&self, id: AssetId, asset: T, ref_type: RefType) -> AssetHandle<T> {
        let metadata = Arc::new(AssetMetadata::new());
        metadata.increment(ref_type);

        self.assets.insert(id, (asset, Arc::clone(&metadata)));

        AssetHandle::new(id, ref_type)
    }

    /// Get an asset by ID.
    ///
    /// Returns None if the asset doesn't exist.
    pub fn get(&self, id: AssetId) -> Option<impl std::ops::Deref<Target = T> + '_> {
        self.assets
            .get(&id)
            .map(|entry| dashmap::mapref::one::Ref::map(entry, |(asset, _)| asset))
    }

    /// Get a mutable reference to an asset by ID.
    ///
    /// Returns None if the asset doesn't exist.
    pub fn get_mut(&self, id: AssetId) -> Option<impl std::ops::DerefMut<Target = T> + '_> {
        self.assets
            .get_mut(&id)
            .map(|entry| dashmap::mapref::one::RefMut::map(entry, |(asset, _)| asset))
    }

    /// Check if an asset with the given ID exists.
    #[must_use]
    pub fn contains(&self, id: AssetId) -> bool {
        self.assets.contains_key(&id)
    }

    /// Remove an asset by ID.
    ///
    /// Returns the asset if it existed.
    pub fn remove(&self, id: AssetId) -> Option<T> {
        self.assets.remove(&id).map(|(_, (asset, _))| asset)
    }

    /// Get the number of assets in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Get the total reference count for an asset.
    #[must_use]
    pub fn refcount(&self, id: AssetId) -> usize {
        self.assets.get(&id).map(|entry| entry.value().1.total_refcount()).unwrap_or(0)
    }

    /// Check if an asset is hard-referenced (cannot be evicted).
    #[must_use]
    pub fn is_hard_referenced(&self, id: AssetId) -> bool {
        self.assets
            .get(&id)
            .map(|entry| entry.value().1.is_hard_referenced())
            .unwrap_or(false)
    }

    /// Increment reference count for an asset.
    pub fn increment_refcount(&self, id: AssetId, ref_type: RefType) {
        if let Some(entry) = self.assets.get(&id) {
            entry.value().1.increment(ref_type);
        }
    }

    /// Decrement reference count for an asset.
    ///
    /// Returns true if the asset should be removed (refcount reached 0).
    pub fn decrement_refcount(&self, id: AssetId, ref_type: RefType) -> bool {
        self.assets
            .get(&id)
            .map(|entry| entry.value().1.decrement(ref_type))
            .unwrap_or(false)
    }

    /// Iterate over all asset IDs.
    pub fn iter_ids(&self) -> impl Iterator<Item = AssetId> + '_ {
        self.assets.iter().map(|entry| *entry.key())
    }

    /// Clear all assets from the registry.
    pub fn clear(&self) {
        self.assets.clear();
    }
}

impl<T> Default for AssetRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestAsset {
        data: String,
    }

    #[test]
    fn test_insert_and_get() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");
        let asset = TestAsset { data: "hello".to_string() };

        let _handle = registry.insert(id, asset.clone());

        assert!(registry.contains(id));

        let retrieved = registry.get(id).expect("Asset should exist");
        assert_eq!(retrieved.data, asset.data);
    }

    #[test]
    fn test_get_mut() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");
        let asset = TestAsset { data: "hello".to_string() };

        let _handle = registry.insert(id, asset);

        {
            let mut retrieved = registry.get_mut(id).expect("Asset should exist");
            retrieved.data = "modified".to_string();
        }

        let retrieved = registry.get(id).expect("Asset should exist");
        assert_eq!(retrieved.data, "modified");
    }

    #[test]
    fn test_remove() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");
        let asset = TestAsset { data: "hello".to_string() };

        let _handle = registry.insert(id, asset.clone());
        assert!(registry.contains(id));

        let removed = registry.remove(id).expect("Asset should exist");
        assert_eq!(removed.data, asset.data);
        assert!(!registry.contains(id));
    }

    #[test]
    fn test_len_and_is_empty() {
        let registry = AssetRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        let id1 = AssetId::from_content(b"test1");
        let id2 = AssetId::from_content(b"test2");

        let _h1 = registry.insert(id1, TestAsset { data: "1".to_string() });
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let _h2 = registry.insert(id2, TestAsset { data: "2".to_string() });
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_refcount_tracking() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");
        let asset = TestAsset { data: "hello".to_string() };

        let _handle = registry.insert(id, asset);
        assert_eq!(registry.refcount(id), 1);

        registry.increment_refcount(id, RefType::Hard);
        assert_eq!(registry.refcount(id), 2);

        registry.increment_refcount(id, RefType::Soft);
        assert_eq!(registry.refcount(id), 3);

        let should_remove = registry.decrement_refcount(id, RefType::Hard);
        assert!(!should_remove);
        assert_eq!(registry.refcount(id), 2);
    }

    #[test]
    fn test_hard_vs_soft_refcount() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");
        let asset = TestAsset { data: "hello".to_string() };

        // Insert with hard reference
        let _handle = registry.insert_with_reftype(id, asset, RefType::Hard);
        assert!(registry.is_hard_referenced(id));

        // Add soft reference
        registry.increment_refcount(id, RefType::Soft);
        assert!(registry.is_hard_referenced(id)); // Still hard referenced

        // Remove hard reference
        registry.decrement_refcount(id, RefType::Hard);
        assert!(!registry.is_hard_referenced(id)); // No longer hard referenced
    }

    #[test]
    fn test_iter_ids() {
        let registry = AssetRegistry::new();

        let id1 = AssetId::from_content(b"test1");
        let id2 = AssetId::from_content(b"test2");
        let id3 = AssetId::from_content(b"test3");

        let _h1 = registry.insert(id1, TestAsset { data: "1".to_string() });
        let _h2 = registry.insert(id2, TestAsset { data: "2".to_string() });
        let _h3 = registry.insert(id3, TestAsset { data: "3".to_string() });

        let ids: Vec<AssetId> = registry.iter_ids().collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_clear() {
        let registry = AssetRegistry::new();

        let id1 = AssetId::from_content(b"test1");
        let id2 = AssetId::from_content(b"test2");

        let _h1 = registry.insert(id1, TestAsset { data: "1".to_string() });
        let _h2 = registry.insert(id2, TestAsset { data: "2".to_string() });

        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let registry = Arc::new(AssetRegistry::new());
        let mut handles = vec![];

        // Spawn multiple threads to insert assets concurrently
        for i in 0..10u32 {
            let registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let id = AssetId::from_content(&i.to_le_bytes());
                registry_clone.insert(id, TestAsset { data: format!("thread {i}") });
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(registry.len(), 10);
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = AssetRegistry::<TestAsset>::new();
        let id = AssetId::from_content(b"nonexistent");

        assert!(registry.get(id).is_none());
        assert!(registry.get_mut(id).is_none());
        assert_eq!(registry.refcount(id), 0);
    }

    #[test]
    fn test_replace_asset() {
        let registry = AssetRegistry::new();
        let id = AssetId::from_content(b"test");

        let _h1 = registry.insert(id, TestAsset { data: "first".to_string() });

        {
            let asset = registry.get(id).expect("Asset should exist");
            assert_eq!(asset.data, "first");
        }

        // Insert again with same ID (replaces)
        let _h2 = registry.insert(id, TestAsset { data: "second".to_string() });

        let asset = registry.get(id).expect("Asset should exist");
        assert_eq!(asset.data, "second");
    }
}
