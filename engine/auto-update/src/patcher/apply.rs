//! Apply binary patches using bspatch.

use crate::error::UpdateError;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info};

/// Apply a binary patch to a file.
pub fn apply_patch<P: AsRef<Path>>(
    old_file: P,
    patch_file: P,
    new_file: P,
) -> Result<(), UpdateError> {
    let old_file = old_file.as_ref();
    let patch_file = patch_file.as_ref();
    let new_file = new_file.as_ref();

    debug!(
        old_file = %old_file.display(),
        patch_file = %patch_file.display(),
        new_file = %new_file.display(),
        "Applying binary patch"
    );

    // Read old file
    let old_data = std::fs::read(old_file)
        .map_err(|e| UpdateError::ioerror(old_file.display().to_string(), e.to_string()))?;

    // Read and decompress patch file
    let compressed_patch = std::fs::read(patch_file)
        .map_err(|e| UpdateError::ioerror(patch_file.display().to_string(), e.to_string()))?;

    let patch_data = zstd::decode_all(&compressed_patch[..]).map_err(|e| {
        UpdateError::patchfailed(
            patch_file.display().to_string(),
            format!("Failed to decompress patch: {}", e),
        )
    })?;

    // Apply patch using qbsdiff
    let mut new_data = Vec::new();
    qbsdiff::Bspatch::new(&patch_data)
        .map_err(|e| {
            UpdateError::patchfailed(
                patch_file.display().to_string(),
                format!("Failed to parse patch: {}", e),
            )
        })?
        .apply(&old_data, &mut new_data)
        .map_err(|e| {
            UpdateError::patchfailed(
                patch_file.display().to_string(),
                format!("Failed to apply patch: {}", e),
            )
        })?;

    // Write new file
    let mut file = std::fs::File::create(new_file)
        .map_err(|e| UpdateError::ioerror(new_file.display().to_string(), e.to_string()))?;

    file.write_all(&new_data)
        .map_err(|e| UpdateError::ioerror(new_file.display().to_string(), e.to_string()))?;

    info!(
        old_file = %old_file.display(),
        patch_file = %patch_file.display(),
        new_file = %new_file.display(),
        old_size = old_data.len(),
        new_size = new_data.len(),
        patch_size = compressed_patch.len(),
        "Binary patch applied successfully"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patcher::create_patch;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_apply_patch() {
        // Create old file
        let mut old_file = NamedTempFile::new().unwrap();
        old_file.write_all(b"Hello, world!").unwrap();
        old_file.flush().unwrap();

        // Create new file
        let mut new_file_original = NamedTempFile::new().unwrap();
        new_file_original.write_all(b"Hello, Rust!").unwrap();
        new_file_original.flush().unwrap();

        // Create patch
        let patch_file = NamedTempFile::new().unwrap();
        create_patch(old_file.path(), new_file_original.path(), patch_file.path()).unwrap();

        // Apply patch
        let new_file = NamedTempFile::new().unwrap();
        let result = apply_patch(old_file.path(), patch_file.path(), new_file.path());
        assert!(result.is_ok());

        // Verify result
        let result_data = std::fs::read(new_file.path()).unwrap();
        assert_eq!(result_data, b"Hello, Rust!");
    }

    #[test]
    fn test_apply_patch_roundtrip() {
        // Create test data
        let old_data = b"The quick brown fox jumps over the lazy dog";
        let new_data = b"The quick brown cat jumps over the lazy dog";

        let mut old_file = NamedTempFile::new().unwrap();
        old_file.write_all(old_data).unwrap();
        old_file.flush().unwrap();

        let mut new_file_original = NamedTempFile::new().unwrap();
        new_file_original.write_all(new_data).unwrap();
        new_file_original.flush().unwrap();

        let patch_file = NamedTempFile::new().unwrap();
        let new_file = NamedTempFile::new().unwrap();

        // Create and apply patch
        create_patch(old_file.path(), new_file_original.path(), patch_file.path()).unwrap();
        apply_patch(old_file.path(), patch_file.path(), new_file.path()).unwrap();

        // Verify
        let result = std::fs::read(new_file.path()).unwrap();
        assert_eq!(result, new_data);
    }
}
