//! SIMD Vec3 type that processes 4 vectors at once.

use crate::Vec3;
use wide::f32x4;

/// Four Vec3s packed for SIMD processing (Structure-of-Arrays layout).
///
/// Processes 4 Vec3 operations in parallel using 128-bit SIMD instructions.
///
/// # Memory Layout
///
/// This type uses cache-line aligned storage (16-byte alignment for 128-bit SIMD).
/// When storing multiple `Vec3x4` in a vector, use `AlignedVec<Vec3x4, 64>` to
/// prevent cache line splits and false sharing.
///
/// # Example
/// ```
/// use engine_math::Vec3;
/// use engine_math::simd::{Vec3x4, vec3_aos_to_soa_4};
/// use engine_math::aligned::AlignedVec;
///
/// // Four positions (Array-of-Structures)
/// let positions = [
///     Vec3::new(1.0, 2.0, 3.0),
///     Vec3::new(4.0, 5.0, 6.0),
///     Vec3::new(7.0, 8.0, 9.0),
///     Vec3::new(10.0, 11.0, 12.0),
/// ];
///
/// // Convert to Structure-of-Arrays for SIMD
/// let pos_simd = vec3_aos_to_soa_4(&positions);
///
/// // SIMD operations process all 4 at once
/// let scaled = pos_simd * 2.0;
///
/// // For bulk storage, use cache-aligned vector
/// let mut bulk_positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();
/// bulk_positions.push(pos_simd);
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))] // 16-byte alignment for 128-bit SIMD
pub struct Vec3x4 {
    /// Four X components packed together
    pub x: f32x4,
    /// Four Y components packed together
    pub y: f32x4,
    /// Four Z components packed together
    pub z: f32x4,
}

impl Vec3x4 {
    /// Create a new Vec3x4 from SIMD lanes.
    #[inline]
    pub fn new(x: f32x4, y: f32x4, z: f32x4) -> Self {
        Self { x, y, z }
    }

    /// Splat a single Vec3 to all 4 lanes.
    #[inline]
    pub fn splat(v: Vec3) -> Self {
        Self { x: f32x4::splat(v.x), y: f32x4::splat(v.y), z: f32x4::splat(v.z) }
    }

