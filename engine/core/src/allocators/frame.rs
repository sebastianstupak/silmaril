//! Frame allocator for per-frame temporary allocations
//!
//! The Frame allocator is optimized for data that lives exactly one frame.
//! It provides extremely fast allocation with zero fragmentation by resetting
//! completely at the end of each frame.
//!
//! # Features
//!
//! - O(1) allocation (bump pointer)
//! - O(1) reset (no deallocation needed)
//! - Zero fragmentation (complete reset each frame)
//! - Cache-friendly linear layout
//! - Thread-safe with interior mutability
//!
//! # Use Cases
//!
//! - Per-frame intermediate buffers
//! - Temporary render data
//! - Query result caching
//! - Frame-local collections
//! - Debug visualizations
//!
//! # Examples
//!
//! ```
//! use engine_core::allocators::FrameAllocator;
//!
//! let mut frame_alloc = FrameAllocator::with_capacity(1024 * 1024); // 1MB
//!
//! // Game loop
//! loop {
//!     // Allocate temporary data
//!     let temp_buffer = frame_alloc.alloc_slice::<f32>(1000);
//!
//!     // ... use temp_buffer for frame processing
//!
//!     // Reset at end of frame
//!     frame_alloc.reset();
//! }
//! ```

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::cell::Cell;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;
use std::slice;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Default frame allocator size: 1MB
const DEFAULT_FRAME_SIZE: usize = 1024 * 1024;

/// Frame allocator for temporary per-frame allocations
///
/// Allocates from a single linear buffer that is reset at the end of each frame.
/// This provides maximum performance for short-lived allocations with zero fragmentation.
///
/// # Thread Safety
///
/// The frame allocator uses `Cell` for interior mutability, making it `!Sync`.
/// Each thread should have its own frame allocator instance.
///
/// # Memory Management
///
/// - Allocates from a contiguous buffer
/// - Can grow automatically when needed
/// - Reset is O(1) - just resets the offset pointer
/// - Memory is only freed when the allocator is dropped
pub struct FrameAllocator {
    /// Pointer to the allocation buffer
    buffer: NonNull<u8>,
    /// Total capacity in bytes
    capacity: usize,
    /// Current allocation offset
    offset: Cell<usize>,
    /// Peak offset this frame (for statistics)
    peak_offset: Cell<usize>,
    /// Whether to allow automatic growth
    allow_growth: bool,
}

