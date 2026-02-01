//! Directory scanning utilities for build scripts

use std::fs;
use std::path::Path;

/// Scan a directory recursively and apply a function to each .rs file
///
/// # Arguments
/// * `dir` - The directory to scan
/// * `callback` - Function called for each Rust file with (path, content)
pub fn scan_directory<F>(dir: &Path, callback: &mut F)
where
    F: FnMut(&Path, &str),
{
    if !dir.exists() {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();

        if path.is_dir() {
            scan_directory(&path, callback);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                callback(&path, &content);
            }
        }
    }
}

/// Check if a file is a test file (allowed to have prints and other test-only code)
pub fn is_test_file(path: &Path) -> bool {
    // Check if in tests/ directory
    if path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }

    // Check if in benches/ directory
    if path.components().any(|c| c.as_os_str() == "benches") {
        return true;
    }

    // Check if in examples/ directory (examples can use println)
    if path.components().any(|c| c.as_os_str() == "examples") {
        return true;
    }

    // Check if filename ends with _test.rs or test_.rs
    if let Some(filename) = path.file_name() {
        let name = filename.to_string_lossy();
        if name.ends_with("_test.rs") || name.starts_with("test_") {
            return true;
        }
    }

    // Check if file contains #[cfg(test)] module
    if let Ok(content) = fs::read_to_string(path) {
        if content.contains("#[cfg(test)]") {
            // File has test module, but we still check non-test code
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file(&PathBuf::from("tests/integration_test.rs")));
        assert!(is_test_file(&PathBuf::from("benches/benchmark.rs")));
        assert!(is_test_file(&PathBuf::from("examples/demo.rs")));
        assert!(is_test_file(&PathBuf::from("src/ecs/entity_test.rs")));
        assert!(is_test_file(&PathBuf::from("src/test_helpers.rs")));

        assert!(!is_test_file(&PathBuf::from("src/lib.rs")));
        assert!(!is_test_file(&PathBuf::from("src/ecs/entity.rs")));
    }
}
