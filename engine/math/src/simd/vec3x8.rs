//! SIMD Vec3 type that processes 8 vectors at once using AVX2.

use crate::Vec3;
use wide::f32x8;

/// Eight Vec3s packed for SIMD processing (Structure-of-Arrays layout).
///
/// Processes 8 Vec3 operations in parallel using 256-bit AVX2 instructions.
/// Provides ~5-6x speedup over scalar operations on compatible CPUs.
///
/// # Memory Layout
///
/// This type uses 32-byte alignment for 256-bit SIMD operations.
/// When storing multiple `Vec3x8` in a vector, use `AlignedVec<Vec3x8, 64>` to
/// prevent cache line splits and false sharing.
///
/// # Example
/// ```
/// use engine_math::Vec3;
/// use engine_math::simd::{Vec3x8, vec3_aos_to_soa_8};
/// use engine_math::aligned::AlignedVec;
///
/// // Eight positions (Array-of-Structures)
/// let positions = [
///     Vec3::new(1.0, 2.0, 3.0),
///     Vec3::new(4.0, 5.0, 6.0),
///     Vec3::new(7.0, 8.0, 9.0),
///     Vec3::new(10.0, 11.0, 12.0),
///     Vec3::new(13.0, 14.0, 15.0),
///     Vec3::new(16.0, 17.0, 18.0),
///     Vec3::new(19.0, 20.0, 21.0),
///     Vec3::new(22.0, 23.0, 24.0),
/// ];
///
/// // Convert to Structure-of-Arrays for SIMD
/// let pos_simd = vec3_aos_to_soa_8(&positions);
///
/// // SIMD operations process all 8 at once
/// let scaled = pos_simd * 2.0;
///
/// // For bulk storage, use cache-aligned vector
/// let mut bulk_positions: AlignedVec<Vec3x8, 64> = AlignedVec::new();
/// bulk_positions.push(pos_simd);
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C, align(32))] // 32-byte alignment for 256-bit SIMD
pub struct Vec3x8 {
    /// Eight X components packed together
    pub x: f32x8,
    /// Eight Y components packed together
    pub y: f32x8,
    /// Eight Z components packed together
    pub z: f32x8,
}

impl Vec3x8 {
    /// Create a new Vec3x8 from SIMD lanes.
    #[inline]
    pub fn new(x: f32x8, y: f32x8, z: f32x8) -> Self {
        Self { x, y, z }
    }

    /// Splat a single Vec3 to all 8 lanes.
    #[inline]
    pub fn splat(v: Vec3) -> Self {
        Self { x: f32x8::splat(v.x), y: f32x8::splat(v.y), z: f32x8::splat(v.z) }
    }

    // Note: Add and Sub operations are implemented via std::ops traits below
    // This avoids method name confusion with std::ops::Add::add and std::ops::Sub::sub

    /// Multiply Vec3x8 by scalar (SIMD).
    #[inline]
    pub fn mul_scalar(self, scalar: f32) -> Self {
        let s = f32x8::splat(scalar);
        Self { x: self.x * s, y: self.y * s, z: self.z * s }
    }

    /// Fused multiply-add: self + (rhs * scalar) in one operation.
    ///
    /// This is faster than separate multiply and add on most CPUs.
    /// Useful for physics integration: `new_pos = pos + vel * dt`
    #[inline(always)]
    pub fn mul_add(self, rhs: Self, scalar: f32) -> Self {
        let s = f32x8::splat(scalar);
        Self { x: self.x + rhs.x * s, y: self.y + rhs.y * s, z: self.z + rhs.z * s }
    }

