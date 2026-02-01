//! Cache-aligned memory allocations for performance-critical structures.
//!
//! Provides `AlignedVec<T, N>` which guarantees N-byte alignment for better
//! cache performance. 64-byte alignment (cache line size) prevents false sharing
//! and cache line splits during SIMD operations.

use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::slice;

/// A vector with guaranteed alignment to N bytes.
///
/// This is critical for SIMD performance as it:
/// 1. Prevents cache line splits (when data straddles two cache lines)
/// 2. Enables use of aligned SIMD load/store instructions (faster than unaligned)
/// 3. Prevents false sharing between threads (each element on separate cache line)
///
/// # Examples
///
/// ```
/// use engine_math::aligned::AlignedVec;
/// use engine_math::Vec3;
///
/// // Create cache-line aligned vector (64 bytes = typical cache line size)
/// let mut positions: AlignedVec<Vec3, 64> = AlignedVec::new();
/// positions.push(Vec3::new(1.0, 2.0, 3.0));
/// positions.push(Vec3::new(4.0, 5.0, 6.0));
///
/// // Guaranteed to be 64-byte aligned
/// assert_eq!(positions.as_ptr() as usize % 64, 0);
/// ```
pub struct AlignedVec<T, const ALIGN: usize> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T, const ALIGN: usize> AlignedVec<T, ALIGN> {
    /// Creates a new empty `AlignedVec`.
    ///
    /// Does not allocate until the first element is pushed.
    #[inline]
    pub fn new() -> Self {
        assert!(ALIGN > 0 && ALIGN.is_power_of_two(), "Alignment must be a power of two");
        assert!(ALIGN >= std::mem::align_of::<T>(), "Alignment must be >= type alignment");

        Self { ptr: NonNull::dangling(), len: 0, capacity: 0, _marker: PhantomData }
    }

    /// Creates a new `AlignedVec` with the specified capacity.
    ///
    /// The allocation will be aligned to `ALIGN` bytes.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut vec = Self::new();
        if capacity > 0 {
            vec.reserve(capacity);
        }
        vec
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements the vector can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns a raw pointer to the vector's buffer.
    ///
    /// This pointer is guaranteed to be aligned to `ALIGN` bytes.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns an unsafe mutable pointer to the vector's buffer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Extracts a slice containing the entire vector.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// Extracts a mutable slice containing the entire vector.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        let required_cap = self.len.checked_add(additional).expect("capacity overflow");

        if required_cap <= self.capacity {
            return;
        }

        // Grow by 2x or required capacity, whichever is larger
        let new_capacity = std::cmp::max(self.capacity.saturating_mul(2), required_cap);

        self.realloc(new_capacity);
    }

    /// Appends an element to the back of the vector.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }

        unsafe {
            ptr::write(self.as_mut_ptr().add(self.len), value);
            self.len += 1;
        }
    }

    /// Removes the last element from the vector and returns it, or `None` if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        unsafe {
            self.len -= 1;
            Some(ptr::read(self.as_ptr().add(self.len)))
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity.
    pub fn clear(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_slice());
            self.len = 0;
        }
    }

    /// Resizes the vector to `new_len` elements.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the difference
    /// with clones of `value`. If `new_len` is less than `len`, the vector is truncated.
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if new_len > self.len {
            self.reserve(new_len - self.len);
            unsafe {
                for i in self.len..new_len {
                    ptr::write(self.as_mut_ptr().add(i), value.clone());
                }
            }
        } else if new_len < self.len {
            unsafe {
                for i in new_len..self.len {
                    ptr::drop_in_place(self.as_mut_ptr().add(i));
                }
            }
        }
        self.len = new_len;
    }

    /// Reallocates the buffer to the specified capacity.
    fn realloc(&mut self, new_capacity: usize) {
        assert!(new_capacity > 0, "capacity must be non-zero");

        let new_layout = Layout::from_size_align(new_capacity * std::mem::size_of::<T>(), ALIGN)
            .expect("invalid layout");

        let new_ptr = if self.capacity == 0 {
            // Initial allocation
            unsafe { alloc(new_layout) }
        } else {
            // Reallocation
            let old_layout =
                Layout::from_size_align(self.capacity * std::mem::size_of::<T>(), ALIGN)
                    .expect("invalid layout");

            unsafe { realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size()) }
        };

        let new_ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => handle_alloc_error(new_layout),
        };

        self.ptr = new_ptr;
        self.capacity = new_capacity;
    }

    /// Returns an iterator over the vector.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns a mutable iterator over the vector.
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, const ALIGN: usize> Default for AlignedVec<T, ALIGN> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const ALIGN: usize> Drop for AlignedVec<T, ALIGN> {
    fn drop(&mut self) {
        // Drop all elements
        unsafe {
            ptr::drop_in_place(self.as_mut_slice());
        }

        // Deallocate memory
        if self.capacity > 0 {
            let layout = Layout::from_size_align(self.capacity * std::mem::size_of::<T>(), ALIGN)
                .expect("invalid layout");

            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T, const ALIGN: usize> Deref for AlignedVec<T, ALIGN> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const ALIGN: usize> DerefMut for AlignedVec<T, ALIGN> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const ALIGN: usize> std::ops::Index<usize> for AlignedVec<T, ALIGN> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        &self.as_slice()[index]
    }
}

impl<T, const ALIGN: usize> std::ops::IndexMut<usize> for AlignedVec<T, ALIGN> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.as_mut_slice()[index]
    }
}