    /// Add two Vec3x4 (SIMD).
    #[inline]
    pub fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }

    /// Subtract two Vec3x4 (SIMD).
    #[inline]
    pub fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }

    /// Multiply Vec3x4 by scalar (SIMD).
    #[inline]
    pub fn mul_scalar(self, scalar: f32) -> Self {
        let s = f32x4::splat(scalar);
        Self { x: self.x * s, y: self.y * s, z: self.z * s }
    }

    /// Fused multiply-add: self + (rhs * scalar) in one operation.
    ///
    /// This is faster than separate multiply and add on most CPUs.
    /// Useful for physics integration: `new_pos = pos + vel * dt`
    #[inline(always)]
    pub fn mul_add(self, rhs: Self, scalar: f32) -> Self {
        let s = f32x4::splat(scalar);
        Self { x: self.x + rhs.x * s, y: self.y + rhs.y * s, z: self.z + rhs.z * s }
    }

    /// Dot product of 4 vector pairs (returns 4 results).
    #[inline]
    pub fn dot(self, other: Self) -> f32x4 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Squared magnitude of 4 vectors (returns 4 results).
    #[inline]
    pub fn magnitude_squared(self) -> f32x4 {
        self.dot(self)
    }

    /// Component-wise minimum.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self { x: self.x.min(other.x), y: self.y.min(other.y), z: self.z.min(other.z) }
    }

    /// Component-wise maximum.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self { x: self.x.max(other.x), y: self.y.max(other.y), z: self.z.max(other.z) }
    }

    /// Convert back to array of 4 Vec3s.
    #[inline]
    pub fn to_array(self) -> [Vec3; 4] {
        let x_arr = self.x.to_array();
        let y_arr = self.y.to_array();
        let z_arr = self.z.to_array();

        [
            Vec3::new(x_arr[0], y_arr[0], z_arr[0]),
            Vec3::new(x_arr[1], y_arr[1], z_arr[1]),
            Vec3::new(x_arr[2], y_arr[2], z_arr[2]),
            Vec3::new(x_arr[3], y_arr[3], z_arr[3]),
        ]
    }

    /// Load from aligned memory (16-byte aligned).
    ///
    /// # Safety
    ///
    /// The pointer must be:
    /// - Valid for reads of 12 f32 values (3 components * 4 vectors)
    /// - Aligned to 16 bytes
    /// - Point to properly initialized data
    ///
    /// Using unaligned pointers will result in undefined behavior on some platforms.
    #[inline]
    pub unsafe fn load_aligned(ptr: *const f32) -> Self {
        debug_assert_eq!(ptr as usize % 16, 0, "Pointer must be 16-byte aligned");

        // Load 4 x components, 4 y components, 4 z components
        // Memory layout: [x0 x1 x2 x3 y0 y1 y2 y3 z0 z1 z2 z3]
        let x = f32x4::new([*ptr.add(0), *ptr.add(1), *ptr.add(2), *ptr.add(3)]);
        let y = f32x4::new([*ptr.add(4), *ptr.add(5), *ptr.add(6), *ptr.add(7)]);
        let z = f32x4::new([*ptr.add(8), *ptr.add(9), *ptr.add(10), *ptr.add(11)]);

        Self { x, y, z }
    }

    /// Store to aligned memory (16-byte aligned).
    ///
    /// # Safety
    ///
    /// The pointer must be:
    /// - Valid for writes of 12 f32 values (3 components * 4 vectors)
    /// - Aligned to 16 bytes
    ///
    /// Using unaligned pointers will result in undefined behavior on some platforms.
    #[inline]
    pub unsafe fn store_aligned(self, ptr: *mut f32) {
        debug_assert_eq!(ptr as usize % 16, 0, "Pointer must be 16-byte aligned");

        let x_arr = self.x.to_array();
        let y_arr = self.y.to_array();
        let z_arr = self.z.to_array();

        // Store in SoA layout: [x0 x1 x2 x3 y0 y1 y2 y3 z0 z1 z2 z3]
        *ptr.add(0) = x_arr[0];
        *ptr.add(1) = x_arr[1];
        *ptr.add(2) = x_arr[2];
        *ptr.add(3) = x_arr[3];
        *ptr.add(4) = y_arr[0];
        *ptr.add(5) = y_arr[1];
        *ptr.add(6) = y_arr[2];
        *ptr.add(7) = y_arr[3];
        *ptr.add(8) = z_arr[0];
        *ptr.add(9) = z_arr[1];
        *ptr.add(10) = z_arr[2];
        *ptr.add(11) = z_arr[3];
    }
}

// Operator overloads for ergonomic API
use std::ops::{Add, Mul, Sub};

impl Add for Vec3x4 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.add(rhs)
    }
}

impl Sub for Vec3x4 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.sub(rhs)
    }
}

impl Mul<f32> for Vec3x4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        self.mul_scalar(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3x4_add() {
        let a = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
        let b = Vec3x4::splat(Vec3::new(4.0, 5.0, 6.0));
        let c = a + b;

        let result = c.to_array();
        for v in &result {
            assert_eq!(*v, Vec3::new(5.0, 7.0, 9.0));
        }
    }

    #[test]
    fn test_vec3x4_mul_scalar() {
        let v = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
        let scaled = v * 2.0;

        let result = scaled.to_array();
        for v in &result {
            assert_eq!(*v, Vec3::new(2.0, 4.0, 6.0));
        }
    }

    #[test]
    fn test_vec3x4_mul_add() {
        let pos = Vec3x4::splat(Vec3::new(0.0, 0.0, 0.0));
        let vel = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
        let dt = 0.1;

        // new_pos = pos + vel * dt
        let new_pos = pos.mul_add(vel, dt);

        let result = new_pos.to_array();
        for v in &result {
            assert!((v.x - 0.1).abs() < 1e-6);
            assert!((v.y - 0.2).abs() < 1e-6);
            assert!((v.z - 0.3).abs() < 1e-6);
        }
    }
}
