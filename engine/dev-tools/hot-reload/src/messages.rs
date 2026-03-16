//! Protocol messages for the silm dev ↔ DevReloadServer TCP channel.
//!
//! Wire format: newline-delimited JSON (`serde_json::to_string` + `\n`).

use serde::{Deserialize, Serialize};

/// Messages sent between `silm dev` and `DevReloadServer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReloadMessage {
    /// Reload the asset at the given project-relative path.
    ReloadAsset { path: String },
    /// Re-read the config file at the given project-relative path.
    ReloadConfig { path: String },
    /// Serialize the ECS world to `.silmaril/dev-state.yaml` and ack when done.
    SerializeState,
    /// Acknowledgement — `SerializeState` is complete.
    Ack,
}
