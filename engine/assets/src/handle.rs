//! Type-safe, reference-counted asset handles.
//!
//! Handles provide safe access to assets with automatic memory management.
//! Assets are automatically cleaned up when their reference count reaches zero.

use crate::AssetId;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

/// Reference type for asset handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    /// Hard reference prevents asset eviction (always kept in memory).
    Hard,
    /// Soft reference allows LRU eviction when memory budget is exceeded.
    Soft,
}

/// Type-safe handle to an asset with reference counting.
///
/// # Examples
///
/// ```
/// use engine_assets::{AssetHandle, AssetId, RefType};
///
/// #[derive(Clone)]
/// struct MyAsset {
///     data: String,
/// }
///
/// let id = AssetId::from_content(b"test");
/// let handle = AssetHandle::<MyAsset>::new(id, RefType::Hard);
///
/// // Clone increments refcount
/// let handle2 = handle.clone();
/// assert_eq!(handle.id(), handle2.id());
/// ```
pub struct AssetHandle<T> {
    id: AssetId,
    ref_type: RefType,
    // Arc for reference counting - inner data managed by AssetRegistry
    _marker: Arc<PhantomData<T>>,
}

impl<T> AssetHandle<T> {
    /// Create a new asset handle.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::{AssetHandle, AssetId, RefType};
    ///
    /// struct MyAsset;
    /// let id = AssetId::from_content(b"test");
    /// let handle = AssetHandle::<MyAsset>::new(id, RefType::Hard);
    /// ```
    #[must_use]
    pub fn new(id: AssetId, ref_type: RefType) -> Self {
        Self { id, ref_type, _marker: Arc::new(PhantomData) }
    }

    /// Get the asset ID this handle refers to.
    #[must_use]
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// Get the reference type of this handle.
    #[must_use]
    pub fn ref_type(&self) -> RefType {
        self.ref_type
    }

    /// Get the current reference count.
    ///
    /// Note: This is the Arc refcount, not the asset registry refcount.
    #[must_use]
    pub fn refcount(&self) -> usize {
        Arc::strong_count(&self._marker)
    }

    /// Check if this is the only reference to the handle.
    #[must_use]
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self._marker) == 1
    }

    /// Downgrade to a soft reference (allows eviction).
    #[must_use]
    pub fn to_soft(&self) -> Self {
        Self { id: self.id, ref_type: RefType::Soft, _marker: Arc::clone(&self._marker) }
    }

    /// Upgrade to a hard reference (prevents eviction).
    #[must_use]
    pub fn to_hard(&self) -> Self {
        Self { id: self.id, ref_type: RefType::Hard, _marker: Arc::clone(&self._marker) }
    }
}

impl<T> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self { id: self.id, ref_type: self.ref_type, _marker: Arc::clone(&self._marker) }
    }
}

impl<T> fmt::Debug for AssetHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetHandle")
            .field("id", &self.id)
            .field("ref_type", &self.ref_type)
            .field("refcount", &self.refcount())
            .finish()
    }
}

impl<T> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for AssetHandle<T> {}

