//! Create binary patches using bsdiff.

use crate::error::UpdateError;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info};

/// Create a binary patch from old file to new file.
pub fn create_patch<P: AsRef<Path>>(
    old_file: P,
    new_file: P,
    patch_file: P,
) -> Result<(), UpdateError> {
    let old_file = old_file.as_ref();
    let new_file = new_file.as_ref();
    let patch_file = patch_file.as_ref();

    debug!(
        old_file = %old_file.display(),
        new_file = %new_file.display(),
        patch_file = %patch_file.display(),
        "Creating binary patch"
    );

    // Read old and new files
    let old_data = std::fs::read(old_file)
        .map_err(|e| UpdateError::ioerror(old_file.display().to_string(), e.to_string()))?;

    let new_data = std::fs::read(new_file)
        .map_err(|e| UpdateError::ioerror(new_file.display().to_string(), e.to_string()))?;

    // Create patch using qbsdiff
    let mut patch_data = Vec::new();
    qbsdiff::Bsdiff::new(&old_data, &new_data)
        .compare(std::io::Cursor::new(&mut patch_data))
        .map_err(|e| {
            UpdateError::patchfailed(
                patch_file.display().to_string(),
                format!("Failed to create patch: {}", e),
            )
        })?;

    // Compress patch with zstd
    let compressed = zstd::encode_all(&patch_data[..], 3).map_err(|e| {
        UpdateError::patchfailed(
            patch_file.display().to_string(),
            format!("Failed to compress patch: {}", e),
        )
    })?;

    // Write patch file
    let mut file = std::fs::File::create(patch_file)
        .map_err(|e| UpdateError::ioerror(patch_file.display().to_string(), e.to_string()))?;

    file.write_all(&compressed)
        .map_err(|e| UpdateError::ioerror(patch_file.display().to_string(), e.to_string()))?;

    let compression_ratio = if !patch_data.is_empty() {
        compressed.len() as f64 / patch_data.len() as f64
    } else {
        1.0
    };

    info!(
        old_file = %old_file.display(),
        new_file = %new_file.display(),
        patch_file = %patch_file.display(),
        old_size = old_data.len(),
        new_size = new_data.len(),
        patch_size = compressed.len(),
        compression_ratio = %format!("{:.2}%", compression_ratio * 100.0),
        "Binary patch created"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_patch() {
        // Create old file
        let mut old_file = NamedTempFile::new().unwrap();
        old_file.write_all(b"Hello, world!").unwrap();
        old_file.flush().unwrap();

        // Create new file
        let mut new_file = NamedTempFile::new().unwrap();
        new_file.write_all(b"Hello, Rust!").unwrap();
        new_file.flush().unwrap();

        // Create patch
        let patch_file = NamedTempFile::new().unwrap();

        let result = create_patch(old_file.path(), new_file.path(), patch_file.path());
        assert!(result.is_ok());

        // Verify patch file exists and has content
        let patch_data = std::fs::read(patch_file.path()).unwrap();
        assert!(!patch_data.is_empty());
    }

    #[test]
    fn test_create_patch_identical_files() {
        let mut old_file = NamedTempFile::new().unwrap();
        old_file.write_all(b"Same content").unwrap();
        old_file.flush().unwrap();

        let mut new_file = NamedTempFile::new().unwrap();
        new_file.write_all(b"Same content").unwrap();
        new_file.flush().unwrap();

        let patch_file = NamedTempFile::new().unwrap();

        let result = create_patch(old_file.path(), new_file.path(), patch_file.path());
        assert!(result.is_ok());
    }
}
