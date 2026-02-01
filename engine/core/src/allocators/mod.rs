//! Memory pooling allocators for faster allocations
//!
//! This module provides specialized allocators designed to minimize fragmentation
//! and improve allocation performance for common game engine patterns:
//!
//! - **Arena Allocator**: Linear allocation for temporary per-frame data
//! - **Pool Allocator**: Fixed-size object pooling (entities, components)
//! - **Frame Allocator**: Resets every frame with zero fragmentation
//!
//! # Design Goals
//!
//! - 5-15% performance improvement in allocation-heavy scenarios
//! - Zero fragmentation for temporary allocations
//! - Cache-friendly memory layouts
//! - Type-safe APIs with minimal overhead
//!
//! # Examples
//!
//! ```
//! use engine_core::allocators::{Arena, PoolAllocator, FrameAllocator};
//! use engine_core::Transform;
//!
//! // Arena for temporary allocations
//! let mut arena = Arena::new();
//! let temp_transforms = arena.alloc_slice::<Transform>(1000);
//!
//! // Pool for fixed-size objects
//! let mut pool = PoolAllocator::<Transform>::with_capacity(1000);
//! let transform_ptr = pool.alloc(Transform::default());
//!
//! // Frame allocator - resets every frame
//! let mut frame_alloc = FrameAllocator::with_capacity(1024 * 1024); // 1MB
//! let temp_data = frame_alloc.alloc_slice::<f32>(100);
//! frame_alloc.reset(); // Called each frame
//! ```

pub mod arena;
pub mod frame;
pub mod pool;

pub use arena::Arena;
pub use frame::FrameAllocator;
pub use pool::PoolAllocator;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestData {
        value: u64,
    }

    #[test]
    fn test_arena_basic() {
        let mut arena = Arena::new();
        let slice = arena.alloc_slice::<u64>(10);
        slice[0] = 42;
        assert_eq!(slice[0], 42);
    }

    #[test]
    fn test_pool_basic() {
        let mut pool = PoolAllocator::<u64>::with_capacity(10);
        unsafe {
            let ptr = pool.alloc(42) as *mut u64;
            assert_eq!(*ptr, 42);
            pool.free(&mut *ptr);
        }
    }

    #[test]
    fn test_frame_basic() {
        let mut frame = FrameAllocator::with_capacity(1024);
        let slice = frame.alloc_slice::<u64>(10);
        slice[0] = 42;
        assert_eq!(slice[0], 42);
        frame.reset();
    }

    #[test]
    fn test_integration_all_allocators() {
        let mut arena = Arena::new();
        let mut pool = PoolAllocator::<TestData>::with_capacity(100);
        let mut frame = FrameAllocator::with_capacity(4096);

        // Use all allocators together
        let arena_slice = arena.alloc_slice::<TestData>(1);
        arena_slice[0] = TestData { value: 1 };

        let frame_slice = frame.alloc_slice::<TestData>(1);
        frame_slice[0] = TestData { value: 3 };

        assert_eq!(arena_slice[0].value, 1);
        assert_eq!(frame_slice[0].value, 3);

        // Cleanup
        frame.reset();

        // Pool test separately to avoid borrowing issues
        unsafe {
            let pool_val = pool.alloc(TestData { value: 2 }) as *mut TestData;
            assert_eq!((*pool_val).value, 2);
            pool.free(&mut *pool_val);
        }
    }
}
