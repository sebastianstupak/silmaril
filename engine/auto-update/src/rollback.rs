//! Rollback system for reverting failed updates.

use crate::error::UpdateError;
use crate::version::Version;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Maximum number of backups to keep.
const MAX_BACKUPS: usize = 2;

/// Information about a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Version of the backed up installation
    pub version: Version,
    /// When the backup was created
    pub created_at: DateTime<Utc>,
    /// Directory containing the backup
    pub backup_dir: PathBuf,
    /// List of backed up files
    pub files: Vec<String>,
}

/// Version history tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// Current version
    pub current_version: Version,
    /// Previous versions with backup information
    pub backups: Vec<BackupInfo>,
}

impl VersionHistory {
    /// Create a new version history.
    pub fn new(current_version: Version) -> Self {
        Self { current_version, backups: Vec::new() }
    }

    /// Add a new backup to the history.
    pub fn add_backup(&mut self, backup: BackupInfo) {
        self.backups.push(backup);

        // Keep only the most recent backups
        if self.backups.len() > MAX_BACKUPS {
            self.backups.drain(0..self.backups.len() - MAX_BACKUPS);
        }
    }

    /// Get the most recent backup.
    pub fn get_latest_backup(&self) -> Option<&BackupInfo> {
        self.backups.last()
    }

    /// Get a backup for a specific version.
    pub fn get_backup(&self, version: &Version) -> Option<&BackupInfo> {
        self.backups.iter().find(|b| b.version == *version)
    }
}

/// Rollback manager.
pub struct RollbackManager {
    backup_root: PathBuf,
    install_root: PathBuf,
}

impl RollbackManager {
    /// Create a new rollback manager.
    pub fn new<P: AsRef<Path>>(backup_root: P, install_root: P) -> Self {
        Self {
            backup_root: backup_root.as_ref().to_path_buf(),
            install_root: install_root.as_ref().to_path_buf(),
        }
    }

    /// Create a backup of the current installation.
    pub fn create_backup(
        &self,
        version: &Version,
        files_to_backup: &[String],
    ) -> Result<BackupInfo, UpdateError> {
        let backup_dir = self.backup_root.join(format!("backup_{}", version));

        info!(
            version = %version,
            backup_dir = %backup_dir.display(),
            file_count = files_to_backup.len(),
            "Creating backup"
        );

        // Create backup directory
        fs::create_dir_all(&backup_dir)
            .map_err(|e| UpdateError::ioerror(backup_dir.display().to_string(), e.to_string()))?;

        let mut backed_up_files = Vec::new();

        // Copy each file to backup
        for file_path in files_to_backup {
            let source = self.install_root.join(file_path);
            if !source.exists() {
                debug!(file = %file_path, "File does not exist, skipping backup");
                continue;
            }

            let dest = backup_dir.join(file_path);

            // Create parent directories
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    UpdateError::ioerror(parent.display().to_string(), e.to_string())
                })?;
            }

            // Copy file
            fs::copy(&source, &dest)
                .map_err(|e| UpdateError::ioerror(source.display().to_string(), e.to_string()))?;

            backed_up_files.push(file_path.clone());
        }

        let backup_info = BackupInfo {
            version: *version,
            created_at: Utc::now(),
            backup_dir,
            files: backed_up_files,
        };

        info!(
            version = %version,
            file_count = backup_info.files.len(),
            "Backup created successfully"
        );

        Ok(backup_info)
    }

    /// Restore from a backup.
    pub fn restore_backup(&self, backup: &BackupInfo) -> Result<(), UpdateError> {
        info!(
            version = %backup.version,
            backup_dir = %backup.backup_dir.display(),
            file_count = backup.files.len(),
            "Restoring from backup"
        );

        if !backup.backup_dir.exists() {
            return Err(UpdateError::rollbackfailed(format!(
                "Backup directory not found: {}",
                backup.backup_dir.display()
            )));
        }

        let mut restored_count = 0;

        // Restore each file
        for file_path in &backup.files {
            let source = backup.backup_dir.join(file_path);
            let dest = self.install_root.join(file_path);

            if !source.exists() {
                warn!(file = %file_path, "Backup file not found, skipping");
                continue;
            }

            // Create parent directories
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    UpdateError::ioerror(parent.display().to_string(), e.to_string())
                })?;
            }

            // Copy file
            fs::copy(&source, &dest).map_err(|e| {
                UpdateError::rollbackfailed(format!("Failed to restore {}: {}", file_path, e))
            })?;

            restored_count += 1;
        }

        info!(
            version = %backup.version,
            restored_count = restored_count,
            "Backup restored successfully"
        );

        Ok(())
    }

    /// Delete old backups to free disk space.
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<(), UpdateError> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_root)
            .map_err(|e| {
                UpdateError::ioerror(self.backup_root.display().to_string(), e.to_string())
            })?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("backup_"))
            .collect();

        if backups.len() <= keep_count {
            return Ok(());
        }

        // Sort by modification time
        backups.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Remove oldest backups
        let to_remove = backups.len() - keep_count;
        for entry in backups.iter().take(to_remove) {
            let path = entry.path();
            info!(path = %path.display(), "Removing old backup");
            fs::remove_dir_all(&path)
                .map_err(|e| UpdateError::ioerror(path.display().to_string(), e.to_string()))?;
        }

        Ok(())
    }

    /// Get disk space used by backups.
    pub fn get_backup_size(&self) -> Result<u64, UpdateError> {
        let mut total_size = 0u64;

        if !self.backup_root.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.backup_root).map_err(|e| {
            UpdateError::ioerror(self.backup_root.display().to_string(), e.to_string())
        })? {
            let entry = entry.map_err(|e| {
                UpdateError::ioerror(self.backup_root.display().to_string(), e.to_string())
            })?;

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                total_size += calculate_dir_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }
}