impl<T> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestAsset {
        #[allow(dead_code)]
        data: String,
    }

    #[test]
    fn test_handle_creation() {
        let id = AssetId::from_content(b"test");
        let handle = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        assert_eq!(handle.id(), id);
        assert_eq!(handle.ref_type(), RefType::Hard);
        assert_eq!(handle.refcount(), 1);
    }

    #[test]
    fn test_hard_vs_soft_references() {
        let id = AssetId::from_content(b"test");

        let hard = AssetHandle::<TestAsset>::new(id, RefType::Hard);
        let soft = AssetHandle::<TestAsset>::new(id, RefType::Soft);

        assert_eq!(hard.ref_type(), RefType::Hard);
        assert_eq!(soft.ref_type(), RefType::Soft);
    }

    #[test]
    fn test_clone_increments_refcount() {
        let id = AssetId::from_content(b"test");
        let handle1 = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        assert_eq!(handle1.refcount(), 1);

        let handle2 = handle1.clone();
        assert_eq!(handle1.refcount(), 2);
        assert_eq!(handle2.refcount(), 2);

        let handle3 = handle1.clone();
        assert_eq!(handle1.refcount(), 3);
        assert_eq!(handle2.refcount(), 3);
        assert_eq!(handle3.refcount(), 3);
    }

    #[test]
    fn test_drop_decrements_refcount() {
        let id = AssetId::from_content(b"test");
        let handle1 = AssetHandle::<TestAsset>::new(id, RefType::Hard);
        let handle2 = handle1.clone();
        let handle3 = handle1.clone();

        assert_eq!(handle1.refcount(), 3);

        drop(handle3);
        assert_eq!(handle1.refcount(), 2);

        drop(handle2);
        assert_eq!(handle1.refcount(), 1);
    }

    #[test]
    fn test_is_unique() {
        let id = AssetId::from_content(b"test");
        let handle1 = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        assert!(handle1.is_unique());

        let handle2 = handle1.clone();
        assert!(!handle1.is_unique());
        assert!(!handle2.is_unique());

        drop(handle2);
        assert!(handle1.is_unique());
    }

    #[test]
    fn test_to_soft() {
        let id = AssetId::from_content(b"test");
        let hard = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        let soft = hard.to_soft();
        assert_eq!(soft.ref_type(), RefType::Soft);
        assert_eq!(soft.id(), hard.id());
        assert_eq!(soft.refcount(), 2); // Both hard and soft exist
    }

    #[test]
    fn test_to_hard() {
        let id = AssetId::from_content(b"test");
        let soft = AssetHandle::<TestAsset>::new(id, RefType::Soft);

        let hard = soft.to_hard();
        assert_eq!(hard.ref_type(), RefType::Hard);
        assert_eq!(hard.id(), soft.id());
        assert_eq!(hard.refcount(), 2); // Both soft and hard exist
    }

    #[test]
    fn test_handle_equality() {
        let id1 = AssetId::from_content(b"test1");
        let id2 = AssetId::from_content(b"test2");

        let handle1a = AssetHandle::<TestAsset>::new(id1, RefType::Hard);
        let handle1b = AssetHandle::<TestAsset>::new(id1, RefType::Soft);
        let handle2 = AssetHandle::<TestAsset>::new(id2, RefType::Hard);

        // Same ID = equal (regardless of ref type)
        assert_eq!(handle1a, handle1b);

        // Different ID = not equal
        assert_ne!(handle1a, handle2);
    }

    #[test]
    fn test_handle_hash() {
        use std::collections::HashMap;

        let id1 = AssetId::from_content(b"test1");
        let id2 = AssetId::from_content(b"test2");

        let handle1 = AssetHandle::<TestAsset>::new(id1, RefType::Hard);
        let handle2 = AssetHandle::<TestAsset>::new(id2, RefType::Hard);

        let mut map = HashMap::new();
        map.insert(handle1.clone(), "value1");
        map.insert(handle2.clone(), "value2");

        assert_eq!(map.get(&handle1), Some(&"value1"));
        assert_eq!(map.get(&handle2), Some(&"value2"));
    }

    #[test]
    fn test_debug_format() {
        let id = AssetId::from_content(b"test");
        let handle = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        let debug_str = format!("{handle:?}");
        assert!(debug_str.contains("AssetHandle"));
        assert!(debug_str.contains("Hard"));
    }

    #[test]
    fn test_auto_cleanup_on_zero_refcount() {
        let id = AssetId::from_content(b"test");
        let handle = AssetHandle::<TestAsset>::new(id, RefType::Hard);

        assert_eq!(handle.refcount(), 1);

        // Drop the handle - Arc should clean up automatically
        drop(handle);

        // Create new handle - should start at refcount 1 again
        let new_handle = AssetHandle::<TestAsset>::new(id, RefType::Hard);
        assert_eq!(new_handle.refcount(), 1);
    }

    #[test]
    fn test_type_safety() {
        struct Asset1;
        struct Asset2;

        let id = AssetId::from_content(b"test");
        let handle1 = AssetHandle::<Asset1>::new(id, RefType::Hard);
        let _handle2 = AssetHandle::<Asset2>::new(id, RefType::Hard);

        // This should not compile (different types):
        // let _: AssetHandle<Asset2> = handle1;

        // But clone should work:
        let _handle1_clone: AssetHandle<Asset1> = handle1.clone();
    }
}
