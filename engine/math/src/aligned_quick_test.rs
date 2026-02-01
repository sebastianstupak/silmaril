// Quick compilation and functionality test for aligned module
#[cfg(test)]
mod quick_aligned_test {
    use crate::aligned::AlignedVec;

    #[test]
    fn aligned_vec_basic_test() {
        let mut vec: AlignedVec<f32, 64> = AlignedVec::new();
        vec.push(1.0);
        vec.push(2.0);
        vec.push(3.0);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1.0);
        assert_eq!(vec[1], 2.0);
        assert_eq!(vec[2], 3.0);

        // Check alignment
        let ptr = vec.as_ptr() as usize;
        assert_eq!(ptr % 64, 0);
    }

    #[cfg(feature = "simd")]
    #[test]
    fn aligned_vec_with_simd_test() {
        use crate::simd::Vec3x4;
        use crate::Vec3;

        let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();

        for i in 0..10 {
            positions.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
        }

        assert_eq!(positions.len(), 10);

        // Check alignment
        let ptr = positions.as_ptr() as usize;
        assert_eq!(ptr % 64, 0);

        // Check Vec3x4 alignment
        assert_eq!(std::mem::align_of::<Vec3x4>(), 16);
    }
}