impl FrameAllocator {
    /// Create a new frame allocator with default capacity (1MB)
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_FRAME_SIZE)
    }

    /// Create a frame allocator with specified capacity
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::FrameAllocator;
    /// // 4MB frame allocator
    /// let frame = FrameAllocator::with_capacity(4 * 1024 * 1024);
    /// assert_eq!(frame.capacity(), 4 * 1024 * 1024);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "Frame allocator capacity must be > 0");

        unsafe {
            let layout = Layout::from_size_align_unchecked(capacity, 64); // 64-byte alignment
            let ptr = alloc(layout);
            assert!(!ptr.is_null(), "Frame allocator allocation failed");

            Self {
                buffer: NonNull::new_unchecked(ptr),
                capacity,
                offset: Cell::new(0),
                peak_offset: Cell::new(0),
                allow_growth: true,
            }
        }
    }

    /// Create a frame allocator that panics instead of growing
    ///
    /// Useful for debugging to catch unexpected allocation patterns.
    pub fn with_capacity_fixed(capacity: usize) -> Self {
        let mut allocator = Self::with_capacity(capacity);
        allocator.allow_growth = false;
        allocator
    }

    /// Allocate a single value in the frame
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::FrameAllocator;
    /// let mut frame = FrameAllocator::new();
    /// let value = frame.alloc(42u64);
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    pub fn alloc<T>(&mut self, value: T) -> &mut T {
        #[cfg(feature = "profiling")]
        profile_scope!("frame_alloc", ProfileCategory::ECS);

        let ptr = self.alloc_raw(size_of::<T>(), align_of::<T>()) as *mut T;
        unsafe {
            ptr.write(value);
            &mut *ptr
        }
    }

    /// Allocate a slice of uninitialized values
    ///
    /// The slice is zero-initialized for safety.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::FrameAllocator;
    /// let mut frame = FrameAllocator::new();
    /// let slice = frame.alloc_slice::<f32>(1000);
    /// assert_eq!(slice.len(), 1000);
    /// ```
    #[inline]
    pub fn alloc_slice<T>(&mut self, count: usize) -> &mut [T] {
        #[cfg(feature = "profiling")]
        profile_scope!("frame_alloc_slice", ProfileCategory::ECS);

        if count == 0 {
            return &mut [];
        }

        let size = size_of::<T>() * count;
        let align = align_of::<T>();
        let ptr = self.alloc_raw(size, align) as *mut T;

        unsafe {
            // Zero-initialize for safety
            std::ptr::write_bytes(ptr, 0, count);
            slice::from_raw_parts_mut(ptr, count)
        }
    }

    /// Allocate a slice and initialize with a function
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::FrameAllocator;
    /// let mut frame = FrameAllocator::new();
    /// let slice = frame.alloc_slice_with(100, |i| i as f32 * 2.0);
    /// assert_eq!(slice[10], 20.0);
    /// ```
    #[inline]
    pub fn alloc_slice_with<T, F>(&mut self, count: usize, mut init: F) -> &mut [T]
    where
        F: FnMut(usize) -> T,
    {
        let slice = self.alloc_slice(count);
        for (i, elem) in slice.iter_mut().enumerate() {
            *elem = init(i);
        }
        slice
    }

    /// Allocate a copy of a slice
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::allocators::FrameAllocator;
    /// let mut frame = FrameAllocator::new();
    /// let original = vec![1, 2, 3, 4, 5];
    /// let copy = frame.alloc_slice_copy(&original);
    /// assert_eq!(copy, &[1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&mut self, slice: &[T]) -> &mut [T] {
        if slice.is_empty() {
            return &mut [];
        }

        let size = size_of::<T>() * slice.len();
        let align = align_of::<T>();
        let ptr = self.alloc_raw(size, align) as *mut T;

        unsafe {
            std::ptr::copy_nonoverlapping(slice.as_ptr(), ptr, slice.len());
            slice::from_raw_parts_mut(ptr, slice.len())
        }
    }

    /// Raw allocation - returns aligned pointer
    #[inline]
    fn alloc_raw(&mut self, size: usize, align: usize) -> *mut u8 {
        let offset = self.offset.get();

        // Align the offset
        let aligned_offset = (offset + align - 1) & !(align - 1);
        let new_offset = aligned_offset + size;

        // Check if we need to grow
        if new_offset > self.capacity {
            if !self.allow_growth {
                panic!(
                    "Frame allocator out of memory: requested {}, capacity {}, used {}",
                    size, self.capacity, offset
                );
            }
            self.grow(new_offset);
        }

        // Update offset and peak
        self.offset.set(new_offset);
        if new_offset > self.peak_offset.get() {
            self.peak_offset.set(new_offset);
        }

        unsafe { self.buffer.as_ptr().add(aligned_offset) }
    }

    /// Grow the allocator to accommodate the requested size
    fn grow(&mut self, min_size: usize) {
        #[cfg(feature = "profiling")]
        profile_scope!("frame_grow", ProfileCategory::ECS);

        // Double capacity or use min_size, whichever is larger
        let new_capacity = self.capacity.max(min_size).next_power_of_two();

        unsafe {
            let old_layout = Layout::from_size_align_unchecked(self.capacity, 64);
            let _new_layout = Layout::from_size_align_unchecked(new_capacity, 64);

            let new_ptr = realloc(self.buffer.as_ptr(), old_layout, new_capacity);
            assert!(!new_ptr.is_null(), "Frame allocator grow failed");

            self.buffer = NonNull::new_unchecked(new_ptr);
            self.capacity = new_capacity;
        }
    }

    /// Reset the frame allocator
    ///
    /// This makes all previous allocations invalid. Calling this
    /// at the end of each frame ensures zero fragmentation.
    ///
    /// # Safety
    ///
    /// All references obtained from this allocator become invalid after reset.
    #[inline]
    pub fn reset(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("frame_reset", ProfileCategory::ECS);

        self.offset.set(0);
        // Keep peak_offset for statistics until next frame
    }

    /// Reset and clear peak statistics
    #[inline]
    pub fn reset_with_stats_clear(&mut self) {
        self.reset();
        self.peak_offset.set(0);
    }

    /// Get the current allocation offset in bytes
    #[inline]
    pub fn used(&self) -> usize {
        self.offset.get()
    }

    /// Get the total capacity in bytes
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the available space in bytes
    #[inline]
    pub fn available(&self) -> usize {
        self.capacity - self.offset.get()
    }

    /// Get the peak usage this frame in bytes
    #[inline]
    pub fn peak_used(&self) -> usize {
        self.peak_offset.get()
    }

    /// Get the current utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        self.offset.get() as f32 / self.capacity as f32
    }

    /// Get the peak utilization this frame (0.0 to 1.0)
    pub fn peak_utilization(&self) -> f32 {
        self.peak_offset.get() as f32 / self.capacity as f32
    }

    /// Check if the allocator is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.offset.get() == 0
    }
}

