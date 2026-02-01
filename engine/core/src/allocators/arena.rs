//! Arena allocator for temporary per-frame allocations
//!
//! The Arena allocator provides fast linear allocation with bulk deallocation.
//! Perfect for temporary data that lives for a single frame or a specific scope.
//!
//! # Features
//!
//! - O(1) allocation (bump pointer)
//! - Zero cost deallocation (bulk reset)
//! - Cache-friendly linear memory layout
//! - Type-safe allocations with proper alignment
//!
//! # Use Cases
//!
//! - Temporary vectors/buffers during frame processing
//! - Intermediate calculation results
//! - Query result caching
//! - String formatting and temporary strings
//!
//! # Examples
//!
//! ```
//! use engine_core::allocators::Arena;
//!
//! let mut arena = Arena::new();
//!
//! // Allocate individual values
//! let value = arena.alloc(42u64);
//! assert_eq!(*value, 42);
//!
//! // Allocate slices
//! let slice = arena.alloc_slice::<f32>(100);
//! assert_eq!(slice.len(), 100);
//!
//! // Reset to reuse memory
//! arena.reset();
//! ```

use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;
use std::slice;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Default chunk size: 64KB
const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Arena allocator for fast temporary allocations
///
/// Allocates memory in large chunks and hands out pieces linearly.
/// When a chunk is full, allocates a new one. All memory is freed
/// when the arena is dropped or reset.
///
/// # Safety
///
/// The arena uses raw pointers internally but provides a safe API.
/// All allocated references are valid for the lifetime of the arena.
pub struct Arena {
    /// Current chunk being allocated from
    current_chunk: Cell<Option<Chunk>>,
    /// List of all allocated chunks
    chunks: Vec<Chunk>,
    /// Default size for new chunks
    chunk_size: usize,
}

/// A single chunk of memory in the arena
#[derive(Clone, Copy)]
struct Chunk {
    /// Pointer to the start of the chunk
    ptr: NonNull<u8>,
    /// Total capacity of this chunk
    capacity: usize,
    /// Current offset into the chunk
    offset: usize,
}

impl Arena {
    /// Create a new arena with default chunk size (64KB)
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create an arena with a custom chunk size
    ///
    /// Larger chunks reduce allocation overhead but may waste memory
    /// if not fully used. Choose based on expected usage patterns.
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        assert!(chunk_size >= 1024, "Chunk size must be at least 1KB");
        assert!(chunk_size.is_power_of_two(), "Chunk size must be power of 2");

        Self {
            current_chunk: Cell::new(None),
            chunks: Vec::new(),
            chunk_size,
        }
    }

    /// Allocate a single value in the arena
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::Arena;
    /// let mut arena = Arena::new();
    /// let value = arena.alloc(42);
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    pub fn alloc<T>(&mut self, value: T) -> &mut T {
        #[cfg(feature = "profiling")]
        profile_scope!("arena_alloc", ProfileCategory::ECS);

        let ptr = self.alloc_raw(size_of::<T>(), align_of::<T>()) as *mut T;
        unsafe {
            ptr.write(value);
            &mut *ptr
        }
    }

    /// Allocate a slice of values in the arena
    ///
    /// The slice is uninitialized - caller must initialize before use.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::Arena;
    /// let mut arena = Arena::new();
    /// let slice = arena.alloc_slice::<u32>(100);
    /// for (i, val) in slice.iter_mut().enumerate() {
    ///     *val = i as u32;
    /// }
    /// ```
    #[inline]
    pub fn alloc_slice<T>(&mut self, count: usize) -> &mut [T] {
        #[cfg(feature = "profiling")]
        profile_scope!("arena_alloc_slice", ProfileCategory::ECS);

        if count == 0 {
            return &mut [];
        }

        let size = size_of::<T>() * count;
        let align = align_of::<T>();
        let ptr = self.alloc_raw(size, align) as *mut T;

        unsafe {
            // Initialize memory to zeros for safety
            std::ptr::write_bytes(ptr, 0, count);
            slice::from_raw_parts_mut(ptr, count)
        }
    }

    /// Allocate space for a value, returning a mutable reference to uninitialized memory
    ///
    /// # Safety
    ///
    /// The caller must initialize the returned reference before use.
    #[inline]
    pub fn alloc_uninitialized<T>(&mut self) -> &mut std::mem::MaybeUninit<T> {
        let ptr = self.alloc_raw(size_of::<T>(), align_of::<T>()) as *mut std::mem::MaybeUninit<T>;
        unsafe { &mut *ptr }
    }

    /// Raw allocation - returns aligned pointer to memory
    #[inline]
    fn alloc_raw(&mut self, size: usize, align: usize) -> *mut u8 {
        // Fast path: try current chunk
        if let Some(mut chunk) = self.current_chunk.get() {
            if let Some(ptr) = chunk.alloc(size, align) {
                self.current_chunk.set(Some(chunk));
                return ptr;
            }
        }

        // Slow path: allocate new chunk
        let chunk_size = self.chunk_size.max(size + align);
        let mut chunk = Chunk::new(chunk_size);
        let ptr = chunk.alloc(size, align).expect("Freshly allocated chunk should have space");

        self.chunks.push(chunk.clone());
        self.current_chunk.set(Some(chunk));

        ptr
    }

    /// Reset the arena, invalidating all allocations
    ///
    /// This doesn't free memory, just resets the allocation pointer.
    /// Memory will be reused for subsequent allocations.
    ///
    /// # Safety
    ///
    /// All references obtained from this arena become invalid after reset.
    /// Using them results in undefined behavior.
    #[inline]
    pub fn reset(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("arena_reset", ProfileCategory::ECS);

        // Reset all chunks for reuse
        for chunk in &mut self.chunks {
            chunk.offset = 0;
        }

        // Set first chunk as current if we have any
        if let Some(chunk) = self.chunks.first().cloned() {
            self.current_chunk.set(Some(chunk));
        } else {
            self.current_chunk.set(None);
        }
    }

    /// Get total allocated capacity in bytes
    pub fn capacity(&self) -> usize {
        self.chunks.iter().map(|c| c.capacity).sum()
    }

    /// Get total used bytes across all chunks
    pub fn used(&self) -> usize {
        self.chunks.iter().map(|c| c.offset).sum()
    }

    /// Get allocation efficiency (used / capacity)
    pub fn efficiency(&self) -> f32 {
        let capacity = self.capacity();
        if capacity == 0 {
            1.0
        } else {
            self.used() as f32 / capacity as f32
        }
    }
}

