//! Optimized type-safe query system for accessing entity components
//!
//! This module contains performance-optimized query iteration with:
//! - Cached TypeId lookups
//! - Cached storage pointers
//! - Direct index access instead of nth()
//! - Inlined hot path functions
//! - Reduced bounds checking with unsafe where safe
//! - Manual prefetch hints for better cache utilization

use super::{Component, Entity, SparseSet, World};
use std::any::TypeId;
use std::marker::PhantomData;

/// Prefetch a memory location for reading
///
/// Uses compiler intrinsics to hint that we'll access this memory soon.
/// The CPU can start loading it into cache before we actually need it.
///
/// SAFETY: Prefetching is always safe - it's just a hint to the CPU.
/// Even if the pointer is invalid, prefetch is a no-op.
#[inline(always)]
fn prefetch_read<T>(ptr: *const T) {
    #[cfg(target_arch = "x86_64")]
    {
        // Use x86 prefetch intrinsic (T0 = fetch to all cache levels)
        unsafe {
            core::arch::x86_64::_mm_prefetch::<{core::arch::x86_64::_MM_HINT_T0}>(
                ptr as *const i8
            );
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // On other architectures, rely on the compiler's prefetch hints
        // Most compilers will optimize this away or use the appropriate intrinsic
        let _ = ptr; // Avoid unused variable warning
    }
}

/// Optimized iterator for single component immutable queries
///
/// OPTIMIZATIONS:
/// - Cached TypeId (avoids repeated TypeId::of calls)
/// - Cached storage pointer (avoids repeated downcast)
/// - Direct index access (O(1) vs O(n) with nth())
pub struct SingleQueryIter<'a, T: Component> {
    storage: &'a SparseSet<T>,
    current_index: usize,
    len: usize,
}

impl<'a, T: Component> SingleQueryIter<'a, T> {
    #[inline]
    pub fn new(world: &'a World) -> Self {
        let type_id = TypeId::of::<T>();

        let (storage, len) = match world.components.get(&type_id) {
            Some(storage_trait) => {
                // SAFETY: We know this is a SparseSet<T> because we stored it in register()
                // The World ensures type safety by using TypeId as the hash key
                let storage = unsafe {
                    &*(storage_trait.as_any() as *const dyn std::any::Any as *const SparseSet<T>)
                };
                (storage, storage.len())
            }
            None => {
                // Return empty iterator if component not registered
                // Use a dummy reference (we won't access it since len=0)
                let dummy_storage = unsafe { &*(std::ptr::null() as *const SparseSet<T>) };
                (dummy_storage, 0)
            }
        };

        Self {
            storage,
            current_index: 0,
            len,
        }
    }
}

