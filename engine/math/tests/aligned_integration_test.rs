//! Integration tests for cache-aligned memory allocations.

#[cfg(feature = "simd")]
mod aligned_simd_tests {
    use engine_math::aligned::AlignedVec;
    use engine_math::simd::Vec3x4;
    use engine_math::Vec3;

    #[test]
    fn test_aligned_vec_with_vec3x4() {
        let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();

        // Add some Vec3x4 elements
        for i in 0..100 {
            positions.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
        }

        // Verify alignment
        let ptr = positions.as_ptr() as usize;
        assert_eq!(ptr % 64, 0, "Pointer must be 64-byte aligned");

        // Verify we can access the data
        assert_eq!(positions.len(), 100);

        // Verify SIMD operations work
        let velocities: AlignedVec<Vec3x4, 64> = (0..100)
            .map(|_| Vec3x4::splat(Vec3::new(0.1, 0.2, 0.3)))
            .collect::<Vec<_>>()
            .into_iter()
            .fold(AlignedVec::new(), |mut acc, v| {
                acc.push(v);
                acc
            });

        assert_eq!(velocities.len(), 100);

        // Physics integration
        let dt = 0.016;
        for i in 0..positions.len() {
            positions[i] = positions[i].mul_add(velocities[i], dt);
        }

        // Verify results
        let first = positions[0].to_array()[0];
        assert!((first.x - 0.0016).abs() < 0.0001);
        assert!((first.y - 0.0032).abs() < 0.0001);
        assert!((first.z - 0.0048).abs() < 0.0001);
    }

    #[test]
    fn test_vec3x4_alignment() {
        // Vec3x4 should be 16-byte aligned
        assert_eq!(std::mem::align_of::<Vec3x4>(), 16);
        assert_eq!(std::mem::size_of::<Vec3x4>(), 48); // 3 * f32x4 = 3 * 16 = 48 bytes
    }

    #[test]
    fn test_aligned_load_store() {
        let mut buffer: AlignedVec<f32, 16> = AlignedVec::with_capacity(12);
        buffer.resize(12, 0.0);

        let test_vec = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));

        unsafe {
            // Store
            test_vec.store_aligned(buffer.as_mut_ptr());

            // Load back
            let loaded = Vec3x4::load_aligned(buffer.as_ptr());

            // Verify
            let loaded_array = loaded.to_array();
            for vec in &loaded_array {
                assert_eq!(vec.x, 1.0);
                assert_eq!(vec.y, 2.0);
                assert_eq!(vec.z, 3.0);
            }
        }
    }

    #[test]
    fn test_aligned_vec_maintains_alignment_after_resize() {
        let mut vec: AlignedVec<Vec3x4, 64> = AlignedVec::new();

        // Force multiple reallocations
        for i in 0..1000 {
            vec.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));

            // Check alignment after each push
            let ptr = vec.as_ptr() as usize;
            assert_eq!(ptr % 64, 0, "Alignment lost after push {}", i);
        }
    }

    #[test]
    fn test_cache_line_separation() {
        // AlignedVec<T, 64> ensures each chunk starts at a cache line boundary
        let mut vec: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(100);

        for i in 0..100 {
            vec.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
        }

        // First element should be cache-line aligned
        let ptr = vec.as_ptr() as usize;
        assert_eq!(ptr % 64, 0);

        // Calculate expected size
        // Vec3x4 is 48 bytes, so with 64-byte cache line alignment,
        // elements will be tightly packed but the vector start is aligned
        assert_eq!(vec.len(), 100);
    }

    #[test]
    fn test_bulk_simd_operations() {
        const SIZE: usize = 10000;
        const CHUNKS: usize = SIZE / 4;

        let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(CHUNKS);
        let mut velocities: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(CHUNKS);

        // Initialize
        for i in 0..CHUNKS {
            positions.push(Vec3x4::splat(Vec3::new(i as f32, 0.0, 0.0)));
            velocities.push(Vec3x4::splat(Vec3::new(0.0, 0.0, 0.0)));
        }

        // Simulate physics for several frames
        let dt = 0.016;
        let gravity = Vec3x4::splat(Vec3::new(0.0, -9.81, 0.0));

        for _ in 0..100 {
            for i in 0..CHUNKS {
                // vel += gravity * dt
                velocities[i] = velocities[i].mul_add(gravity, dt);
                // pos += vel * dt
                positions[i] = positions[i].mul_add(velocities[i], dt);
            }
        }

        // Verify some physics occurred
        let final_pos = positions[0].to_array()[0];
        assert!(final_pos.y < -1.0, "Gravity should have pulled objects down");
    }
}

#[cfg(not(feature = "simd"))]
mod no_simd_tests {
    #[test]
    fn simd_feature_not_enabled() {
        // Placeholder test when SIMD is not enabled
        assert!(true);
    }
}
