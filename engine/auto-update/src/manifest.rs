//! Update manifest parsing and validation.

use crate::error::UpdateError;
use crate::version::Version;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Update manifest describing an available update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Version of this update
    pub version: Version,
    /// Release date
    pub release_date: DateTime<Utc>,
    /// Human-readable changelog
    pub changelog: String,
    /// List of files in this update
    pub files: Vec<FileInfo>,
    /// Signature for manifest verification
    pub signature: Option<String>,
    /// Update channel (stable, beta, etc.)
    pub channel: String,
    /// Minimum compatible version
    pub min_version: Option<Version>,
}

/// Information about a file in an update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Relative path to the file
    pub path: String,
    /// SHA-256 hash of the file
    pub sha256: String,
    /// File size in bytes
    pub size: u64,
    /// URL to download the file
    pub url: String,
    /// Optional patch file for differential update
    pub patch: Option<PatchInfo>,
}

/// Information about a patch file for differential updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInfo {
    /// URL to download the patch
    pub url: String,
    /// SHA-256 hash of the patch file
    pub sha256: String,
    /// Size of the patch file in bytes
    pub size: u64,
    /// Version this patch applies from
    pub from_version: Version,
}

/// Manifest metadata for quick version checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestIndex {
    /// Latest version per channel
    pub channels: HashMap<String, Version>,
    /// URL template for fetching full manifests
    pub manifest_url_template: String,
}

impl UpdateManifest {
    /// Validate the manifest structure and data.
    pub fn validate(&self) -> Result<(), UpdateError> {
        if self.files.is_empty() {
            return Err(UpdateError::invalidmanifest("Manifest contains no files".to_string()));
        }

        for file in &self.files {
            if file.path.is_empty() {
                return Err(UpdateError::invalidmanifest("File path cannot be empty".to_string()));
            }

            if file.sha256.len() != 64 {
                return Err(UpdateError::invalidmanifest(format!(
                    "Invalid SHA-256 hash length for file: {}",
                    file.path
                )));
            }

            if file.url.is_empty() {
                return Err(UpdateError::invalidmanifest(format!(
                    "Missing URL for file: {}",
                    file.path
                )));
            }
        }

        if self.channel.is_empty() {
            return Err(UpdateError::invalidmanifest("Channel name cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Calculate total download size (preferring patches if available).
    pub fn total_download_size(&self, current_version: Option<&Version>) -> u64 {
        self.files
            .iter()
            .map(|file| {
                if let (Some(current), Some(patch)) = (current_version, &file.patch) {
                    if patch.from_version == *current {
                        return patch.size;
                    }
                }
                file.size
            })
            .sum()
    }

    /// Check if this update is compatible with a given version.
    pub fn is_compatible_with(&self, version: &Version) -> bool {
        if let Some(min_version) = &self.min_version {
            version >= min_version
        } else {
            true
        }
    }
}

impl ManifestIndex {
    /// Get the latest version for a channel.
    pub fn get_latest_version(&self, channel: &str) -> Result<&Version, UpdateError> {
        self.channels
            .get(channel)
            .ok_or_else(|| UpdateError::channelnotfound(channel.to_string()))
    }

    /// Get the manifest URL for a specific version and channel.
    pub fn get_manifest_url(&self, channel: &str, version: &Version) -> String {
        self.manifest_url_template
            .replace("{channel}", channel)
            .replace("{version}", &version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> UpdateManifest {
        UpdateManifest {
            version: Version::new(1, 0, 1),
            release_date: Utc::now(),
            changelog: "Bug fixes".to_string(),
            files: vec![FileInfo {
                path: "game.exe".to_string(),
                sha256: "a".repeat(64),
                size: 1024,
                url: "https://example.com/game.exe".to_string(),
                patch: None,
            }],
            signature: None,
            channel: "stable".to_string(),
            min_version: None,
        }
    }

    #[test]
    fn test_manifest_validation() {
        let manifest = create_test_manifest();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_validation_no_files() {
        let mut manifest = create_test_manifest();
        manifest.files.clear();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_manifest_validation_invalid_hash() {
        let mut manifest = create_test_manifest();
        manifest.files[0].sha256 = "short".to_string();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_total_download_size_no_patches() {
        let manifest = create_test_manifest();
        assert_eq!(manifest.total_download_size(None), 1024);
    }

    #[test]
    fn test_total_download_size_with_patch() {
        let mut manifest = create_test_manifest();
        manifest.files[0].patch = Some(PatchInfo {
            url: "https://example.com/patch".to_string(),
            sha256: "b".repeat(64),
            size: 256,
            from_version: Version::new(1, 0, 0),
        });

        let current = Version::new(1, 0, 0);
        assert_eq!(manifest.total_download_size(Some(&current)), 256);

        let other = Version::new(0, 9, 0);
        assert_eq!(manifest.total_download_size(Some(&other)), 1024);
    }

    #[test]
    fn test_manifest_compatibility() {
        let mut manifest = create_test_manifest();
        manifest.min_version = Some(Version::new(1, 0, 0));

        assert!(manifest.is_compatible_with(&Version::new(1, 0, 0)));
        assert!(manifest.is_compatible_with(&Version::new(1, 0, 5)));
        assert!(!manifest.is_compatible_with(&Version::new(0, 9, 0)));
    }

    #[test]
    fn test_manifest_index() {
        let mut channels = HashMap::new();
        channels.insert("stable".to_string(), Version::new(1, 0, 0));
        channels.insert("beta".to_string(), Version::new(1, 1, 0));

        let index = ManifestIndex {
            channels,
            manifest_url_template: "https://cdn.example.com/{channel}/{version}/manifest.json"
                .to_string(),
        };

        assert_eq!(index.get_latest_version("stable").unwrap(), &Version::new(1, 0, 0));
        assert_eq!(index.get_latest_version("beta").unwrap(), &Version::new(1, 1, 0));
        assert!(index.get_latest_version("alpha").is_err());

        let url = index.get_manifest_url("stable", &Version::new(1, 0, 0));
        assert_eq!(url, "https://cdn.example.com/stable/1.0.0/manifest.json");
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = create_test_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: UpdateManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest.version, deserialized.version);
        assert_eq!(manifest.files.len(), deserialized.files.len());
    }
}