impl Drop for FrameAllocator {
    fn drop(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("frame_drop", ProfileCategory::ECS);

        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, 64);
            dealloc(self.buffer.as_ptr(), layout);
        }
    }
}

impl Default for FrameAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for FrameAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_single_allocation() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let val = frame.alloc(42u64);
        assert_eq!(*val, 42);
        assert!(frame.used() >= size_of::<u64>());
    }

    #[test]
    fn test_frame_multiple_allocations() {
        let mut frame = FrameAllocator::with_capacity(1024);

        let slice = frame.alloc_slice::<u32>(3);
        slice[0] = 1;
        slice[1] = 2;
        slice[2] = 3;

        assert_eq!(slice[0], 1);
        assert_eq!(slice[1], 2);
        assert_eq!(slice[2], 3);
    }

    #[test]
    fn test_frame_slice_allocation() {
        let mut frame = FrameAllocator::with_capacity(4096);
        let slice = frame.alloc_slice::<f32>(100);

        assert_eq!(slice.len(), 100);

        for (i, val) in slice.iter_mut().enumerate() {
            *val = i as f32;
        }

        for (i, val) in slice.iter().enumerate() {
            assert_eq!(*val, i as f32);
        }
    }

    #[test]
    fn test_frame_reset() {
        let mut frame = FrameAllocator::with_capacity(1024);

        frame.alloc(42u64);
        frame.alloc_slice::<u32>(10);

        let used_before = frame.used();
        assert!(used_before > 0);

        frame.reset();

        assert_eq!(frame.used(), 0);
        assert_eq!(frame.peak_used(), used_before);
    }

    #[test]
    fn test_frame_alignment() {
        let mut frame = FrameAllocator::with_capacity(1024);

        {
            let byte_slice = frame.alloc_slice::<u8>(1);
            assert!(byte_slice.len() > 0);
        }

        {
            let int_slice = frame.alloc_slice::<u32>(1);
            assert_eq!(int_slice.as_ptr() as usize % align_of::<u32>(), 0);
        }

        {
            let long_slice = frame.alloc_slice::<u64>(1);
            assert_eq!(long_slice.as_ptr() as usize % align_of::<u64>(), 0);
        }
    }

    #[test]
    fn test_frame_growth() {
        let mut frame = FrameAllocator::with_capacity(64);

        // Allocate more than initial capacity
        let slice = frame.alloc_slice::<u64>(100); // 800 bytes

        assert_eq!(slice.len(), 100);
        assert!(frame.capacity() >= 800);
    }

    #[test]
    #[should_panic(expected = "out of memory")]
    fn test_frame_fixed_no_growth() {
        let mut frame = FrameAllocator::with_capacity_fixed(64);
        // This should panic - trying to allocate more than capacity
        frame.alloc_slice::<u64>(100);
    }

    #[test]
    fn test_frame_utilization() {
        let mut frame = FrameAllocator::with_capacity(1024);
        frame.alloc_slice::<u8>(512);

        let util = frame.utilization();
        assert!(util >= 0.5 && util <= 1.0);
    }

    #[test]
    fn test_frame_peak_tracking() {
        let mut frame = FrameAllocator::with_capacity(1024);

        frame.alloc_slice::<u8>(100);
        let peak1 = frame.peak_used();

        frame.alloc_slice::<u8>(200);
        let peak2 = frame.peak_used();

        assert!(peak2 > peak1);

        frame.reset();
        assert_eq!(frame.used(), 0);
        assert_eq!(frame.peak_used(), peak2); // Peak persists
    }

    #[test]
    fn test_frame_slice_with() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let slice = frame.alloc_slice_with(10, |i| i as u32 * 2);

        for (i, val) in slice.iter().enumerate() {
            assert_eq!(*val, (i * 2) as u32);
        }
    }

    #[test]
    fn test_frame_slice_copy() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let original = vec![1, 2, 3, 4, 5];
        let copy = frame.alloc_slice_copy(&original);

        assert_eq!(copy, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_frame_empty_slice() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let slice = frame.alloc_slice::<u64>(0);
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn test_frame_multiple_resets() {
        let mut frame = FrameAllocator::with_capacity(1024);

        for _ in 0..10 {
            frame.alloc_slice::<u64>(10);
            assert!(frame.used() > 0);
            frame.reset();
            assert_eq!(frame.used(), 0);
        }
    }

    #[test]
    fn test_frame_available() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let initial = frame.available();
        assert_eq!(initial, 1024);

        frame.alloc_slice::<u8>(512);
        let remaining = frame.available();
        assert!(remaining < initial);
        assert!(remaining >= 512);
    }
}
