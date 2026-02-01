//! Utilities for converting between AoS and SoA layouts.

use super::{Vec3x4, Vec3x8};
use crate::Vec3;
use wide::{f32x4, f32x8};

/// Convert 4 Vec3s from Array-of-Structures to Structure-of-Arrays.
///
/// This is the conversion needed before SIMD processing.
///
/// # Performance
/// This conversion has some overhead (~5ns), but the SIMD operations
/// are 2-4x faster, so the tradeoff is worth it for batches of 4+.
#[inline]
pub fn vec3_aos_to_soa_4(aos: &[Vec3; 4]) -> Vec3x4 {
    Vec3x4 {
        x: f32x4::new([aos[0].x, aos[1].x, aos[2].x, aos[3].x]),
        y: f32x4::new([aos[0].y, aos[1].y, aos[2].y, aos[3].y]),
        z: f32x4::new([aos[0].z, aos[1].z, aos[2].z, aos[3].z]),
    }
}

/// Convert Vec3x4 back to array of 4 Vec3s.
///
/// This is the conversion needed after SIMD processing to write back to ECS.
#[inline]
pub fn vec3_soa_to_aos_4(soa: &Vec3x4) -> [Vec3; 4] {
    soa.to_array()
}

/// Convert 8 Vec3s from Array-of-Structures to Structure-of-Arrays.
///
/// This is the conversion needed before AVX2 SIMD processing.
///
/// # Performance
/// This conversion has some overhead (~8ns), but the AVX2 operations
/// are 5-6x faster, so the tradeoff is worth it for batches of 8+.
#[inline]
pub fn vec3_aos_to_soa_8(aos: &[Vec3; 8]) -> Vec3x8 {
    Vec3x8 {
        x: f32x8::new([
            aos[0].x, aos[1].x, aos[2].x, aos[3].x, aos[4].x, aos[5].x, aos[6].x, aos[7].x,
        ]),
        y: f32x8::new([
            aos[0].y, aos[1].y, aos[2].y, aos[3].y, aos[4].y, aos[5].y, aos[6].y, aos[7].y,
        ]),
        z: f32x8::new([
            aos[0].z, aos[1].z, aos[2].z, aos[3].z, aos[4].z, aos[5].z, aos[6].z, aos[7].z,
        ]),
    }
}

/// Convert Vec3x8 back to array of 8 Vec3s.
///
/// This is the conversion needed after AVX2 SIMD processing to write back to ECS.
#[inline]
pub fn vec3_soa_to_aos_8(soa: &Vec3x8) -> [Vec3; 8] {
    soa.to_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aos_soa_roundtrip_4() {
        let original = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
            Vec3::new(10.0, 11.0, 12.0),
        ];

        let soa = vec3_aos_to_soa_4(&original);
        let result = vec3_soa_to_aos_4(&soa);

        for i in 0..4 {
            assert_eq!(original[i], result[i]);
        }
    }

    #[test]
    fn test_aos_soa_roundtrip_8() {
        let original = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
            Vec3::new(10.0, 11.0, 12.0),
            Vec3::new(13.0, 14.0, 15.0),
            Vec3::new(16.0, 17.0, 18.0),
            Vec3::new(19.0, 20.0, 21.0),
            Vec3::new(22.0, 23.0, 24.0),
        ];

        let soa = vec3_aos_to_soa_8(&original);
        let result = vec3_soa_to_aos_8(&soa);

        for i in 0..8 {
            assert_eq!(original[i], result[i]);
        }
    }
}
