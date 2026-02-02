//! Version management and comparison.

use crate::error::UpdateError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Semantic version (major.minor.patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl Version {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Check if this version is compatible with another version.
    ///
    /// Versions are compatible if they have the same major version.
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major
    }

    /// Check if this version is newer than another version.
    pub fn is_newer_than(&self, other: &Version) -> bool {
        self > other
    }
}

impl FromStr for Version {
    type Err = UpdateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(UpdateError::invalidversion(s.to_string()));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| UpdateError::invalidversion(s.to_string()))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| UpdateError::invalidversion(s.to_string()))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| UpdateError::invalidversion(s.to_string()))?;

        Ok(Version::new(major, minor, patch))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = "1.2.3".parse::<Version>().unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parsing_invalid() {
        assert!("1.2".parse::<Version>().is_err());
        assert!("1.2.3.4".parse::<Version>().is_err());
        assert!("a.b.c".parse::<Version>().is_err());
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(format!("{}", v), "1.2.3");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(1, 1, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 5, 3);
        let v3 = Version::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn test_version_newer_than() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);

        assert!(v2.is_newer_than(&v1));
        assert!(!v1.is_newer_than(&v2));
    }
}
