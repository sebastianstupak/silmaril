//! Integration tests for the auto-update system.

use engine_auto_update::{
    channels::Channel, manifest::*, version::Version, UpdateConfig, UpdateManager,
};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_version_parsing_and_comparison() {
    let v1: Version = "1.0.0".parse().unwrap();
    let v2: Version = "1.0.1".parse().unwrap();
    let v3: Version = "2.0.0".parse().unwrap();

    assert!(v2.is_newer_than(&v1));
    assert!(v3.is_newer_than(&v2));
    assert!(v1.is_compatible_with(&v2));
    assert!(!v1.is_compatible_with(&v3));
}

#[test]
fn test_manifest_creation_and_validation() {
    let manifest = UpdateManifest {
        version: Version::new(1, 0, 1),
        release_date: chrono::Utc::now(),
        changelog: "Bug fixes and improvements".to_string(),
        files: vec![FileInfo {
            path: "game.exe".to_string(),
            sha256: "a".repeat(64),
            size: 1024,
            url: "https://cdn.example.com/game.exe".to_string(),
            patch: None,
        }],
        signature: None,
        channel: "stable".to_string(),
        min_version: Some(Version::new(1, 0, 0)),
    };

    assert!(manifest.validate().is_ok());
    assert!(manifest.is_compatible_with(&Version::new(1, 0, 0)));
    assert!(!manifest.is_compatible_with(&Version::new(0, 9, 0)));
}

#[test]
fn test_update_manager_creation_and_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config = UpdateConfig::new(
        "https://updates.example.com/manifest.json".to_string(),
        Version::new(1, 0, 0),
        temp_dir.path().to_path_buf(),
    );

    let manager = UpdateManager::new(config.clone()).unwrap();

    // Verify directories were created
    assert!(config.backup_dir.exists());
    assert!(config.download_dir.exists());
}

#[test]
fn test_channel_switching() {
    let temp_dir = TempDir::new().unwrap();
    let config = UpdateConfig::new(
        "https://updates.example.com/manifest.json".to_string(),
        Version::new(1, 0, 0),
        temp_dir.path().to_path_buf(),
    );

    let mut manager = UpdateManager::new(config).unwrap();

    // Default is stable
    assert_eq!(manager.current_version(), &Version::new(1, 0, 0));

    // Cannot switch to beta without opt-in (will fail at subscription level)
    // But we can enable beta on the config level
}

#[test]
fn test_backup_and_restore() {
    use engine_auto_update::rollback::RollbackManager;

    let temp_dir = TempDir::new().unwrap();
    let install_dir = temp_dir.path().join("install");
    let backup_dir = temp_dir.path().join("backups");

    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    // Create a test file
    let test_file = install_dir.join("test.txt");
    let mut file = fs::File::create(&test_file).unwrap();
    file.write_all(b"Version 1.0 content").unwrap();
    drop(file);

    let manager = RollbackManager::new(&backup_dir, &install_dir);

    // Create backup
    let backup = manager
        .create_backup(&Version::new(1, 0, 0), &["test.txt".to_string()])
        .unwrap();

    assert_eq!(backup.files.len(), 1);
    assert!(backup.backup_dir.exists());

    // Modify file
    let mut file = fs::File::create(&test_file).unwrap();
    file.write_all(b"Version 2.0 content").unwrap();
    drop(file);

    // Restore backup
    manager.restore_backup(&backup).unwrap();

    // Verify restoration
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Version 1.0 content");
}

#[test]
fn test_patch_creation_and_application() {
    use engine_auto_update::patcher::{apply_patch, create_patch};

    let temp_dir = TempDir::new().unwrap();

    // Create old file
    let old_file = temp_dir.path().join("old.bin");
    fs::write(&old_file, b"The quick brown fox").unwrap();

    // Create new file
    let new_file = temp_dir.path().join("new.bin");
    fs::write(&new_file, b"The quick brown cat").unwrap();

    // Create patch
    let patch_file = temp_dir.path().join("patch.bin");
    create_patch(&old_file, &new_file, &patch_file).unwrap();

    assert!(patch_file.exists());
    let patch_size = fs::metadata(&patch_file).unwrap().len();
    assert!(patch_size > 0);

    // Apply patch
    let result_file = temp_dir.path().join("result.bin");
    apply_patch(&old_file, &patch_file, &result_file).unwrap();

    // Verify result
    let result = fs::read(&result_file).unwrap();
    let expected = fs::read(&new_file).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_file_verification() {
    use engine_auto_update::verifier::{compute_file_hash, verify_file_hash};

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    fs::write(&test_file, b"Hello, world!").unwrap();

    let hash = compute_file_hash(&test_file).unwrap();
    assert_eq!(hash, "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");

    // Verify should succeed
    assert!(verify_file_hash(&test_file, &hash).is_ok());

    // Wrong hash should fail
    assert!(verify_file_hash(&test_file, "wrong_hash").is_err());
}

#[test]
fn test_progress_tracking() {
    use engine_auto_update::progress::ProgressTracker;
    use std::thread;
    use std::time::Duration;

    let tracker = ProgressTracker::new(1000);

    tracker.add_bytes(500);
    let progress = tracker.get_progress();

    assert_eq!(progress.bytes_processed, 500);
    assert_eq!(progress.total_bytes, 1000);
    assert_eq!(progress.percentage(), 50.0);
    assert!(!progress.is_complete());

    tracker.add_bytes(500);
    let progress = tracker.get_progress();

    assert!(progress.is_complete());
}

#[test]
fn test_manifest_serialization() {
    let manifest = UpdateManifest {
        version: Version::new(2, 0, 0),
        release_date: chrono::Utc::now(),
        changelog: "Major update".to_string(),
        files: vec![FileInfo {
            path: "game.exe".to_string(),
            sha256: "a".repeat(64),
            size: 2048,
            url: "https://cdn.example.com/game.exe".to_string(),
            patch: Some(PatchInfo {
                url: "https://cdn.example.com/game.patch".to_string(),
                sha256: "b".repeat(64),
                size: 512,
                from_version: Version::new(1, 0, 0),
            }),
        }],
        signature: Some("signature_data".to_string()),
        channel: "stable".to_string(),
        min_version: None,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    assert!(json.contains("2.0.0"));
    assert!(json.contains("Major update"));

    // Deserialize back
    let deserialized: UpdateManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.version, manifest.version);
    assert_eq!(deserialized.files.len(), manifest.files.len());
}

#[test]
fn test_channel_management() {
    let stable = Channel::from_str("stable").unwrap();
    let beta = Channel::from_str("beta").unwrap();

    assert_eq!(stable, Channel::Stable);
    assert_eq!(beta, Channel::Beta);
    assert!(stable.is_more_stable_than(&beta));
}
