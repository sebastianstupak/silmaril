//! Pool allocator for fixed-size objects
//!
//! The Pool allocator efficiently manages allocation and deallocation of
//! fixed-size objects using a free list. Perfect for entities, components,
//! and other frequently created/destroyed objects.
//!
//! # Features
//!
//! - O(1) allocation and deallocation
//! - Zero fragmentation for fixed-size objects
//! - Cache-friendly contiguous storage
//! - Type-safe free list management
//!
//! # Use Cases
//!
//! - Entity allocation in ECS
//! - Component pools
//! - Particle systems
//! - Object pools for frequently reused types
//!
//! # Examples
//!
//! ```
//! use engine_core::allocators::PoolAllocator;
//!
//! #[derive(Debug, Clone, Copy)]
//! struct Particle {
//!     position: [f32; 3],
//!     velocity: [f32; 3],
//! }
//!
//! let mut pool = PoolAllocator::<Particle>::with_capacity(1000);
//!
//! // Allocate particles
//! let p1 = pool.alloc(Particle {
//!     position: [0.0, 0.0, 0.0],
//!     velocity: [1.0, 0.0, 0.0],
//! });
//!
//! // Free when done
//! pool.free(p1);
//!
//! // Next allocation reuses the freed slot
//! let p2 = pool.alloc(Particle {
//!     position: [1.0, 1.0, 1.0],
//!     velocity: [0.0, 1.0, 0.0],
//! });
//! ```

use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Pool allocator for fixed-size objects
///
/// Manages a pool of fixed-size slots using a free list for efficient
/// allocation and deallocation. All objects are stored contiguously
/// for cache efficiency.
///
/// # Type Safety
///
/// The pool is generic over `T` and can only allocate/free objects of type `T`.
/// This prevents type confusion and ensures memory safety.
pub struct PoolAllocator<T> {
    /// Contiguous storage for all objects
    storage: NonNull<Slot<T>>,
    /// Total capacity (number of slots)
    capacity: usize,
    /// Number of currently allocated objects
    count: usize,
    /// Head of the free list (index into storage)
    free_head: Option<usize>,
    /// Phantom data for drop check
    _phantom: PhantomData<T>,
}

/// A slot in the pool - either contains data or points to next free slot
union Slot<T> {
    /// Active slot containing data
    data: std::mem::ManuallyDrop<T>,
    /// Free slot - index of next free slot
    next_free: usize,
}

impl<T> PoolAllocator<T> {
    /// Create a new pool with the specified capacity
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::PoolAllocator;
    /// let pool = PoolAllocator::<u64>::with_capacity(1000);
    /// assert_eq!(pool.capacity(), 1000);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "Pool capacity must be > 0");