// Safety: AlignedVec<T, ALIGN> is Send if T is Send
unsafe impl<T: Send, const ALIGN: usize> Send for AlignedVec<T, ALIGN> {}

// Safety: AlignedVec<T, ALIGN> is Sync if T is Sync
unsafe impl<T: Sync, const ALIGN: usize> Sync for AlignedVec<T, ALIGN> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let mut vec: AlignedVec<f32, 64> = AlignedVec::new();
        vec.push(1.0);
        vec.push(2.0);
        vec.push(3.0);

        let ptr = vec.as_ptr() as usize;
        assert_eq!(ptr % 64, 0, "Pointer must be 64-byte aligned");
    }

    #[test]
    fn test_alignment_after_realloc() {
        let mut vec: AlignedVec<f32, 64> = AlignedVec::with_capacity(2);

        // Force reallocation
        for i in 0..100 {
            vec.push(i as f32);
            let ptr = vec.as_ptr() as usize;
            assert_eq!(ptr % 64, 0, "Pointer must remain 64-byte aligned after realloc");
        }
    }

    #[test]
    fn test_push_pop() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();

        vec.push(1);
        vec.push(2);
        vec.push(3);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.pop(), Some(2));
        assert_eq!(vec.pop(), Some(1));
        assert_eq!(vec.pop(), None);
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();
        vec.push(1);
        vec.push(2);
        vec.push(3);

        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.capacity() > 0); // Capacity should remain
    }

    #[test]
    fn test_resize() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();
        vec.resize(5, 42);

        assert_eq!(vec.len(), 5);
        for &val in vec.iter() {
            assert_eq!(val, 42);
        }
    }

    #[test]
    fn test_indexing() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();
        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert_eq!(vec[0], 10);
        assert_eq!(vec[1], 20);
        assert_eq!(vec[2], 30);

        vec[1] = 99;
        assert_eq!(vec[1], 99);
    }

    #[test]
    fn test_deref() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();
        vec.push(1);
        vec.push(2);
        vec.push(3);

        // Test that we can use slice methods
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.first(), Some(&1));
        assert_eq!(vec.last(), Some(&3));
    }

    #[test]
    fn test_with_capacity() {
        let vec: AlignedVec<i32, 64> = AlignedVec::with_capacity(100);
        assert_eq!(vec.len(), 0);
        assert!(vec.capacity() >= 100);

        let ptr = vec.as_ptr() as usize;
        assert_eq!(ptr % 64, 0, "Pre-allocated pointer must be aligned");
    }

    #[test]
    fn test_reserve() {
        let mut vec: AlignedVec<i32, 64> = AlignedVec::new();
        vec.reserve(50);

        assert!(vec.capacity() >= 50);
        let ptr = vec.as_ptr() as usize;
        assert_eq!(ptr % 64, 0, "Reserved pointer must be aligned");
    }

    #[test]
    #[should_panic(expected = "Alignment must be a power of two")]
    fn test_invalid_alignment() {
        let _vec: AlignedVec<i32, 63> = AlignedVec::new();
    }

    #[test]
    fn test_drop_with_complex_type() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let drop_count = Arc::new(AtomicUsize::new(0));

        struct DropCounter {
            counter: Arc<AtomicUsize>,
        }

        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        {
            let mut vec: AlignedVec<DropCounter, 64> = AlignedVec::new();
            for _ in 0..10 {
                vec.push(DropCounter { counter: drop_count.clone() });
            }
            // Vec goes out of scope, should drop all elements
        }

        assert_eq!(drop_count.load(Ordering::SeqCst), 10);
    }
}
