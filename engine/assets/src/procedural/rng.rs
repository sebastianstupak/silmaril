//! Deterministic random number generator for procedural generation
//!
//! Uses xorshift64* algorithm for fast, deterministic random number generation
//! that produces identical sequences across all platforms.

/// Deterministic random number generator using xorshift64*
///
/// This RNG is:
/// - Fast (single cycle on modern CPUs)
/// - Deterministic (same seed = same sequence)
/// - Cross-platform (no platform-specific behavior)
/// - Non-cryptographic (don't use for security!)
///
/// # Algorithm
///
/// Uses xorshift64* from Marsaglia (2003) with multiplier from Vigna (2016).
/// This is explicitly designed to be cross-platform deterministic.
///
/// # Examples
///
/// ```
/// use engine_assets::procedural::SeededRng;
///
/// let mut rng1 = SeededRng::new(42);
/// let mut rng2 = SeededRng::new(42);
///
/// // Same seed produces same sequence
/// assert_eq!(rng1.next_u32(), rng2.next_u32());
/// assert_eq!(rng1.next_u32(), rng2.next_u32());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    /// Create a new RNG with the given seed
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let rng = SeededRng::new(12345);
    /// ```
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Ensure seed is never zero (xorshift requirement)
        let state = if seed == 0 { 1 } else { seed };
        Self { state }
    }

    /// Generate next u64 value
    ///
    /// Uses xorshift64* algorithm from Marsaglia (2003).
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Generate next u32 value
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let mut rng = SeededRng::new(42);
    /// let value = rng.next_u32();
    /// assert!(value <= u32::MAX as u32);
    /// ```
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Generate next f32 value in [0.0, 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let mut rng = SeededRng::new(42);
    /// let value = rng.next_f32();
    /// assert!(value >= 0.0 && value < 1.0);
    /// ```
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        // Use upper 24 bits for mantissa (IEEE 754 single precision)
        // This ensures uniform distribution in [0.0, 1.0)
        let value = self.next_u32() >> 8; // Use upper 24 bits
        (value as f32) * (1.0 / 16_777_216.0) // 2^24
    }

    /// Generate next f64 value in [0.0, 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let mut rng = SeededRng::new(42);
    /// let value = rng.next_f64();
    /// assert!(value >= 0.0 && value < 1.0);
    /// ```
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Use upper 53 bits for mantissa (IEEE 754 double precision)
        let value = self.next_u64() >> 11; // Use upper 53 bits
        (value as f64) * (1.0 / 9_007_199_254_740_992.0) // 2^53
    }

    /// Generate next value in range [min, max)
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let mut rng = SeededRng::new(42);
    /// let value = rng.next_range(10.0, 20.0);
    /// assert!(value >= 10.0 && value < 20.0);
    /// ```
    #[inline]
    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Generate next integer in range [min, max) (exclusive max)
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::procedural::SeededRng;
    ///
    /// let mut rng = SeededRng::new(42);
    /// let value = rng.next_range_u32(10, 20);
    /// assert!(value >= 10 && value < 20);
    /// ```
    #[inline]
    pub fn next_range_u32(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = max - min;
        min + (self.next_u32() % range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_non_zero_seed() {
        let rng = SeededRng::new(42);
        assert_eq!(rng.state, 42);
    }

    #[test]
    fn test_new_zero_seed() {
        // Zero seed should be converted to 1
        let rng = SeededRng::new(0);
        assert_eq!(rng.state, 1);
    }

    #[test]
    fn test_determinism_u32() {
        let mut rng1 = SeededRng::new(12345);
        let mut rng2 = SeededRng::new(12345);

        // Same seed produces same sequence
        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn test_determinism_f32() {
        let mut rng1 = SeededRng::new(54321);
        let mut rng2 = SeededRng::new(54321);

        // Same seed produces same sequence
        for _ in 0..100 {
            assert_eq!(rng1.next_f32(), rng2.next_f32());
        }
    }

    #[test]
    fn test_different_seeds_produce_different_sequences() {
        let mut rng1 = SeededRng::new(111);
        let mut rng2 = SeededRng::new(222);

        // Different seeds should produce different values
        let values1: Vec<u32> = (0..10).map(|_| rng1.next_u32()).collect();
        let values2: Vec<u32> = (0..10).map(|_| rng2.next_u32()).collect();

        assert_ne!(values1, values2);
    }

    #[test]
    fn test_f32_range() {
        let mut rng = SeededRng::new(999);

        // Test that all values are in [0.0, 1.0)
        for _ in 0..1000 {
            let value = rng.next_f32();
            assert!(value >= 0.0);
            assert!(value < 1.0);
        }
    }

    #[test]
    fn test_f64_range() {
        let mut rng = SeededRng::new(888);

        // Test that all values are in [0.0, 1.0)
        for _ in 0..1000 {
            let value = rng.next_f64();
            assert!(value >= 0.0);
            assert!(value < 1.0);
        }
    }

    #[test]
    fn test_next_range() {
        let mut rng = SeededRng::new(777);

        // Test custom range
        for _ in 0..1000 {
            let value = rng.next_range(10.0, 20.0);
            assert!(value >= 10.0);
            assert!(value < 20.0);
        }
    }

    #[test]
    fn test_next_range_u32() {
        let mut rng = SeededRng::new(666);

        // Test integer range
        for _ in 0..1000 {
            let value = rng.next_range_u32(5, 15);
            assert!(value >= 5);
            assert!(value < 15);
        }
    }

    #[test]
    fn test_next_range_u32_edge_cases() {
        let mut rng = SeededRng::new(555);

        // Min == Max
        assert_eq!(rng.next_range_u32(10, 10), 10);

        // Min > Max (should return min)
        assert_eq!(rng.next_range_u32(20, 10), 20);
    }

    #[test]
    fn test_distribution_uniformity() {
        let mut rng = SeededRng::new(444);
        let mut buckets = [0u32; 10];

        // Generate 10000 values and count distribution
        for _ in 0..10000 {
            let value = rng.next_f32();
            let bucket = (value * 10.0).floor() as usize;
            if bucket < 10 {
                buckets[bucket] += 1;
            }
        }

        // Each bucket should have roughly 1000 values (±20%)
        for count in buckets {
            assert!(count > 800 && count < 1200, "Bucket count: {}", count);
        }
    }

    #[test]
    fn test_clone() {
        let rng1 = SeededRng::new(333);
        let mut rng2 = rng1.clone();

        // Cloned RNG should produce same sequence
        let mut rng3 = SeededRng::new(333);

        assert_eq!(rng2.next_u32(), rng3.next_u32());
    }

    #[test]
    fn test_debug_format() {
        let rng = SeededRng::new(222);
        let debug_str = format!("{:?}", rng);
        assert!(debug_str.contains("SeededRng"));
        assert!(debug_str.contains("state"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property test: Same seed always produces same first value
        #[test]
        fn prop_determinism(seed: u64) {
            let mut rng1 = SeededRng::new(seed);
            let mut rng2 = SeededRng::new(seed);
            prop_assert_eq!(rng1.next_u32(), rng2.next_u32());
        }

        /// Property test: f32 values are always in [0.0, 1.0)
        #[test]
        fn prop_f32_range(seed: u64) {
            let mut rng = SeededRng::new(seed);
            let value = rng.next_f32();
            prop_assert!(value >= 0.0 && value < 1.0);
        }

        /// Property test: Custom range is respected
        #[test]
        fn prop_custom_range(seed: u64, min in 0.0f32..100.0f32, max in 100.0f32..200.0f32) {
            let mut rng = SeededRng::new(seed);
            let value = rng.next_range(min, max);
            prop_assert!(value >= min && value < max);
        }
    }
}