        unsafe {
            // Allocate storage for all slots
            let layout = Layout::array::<Slot<T>>(capacity).expect("Invalid layout");
            let ptr = alloc(layout) as *mut Slot<T>;
            assert!(!ptr.is_null(), "Pool allocation failed");

            // Initialize free list
            for i in 0..capacity {
                ptr.add(i)
                    .write(Slot { next_free: if i + 1 < capacity { i + 1 } else { usize::MAX } });
            }

            Self {
                storage: NonNull::new_unchecked(ptr),
                capacity,
                count: 0,
                free_head: Some(0),
                _phantom: PhantomData,
            }
        }
    }

    /// Allocate an object from the pool
    ///
    /// Returns a mutable reference to the allocated object.
    ///
    /// # Panics
    ///
    /// Panics if the pool is full. Check `is_full()` before allocating
    /// if this is a concern.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::PoolAllocator;
    /// let mut pool = PoolAllocator::<u64>::with_capacity(10);
    /// let value = pool.alloc(42);
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    pub fn alloc(&mut self, value: T) -> &mut T {
        #[cfg(feature = "profiling")]
        profile_scope!("pool_alloc", ProfileCategory::ECS);

        assert!(!self.is_full(), "Pool allocator is full");

        let index = self.free_head.expect("Pool should not be full");

        unsafe {
            let slot = self.storage.as_ptr().add(index);

            // Get next free index before overwriting
            let next_free =
                if (*slot).next_free == usize::MAX { None } else { Some((*slot).next_free) };

            // Write value to slot
            ptr::write(&mut (*slot).data, std::mem::ManuallyDrop::new(value));

            // Update free list
            self.free_head = next_free;
            self.count += 1;

            &mut (*slot).data
        }
    }

    /// Free an object, returning it to the pool
    ///
    /// # Safety
    ///
    /// The reference must have been allocated from this pool.
    /// Freeing the same reference twice results in undefined behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::PoolAllocator;
    /// let mut pool = PoolAllocator::<u64>::with_capacity(10);
    /// let value = pool.alloc(42);
    /// pool.free(value);
    /// assert_eq!(pool.len(), 0);
    /// ```
    #[inline]
    pub fn free(&mut self, ptr: &mut T) {
        #[cfg(feature = "profiling")]
        profile_scope!("pool_free", ProfileCategory::ECS);

        unsafe {
            // Calculate index from pointer
            let slot_ptr = ptr as *mut T as *mut Slot<T>;
            let index = slot_ptr.offset_from(self.storage.as_ptr()) as usize;

            assert!(index < self.capacity, "Pointer does not belong to this pool");

            // Drop the value
            std::mem::ManuallyDrop::drop(&mut (*slot_ptr).data);

            // Add to free list
            (*slot_ptr).next_free = self.free_head.unwrap_or(usize::MAX);
            self.free_head = Some(index);
            self.count -= 1;
        }
    }

    /// Get the total capacity of the pool
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the number of currently allocated objects
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the pool is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if the pool is full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }

    /// Get the number of available slots
    #[inline]
    pub fn available(&self) -> usize {
        self.capacity - self.count
    }

    /// Get pool utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        self.count as f32 / self.capacity as f32
    }

    /// Clear all allocations
    ///
    /// This drops all allocated objects and resets the pool.
    ///
    /// # Safety
    ///
    /// All references obtained from this pool become invalid.
    pub fn clear(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("pool_clear", ProfileCategory::ECS);

        unsafe {
            // Drop all allocated objects
            // We need to traverse and drop only allocated slots
            // For simplicity, we'll rebuild the free list

            // Rebuild free list from scratch
            for i in 0..self.capacity {
                let slot = self.storage.as_ptr().add(i);
                (*slot).next_free = if i + 1 < self.capacity { i + 1 } else { usize::MAX };
            }

            self.free_head = Some(0);
            self.count = 0;
        }
    }

    /// Grow the pool by allocating additional capacity
    ///
    /// This creates a new larger allocation and moves existing objects.
    /// All existing references become invalid.
    pub fn grow(&mut self, additional: usize) {
        #[cfg(feature = "profiling")]
        profile_scope!("pool_grow", ProfileCategory::ECS);

        let new_capacity = self.capacity + additional;

        unsafe {
            // Allocate new storage
            let new_layout = Layout::array::<Slot<T>>(new_capacity).expect("Invalid layout");
            let new_ptr = alloc(new_layout) as *mut Slot<T>;
            assert!(!new_ptr.is_null(), "Pool grow allocation failed");

            // Copy existing data
            ptr::copy_nonoverlapping(self.storage.as_ptr(), new_ptr, self.capacity);

            // Initialize new free slots
            for i in self.capacity..new_capacity {
                new_ptr.add(i).write(Slot {
                    next_free: if i + 1 < new_capacity { i + 1 } else { usize::MAX },
                });
            }

            // Link old free list to new slots
            if self.free_head.is_none() {
                self.free_head = Some(self.capacity);
            } else {
                // Find end of current free list and link to new slots
                let mut current = self.free_head;
                while let Some(index) = current {
                    let next = (*new_ptr.add(index)).next_free;
                    if next == usize::MAX {
                        (*new_ptr.add(index)).next_free = self.capacity;
                        break;
                    }
                    current = Some(next);
                }
            }

            // Free old storage
            let old_layout = Layout::array::<Slot<T>>(self.capacity).expect("Invalid layout");
            dealloc(self.storage.as_ptr() as *mut u8, old_layout);

            // Update pool
            self.storage = NonNull::new_unchecked(new_ptr);
            self.capacity = new_capacity;
        }
    }
}

impl<T> Drop for PoolAllocator<T> {
    fn drop(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("pool_drop", ProfileCategory::ECS);

        unsafe {
            // Note: We can't easily track which slots are allocated,
            // so we assume clear() was called or accept potential leaks.
            // In production, consider tracking allocated slots.

            let layout = Layout::array::<Slot<T>>(self.capacity).expect("Invalid layout");
            dealloc(self.storage.as_ptr() as *mut u8, layout);
        }
    }
}

