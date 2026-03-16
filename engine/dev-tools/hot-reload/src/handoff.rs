//! State handoff: serialize/restore ECS world across a dev restart.
//!
//! `StateHandoff` saves the ECS world to a YAML file in `.silmaril/dev-state.yaml`
//! before a hot-restart, then restores it on the next startup. This preserves
//! entity state across the rebuild/restart cycle so the developer can continue
//! where they left off.

use crate::error::DevError;
use engine_core::ecs::World;
use engine_core::serialization::{Format, Serializable, WorldState};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Outcome of a [`StateHandoff::restore`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreResult {
    /// World state was successfully restored from the handoff file.
    Restored,
    /// No handoff file found (or it was corrupt): start with a clean world.
    CleanStart,
}

/// Saves and restores ECS world state across dev-server restarts.
///
/// The state is written to `<project_root>/.silmaril/dev-state.yaml`. That
/// file is consumed (deleted) on a successful restore so subsequent cold starts
/// always begin cleanly.
pub struct StateHandoff {
    silmaril_dir: PathBuf,
}

impl StateHandoff {
    /// Create a new `StateHandoff` rooted at `project_root`.
    ///
    /// The state file will live at `<project_root>/.silmaril/dev-state.yaml`.
    pub fn new(project_root: &Path) -> Self {
        Self { silmaril_dir: project_root.join(".silmaril") }
    }

    fn state_path(&self) -> PathBuf {
        self.silmaril_dir.join("dev-state.yaml")
    }

    /// Returns `true` if a handoff file exists on disk.
    pub fn exists(&self) -> bool {
        self.state_path().exists()
    }

    /// Serialize the world to the handoff file.
    ///
    /// This is a synchronous, blocking operation. Call it from async contexts
    /// via `tokio::task::spawn_blocking`.
    pub fn save(&self, world: &World) -> Result<(), DevError> {
        use std::io::Write as _;

        std::fs::create_dir_all(&self.silmaril_dir).map_err(|e| DevError::SerializeFailed {
            reason: format!("could not create .silmaril/: {e}"),
        })?;

        let snapshot = WorldState::snapshot(world);
        let yaml_bytes =
            Serializable::serialize(&snapshot, Format::Yaml).map_err(|e| DevError::SerializeFailed {
                reason: format!("yaml serialization failed: {e}"),
            })?;

        let path = self.state_path();
        let mut file = std::fs::File::create(&path).map_err(|e| DevError::SerializeFailed {
            reason: format!("could not create state file: {e}"),
        })?;
        file.write_all(&yaml_bytes).map_err(|e| DevError::SerializeFailed {
            reason: format!("write failed: {e}"),
        })?;
        file.flush().map_err(|e| DevError::SerializeFailed {
            reason: format!("flush failed: {e}"),
        })?;
        file.sync_all().map_err(|e| DevError::SerializeFailed {
            reason: format!("sync_all failed: {e}"),
        })?;

        info!(path = %path.display(), entity_count = snapshot.metadata.entity_count, "dev state saved");
        Ok(())
    }

    /// Restore the world from the handoff file.
    ///
    /// Returns [`RestoreResult::Restored`] when the world was populated from the
    /// file. Returns [`RestoreResult::CleanStart`] when no file exists or the file
    /// is corrupt (the corrupt file is deleted so the next call also gets a clean
    /// start). The world is **not** modified on a `CleanStart`.
    pub fn restore(&self, world: &mut World) -> Result<RestoreResult, DevError> {
        let path = self.state_path();

        if !path.exists() {
            return Ok(RestoreResult::CleanStart);
        }

        let raw = std::fs::read(&path).map_err(|e| DevError::RestoreFailed {
            reason: format!("could not read state file: {e}"),
        })?;

        match <WorldState as Serializable>::deserialize(&raw, Format::Yaml) {
            Ok(state) => {
                state.restore(world);
                let _ = std::fs::remove_file(&path);
                info!(
                    path = %path.display(),
                    entity_count = state.metadata.entity_count,
                    "dev state restored"
                );
                Ok(RestoreResult::Restored)
            }
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = ?e,
                    "dev state file corrupt — starting clean"
                );
                let _ = std::fs::remove_file(&path);
                Ok(RestoreResult::CleanStart)
            }
        }
    }
}
