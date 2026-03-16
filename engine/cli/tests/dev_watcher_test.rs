// Tests for FileWatcher change classification
use std::path::PathBuf;

#[test]
fn test_classify_rs_file_in_shared_returns_code() {
    use silm::commands::dev::watcher::{classify_path, ChangeKind};
    let path = PathBuf::from("shared/src/lib.rs");
    assert!(matches!(classify_path(&path), Some(ChangeKind::Code { .. })));
}

#[test]
fn test_classify_png_in_assets_returns_asset() {
    use silm::commands::dev::watcher::{classify_path, ChangeKind};
    let path = PathBuf::from("assets/textures/grass.png");
    assert!(matches!(classify_path(&path), Some(ChangeKind::Asset { .. })));
}

#[test]
fn test_classify_ron_in_config_returns_config() {
    use silm::commands::dev::watcher::{classify_path, ChangeKind};
    let path = PathBuf::from("config/server.ron");
    assert!(matches!(classify_path(&path), Some(ChangeKind::Config { .. })));
}

#[test]
fn test_classify_unknown_extension_returns_none() {
    use silm::commands::dev::watcher::classify_path;
    let path = PathBuf::from("README.md");
    assert!(classify_path(&path).is_none());
}