/// Calculate the total size of a directory.
fn calculate_dir_size(dir: &Path) -> Result<u64, UpdateError> {
    let mut size = 0u64;

    for entry in fs::read_dir(dir).map_err(|e: std::io::Error| {
        UpdateError::ioerror(dir.display().to_string(), e.to_string())
    })? {
        let entry = entry.map_err(|e: std::io::Error| {
            UpdateError::ioerror(dir.display().to_string(), e.to_string())
        })?;
        let metadata = entry.metadata().map_err(|e: std::io::Error| {
            UpdateError::ioerror(entry.path().display().to_string(), e.to_string())
        })?;

        if metadata.is_dir() {
            size += calculate_dir_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_version_history() {
        let mut history = VersionHistory::new(Version::new(1, 0, 0));
        assert_eq!(history.current_version, Version::new(1, 0, 0));
        assert!(history.backups.is_empty());

        let backup = BackupInfo {
            version: Version::new(0, 9, 0),
            created_at: Utc::now(),
            backup_dir: PathBuf::from("/backup"),
            files: vec!["file1.txt".to_string()],
        };

        history.add_backup(backup.clone());
        assert_eq!(history.backups.len(), 1);
        assert_eq!(history.get_latest_backup().unwrap().version, backup.version);
    }

    #[test]
    fn test_version_history_max_backups() {
        let mut history = VersionHistory::new(Version::new(1, 0, 0));

        for i in 0..5 {
            let backup = BackupInfo {
                version: Version::new(0, i as u32, 0),
                created_at: Utc::now(),
                backup_dir: PathBuf::from(format!("/backup{}", i)),
                files: vec![],
            };
            history.add_backup(backup);
        }

        // Should keep only MAX_BACKUPS
        assert_eq!(history.backups.len(), MAX_BACKUPS);
        assert_eq!(history.get_latest_backup().unwrap().version, Version::new(0, 4, 0));
    }

    #[test]
    fn test_create_and_restore_backup() {
        let temp_dir = TempDir::new().unwrap();
        let install_dir = temp_dir.path().join("install");
        let backup_dir = temp_dir.path().join("backups");

        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir_all(&backup_dir).unwrap();

        // Create a test file
        let test_file = install_dir.join("test.txt");
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"Original content").unwrap();

        let manager = RollbackManager::new(&backup_dir, &install_dir);

        // Create backup
        let backup = manager
            .create_backup(&Version::new(1, 0, 0), &["test.txt".to_string()])
            .unwrap();

        assert_eq!(backup.files.len(), 1);
        assert!(backup.backup_dir.exists());

        // Modify original file
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"Modified content").unwrap();

        // Restore backup
        manager.restore_backup(&backup).unwrap();

        // Verify restoration
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Original content");
    }

    #[test]
    fn test_backup_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let install_dir = temp_dir.path().join("install");
        let backup_dir = temp_dir.path().join("backups");

        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir_all(&backup_dir).unwrap();

        let manager = RollbackManager::new(&backup_dir, &install_dir);

        // Initially no backups
        assert_eq!(manager.get_backup_size().unwrap(), 0);
    }
}
