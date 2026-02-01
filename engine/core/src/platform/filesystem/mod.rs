//! Filesystem abstraction layer.
//!
//! This module provides cross-platform file I/O with path normalization.
//! Handles platform differences like path separators (Windows `\` vs Unix `/`).

use crate::PlatformError;
use std::path::{Path, PathBuf};

/// Trait for platform-specific filesystem backends.
///
/// This abstracts file I/O operations and ensures paths are
/// normalized correctly for each platform.
pub trait FileSystemBackend: Send + Sync {
    /// Read entire file contents into a byte vector.
    ///
    /// # Errors
    ///
    /// Returns `PlatformError::FileSystemError` if:
    /// - File doesn't exist
    /// - Permission denied
    /// - I/O error during read
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, PlatformError>;

    /// Write byte vector to file, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns `PlatformError::FileSystemError` if:
    /// - Parent directory doesn't exist
    /// - Permission denied
    /// - I/O error during write
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), PlatformError>;

    /// Check if a file exists.
    fn file_exists(&self, path: &Path) -> bool;

    /// Normalize a path for the current platform.
    ///
    /// This converts path separators and resolves `.` and `..` components.
    /// On Windows, converts `/` to `\`. On Unix, keeps `/`.
    fn normalize_path(&self, path: &Path) -> PathBuf;

    /// Read a file as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist, can't be read, or contains invalid UTF-8.
    fn read_to_string(&self, path: &Path) -> Result<String, PlatformError> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|e| {
            PlatformError::filesystemerror(
                "read_to_string".to_string(),
                path.display().to_string(),
                format!("Invalid UTF-8: {}", e),
            )
        })
    }

    /// Write a string to a file.
    ///
    /// # Errors
    ///
    /// Returns error if file can't be written.
    fn write_string(&self, path: &Path, content: &str) -> Result<(), PlatformError> {
        self.write_file(path, content.as_bytes())
    }
}

mod native;

pub use native::NativeFileSystem;

/// Create a filesystem backend for the current platform.
pub fn create_filesystem_backend() -> Box<dyn FileSystemBackend> {
    Box::new(NativeFileSystem::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_creation() {
        let _fs = create_filesystem_backend();
    }

    #[test]
    fn test_read_file() {
        let fs = create_filesystem_backend();

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_read_file.txt");
        std::fs::write(&test_file, b"test content").unwrap();

        let content = fs.read_file(&test_file).unwrap();
        assert_eq!(content, b"test content");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_write_file() {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_write_file.txt");

        fs.write_file(&test_file, b"hello world").unwrap();

        let content = std::fs::read(&test_file).unwrap();
        assert_eq!(content, b"hello world");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_file_exists() {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let existing_file = temp_dir.join("test_exists.txt");
        let non_existing = temp_dir.join("test_not_exists_12345.txt");

        std::fs::write(&existing_file, b"test").unwrap();

        assert!(fs.file_exists(&existing_file));
        assert!(!fs.file_exists(&non_existing));

        // Cleanup
        std::fs::remove_file(&existing_file).ok();
    }

    #[test]
    fn test_read_to_string() {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_string.txt");
        std::fs::write(&test_file, "Hello, 世界!").unwrap();

        let content = fs.read_to_string(&test_file).unwrap();
        assert_eq!(content, "Hello, 世界!");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_write_string() {
        let fs = create_filesystem_backend();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_write_string.txt");

        fs.write_string(&test_file, "こんにちは").unwrap();

        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "こんにちは");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_read_nonexistent_file() {
        let fs = create_filesystem_backend();

        let result = fs.read_file(Path::new("/nonexistent/file/path.txt"));
        assert!(result.is_err());

        if let Err(PlatformError::FileSystemError { operation, .. }) = result {
            assert_eq!(operation, "read");
        } else {
            panic!("Expected FileSystemError");
        }
    }

    #[test]
    fn test_normalize_path() {
        let fs = create_filesystem_backend();

        let path = Path::new("foo/bar/../baz/./qux.txt");
        let normalized = fs.normalize_path(path);

        // Should resolve .. and .
        let normalized_str = normalized.to_string_lossy();

        #[cfg(windows)]
        assert!(
            normalized_str.contains("foo\\baz\\qux.txt")
                || normalized_str.contains("foo/baz/qux.txt")
        );

        #[cfg(unix)]
        assert!(normalized_str.contains("foo/baz/qux.txt"));
    }
}
