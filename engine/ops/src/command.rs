//! Typed command enum — the single input to CommandProcessor::execute().

use serde::{Deserialize, Serialize};
use crate::undo::EntityId;

/// All mutations that go through the undo/redo system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateCommand {
    CreateEntity { name: Option<String> },
    DeleteEntity { id: EntityId },
    RenameEntity { id: EntityId, name: Option<String> },
    DuplicateEntity { id: EntityId },
    SetComponent { id: EntityId, type_name: String, data: serde_json::Value },
    AddComponent { id: EntityId, type_name: String, data: serde_json::Value },
    RemoveComponent { id: EntityId, type_name: String },
}
