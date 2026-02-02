//! Content-addressable asset IDs using Blake3 hashing.
//!
//! AssetId provides a unique identifier for assets based on their content.
//! The same content always produces the same ID (deterministic).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Content-addressable asset ID (32-byte Blake3 hash).
///
/// # Examples
///
/// ```
/// use engine_assets::AssetId;
///
/// let data = b"Hello, world!";
/// let id = AssetId::from_content(data);
///
/// // Same content produces same ID
/// let id2 = AssetId::from_content(data);
/// assert_eq!(id, id2);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId([u8; 32]);

impl AssetId {
    /// Create an AssetId from content using Blake3 hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::AssetId;
    ///
    /// let id = AssetId::from_content(b"test data");
    /// ```
    pub fn from_content(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }

    /// Create an AssetId from a seed and parameters (for procedural generation).
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_assets::AssetId;
    ///
    /// let id = AssetId::from_seed_and_params(12345, b"terrain_params");
    /// ```
    pub fn from_seed_and_params(seed: u64, params: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed.to_le_bytes());
        hasher.update(params);
        Self(*hasher.finalize().as_bytes())
    }

    /// Get the raw bytes of the asset ID.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create an AssetId from raw bytes (for deserialization).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as hex string
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AssetId({})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        // Same content should produce same ID
        let data = b"Hello, world!";
        let id1 = AssetId::from_content(data);
        let id2 = AssetId::from_content(data);

        assert_eq!(id1, id2, "Same content should produce same ID");
    }

    #[test]
    fn test_different_content_different_id() {
        // Different content should produce different IDs
        let id1 = AssetId::from_content(b"content 1");
        let id2 = AssetId::from_content(b"content 2");

        assert_ne!(id1, id2, "Different content should produce different IDs");
    }

    #[test]
    fn test_display_trait() {
        let id = AssetId::from_content(b"test");
        let display_str = format!("{id}");

        // Should be 64 hex characters (32 bytes * 2)
        assert_eq!(display_str.len(), 64, "Display should show 64 hex chars");

        // Should only contain hex characters
        assert!(
            display_str.chars().all(|c| c.is_ascii_hexdigit()),
            "Display should only contain hex digits"
        );
    }

    #[test]
    fn test_debug_trait() {
        let id = AssetId::from_content(b"test");
        let debug_str = format!("{id:?}");

        // Debug should contain "AssetId(" prefix
        assert!(debug_str.starts_with("AssetId("), "Debug should start with AssetId(");
        assert!(debug_str.ends_with(')'), "Debug should end with )");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let id = AssetId::from_content(b"serialize me");

        // Serialize to bytes
        let serialized = bincode::serialize(&id).expect("Serialization should succeed");

        // Deserialize back
        let deserialized: AssetId =
            bincode::deserialize(&serialized).expect("Deserialization should succeed");

        assert_eq!(id, deserialized, "Roundtrip should preserve ID");
    }

    #[test]
    fn test_hash_trait() {
        use std::collections::HashMap;

        let id1 = AssetId::from_content(b"key1");
        let id2 = AssetId::from_content(b"key2");

        let mut map = HashMap::new();
        map.insert(id1, "value1");
        map.insert(id2, "value2");

        assert_eq!(map.get(&id1), Some(&"value1"));
        assert_eq!(map.get(&id2), Some(&"value2"));
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let id = AssetId::from_content(b"test");
        let bytes = *id.as_bytes();
        let id2 = AssetId::from_bytes(bytes);

        assert_eq!(id, id2, "from_bytes should preserve ID");
    }

    #[test]
    fn test_seed_and_params_deterministic() {
        let seed = 12345_u64;
        let params = b"terrain_params";

        let id1 = AssetId::from_seed_and_params(seed, params);
        let id2 = AssetId::from_seed_and_params(seed, params);

        assert_eq!(id1, id2, "Same seed and params should produce same ID");
    }

    #[test]
    fn test_seed_and_params_different_seed() {
        let params = b"terrain_params";

        let id1 = AssetId::from_seed_and_params(1, params);
        let id2 = AssetId::from_seed_and_params(2, params);

        assert_ne!(id1, id2, "Different seeds should produce different IDs");
    }

    #[test]
    fn test_seed_and_params_different_params() {
        let seed = 12345_u64;

        let id1 = AssetId::from_seed_and_params(seed, b"params1");
        let id2 = AssetId::from_seed_and_params(seed, b"params2");

        assert_ne!(id1, id2, "Different params should produce different IDs");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property test: Same content always produces same ID
        #[test]
        fn prop_deterministic(data: Vec<u8>) {
            let id1 = AssetId::from_content(&data);
            let id2 = AssetId::from_content(&data);
            prop_assert_eq!(id1, id2);
        }

        /// Property test: Serialization roundtrip preserves ID
        #[test]
        fn prop_serialization_roundtrip(data: Vec<u8>) {
            let id = AssetId::from_content(&data);
            let serialized = bincode::serialize(&id).unwrap();
            let deserialized: AssetId = bincode::deserialize(&serialized).unwrap();
            prop_assert_eq!(id, deserialized);
        }

        /// Property test: Seed + params is deterministic
        #[test]
        fn prop_seed_params_deterministic(seed: u64, params: Vec<u8>) {
            let id1 = AssetId::from_seed_and_params(seed, &params);
            let id2 = AssetId::from_seed_and_params(seed, &params);
            prop_assert_eq!(id1, id2);
        }

        /// Property test: Display produces valid hex
        #[test]
        fn prop_display_valid_hex(data: Vec<u8>) {
            let id = AssetId::from_content(&data);
            let display_str = format!("{id}");
            prop_assert_eq!(display_str.len(), 64);
            prop_assert!(display_str.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