impl Chunk {
    fn new(size: usize) -> Self {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, 64); // 64-byte alignment for cache lines
            let ptr = alloc(layout);
            assert!(!ptr.is_null(), "Arena chunk allocation failed");

            Self {
                ptr: NonNull::new_unchecked(ptr),
                capacity: size,
                offset: 0,
            }
        }
    }

    #[inline]
    fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // Align the current offset
        let offset = (self.offset + align - 1) & !(align - 1);

        // Check if we have space
        if offset + size > self.capacity {
            return None;
        }

        // Update offset and return pointer
        self.offset = offset + size;
        Some(unsafe { self.ptr.as_ptr().add(offset) })
    }

    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            capacity: self.capacity,
            offset: self.offset,
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("arena_drop", ProfileCategory::ECS);

        for chunk in &self.chunks {
            unsafe {
                let layout = Layout::from_size_align_unchecked(chunk.capacity, 64);
                dealloc(chunk.ptr.as_ptr(), layout);
            }
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Arena {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_single_allocation() {
        let mut arena = Arena::new();
        let val = arena.alloc(42u64);
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_arena_multiple_allocations() {
        let mut arena = Arena::new();
        let slice = arena.alloc_slice::<u32>(3);
        slice[0] = 1;
        slice[1] = 2;
        slice[2] = 3;

        assert_eq!(slice[0], 1);
        assert_eq!(slice[1], 2);
        assert_eq!(slice[2], 3);
    }

    #[test]
    fn test_arena_slice_allocation() {
        let mut arena = Arena::new();
        let slice = arena.alloc_slice::<u32>(100);

        assert_eq!(slice.len(), 100);
        for (i, val) in slice.iter_mut().enumerate() {
            *val = i as u32;
        }

        for (i, val) in slice.iter().enumerate() {
            assert_eq!(*val, i as u32);
        }
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = Arena::new();

        // Allocate types with different alignments
        {
            let byte_slice = arena.alloc_slice::<u8>(1);
            assert!(byte_slice.len() > 0);
        }

        {
            let int_slice = arena.alloc_slice::<u32>(1);
            assert_eq!(int_slice.as_ptr() as usize % align_of::<u32>(), 0);
        }

        {
            let long_slice = arena.alloc_slice::<u64>(1);
            assert_eq!(long_slice.as_ptr() as usize % align_of::<u64>(), 0);
        }
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = Arena::new();

        let before_capacity = arena.capacity();

        // Allocate some data
        let _a = arena.alloc(1u64);
        let _b = arena.alloc(2u64);

        let used = arena.used();
        assert!(used > 0);

        // Reset
        arena.reset();

        // Should reuse memory
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.capacity(), before_capacity);
    }

    #[test]
    fn test_arena_large_allocation() {
        let mut arena = Arena::with_chunk_size(1024);

        // Allocate larger than chunk size
        let large = arena.alloc_slice::<u64>(200); // 1600 bytes
        assert_eq!(large.len(), 200);
    }

    #[test]
    fn test_arena_efficiency() {
        let mut arena = Arena::with_chunk_size(1024);
        arena.alloc_slice::<u8>(512);

        let efficiency = arena.efficiency();
        assert!(efficiency > 0.4 && efficiency <= 1.0);
    }

    #[test]
    fn test_arena_empty_slice() {
        let mut arena = Arena::new();
        let slice = arena.alloc_slice::<u64>(0);
        assert_eq!(slice.len(), 0);
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct AlignedData {
        _pad: [u8; 63],
        value: u64,
    }

    #[test]
    fn test_arena_custom_alignment() {
        let mut arena = Arena::new();
        let data = arena.alloc(AlignedData {
            _pad: [0; 63],
            value: 42,
        });

        assert_eq!(data.value, 42);
    }
}