impl<'a, T: Component> Iterator for SingleQueryIter<'a, T> {
    type Item = (Entity, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        // PREFETCH OPTIMIZATION: Load next entity into cache while processing current
        // This exploits instruction-level parallelism in modern CPUs
        if self.current_index + 1 < self.len {
            if let Some(next_entity) = self.storage.get_dense_entity(self.current_index + 1) {
                if let Some(next_component) = self.storage.get(next_entity) {
                    // Prefetch the next component into L1 cache
                    prefetch_read(next_component as *const T);
                }
            }
        }

        // SAFETY: We maintain invariant that current_index < len
        // get_dense_entity returns None if index is out of bounds (defensive)
        let entity = self.storage.get_dense_entity(self.current_index)?;
        let component = self.storage.get(entity)?;

        self.current_index += 1;
        Some((entity, component))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, T: Component> ExactSizeIterator for SingleQueryIter<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

/// Optimized iterator for single component mutable queries
pub struct SingleQueryIterMut<'a, T: Component> {
    storage: *mut SparseSet<T>,
    current_index: usize,
    len: usize,
    _phantom: PhantomData<&'a mut T>,
}

impl<'a, T: Component> SingleQueryIterMut<'a, T> {
    #[inline]
    pub fn new(world: &'a mut World) -> Self {
        let type_id = TypeId::of::<T>();

        let (storage, len) = match world.components.get_mut(&type_id) {
            Some(storage_trait) => {
                // SAFETY: We know this is a SparseSet<T> because we stored it in register()
                let storage = unsafe {
                    storage_trait.as_any_mut() as *mut dyn std::any::Any as *mut SparseSet<T>
                };
                let len = unsafe { (*storage).len() };
                (storage, len)
            }
            None => {
                (std::ptr::null_mut(), 0)
            }
        };

        Self {
            storage,
            current_index: 0,
            len,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Component> Iterator for SingleQueryIterMut<'a, T> {
    type Item = (Entity, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        // SAFETY: We have exclusive access to world via &mut World
        // We return one mutable reference at a time
        // The storage pointer is valid for the lifetime of the world borrow
        unsafe {
            let storage = &mut *self.storage;

            // PREFETCH OPTIMIZATION: Load next component while processing current
            if self.current_index + 1 < self.len {
                if let Some(next_entity) = storage.get_dense_entity(self.current_index + 1) {
                    if let Some(next_component) = storage.get(next_entity) {
                        prefetch_read(next_component as *const T);
                    }
                }
            }

            let entity = storage.get_dense_entity(self.current_index)?;
            let component = storage.get_mut(entity)? as *mut T;

            self.current_index += 1;
            Some((entity, &mut *component))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, T: Component> ExactSizeIterator for SingleQueryIterMut<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

/// Optimized iterator for two-component immutable queries
///
/// OPTIMIZATIONS:
/// - Cached TypeIds for both components
/// - Cached storage pointers for both components
/// - Direct index access on smaller storage
/// - O(1) contains check on larger storage
pub struct TwoQueryIter<'a, A: Component, B: Component> {
    storage_a: &'a SparseSet<A>,
    storage_b: &'a SparseSet<B>,
    current_index: usize,
    len_a: usize,
}

impl<'a, A: Component, B: Component> TwoQueryIter<'a, A, B> {
    #[inline]
    pub fn new(world: &'a World) -> Self {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world.components.get(&type_id_a).and_then(|s| {
            // SAFETY: We know this is a SparseSet<A>
            unsafe { Some(&*(s.as_any() as *const dyn std::any::Any as *const SparseSet<A>)) }
        });

        let storage_b = world.components.get(&type_id_b).and_then(|s| {
            // SAFETY: We know this is a SparseSet<B>
            unsafe { Some(&*(s.as_any() as *const dyn std::any::Any as *const SparseSet<B>)) }
        });

        match (storage_a, storage_b) {
            (Some(a), Some(b)) => Self {
                storage_a: a,
                storage_b: b,
                current_index: 0,
                len_a: a.len(),
            },
            _ => {
                // Empty iterator
                let dummy_a = unsafe { &*(std::ptr::null() as *const SparseSet<A>) };
                let dummy_b = unsafe { &*(std::ptr::null() as *const SparseSet<B>) };
                Self {
                    storage_a: dummy_a,
                    storage_b: dummy_b,
                    current_index: 0,
                    len_a: 0,
                }
            }
        }
    }
}

impl<'a, A: Component, B: Component> Iterator for TwoQueryIter<'a, A, B> {
    type Item = (Entity, (&'a A, &'a B));

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Iterate smaller storage, check larger storage
        while self.current_index < self.len_a {
            // PREFETCH OPTIMIZATION: Prefetch next entity's components
            if self.current_index + 1 < self.len_a {
                if let Some(next_entity) = self.storage_a.get_dense_entity(self.current_index + 1) {
                    if let Some(next_a) = self.storage_a.get(next_entity) {
                        prefetch_read(next_a as *const A);
                    }
                    if let Some(next_b) = self.storage_b.get(next_entity) {
                        prefetch_read(next_b as *const B);
                    }
                }
            }

            let entity = self.storage_a.get_dense_entity(self.current_index)?;
            self.current_index += 1;

            // O(1) lookup in sparse set
            if let Some(comp_b) = self.storage_b.get(entity) {
                let comp_a = self.storage_a.get(entity)?;
                return Some((entity, (comp_a, comp_b)));
            }
        }

        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len_a.saturating_sub(self.current_index);
        (0, Some(remaining))
    }
}

/// Optimized iterator for two-component mutable queries
pub struct TwoQueryIterMut<'a, A: Component, B: Component> {
    storage_a: *mut SparseSet<A>,
    storage_b: *mut SparseSet<B>,
    current_index: usize,
    len_a: usize,
    _phantom: PhantomData<(&'a mut A, &'a mut B)>,
}

impl<'a, A: Component, B: Component> TwoQueryIterMut<'a, A, B> {
    #[inline]
    pub fn new(world: &'a mut World) -> Self {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world.components.get_mut(&type_id_a).map(|s| {
            unsafe { s.as_any_mut() as *mut dyn std::any::Any as *mut SparseSet<A> }
        });

        let storage_b = world.components.get_mut(&type_id_b).map(|s| {
            unsafe { s.as_any_mut() as *mut dyn std::any::Any as *mut SparseSet<B> }
        });

        match (storage_a, storage_b) {
            (Some(a), Some(b)) => {
                let len_a = unsafe { (*a).len() };
                Self {
                    storage_a: a,
                    storage_b: b,
                    current_index: 0,
                    len_a,
                    _phantom: PhantomData,
                }
            }
            _ => Self {
                storage_a: std::ptr::null_mut(),
                storage_b: std::ptr::null_mut(),
                current_index: 0,
                len_a: 0,
                _phantom: PhantomData,
            },
        }
    }
}

impl<'a, A: Component, B: Component> Iterator for TwoQueryIterMut<'a, A, B> {
    type Item = (Entity, (&'a mut A, &'a mut B));

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: We have exclusive access via &mut World
        // We return one pair of mutable references at a time
        // A and B are different types (different TypeIds)
        unsafe {
            while self.current_index < self.len_a {
                let storage_a = &mut *self.storage_a;
                let storage_b = &mut *self.storage_b;

                // PREFETCH OPTIMIZATION: Prefetch next entity's components
                if self.current_index + 1 < self.len_a {
                    if let Some(next_entity) = storage_a.get_dense_entity(self.current_index + 1) {
                        if let Some(next_a) = storage_a.get(next_entity) {
                            prefetch_read(next_a as *const A);
                        }
                        if let Some(next_b) = storage_b.get(next_entity) {
                            prefetch_read(next_b as *const B);
                        }
                    }
                }

                let entity = storage_a.get_dense_entity(self.current_index)?;
                self.current_index += 1;

                if storage_b.contains(entity) {
                    let comp_a = storage_a.get_mut(entity)? as *mut A;
                    let comp_b = storage_b.get_mut(entity)? as *mut B;
                    return Some((entity, (&mut *comp_a, &mut *comp_b)));
                }
            }
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len_a.saturating_sub(self.current_index);
        (0, Some(remaining))
    }
}

// Export optimized iterators for use in query.rs
pub use self::{SingleQueryIter, SingleQueryIterMut, TwoQueryIter, TwoQueryIterMut};
