//! Native filesystem implementation using std::fs.

use super::FileSystemBackend;
use crate::PlatformError;
use std::path::{Path, PathBuf};

/// Native filesystem backend using std::fs.
///
/// This provides basic file I/O operations with path normalization.
pub struct NativeFileSystem;

impl NativeFileSystem {
    /// Create a new native filesystem backend.
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemBackend for NativeFileSystem {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, PlatformError> {
        std::fs::read(path).map_err(|e| {
            PlatformError::filesystemerror(
                "read".to_string(),
                path.display().to_string(),
                e.to_string(),
            )
        })
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), PlatformError> {
        std::fs::write(path, data).map_err(|e| {
            PlatformError::filesystemerror(
                "write".to_string(),
                path.display().to_string(),
                e.to_string(),
            )
        })
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn normalize_path(&self, path: &Path) -> PathBuf {
        // Convert the path to an absolute path and normalize it
        // This handles . and .. components
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::CurDir => {
                    // Skip current directory markers
                }
                std::path::Component::ParentDir => {
                    // Go up one level
                    components.pop();
                }
                _ => {
                    components.push(component);
                }
            }
        }

        components.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_fs_creation() {
        let _fs = NativeFileSystem::new();
    }

    #[test]
    fn test_native_fs_read_write() {
        let fs = NativeFileSystem::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("native_fs_test.txt");

        fs.write_file(&test_file, b"test data").unwrap();
        let content = fs.read_file(&test_file).unwrap();

        assert_eq!(content, b"test data");

        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_native_fs_normalize() {
        let fs = NativeFileSystem::new();

        let path = Path::new("foo/./bar/../baz");
        let normalized = fs.normalize_path(path);

        let normalized_str = normalized.to_string_lossy();
        assert!(normalized_str.ends_with("baz"));
        assert!(!normalized_str.contains(".."));
        assert!(!normalized_str.contains("/./"));
    }
}