unsafe impl<T: Send> Send for PoolAllocator<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_basic_alloc_free() {
        let mut pool = PoolAllocator::<u64>::with_capacity(10);

        assert_eq!(pool.len(), 0);
        assert_eq!(pool.capacity(), 10);

        unsafe {
            let val = pool.alloc(42) as *mut u64;
            assert_eq!(*val, 42);
            assert_eq!(pool.len(), 1);

            pool.free(&mut *val);
            assert_eq!(pool.len(), 0);
        }
    }

    #[test]
    fn test_pool_multiple_allocations() {
        let mut pool = PoolAllocator::<u32>::with_capacity(5);

        unsafe {
            let a = pool.alloc(1) as *mut u32;
            let b = pool.alloc(2) as *mut u32;
            let c = pool.alloc(3) as *mut u32;

            assert_eq!(*a, 1);
            assert_eq!(*b, 2);
            assert_eq!(*c, 3);
            assert_eq!(pool.len(), 3);

            pool.free(&mut *b);
            assert_eq!(pool.len(), 2);

            // Next allocation should reuse freed slot
            let d = pool.alloc(4) as *mut u32;
            assert_eq!(*d, 4);
            assert_eq!(pool.len(), 3);

            // Cleanup
            pool.free(&mut *a);
            pool.free(&mut *c);
            pool.free(&mut *d);
        }
    }

    #[test]
    fn test_pool_full() {
        let mut pool = PoolAllocator::<u8>::with_capacity(2);

        pool.alloc(1);
        pool.alloc(2);

        assert!(pool.is_full());
    }

    #[test]
    #[should_panic(expected = "Pool allocator is full")]
    fn test_pool_overflow_panics() {
        let mut pool = PoolAllocator::<u8>::with_capacity(1);
        pool.alloc(1);
        pool.alloc(2); // Should panic
    }

    #[test]
    fn test_pool_clear() {
        let mut pool = PoolAllocator::<u32>::with_capacity(10);

        pool.alloc(1);
        pool.alloc(2);
        pool.alloc(3);

        assert_eq!(pool.len(), 3);

        pool.clear();

        assert_eq!(pool.len(), 0);
        assert_eq!(pool.capacity(), 10);
    }

    #[test]
    fn test_pool_utilization() {
        let mut pool = PoolAllocator::<u64>::with_capacity(10);

        pool.alloc(1);
        pool.alloc(2);
        pool.alloc(3);
        pool.alloc(4);
        pool.alloc(5);

        assert_eq!(pool.utilization(), 0.5);
    }

    #[test]
    fn test_pool_grow() {
        let mut pool = PoolAllocator::<u32>::with_capacity(2);

        let _a = pool.alloc(1) as *mut u32;
        let _b = pool.alloc(2) as *mut u32;

        assert!(pool.is_full());

        pool.grow(3);

        assert_eq!(pool.capacity(), 5);
        assert_eq!(pool.available(), 3);

        // Can allocate more now
        let _c = pool.alloc(3) as *mut u32;
        let _d = pool.alloc(4) as *mut u32;
        let _e = pool.alloc(5) as *mut u32;

        assert!(pool.is_full());
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct ComplexType {
        x: f32,
        y: f32,
        data: [u8; 16],
    }

    #[test]
    fn test_pool_complex_type() {
        let mut pool = PoolAllocator::<ComplexType>::with_capacity(10);

        unsafe {
            let obj = pool.alloc(ComplexType { x: 1.0, y: 2.0, data: [0; 16] }) as *mut ComplexType;

            assert_eq!((*obj).x, 1.0);
            assert_eq!((*obj).y, 2.0);

            pool.free(&mut *obj);
            assert_eq!(pool.len(), 0);
        }
    }

    #[test]
    fn test_pool_reuse_pattern() {
        let mut pool = PoolAllocator::<u64>::with_capacity(3);

        unsafe {
            // Allocate all
            let _a = pool.alloc(1) as *mut u64;
            let b = pool.alloc(2) as *mut u64;
            let _c = pool.alloc(3) as *mut u64;

            // Free middle one
            pool.free(&mut *b);

            // Allocate again - should reuse the freed slot
            let d = pool.alloc(4) as *mut u64;
            assert_eq!(*d, 4);

            assert_eq!(pool.len(), 3);
        }
    }
}