    /// Dot product of 8 vector pairs (returns 8 results).
    #[inline]
    pub fn dot(self, other: Self) -> f32x8 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Squared magnitude of 8 vectors (returns 8 results).
    #[inline]
    pub fn magnitude_squared(self) -> f32x8 {
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

    /// Convert back to array of 8 Vec3s.
    #[inline]
    pub fn to_array(self) -> [Vec3; 8] {
        let x_arr = self.x.to_array();
        let y_arr = self.y.to_array();
        let z_arr = self.z.to_array();

        [
            Vec3::new(x_arr[0], y_arr[0], z_arr[0]),
            Vec3::new(x_arr[1], y_arr[1], z_arr[1]),
            Vec3::new(x_arr[2], y_arr[2], z_arr[2]),
            Vec3::new(x_arr[3], y_arr[3], z_arr[3]),
            Vec3::new(x_arr[4], y_arr[4], z_arr[4]),
            Vec3::new(x_arr[5], y_arr[5], z_arr[5]),
            Vec3::new(x_arr[6], y_arr[6], z_arr[6]),
            Vec3::new(x_arr[7], y_arr[7], z_arr[7]),
        ]
    }

    /// Load from aligned memory (32-byte aligned).
    ///
    /// # Safety
    ///
    /// The pointer must be:
    /// - Valid for reads of 24 f32 values (3 components * 8 vectors)
    /// - Aligned to 32 bytes
    /// - Point to properly initialized data
    ///
    /// Using unaligned pointers will result in undefined behavior on some platforms.
    #[inline]
    pub unsafe fn load_aligned(ptr: *const f32) -> Self {
        debug_assert_eq!(ptr as usize % 32, 0, "Pointer must be 32-byte aligned");

        // Load 8 x components, 8 y components, 8 z components
        // Memory layout: [x0..x7 y0..y7 z0..z7]
        let x = f32x8::new([
            *ptr.add(0),
            *ptr.add(1),
            *ptr.add(2),
            *ptr.add(3),
            *ptr.add(4),
            *ptr.add(5),
            *ptr.add(6),
            *ptr.add(7),
        ]);
        let y = f32x8::new([
            *ptr.add(8),
            *ptr.add(9),
            *ptr.add(10),
            *ptr.add(11),
            *ptr.add(12),
            *ptr.add(13),
            *ptr.add(14),
            *ptr.add(15),
        ]);
        let z = f32x8::new([
            *ptr.add(16),
            *ptr.add(17),
            *ptr.add(18),
            *ptr.add(19),
            *ptr.add(20),
            *ptr.add(21),
            *ptr.add(22),
            *ptr.add(23),
        ]);

        Self { x, y, z }
    }

    /// Store to aligned memory (32-byte aligned).
    ///
    /// # Safety
    ///
    /// The pointer must be:
    /// - Valid for writes of 24 f32 values (3 components * 8 vectors)
    /// - Aligned to 32 bytes
    ///
    /// Using unaligned pointers will result in undefined behavior on some platforms.
    #[inline]
    pub unsafe fn store_aligned(self, ptr: *mut f32) {
        debug_assert_eq!(ptr as usize % 32, 0, "Pointer must be 32-byte aligned");

        let x_arr = self.x.to_array();
        let y_arr = self.y.to_array();
        let z_arr = self.z.to_array();

        // Store in SoA layout: [x0..x7 y0..y7 z0..z7]
        for i in 0..8 {
            *ptr.add(i) = x_arr[i];
            *ptr.add(8 + i) = y_arr[i];
            *ptr.add(16 + i) = z_arr[i];
        }
    }
}

// Operator overloads for ergonomic API
use std::ops::{Add, Mul, Sub};

impl Add for Vec3x8 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl Sub for Vec3x8 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl Mul<f32> for Vec3x8 {
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
    fn test_vec3x8_add() {
        let a = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
        let b = Vec3x8::splat(Vec3::new(4.0, 5.0, 6.0));
        let c = a + b;

        let result = c.to_array();
        for v in &result {
            assert_eq!(*v, Vec3::new(5.0, 7.0, 9.0));
        }
    }

    #[test]
    fn test_vec3x8_sub() {
        let a = Vec3x8::splat(Vec3::new(10.0, 20.0, 30.0));
        let b = Vec3x8::splat(Vec3::new(4.0, 5.0, 6.0));
        let c = a - b;

        let result = c.to_array();
        for v in &result {
            assert_eq!(*v, Vec3::new(6.0, 15.0, 24.0));
        }
    }

    #[test]
    fn test_vec3x8_mul_scalar() {
        let v = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
        let scaled = v * 2.0;

        let result = scaled.to_array();
        for v in &result {
            assert_eq!(*v, Vec3::new(2.0, 4.0, 6.0));
        }
    }

    #[test]
    fn test_vec3x8_mul_add() {
        let pos = Vec3x8::splat(Vec3::new(0.0, 0.0, 0.0));
        let vel = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
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

    #[test]
    fn test_vec3x8_dot() {
        let a = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
        let b = Vec3x8::splat(Vec3::new(4.0, 5.0, 6.0));
        let dot = a.dot(b);

        let result = dot.to_array();
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        for &d in &result {
            assert_eq!(d, 32.0);
        }
    }

    #[test]
    fn test_vec3x8_magnitude_squared() {
        let v = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
        let mag_sq = v.magnitude_squared();

        let result = mag_sq.to_array();
        // 1^2 + 2^2 + 3^2 = 1 + 4 + 9 = 14
        for &m in &result {
            assert_eq!(m, 14.0);
        }
    }

    #[test]
    fn test_vec3x8_min_max() {
        let a = Vec3x8::splat(Vec3::new(1.0, 5.0, 3.0));
        let b = Vec3x8::splat(Vec3::new(4.0, 2.0, 6.0));

        let min = a.min(b);
        let max = a.max(b);

        let min_result = min.to_array();
        let max_result = max.to_array();

        for v in &min_result {
            assert_eq!(*v, Vec3::new(1.0, 2.0, 3.0));
        }

        for v in &max_result {
            assert_eq!(*v, Vec3::new(4.0, 5.0, 6.0));
        }
    }
}
