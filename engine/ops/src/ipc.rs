//! IPC surface types: error envelope, command result, action summaries.

use serde::{Deserialize, Serialize};
use engine_core::EngineError;
use crate::error::OpsError;
use crate::template::TemplateState;

/// Monotonically increasing action identifier assigned by CommandProcessor.
pub type ActionId = u64;

/// Serializable error envelope for Tauri IPC responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: u32,
    pub message: String,
}

impl From<OpsError> for IpcError {
    fn from(e: OpsError) -> Self {
        IpcError { code: e.code() as u32, message: e.to_string() }
    }
}

/// Result of a successfully executed TemplateCommand.
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,
}

/// Human-readable summary of a single undo/redo entry — for history display.
#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub action_id: ActionId,
    pub description: String,
}
